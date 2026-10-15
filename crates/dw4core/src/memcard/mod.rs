//! Save containers: PS2 cards, GameCube raw cards, GCI files and bare savedata.
//!
//! `ps2-memcard` 0.2.2 was evaluated and rejected (spec 6.1): it sizes a
//! directory from the `.` entry's `length`, which is 0 on this card, where the
//! working reference bounds by the *parent* entry's length. Its `CardBuilder`
//! also rebuilds whole cards rather than writing in place.
//!
//! Both card filesystems are implemented natively. GCI files use a wrapper
//! that preserves the GameCube file header/banner around the serialized save.

mod ecc;
mod entry;
mod fat;
mod format;
mod gamecube;
mod gci;
mod geometry;
mod ps2;
mod raw;

use std::path::Path;

pub use ecc::{ECC_CHUNK, ECC_CHUNK_BYTES, ecc_chunk, page_spare};
pub use entry::{
    ENTRY_SIZE, Entry, MODE_DIR, MODE_DIR_ENTRY, MODE_EXISTS, MODE_FILE, MODE_FILE_ENTRY,
    MODE_HIDDEN, MODE_ROOT_DOTDOT, MODE_SAVE_DIR_DOTDOT,
};
pub use fat::{ALLOCATED_BIT, CHAIN_END, FatTable, UNALLOCATED};
pub use format::{format_card, format_card_for_region};
pub use gamecube::{GC_CARD_BLOCK, GameCubeMemcard, is_gamecube_card};
pub use gci::{GCI_BANNER_SIZE, GCI_HEADER_SIZE, GCI_MIN_SIZE, GCI_SAVE_OFFSET};
pub use gci::{is_gci, read_save as read_gci_save, write_save as write_gci_save};
pub use geometry::{CardKind, Geometry, SB_MAGIC, SB_SIZE, Superblock};
pub use ps2::{LocatedFile, Ps2Memcard};
pub use raw::RawFile;

/// The directory holding the save, inside the card.
pub const SAVE_DIR: &str = "BASLUS-20836savedata";

/// The save file's name inside that directory.
pub const SAVE_FILE: &str = "BASLUS-20836savedata";

/// A place a save can live: a card image or a bare file.
///
/// Backends differ in where the bytes go. Callers that do
/// not care which they have use [`load_save`] and [`render_container`].
pub trait CardBackend {
    /// Open the container at `path`.
    ///
    /// # Errors
    /// Implementation-defined; see each implementation.
    fn open(path: &Path) -> crate::Result<Self>
    where
        Self: Sized;

    /// Read `dir`/`file`.
    ///
    /// # Errors
    /// [`crate::Error::SaveNotFound`] if absent.
    fn read_save(&mut self, dir: &str, file: &str) -> crate::Result<Vec<u8>>;

    /// Write `data` to `dir`/`file`.
    ///
    /// # Errors
    /// Implementation-defined.
    fn write_save(&mut self, dir: &str, file: &str, data: &[u8]) -> crate::Result<()>;
}

/// Whether `bytes` look like a PS2 memory-card image.
///
/// The same test the Python editor's `is_memcard` makes: the superblock magic at
/// the very start.
#[must_use]
pub fn is_memcard(bytes: &[u8]) -> bool {
    bytes.len() >= SB_SIZE && &bytes[..28] == SB_MAGIC
}

/// Whether `bytes` are a bare 81,920-byte save.
#[must_use]
pub fn is_raw_save(bytes: &[u8]) -> bool {
    bytes.len() == crate::SAVE_SIZE
}

/// Whether `bytes` are a GameCube DW4/DWX GCI save file.
#[must_use]
pub fn is_gamecube_gci(bytes: &[u8]) -> bool {
    gci::is_gci(bytes)
}

/// Read a save from either container, deciding by the path's contents.
///
/// # Errors
/// [`crate::Error::File`] if unreadable, or whatever the container reports.
pub fn load_save(path: &Path) -> crate::Result<Vec<u8>> {
    Ok(load_save_with_region(path, None)?.0)
}

