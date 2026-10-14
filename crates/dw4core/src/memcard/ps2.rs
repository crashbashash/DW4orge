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
        self.read_save_at(dir, file)
    }

    /// The body of [`Self::read_save`], for unambiguous delegation from the
    /// [`CardBackend`] impl.
    fn read_save_at(&mut self, dir: &str, file: &str) -> Result<Vec<u8>, Error> {
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

    /// Write `data` over `dir`/`file`, in place.
    ///
    /// Only the clusters the file already occupies are touched; every other
    /// byte of the image is left alone. Each written page gets a freshly
    /// computed ECC and zeroed trailing spare bytes, which reproduces the
    /// original image exactly when the content is unchanged.
    ///
    /// The save is a fixed 81,920 bytes, so its chain is always long enough.
    /// A longer write is refused rather than allowed to run off the chain.
    ///
    /// # Errors
    /// [`Error::BadCard`] if the chain is too short for `data`.
    pub fn write_save(&mut self, dir: &str, file: &str, data: &[u8]) -> Result<(), Error> {
        self.write_save_at(dir, file, data)
    }

    /// The body of [`Self::write_save`], for unambiguous delegation.
    fn write_save_at(&mut self, dir: &str, file: &str, data: &[u8]) -> Result<(), Error> {
        let located = self.locate(dir, file)?;

        let needed = data.len().div_ceil(self.geometry.cluster_size).max(1);
        if located.chain.len() < needed {
            return Err(Error::BadCard(format!(
                "{dir}/{file}: {} bytes need {needed} clusters but the chain has {}",
                data.len(),
                located.chain.len()
            )));
        }

        let mut written = 0usize;
        for cluster in &located.chain {
            if written >= data.len() {
                break;
            }
            let take = (data.len() - written).min(self.geometry.cluster_size);
            self.write_cluster(*cluster, &data[written..written + take]);
            written += take;
        }

        Ok(())
    }

    /// Write one data-area cluster, recomputing each page's ECC.
    fn write_cluster(&mut self, relative: u32, data: &[u8]) {
        // `relative`, not absolute: the data area starts at `alloc_offset`, so
        // treating this as an absolute cluster writes into the FAT region.
        let first_page = self.geometry.cluster_first_page(relative);
        let page_size = self.geometry.page_size;

        for i in 0..self.geometry.pages_per_cluster {
            let start = i * page_size;
            if start >= data.len() {
                break;
            }
            let end = (start + page_size).min(data.len());
            self.write_page(first_page + i, &data[start..end]);
        }
    }

    /// Write one page's data and spare area, in place.
    ///
    /// A partial page keeps the bytes after `data`, rather than zeroing them:
    /// only the save's own bytes should ever change.
    fn write_page(&mut self, page: usize, data: &[u8]) {
        let at = self.geometry.page_offset(page);
        let page_size = self.geometry.page_size;

        let mut buf = self.image[at..at + page_size].to_vec();
        buf[..data.len()].copy_from_slice(data);
        self.image[at..at + page_size].copy_from_slice(&buf);

        if !self.geometry.has_spare() {
            return;
        }

        let spare = crate::memcard::ecc::page_spare(&buf);
        let spare_at = at + page_size;
        self.image[spare_at..spare_at + spare.len()].copy_from_slice(&spare);
        // Zero the rest of the spare area, as the reference does. On a real
        // card those bytes are already zero, which is what makes an unchanged
        // rewrite reproduce the image byte for byte.
        let raw_end = at + self.geometry.raw_page_size;
        for b in &mut self.image[spare_at + spare.len()..raw_end] {
            *b = 0;
        }
    }
}

impl crate::memcard::CardBackend for Ps2Memcard {
    fn open(path: &std::path::Path) -> Result<Self, Error> {
        let image = std::fs::read(path).map_err(|source| Error::File {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_image(image)
    }

    fn read_save(&mut self, dir: &str, file: &str) -> crate::Result<Vec<u8>> {
        self.read_save_at(dir, file)
    }

    fn write_save(&mut self, dir: &str, file: &str, data: &[u8]) -> crate::Result<()> {
        self.write_save_at(dir, file, data)
    }
}
