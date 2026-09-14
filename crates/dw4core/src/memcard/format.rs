//! Synthesising a standard 8 MB memory card from nothing.
//!
//! Every constant here is measured from the reference cards under
//! `Decomp/DW4/` and pinned by tests; see `docs/card-creation-design.md`.
//!
//! **Naming:** `fat.rs` names `UNALLOCATED`/`CHAIN_END` from the reader's masked
//! view, where the on-disk meanings are reversed. This module uses its own
//! write-side names and must not borrow those.

use crate::Error;
use crate::memcard::ecc::page_spare;
use crate::memcard::geometry::{CardKind, Geometry, Superblock};
use crate::memcard::{
    ALLOCATED_BIT, ENTRY_SIZE, Entry, MODE_DIR_ENTRY, MODE_FILE_ENTRY, MODE_ROOT_DOTDOT,
    MODE_SAVE_DIR_DOTDOT, SAVE_DIR, SAVE_FILE,
};

/// Bytes in a standard card, spare included.
pub const CARD_SIZE: usize = 16_384 * 528;

/// Page 0's data, constant across every reference card.
const SUPERBLOCK_PAGE: &[u8] = include_bytes!("../../data/card/superblock.bin");
/// The save's icon descriptor.
const ICON_SYS: &[u8] = include_bytes!("../../data/card/icon.sys");
/// The save's animated icon.
const ICON1_ICO: &[u8] = include_bytes!("../../data/card/icon1.ico");

const _: () = assert!(SUPERBLOCK_PAGE.len() == 512);
const _: () = assert!(ICON_SYS.len() == 964);
const _: () = assert!(ICON1_ICO.len() == 34_156);

/// The save directory's cluster chain.
const SAVE_DIR_CHAIN: [u32; 3] = [2, 3, 4];
/// First cluster of `icon1.ico` and how many it spans.
const ICON1_ICO_CLUSTER: u32 = 5;
const ICON1_ICO_CLUSTERS: u32 = 34;
/// First cluster of the save and how many it spans.
const SAVE_CLUSTER: u32 = 39;
const SAVE_CLUSTERS: u32 = 80;
/// The single cluster holding `icon.sys`.
const ICON_SYS_CLUSTER: u32 = 119;

/// A raw free FAT entry. Note the allocated bit is **clear**.
const FAT_FREE: u32 = 0x7FFF_FFFF;
/// A raw chain-end FAT entry: the allocated bit plus `CHAIN_END`.
const FAT_CHAIN_END: u32 = 0xFFFF_FFFF;

/// The geometry of the standard card, from the committed page 0.
///
/// # Errors
/// [`Error::BadCardGeometry`] if the template does not parse or its size is not
/// a known card kind.
fn page0_geometry() -> Result<Geometry, Error> {
    let sb = Superblock::parse(SUPERBLOCK_PAGE)?;
    let kind = CardKind::of_size(CARD_SIZE)
        .ok_or_else(|| Error::BadCardGeometry(format!("{CARD_SIZE} is not a known card size")))?;
    Ok(Geometry::from_superblock(&sb, kind))
}

/// Write one page's data and freshly computed spare area.
fn write_page(image: &mut [u8], geom: &Geometry, page: usize, data: &[u8]) {
    debug_assert_eq!(data.len(), geom.page_size);
    let at = geom.page_offset(page);
    image[at..at + geom.page_size].copy_from_slice(data);
    let spare = page_spare(data);
    image[at + geom.page_size..at + geom.page_size + spare.len()].copy_from_slice(&spare);
}

/// Write a full cluster at an **absolute** cluster number, with ECC.
fn write_absolute_cluster(image: &mut [u8], geom: &Geometry, absolute: u32, data: &[u8]) {
    debug_assert_eq!(data.len(), geom.cluster_size);
    for i in 0..geom.pages_per_cluster {
        let page = absolute as usize * geom.pages_per_cluster + i;
        write_page(
            image,
            geom,
            page,
            &data[i * geom.page_size..(i + 1) * geom.page_size],
        );
    }
}