/// Read bytes and retain the card identity. Raw saves have no detectable region.
pub fn load_save_with_region(
    path: &Path,
    requested: Option<crate::Region>,
) -> crate::Result<(Vec<u8>, Option<crate::Region>)> {
    let bytes = std::fs::read(path).map_err(|source| crate::Error::File {
        path: path.to_path_buf(),
        source,
    })?;
    load_save_from_bytes_with_region(&bytes, requested)
}

/// Read a save out of an in-memory container.
pub fn load_save_from_bytes(bytes: &[u8]) -> crate::Result<Vec<u8>> {
    Ok(load_save_from_bytes_with_region(bytes, None)?.0)
}

pub fn load_save_from_bytes_with_region(
    bytes: &[u8],
    requested: Option<crate::Region>,
) -> crate::Result<(Vec<u8>, Option<crate::Region>)> {
    if is_memcard(bytes) {
        let mut card = Ps2Memcard::from_image(bytes.to_vec())?;
        let region = card.save_region(requested)?;
        Ok((
            card.read_save(region.save_name(), region.save_name())?,
            Some(region),
        ))
    } else if is_gamecube_card(bytes) {
        let card = GameCubeMemcard::from_image(bytes.to_vec())?;
        let region = card.save_region(requested)?;
        Ok((card.read_save(region)?, Some(region)))
    } else if is_gamecube_gci(bytes) {
        Ok((gci::read_save(bytes)?, None))
    } else {
        Ok((
            RawFile::from_bytes(bytes.to_vec(), Path::new(""))?.into_bytes(),
            requested,
        ))
    }
}

/// Produce the complete bytes to write for `path`, without writing them.
///
/// Reproduces the Python `save_any` semantics (spec 6.1):
///
/// - an existing memory card is rewritten **in place**, returning the whole
///   image with only the save's clusters changed;
/// - a `.ps2` that does not exist is created by copying `source` and swapping in
///   the save data, or, when there is no source, by synthesising a standard
///   card ([`format_card`]);
/// - anything else is a bare 81,920-byte save.
///
/// Returning bytes rather than writing them keeps the atomic-write, `.bak` and
/// verification policy in one place ([`crate::document::Document::save`]) for
/// every container type.
///
/// # Errors
/// [`crate::Error::NoSave`] if a `.ps2` donor is named but is not a card, or
/// whatever [`format_card`] reports.
pub fn render_container(path: &Path, source: Option<&Path>, data: &[u8]) -> crate::Result<Vec<u8>> {
    render_container_for_region(path, source, data, None)
}

pub fn render_container_for_region(
    path: &Path,
    source: Option<&Path>,
    data: &[u8],
    region: Option<crate::Region>,
) -> crate::Result<Vec<u8>> {
    render_container_for_region_and_gci(path, source, None, data, region)
}

/// Render a save, retaining a loaded GCI as donor metadata for Save As.
pub fn render_container_for_region_and_gci(
    path: &Path,
    source: Option<&Path>,
    source_gci: Option<&Path>,
    data: &[u8],
    region: Option<crate::Region>,
) -> crate::Result<Vec<u8>> {
    render_container_with_sources(path, source, source_gci, None, data, region)
}

