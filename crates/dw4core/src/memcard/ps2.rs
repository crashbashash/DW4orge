//! Reading and writing the save inside a real card image.

use crate::Error;
use crate::memcard::entry::{ENTRY_SIZE, Entry};
use crate::memcard::fat::FatTable;
use crate::memcard::geometry::{CardKind, Geometry, Superblock};

/// A file located inside the card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatedFile {
    /// The directory entry's first cluster, relative to `alloc_offset`.
    pub first_cluster: u32,
    /// The length the entry declares.
    pub length: u32,
    /// The full cluster chain.
    pub chain: Vec<u32>,
    /// The entry's raw 512-byte slot, so a write can preserve it.
    pub slot: [u8; ENTRY_SIZE],
}

/// A PS2 memory-card image, read into memory.
#[derive(Debug, Clone)]
pub struct Ps2Memcard {
    image: Vec<u8>,
    superblock: Superblock,
    geometry: Geometry,
    fat: FatTable,
}

impl Ps2Memcard {
    /// Parse an in-memory card image.
    ///
    /// # Errors
    /// Propagates superblock, geometry and FAT errors.
    pub fn from_image(image: Vec<u8>) -> Result<Self, Error> {
        let kind = CardKind::of_size(image.len()).ok_or_else(|| {
            Error::NotAMemcard(format!(
                "{} bytes is neither 8388608 (data only) nor 8650752 (with spare)",
                image.len()
            ))
        })?;
        let superblock = Superblock::parse(&image)?;
        let geometry = Geometry::from_superblock(&superblock, kind);

        if image.len() != geometry.total_pages() * geometry.raw_page_size {
            return Err(Error::BadCard(format!(
                "image is {} bytes, but the superblock implies {} pages of {} bytes",
                image.len(),
                geometry.total_pages(),
                geometry.raw_page_size
            )));
        }

        let fat = FatTable::from_card(&image, &superblock, &geometry)?;
        Ok(Self {
            image,
            superblock,
            geometry,
            fat,
        })
    }

    /// Read and parse the card at `path`.
    ///
    /// # Errors
    /// [`Error::File`] if the file cannot be read.
    pub fn open(path: &std::path::Path) -> Result<Self, Error> {
        let image = std::fs::read(path).map_err(|source| Error::File {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_image(image)
    }

    /// The raw image, for a byte-identical comparison.
    #[must_use]
    pub fn image(&self) -> &[u8] {
        &self.image
    }

    /// The parsed superblock.
    #[must_use]
    pub fn superblock(&self) -> &Superblock {
        &self.superblock
    }

    /// The derived geometry.
    #[must_use]
    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }

    /// The allocation table.
    #[must_use]
    pub fn fat(&self) -> &FatTable {
        &self.fat
    }

    /// The data of a **relative** cluster, with the spare areas stripped.
    fn cluster_data(&self, relative: u32) -> Result<Vec<u8>, Error> {
        self.geometry
            .read_cluster(&self.image, self.geometry.alloc_offset + relative)
            .ok_or_else(|| Error::BadCard(format!("cluster {relative} is outside the image")))
    }

    /// The entries of the directory in `cluster`, bounded by `count`.
    ///
    /// The bound comes from the **parent** entry's `length`; the directory's own
    /// `.` entry is ignored. That is the rule `ps2-memcard` gets wrong: on this
    /// card the save directory's `.` holds 0 while its parent holds 5.
    ///
    /// `.` and `..` are filtered out, so the caller sees only real children.
    fn directory_entries(&self, cluster: u32, count: u32) -> Result<Vec<Entry>, Error> {
        let chain = self.fat.chain(cluster)?;
        let mut out: Vec<Entry> = Vec::new();

        for c in chain {
            if out.len() >= count as usize {
                break;
            }
            let data = self.cluster_data(c)?;
            // `ENTRY_SIZE` is a compile-time constant, so the chunking is
            // checked at compile time.
            let (slots, _rest) = data.as_chunks::<ENTRY_SIZE>();
            for slot in slots {
                if out.len() >= count as usize {
                    break;
                }
                if let Some(entry) = Entry::parse(slot) {
                    out.push(entry);
                }
            }
        }

        Ok(out.into_iter().filter(|e| !e.is_dot()).collect())
    }

    /// The root directory's `.` entry, which is the root itself.
    fn root_entry(&self) -> Result<Entry, Error> {
        let data = self.cluster_data(self.superblock.rootdir_cluster)?;
        Entry::parse(&data[..ENTRY_SIZE])
            .ok_or_else(|| Error::BadCard("the root directory entry did not decode".to_string()))
    }

    /// Locate `dir`/`file` inside the card.
    ///
    /// # Errors
    /// [`Error::SaveNotFound`] if either name is absent.
    pub fn locate(&self, dir: &str, file: &str) -> Result<LocatedFile, Error> {
        let root = self.root_entry()?;
        let mut dirs = self.directory_entries(root.cluster(), root.length())?;
        let dir_entry = dirs
            .drain(..)
            .find(|e| e.is_dir() && e.name() == dir)
            .ok_or_else(|| Error::SaveNotFound {
                dir: dir.to_string(),
                file: file.to_string(),
            })?;

        let mut files = self.directory_entries(dir_entry.cluster(), dir_entry.length())?;
        let file_entry = files
            .drain(..)
            .find(|e| e.is_file() && e.name() == file)
            .ok_or_else(|| Error::SaveNotFound {
                dir: dir.to_string(),
                file: file.to_string(),
            })?;

        Ok(LocatedFile {
            first_cluster: file_entry.cluster(),
            length: file_entry.length(),
            chain: self.fat.chain(file_entry.cluster())?,
            slot: file_entry.encode(),
        })
    }

    /// Read `dir`/`file` out of the card.
    ///
    /// # Errors
    /// Propagates [`Self::locate`] and image-bounds errors.
    pub fn read_save(&mut self, dir: &str, file: &str) -> Result<Vec<u8>, Error> {
        let located = self.locate(dir, file)?;
        let mut out = Vec::with_capacity(located.length as usize);

        for cluster in &located.chain {
            if out.len() >= located.length as usize {
                break;
            }
            let data = self.cluster_data(*cluster)?;
            let take = (located.length as usize - out.len()).min(data.len());
            out.extend_from_slice(&data[..take]);
        }

        Ok(out)
    }
}