/// Write a full cluster at a **relative** (data-area) cluster number.
fn write_relative_cluster(image: &mut [u8], geom: &Geometry, relative: u32, data: &[u8]) {
    write_absolute_cluster(image, geom, geom.alloc_offset + relative, data);
}

/// Allocate a consecutive chain of `len` clusters and end it.
fn link_chain(fat: &mut [u32], start: u32, len: u32) {
    let end = start + len - 1;
    for (offset, slot) in fat[start as usize..end as usize].iter_mut().enumerate() {
        *slot = ALLOCATED_BIT | (start + offset as u32 + 1);
    }
    fat[end as usize] = FAT_CHAIN_END;
}

/// The whole FAT for the layout this module writes.
fn fat_entries(geom: &Geometry) -> Vec<u32> {
    let mut fat = vec![FAT_FREE; geom.clusters_per_card as usize];

    // Root: 0 -> 1.
    fat[0] = ALLOCATED_BIT | 1;
    fat[1] = FAT_CHAIN_END;
    // Save directory: 2 -> 3 -> 4.
    fat[SAVE_DIR_CHAIN[0] as usize] = ALLOCATED_BIT | SAVE_DIR_CHAIN[1];
    fat[SAVE_DIR_CHAIN[1] as usize] = ALLOCATED_BIT | SAVE_DIR_CHAIN[2];
    fat[SAVE_DIR_CHAIN[2] as usize] = FAT_CHAIN_END;
    link_chain(&mut fat, ICON1_ICO_CLUSTER, ICON1_ICO_CLUSTERS);
    link_chain(&mut fat, SAVE_CLUSTER, SAVE_CLUSTERS);
    // icon.sys is a single cluster.
    fat[ICON_SYS_CLUSTER as usize] = FAT_CHAIN_END;

    fat
}

/// Write the indirect FAT: the FAT clusters, then `0xFFFFFFFF`.
fn write_ifc(image: &mut [u8], geom: &Geometry, ifc_cluster: u32) {
    let first_fat = ifc_cluster + 1;
    let fat_clusters = (geom.alloc_offset - first_fat) as usize;
    let mut entries = vec![0xFFFF_FFFFu32; geom.cluster_size / 4];
    for (i, e) in entries.iter_mut().take(fat_clusters).enumerate() {
        *e = first_fat + i as u32;
    }
    let bytes: Vec<u8> = entries.iter().flat_map(|v| v.to_le_bytes()).collect();
    write_absolute_cluster(image, geom, ifc_cluster, &bytes);
}

/// Write the FAT across its clusters, starting at `first_fat`.
fn write_fat(image: &mut [u8], geom: &Geometry, first_fat: u32, entries: &[u32]) {
    for (i, chunk) in entries.chunks(geom.fat_per_cluster).enumerate() {
        let bytes: Vec<u8> = chunk.iter().flat_map(|v| v.to_le_bytes()).collect();
        write_absolute_cluster(image, geom, first_fat + i as u32, &bytes);
    }
}

/// A cluster buffer: erased, with `data` copied in from the start.
fn cluster_with(data: &[u8], cluster_size: usize) -> Vec<u8> {
    let mut buf = vec![0xFFu8; cluster_size];
    let take = data.len().min(cluster_size);
    buf[..take].copy_from_slice(&data[..take]);
    buf
}

/// Pack entries into one cluster, padding the rest with erased slots.
fn entry_cluster(entries: &[[u8; ENTRY_SIZE]], cluster_size: usize) -> Vec<u8> {
    assert!(entries.len() * ENTRY_SIZE <= cluster_size);
    let mut buf = vec![0xFFu8; cluster_size];
    for (i, entry) in entries.iter().enumerate() {
        buf[i * ENTRY_SIZE..(i + 1) * ENTRY_SIZE].copy_from_slice(entry);
    }
    buf
}

/// Write a file across consecutive clusters, 0xFF-padding the last.
fn write_file(image: &mut [u8], geom: &Geometry, first: u32, data: &[u8]) {
    for (i, chunk) in data.chunks(geom.cluster_size).enumerate() {
        let buf = cluster_with(chunk, geom.cluster_size);
        write_relative_cluster(image, geom, first + i as u32, &buf);
    }
}

