//! A bare 81,920-byte save file, with no filesystem around it.

use crate::Error;
use crate::memcard::CardBackend;
use std::path::{Path, PathBuf};

/// A raw save file on disk.
#[derive(Debug, Clone)]
pub struct RawFile {
    bytes: Vec<u8>,
    path: PathBuf,
}

impl RawFile {
    /// Read a raw save file.
    ///
    /// # Errors
    /// [`Error::File`] if unreadable, [`Error::BadSaveSize`] if the wrong length.
    pub fn open(path: &Path) -> Result<Self, Error> {
        let bytes = std::fs::read(path).map_err(|source| Error::File {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_bytes(bytes, path)
    }

    /// Wrap bytes already read, checking they are a save.
    ///
    /// # Errors
    /// [`Error::BadSaveSize`] if `bytes` is not [`crate::SAVE_SIZE`] long.
    pub fn from_bytes(bytes: Vec<u8>, path: &Path) -> Result<Self, Error> {
        crate::SaveData::parse(&bytes)?;
        Ok(Self {
            bytes,
            path: path.to_path_buf(),
        })
    }

    /// The save bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// The path it was read from.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Consume the wrapper, returning the bytes.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    /// The bytes, for a read through the trait.
    fn read_bytes(&self) -> Vec<u8> {
        self.bytes.clone()
    }

    /// Replace the bytes, for a write through the trait.
    fn write_bytes(&mut self, data: &[u8]) {
        self.bytes = data.to_vec();
    }
}

/// A raw file has no directory structure, so `dir` and `file` are ignored.
impl CardBackend for RawFile {
    fn open(path: &Path) -> crate::Result<Self> {
        // Not `Self::open`: the inherent and trait methods share a name, and
        // delegating by that name would be ambiguous at best.
        let bytes = std::fs::read(path).map_err(|source| Error::File {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_bytes(bytes, path)
    }

    fn read_save(&mut self, _dir: &str, _file: &str) -> crate::Result<Vec<u8>> {
        Ok(self.read_bytes())
    }

    fn write_save(&mut self, _dir: &str, _file: &str, data: &[u8]) -> crate::Result<()> {
        self.write_bytes(data);
        Ok(())
    }
}
