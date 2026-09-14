//! The PS2 memory-card filesystem, implemented natively.
//!
//! `ps2-memcard` 0.2.2 was evaluated and rejected (spec 6.1): it sizes a
//! directory from the `.` entry's `length`, which is 0 on this card, where the
//! working reference bounds by the *parent* entry's length. Its `CardBuilder`
//! also rebuilds whole cards rather than writing in place.
//!
//! Everything here was validated against the real `Mcd001.ps2` before this
//! module was written; the measured values are recorded in the implementation
//! plan and asserted by `tests/memcard.rs`.

mod ecc;
mod entry;
mod fat;
mod geometry;
mod ps2;
mod raw;

use std::path::Path;

pub use ecc::{ECC_CHUNK, ECC_CHUNK_BYTES, ecc_chunk, page_spare};
pub use entry::{ENTRY_SIZE, Entry, MODE_DIR, MODE_EXISTS, MODE_FILE, MODE_HIDDEN};
pub use fat::{ALLOCATED_BIT, CHAIN_END, FatTable, UNALLOCATED};
pub use geometry::{CardKind, Geometry, SB_MAGIC, SB_SIZE, Superblock};
pub use ps2::{LocatedFile, Ps2Memcard};
pub use raw::RawFile;

/// The directory holding the save, inside the card.
pub const SAVE_DIR: &str = "BASLUS-20836savedata";

/// The save file's name inside that directory.
pub const SAVE_FILE: &str = "BASLUS-20836savedata";

/// A place a save can live: a card image or a bare file.
///
/// The two implementations differ only in where the bytes go. Callers that do
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

/// Read a save from either container, deciding by the path's contents.
///
/// # Errors
/// [`crate::Error::File`] if unreadable, or whatever the container reports.
pub fn load_save(path: &Path) -> crate::Result<Vec<u8>> {
    let bytes = std::fs::read(path).map_err(|source| crate::Error::File {
        path: path.to_path_buf(),
        source,
    })?;
    if is_memcard(&bytes) {
        Ps2Memcard::from_image(bytes)?.read_save(SAVE_DIR, SAVE_FILE)
    } else {
        Ok(RawFile::from_bytes(bytes, path)?.into_bytes())
    }
}

/// Read a save out of an in-memory container, deciding by content.
///
/// The path-based [`load_save`] reads the file for you; this is the same
/// dispatch for bytes already in memory.
///
/// # Errors
/// Whatever the detected container reports.
pub fn load_save_from_bytes(bytes: &[u8]) -> crate::Result<Vec<u8>> {
    if is_memcard(bytes) {
        Ps2Memcard::from_image(bytes.to_vec())?.read_save(SAVE_DIR, SAVE_FILE)
    } else {
        Ok(RawFile::from_bytes(bytes.to_vec(), Path::new(""))?.into_bytes())
    }
}

/// Produce the complete bytes to write for `path`, without writing them.
///
/// Reproduces the Python `save_any` semantics (spec 6.1):
///
/// - an existing memory card is rewritten **in place**, returning the whole
///   image with only the save's clusters changed;
/// - a `.ps2` that does not exist is created by copying `source` and swapping in
///   the save data;
/// - anything else is a bare 81,920-byte save.
///
/// Returning bytes rather than writing them keeps the atomic-write, `.bak` and
/// verification policy in one place ([`crate::document::Document::save`]) for
/// every container type.
///
/// # Errors
/// [`crate::Error::NoSave`] if a new `.ps2` is requested with no usable source.
pub fn render_container(path: &Path, source: Option<&Path>, data: &[u8]) -> crate::Result<Vec<u8>> {
    if path.exists() {
        let bytes = std::fs::read(path).map_err(|source_err| crate::Error::File {
            path: path.to_path_buf(),
            source: source_err,
        })?;
        if is_memcard(&bytes) {
            let mut card = Ps2Memcard::from_image(bytes)?;
            card.write_save(SAVE_DIR, SAVE_FILE, data)?;
            return Ok(card.image().to_vec());
        }
    } else if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("ps2"))
    {
        let source = source.ok_or_else(|| {
            crate::Error::NoSave(
                "cannot create a new .ps2 without a source card image; open an existing .ps2 first"
                    .to_string(),
            )
        })?;
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
        card.write_save(SAVE_DIR, SAVE_FILE, data)?;
        return Ok(card.image().to_vec());
    }

    Ok(data.to_vec())
}