/// The root directory's entries, in slot order.
fn root_entries() -> Vec<[u8; ENTRY_SIZE]> {
    vec![
        Entry::new(MODE_DIR_ENTRY, 3, 0, ".").encode(),
        Entry::new(MODE_ROOT_DOTDOT, 0, 0, "..").encode(),
        Entry::new(MODE_DIR_ENTRY, 5, SAVE_DIR_CHAIN[0], SAVE_DIR).encode(),
    ]
}

/// The save directory's entries, in slot order.
fn save_dir_entries(save_len: u32) -> Vec<[u8; ENTRY_SIZE]> {
    vec![
        Entry::new(MODE_DIR_ENTRY, 0, 0, ".").encode(),
        Entry::new(MODE_SAVE_DIR_DOTDOT, 0, 0, "..").encode(),
        Entry::new(
            MODE_FILE_ENTRY,
            ICON1_ICO.len() as u32,
            ICON1_ICO_CLUSTER,
            "icon1.ico",
        )
        .encode(),
        Entry::new(MODE_FILE_ENTRY, save_len, SAVE_CLUSTER, SAVE_FILE).encode(),
        Entry::new(
            MODE_FILE_ENTRY,
            ICON_SYS.len() as u32,
            ICON_SYS_CLUSTER,
            "icon.sys",
        )
        .encode(),
    ]
}

