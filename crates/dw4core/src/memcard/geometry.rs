//! Card geometry and the superblock.
//!
//! The values this parses were measured from the real `Mcd001.ps2` and are
//! asserted in `tests/memcard.rs`; see the implementation plan's Background
//! section for the table.
use crate::Error;

/// Size of the superblock structure at the start of page 0.
pub const SB_SIZE: usize = 340;

/// The superblock's magic, 28 bytes, NUL-padded.
pub const SB_MAGIC: &[u8; 28] = b"Sony PS2 Memory Card Format ";

/// Which of the two card image forms this is.
///
/// Both hold 16,384 pages; they differ only in whether the 16-byte per-page
/// spare area (which holds the ECC) is present.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardKind {
    /// 8,388,608 bytes: page data only, no spare area.
    DataOnly,
    /// 8,650,752 bytes: a hardware dump, 528-byte pages.
    WithSpare,
}

impl CardKind {
    /// Classify a card image by its length.
    #[must_use]
    pub fn of_size(len: usize) -> Option<Self> {
        match len {
            8_388_608 => Some(Self::DataOnly),
            8_650_752 => Some(Self::WithSpare),
            _ => None,
        }
    }

    /// Whether this form carries a per-page spare area.
    #[must_use]
    pub fn has_spare(self) -> bool {
        matches!(self, Self::WithSpare)
    }
}

/// The card's superblock, at the start of page 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Superblock {
    /// Version string, e.g. `"1.2.0.0"`.
    pub version: String,
    /// Bytes of page data (512 on a real card).
    pub page_size: u16,
    /// Pages in a cluster (2).
    pub pages_per_cluster: u16,
    /// Pages in an erasable block (16).
    pub pages_per_block: u16,
    /// Total clusters (8192).
    pub clusters_per_card: u32,
    /// First cluster of the data area (41).
    pub alloc_offset: u32,
    /// Number of clusters in the data area (8135).
    pub alloc_end: u32,
    /// The root directory's cluster, relative to `alloc_offset` (0).
    pub rootdir_cluster: u32,
    /// Indirect-FAT clusters, **absolute** cluster numbers. Zeros removed.
    pub ifc_list: Vec<u32>,
    /// Card type byte.
    pub card_type: u8,
    /// Card flags byte.
    pub card_flags: u8,
}

impl Superblock {
    /// Parse and validate the superblock at the start of `card`.
    ///
    /// # Errors
    /// [`Error::NotAMemcard`] if `card` is too short or the magic is wrong;
    /// [`Error::BadCardGeometry`] if the fields are self-inconsistent.
    pub fn parse(card: &[u8]) -> Result<Self, Error> {
        if card.len() < SB_SIZE {
            return Err(Error::NotAMemcard(format!(
                "image is {} bytes, too short for a {SB_SIZE}-byte superblock",
                card.len()
            )));
        }
        if &card[..28] != SB_MAGIC {
            return Err(Error::NotAMemcard(
                "missing the 'Sony PS2 Memory Card Format' magic".to_string(),
            ));
        }

        let u16_at = |o: usize| u16::from_le_bytes([card[o], card[o + 1]]);
        let u32_at =
            |o: usize| u32::from_le_bytes([card[o], card[o + 1], card[o + 2], card[o + 3]]);
        let text = |o: usize, len: usize| {
            let raw = &card[o..o + len];
            let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
            String::from_utf8_lossy(&raw[..end]).into_owned()
        };

        let sb = Self {
            version: text(0x1C, 12),
            page_size: u16_at(0x28),
            pages_per_cluster: u16_at(0x2A),
            pages_per_block: u16_at(0x2C),
            clusters_per_card: u32_at(0x30),
            alloc_offset: u32_at(0x34),
            alloc_end: u32_at(0x38),
            rootdir_cluster: u32_at(0x3C),
            // 32 u32s at 0x50; only non-zero entries name a real cluster.
            ifc_list: (0..32)
                .map(|i| u32_at(0x50 + i * 4))
                .filter(|v| *v != 0)
                .collect(),
            card_type: card[0x150],
            card_flags: card[0x151],
        };

        sb.validate()?;
        Ok(sb)
    }

