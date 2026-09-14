//! The card's file allocation table.
//!
//! Two levels of indirection: `ifc_list` names the indirect-FAT clusters
//! (**absolute** cluster numbers); each holds cluster numbers of FAT clusters
//! (**also absolute**); each FAT cluster holds u32 entries indexed by
//! **relative** data cluster number.

use crate::Error;
use crate::memcard::geometry::{Geometry, Superblock};

/// Set on an allocated FAT entry; XORed off when the value is read.
pub const ALLOCATED_BIT: u32 = 0x8000_0000;
/// A free cluster, in one of its two spellings.
pub const UNALLOCATED: u32 = 0xFFFF_FFFF;
/// Ends a cluster chain.
pub const CHAIN_END: u32 = 0x7FFF_FFFF;

/// A safety bound on chain length, so a corrupt table cannot hang a walk.
const MAX_CHAIN: usize = 1 << 20;

/// Read a cluster's bytes as little-endian u32s.
fn cluster_u32s(blob: &[u8]) -> Vec<u32> {
    let (chunks, _rest) = blob.as_chunks::<4>();
    chunks
        .iter()
        .map(|chunk| u32::from_le_bytes(*chunk))
        .collect()
}

/// The flattened, indexed allocation table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FatTable {
    /// Every FAT entry, laid out so entry `n` is `entries[n]`.
    entries: Vec<u32>,
    /// The clusters the entries were read from, in order.
    fat_clusters: Vec<u32>,
    /// `cluster_size / 4`.
    fat_per_cluster: usize,
    /// Clusters in the data area.
    alloc_end: u32,
}

impl FatTable {
    /// Read and flatten the allocation table.
    ///
    /// # Errors
    /// [`Error::BadCard`] if a FAT cluster lies outside the image, or
    /// [`Error::BadCardGeometry`] if `fat_per_cluster` is 0.
    pub fn from_card(card: &[u8], sb: &Superblock, geom: &Geometry) -> Result<Self, Error> {
        if geom.fat_per_cluster == 0 {
            return Err(Error::BadCardGeometry(
                "cluster_size is smaller than a u32".to_string(),
            ));
        }

        let read_cluster = |absolute: u32| -> Result<Vec<u8>, Error> {
            geom.read_cluster(card, absolute).ok_or_else(|| {
                Error::BadCard(format!(
                    "FAT cluster {absolute} runs past the image ({} bytes)",
                    card.len()
                ))
            })
        };

        // Level 1: the indirect-FAT clusters named by ifc_list, flattened. The
        // reference drops UNALLOCATED entries here.
        let mut fat_clusters = Vec::new();
        for &indirect in &sb.ifc_list {
            for value in cluster_u32s(&read_cluster(indirect)?) {
                if value != UNALLOCATED {
                    fat_clusters.push(value);
                }
            }
        }

        // Level 2: the FAT clusters themselves, concatenated.
        let mut entries = Vec::with_capacity(fat_clusters.len() * geom.fat_per_cluster);
        for &cluster in &fat_clusters {
            entries.extend(cluster_u32s(&read_cluster(cluster)?));
        }

        Ok(Self {
            entries,
            fat_clusters,
            fat_per_cluster: geom.fat_per_cluster,
            alloc_end: sb.alloc_end,
        })
    }

    /// The clusters the table was read from.
    #[must_use]
    pub fn fat_clusters(&self) -> &[u32] {
        &self.fat_clusters
    }

    /// How many data clusters the table covers.
    #[must_use]
    pub fn cluster_count(&self) -> u32 {
        u32::try_from(self.entries.len()).unwrap_or(u32::MAX)
    }

    /// The raw stored value for `relative`, without masking.
    ///
    /// The reference indexes a two-level matrix as
    /// `matrix[(n / fat_per_cluster) % fat_per_cluster][n % fat_per_cluster]`.
    /// This has already flattened that matrix into one `Vec`, so the row offset
    /// must be multiplied back in.
    #[must_use]
    pub fn raw_entry(&self, relative: u32) -> u32 {
        let n = relative as usize;
        let row = (n / self.fat_per_cluster) % self.fat_per_cluster;
        let column = n % self.fat_per_cluster;
        self.entries
            .get(row * self.fat_per_cluster + column)
            .copied()
            .unwrap_or(UNALLOCATED)
    }

    /// The next cluster in the chain, or `None` at the end.
    ///
    /// Both spellings of a free cluster terminate: `0xFFFFFFFF` has the
    /// allocated bit set and masks down to [`CHAIN_END`], while `0x7FFFFFFF` is
    /// already [`CHAIN_END`].
    #[must_use]
    pub fn next(&self, relative: u32) -> Option<u32> {
        let value = self.raw_entry(relative);
        let next = if value & ALLOCATED_BIT != 0 {
            value ^ ALLOCATED_BIT
        } else {
            value
        };
        if next == CHAIN_END { None } else { Some(next) }
    }

    /// Walk the chain starting at `start`.
    ///
    /// # Errors
    /// [`Error::BadClusterChain`] if a cluster is outside the data area, the
    /// chain revisits a cluster (a loop), or it exceeds [`MAX_CHAIN`].
    pub fn chain(&self, start: u32) -> Result<Vec<u32>, Error> {
        let mut out = Vec::new();
        let mut seen = std::collections::HashSet::new();
        let mut current = Some(start);

        while let Some(cluster) = current {
            if cluster >= self.alloc_end {
                return Err(Error::BadClusterChain {
                    start,
                    reason: format!(
                        "cluster {cluster} is outside the data area (0..{})",
                        self.alloc_end
                    ),
                });
            }
            if !seen.insert(cluster) {
                return Err(Error::BadClusterChain {
                    start,
                    reason: format!("cluster {cluster} repeats, so the chain loops"),
                });
            }
            if out.len() >= MAX_CHAIN {
                return Err(Error::BadClusterChain {
                    start,
                    reason: format!("chain exceeds {MAX_CHAIN} clusters"),
                });
            }
            out.push(cluster);
            current = self.next(cluster);
        }

        Ok(out)
    }
}