/// Synthesise a standard 8 MB card holding `save`.
///
/// The layout is in `docs/card-creation-design.md`; every cluster number is a
/// constant pinned by tests.
///
/// # Errors
/// [`Error::BadSaveSize`] if `save` is not [`crate::SAVE_SIZE`] bytes, or
/// whatever parsing the committed page-0 template reports.
pub fn format_card(save: &[u8]) -> Result<Vec<u8>, Error> {
    if save.len() != crate::SAVE_SIZE {
        return Err(Error::BadSaveSize {
            expected: crate::SAVE_SIZE,
            actual: save.len(),
        });
    }

    let sb = Superblock::parse(SUPERBLOCK_PAGE)?;
    let geom = page0_geometry()?;
    let mut image = vec![0xFFu8; CARD_SIZE];

    // Page 0: the constant template, spare recomputed.
    write_page(&mut image, &geom, 0, SUPERBLOCK_PAGE);

    // Allocation tables. The indirect FAT names the FAT clusters, which run
    // from just after it up to the first data cluster.
    let ifc_cluster = *sb.ifc_list.first().ok_or_else(|| {
        Error::BadCardGeometry("the superblock names no indirect FAT".to_string())
    })?;
    write_ifc(&mut image, &geom, ifc_cluster);
    write_fat(&mut image, &geom, ifc_cluster + 1, &fat_entries(&geom));

    // Directories: root spans 0..=1 (3 entries), the save directory 2..=4 (5).
    let root = root_entries();
    write_relative_cluster(
        &mut image,
        &geom,
        0,
        &entry_cluster(&root[..2], geom.cluster_size),
    );
    write_relative_cluster(
        &mut image,
        &geom,
        1,
        &entry_cluster(&root[2..], geom.cluster_size),
    );
    let dir = save_dir_entries(save.len() as u32);
    write_relative_cluster(
        &mut image,
        &geom,
        SAVE_DIR_CHAIN[0],
        &entry_cluster(&dir[..2], geom.cluster_size),
    );
    write_relative_cluster(
        &mut image,
        &geom,
        SAVE_DIR_CHAIN[1],
        &entry_cluster(&dir[2..4], geom.cluster_size),
    );
    write_relative_cluster(
        &mut image,
        &geom,
        SAVE_DIR_CHAIN[2],
        &entry_cluster(&dir[4..], geom.cluster_size),
    );

    // Payloads.
    write_file(&mut image, &geom, ICON1_ICO_CLUSTER, ICON1_ICO);
    write_file(&mut image, &geom, SAVE_CLUSTER, save);
    write_file(&mut image, &geom, ICON_SYS_CLUSTER, ICON_SYS);

    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_standard_card_has_the_measured_geometry() {
        let geom = page0_geometry().expect("page 0 parses");
        assert_eq!(geom.page_size, 512);
        assert_eq!(geom.pages_per_cluster, 2);
        assert_eq!(geom.clusters_per_card, 8192);
        assert_eq!(geom.alloc_offset, 41);
        assert_eq!(geom.alloc_end, 8135);
        assert_eq!(geom.raw_page_size, 528);
        assert_eq!(geom.fat_per_cluster, 256);
    }

    #[test]
    fn only_the_expected_chains_are_allocated() {
        let geom = page0_geometry().expect("page 0 parses");
        let fat = fat_entries(&geom);
        assert_eq!(fat.len(), 8192);
        assert_eq!(fat[0], ALLOCATED_BIT | 1);
        assert_eq!(fat[1], FAT_CHAIN_END);
        assert_eq!(fat[4], FAT_CHAIN_END);
        assert_eq!(fat[38], FAT_CHAIN_END);
        assert_eq!(fat[118], FAT_CHAIN_END);
        assert_eq!(fat[119], FAT_CHAIN_END);
        // Everything not in a chain of this layout is free.
        for n in [120, 121, 500, 8191] {
            assert_eq!(fat[n], FAT_FREE, "cluster {n}");
        }
    }

    #[test]
    fn every_allocated_entry_points_at_the_next_cluster() {
        let geom = page0_geometry().expect("page 0 parses");
        let fat = fat_entries(&geom);
        for (offset, value) in fat[5..38].iter().enumerate() {
            let n = 5 + offset as u32;
            assert_eq!(*value, ALLOCATED_BIT | (n + 1), "icon cluster {n}");
        }
        for (offset, value) in fat[39..118].iter().enumerate() {
            let n = 39 + offset as u32;
            assert_eq!(*value, ALLOCATED_BIT | (n + 1), "save cluster {n}");
        }
    }

    #[test]
    fn the_indirect_fat_names_the_fat_clusters() {
        let geom = page0_geometry().expect("page 0 parses");
        let mut image = vec![0xFFu8; CARD_SIZE];
        write_ifc(&mut image, &geom, 8);
        // `read_cluster` skips each page's spare area: a cluster is not a
        // contiguous run of the image, so slicing it directly reads ECC as data.
        let cluster = geom.read_cluster(&image, 8).expect("cluster 8 is in range");
        let values: Vec<u32> = cluster
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_le_bytes(*c))
            .collect();
        assert_eq!(&values[..3], &[9, 10, 11]);
        assert_eq!(&values[..32], &(9..=40).collect::<Vec<u32>>()[..]);
        assert!(values[32..].iter().all(|v| *v == 0xFFFF_FFFF));
    }

    #[test]
    fn page_zero_is_the_committed_template_with_fresh_ecc() {
        let geom = page0_geometry().expect("page 0 parses");
        let mut image = vec![0xFFu8; CARD_SIZE];
        write_page(&mut image, &geom, 0, SUPERBLOCK_PAGE);
        assert_eq!(&image[..512], SUPERBLOCK_PAGE);
        assert_eq!(&image[512..528], page_spare(SUPERBLOCK_PAGE).as_slice());
        assert_eq!(&image[524..528], &[0, 0, 0, 0]);
    }

    #[test]
    fn every_page_written_here_carries_valid_ecc() {
        let geom = page0_geometry().expect("page 0 parses");
        let mut image = vec![0xFFu8; CARD_SIZE];
        write_page(&mut image, &geom, 0, SUPERBLOCK_PAGE);
        write_ifc(&mut image, &geom, 8);
        write_fat(&mut image, &geom, 9, &fat_entries(&geom));
        for page in 0..82 {
            let at = geom.page_offset(page);
            let data = &image[at..at + geom.page_size];
            let spare = &image[at + geom.page_size..at + geom.page_size + geom.spare_size];
            if data.iter().all(|b| *b == 0xFF) {
                continue; // never written, so its spare stays erased
            }
            assert_eq!(page_spare(data).as_slice(), spare, "page {page}");
        }
    }
}