/// Render a destination with PS2, GCI or GameCube raw-card donor metadata.
pub fn render_container_with_sources(
    path: &Path,
    source: Option<&Path>,
    source_gci: Option<&Path>,
    source_gc_card: Option<&Path>,
    data: &[u8],
    region: Option<crate::Region>,
) -> crate::Result<Vec<u8>> {
    if path.exists() {
        let bytes = std::fs::read(path).map_err(|source_err| crate::Error::File {
            path: path.to_path_buf(),
            source: source_err,
        })?;
        if is_memcard(&bytes) {
            require_ps2_data(data)?;
            let mut card = Ps2Memcard::from_image(bytes)?;
            let region = card.save_region(region)?;
            card.write_save(region.save_name(), region.save_name(), data)?;
            return Ok(card.image().to_vec());
        }
        if is_gamecube_card(&bytes) {
            let mut card = GameCubeMemcard::from_image(bytes)?;
            let region = card.save_region(region)?;
            card.write_save(region, data)?;
            return Ok(card.image().to_vec());
        }
        if is_gamecube_gci(&bytes) {
            return gci::write_save(&bytes, data);
        }
        if is_raw_save(&bytes) {
            return Ok(data.to_vec());
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("raw") || e.eq_ignore_ascii_case("gcp"))
        {
            return Err(crate::Error::NoSave(
                "destination is not a supported GameCube card or raw save".into(),
            ));
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("gci"))
        {
            return Err(crate::Error::NoSave(format!(
                "{} is not a Digimon World 4 GameCube GCI file",
                path.display()
            )));
        }
    } else if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("ps2"))
    {
        require_ps2_data(data)?;
        // No donor card: synthesise a standard one. A donor, when given, is
        // still copied so its icons and any other files survive.
        let Some(source) = source else {
            return format_card_for_region(data, region.unwrap_or_default());
        };
        let bytes = std::fs::read(source).map_err(|source_err| crate::Error::File {
            path: source.to_path_buf(),
            source: source_err,
        })?;
        if !is_memcard(&bytes) {
            return Err(crate::Error::NoSave(format!(
                "{}: {} is not a memory card",
                path.display(),
                source.display()
            )));
        }
        let mut card = Ps2Memcard::from_image(bytes)?;
        let region = card.save_region(region)?;
        card.write_save(region.save_name(), region.save_name(), data)?;
        return Ok(card.image().to_vec());
    } else if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("gci"))
    {
        if let Some(donor) = source_gc_card {
            let bytes = std::fs::read(donor).map_err(|source_err| crate::Error::File {
                path: donor.to_path_buf(),
                source: source_err,
            })?;
            let card = GameCubeMemcard::from_image(bytes)?;
            let region = card.save_region(region)?;
            return gci::write_save(&card.export_gci(region)?, data);
        }
        let donor = source_gci.or_else(|| {
            source.filter(|candidate| {
                candidate
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("gci"))
            })
        });
        let Some(donor) = donor else {
            return gci::create_save(data, region.unwrap_or_default());
        };
        let bytes = std::fs::read(donor).map_err(|source_err| crate::Error::File {
            path: donor.to_path_buf(),
            source: source_err,
        })?;
        return gci::write_save(&bytes, data);
    }

    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("raw") || e.eq_ignore_ascii_case("gcp"))
    {
        if let Some(donor) = source_gc_card {
            let bytes = std::fs::read(donor).map_err(|source_err| crate::Error::File {
                path: donor.to_path_buf(),
                source: source_err,
            })?;
            let mut card = GameCubeMemcard::from_image(bytes)?;
            let region = card.save_region(region)?;
            card.write_save(region, data)?;
            return Ok(card.image().to_vec());
        }
        if crate::SaveData::parse(data)?.byte_order() == crate::ByteOrder::Big {
            let mut file = if let Some(donor) = source_gci {
                let original = std::fs::read(donor).map_err(|source| crate::Error::File {
                    path: donor.to_path_buf(),
                    source,
                })?;
                gci::write_save(&original, data)?
            } else {
                gci::create_save(data, region.unwrap_or_default())?
            };
            let region = region.or_else(|| gci::region(&file)).unwrap_or_default();
            // Preserve GCI artwork/metadata; only an explicit regional export
            // changes the game code. Unknown raw data still defaults to USA.
            file[..4].copy_from_slice(gci::game_code(region));
            return gamecube::format_card(Some(&file), 16, region);
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("gcp"))
        {
            return Err(crate::Error::NoSave(
                "GameCube cards require big-endian savedata".into(),
            ));
        }
    }

    Ok(data.to_vec())
}

fn require_ps2_data(data: &[u8]) -> crate::Result<()> {
    if crate::SaveData::parse(data)?.byte_order() == crate::ByteOrder::Big {
        return Err(crate::Error::NoSave(
            "GameCube savedata cannot be written to a PS2 card without conversion".into(),
        ));
    }
    Ok(())
}