    /// Whether the geometry is self-consistent (spec 6.1).
    ///
    /// # Errors
    /// [`Error::BadCardGeometry`] naming the offending field.
    pub fn validate(&self) -> Result<(), Error> {
        if self.page_size == 0 {
            return Err(Error::BadCardGeometry("page_size is 0".to_string()));
        }
        if self.pages_per_cluster == 0 {
            return Err(Error::BadCardGeometry("pages_per_cluster is 0".to_string()));
        }
        if self.alloc_offset + self.alloc_end > self.clusters_per_card {
            return Err(Error::BadCardGeometry(format!(
                "alloc_offset {} + alloc_end {} exceeds clusters_per_card {}",
                self.alloc_offset, self.alloc_end, self.clusters_per_card
            )));
        }
        if self.ifc_list.is_empty() {
            return Err(Error::BadCardGeometry(
                "ifc_list has no non-zero entries".to_string(),
            ));
        }
        Ok(())
    }
}

/// Derived sizes and addressing for a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    /// Bytes of page data.
    pub page_size: usize,
    /// Pages per cluster.
    pub pages_per_cluster: usize,
    /// Bytes of spare area per page: `(page_size / 128) * 4`.
    pub spare_size: usize,
    /// Page data plus spare area.
    pub raw_page_size: usize,
    /// Bytes per cluster: `page_size * pages_per_cluster`.
    pub cluster_size: usize,
    /// u32 entries per FAT cluster.
    pub fat_per_cluster: usize,
    /// Total clusters on the card.
    pub clusters_per_card: u32,
    /// First data cluster.
    pub alloc_offset: u32,
    /// Clusters in the data area.
    pub alloc_end: u32,
}

impl Geometry {
    /// Derive the geometry from a parsed superblock.
    #[must_use]
    pub fn from_superblock(sb: &Superblock) -> Self {
        let page_size = usize::from(sb.page_size);
        let pages_per_cluster = usize::from(sb.pages_per_cluster);
        let spare_size = (page_size / 128) * 4;
        let cluster_size = page_size * pages_per_cluster;
        Self {
            page_size,
            pages_per_cluster,
            spare_size,
            raw_page_size: page_size + spare_size,
            cluster_size,
            fat_per_cluster: cluster_size / 4,
            clusters_per_card: sb.clusters_per_card,
            alloc_offset: sb.alloc_offset,
            alloc_end: sb.alloc_end,
        }
    }

    /// Total pages in the image.
    #[must_use]
    pub fn total_pages(&self) -> usize {
        self.clusters_per_card as usize * self.pages_per_cluster
    }

    /// Byte offset of page `page` (an absolute page number).
    #[must_use]
    pub fn page_offset(&self, page: usize) -> usize {
        page * self.raw_page_size
    }

    /// Byte offset of cluster `relative`, a **data-area** cluster.
    ///
    /// System structures (the FAT) are addressed absolutely and must use
    /// [`Self::absolute_cluster_offset`] instead. Mixing the two up is the
    /// easiest mistake to make here.
    #[must_use]
    pub fn cluster_offset(&self, relative: u32) -> usize {
        (self.alloc_offset + relative) as usize * self.cluster_size
    }

    /// Byte offset of an **absolute** cluster number, used for FAT clusters.
    #[must_use]
    pub fn absolute_cluster_offset(&self, absolute: u32) -> usize {
        absolute as usize * self.cluster_size
    }

    /// Whether `relative` is inside the data area.
    #[must_use]
    pub fn is_data_cluster(&self, relative: u32) -> bool {
        relative < self.alloc_end
    }
}
