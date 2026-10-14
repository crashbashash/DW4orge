//! GameCube raw memory-card filesystem (8192-byte blocks, mirrored metadata).
//!
//! Reads the current directory and BAT using their signed update counters.
//! Same-size save writes change only the existing file's data blocks, leaving
//! the card header, directory entries, allocation tables and other saves intact.

use std::collections::HashSet;

use super::gci;
use crate::{Error, Region, Result};

pub const GC_CARD_BLOCK: usize = 0x2000;
const FIRST_DATA_BLOCK: usize = 5;
const DIRECTORY_ENTRIES: usize = 127;

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_be_bytes([bytes[at], bytes[at + 1]])
}

fn bad(reason: impl Into<String>) -> Error {
    Error::BadCard(format!("GameCube: {}", reason.into()))
}

/// Identify a raw GameCube card by supported geometry and header fields.
#[must_use]
pub fn is_gamecube_card(bytes: &[u8]) -> bool {
    matches!(
        bytes.len(),
        0x80000 | 0x100000 | 0x200000 | 0x400000 | 0x800000 | 0x1000000
    ) && usize::from(u16_at(bytes, 0x22)) * 0x20000 == bytes.len()
        && u16_at(bytes, 0x24) <= 1
}

/// Nintendo's wrapping sum and complement sum of big-endian 16-bit words.
fn checksums(bytes: &[u8]) -> (u16, u16) {
    let (mut sum, mut inverse) = (0u16, 0u16);
    for word in bytes.as_chunks::<2>().0 {
        let value = u16::from_be_bytes([word[0], word[1]]);
        sum = sum.wrapping_add(value);
        inverse = inverse.wrapping_add(!value);
    }
    (
        if sum == 0xffff { 0 } else { sum },
        if inverse == 0xffff { 0 } else { inverse },
    )
}

fn valid_checksums(bytes: &[u8], start: usize, end: usize, stored: usize) -> bool {
    checksums(&bytes[start..end]) == (u16_at(bytes, stored), u16_at(bytes, stored + 2))
}

/// A validated GameCube raw card image.
#[derive(Debug, Clone)]
pub struct GameCubeMemcard {
    image: Vec<u8>,
    directory: usize,
    bat: usize,
}

impl GameCubeMemcard {
    pub fn from_image(image: Vec<u8>) -> Result<Self> {
        if !is_gamecube_card(&image) {
            return Err(bad("unsupported size or header geometry"));
        }
        if !valid_checksums(&image, 0, 0x1fc, 0x1fc) {
            return Err(bad("card header checksum mismatch"));
        }
        let blocks = image.len() / GC_CARD_BLOCK;
        let dir_ok = [1usize, 2].map(|b| {
            let base = b * GC_CARD_BLOCK;
            valid_checksums(&image, base, base + 0x1ffc, base + 0x1ffc)
        });
        let bat_ok = [3usize, 4].map(|b| {
            let base = b * GC_CARD_BLOCK;
            let free = (0..blocks - FIRST_DATA_BLOCK)
                .filter(|i| u16_at(&image, base + 0xa + i * 2) == 0)
                .count();
            valid_checksums(&image, base + 4, base + GC_CARD_BLOCK, base)
                && usize::from(u16_at(&image, base + 6)) == free
        });
        // Retail accepts one corrupt metadata copy and repairs it. Two corrupt
        // copies cause a corrupted-card result, even if one dir/BAT pair works.
        if dir_ok
            .iter()
            .chain(bat_ok.iter())
            .filter(|ok| !**ok)
            .count()
            >= 2
        {
            return Err(bad("multiple directory/allocation copies are corrupt"));
        }
        let choose = |ok: [bool; 2], a: usize, b: usize, counter: usize| {
            if !ok[0] {
                b
            } else if !ok[1] {
                a
            } else {
                let first = u16_at(&image, a * GC_CARD_BLOCK + counter) as i16;
                let second = u16_at(&image, b * GC_CARD_BLOCK + counter) as i16;
                if first >= second { a } else { b }
            }
        };
        let directory = choose(dir_ok, 1, 2, 0x1ffa) * GC_CARD_BLOCK;
        let bat = choose(bat_ok, 3, 4, 4) * GC_CARD_BLOCK;
        let card = Self {
            image,
            directory,
            bat,
        };
        let mut allocated = HashSet::new();
        for index in 0..DIRECTORY_ENTRIES {
            if card.entry(index)[..4] == [0xff; 4] {
                continue;
            }
            for block in card.chain(index)? {
                if !allocated.insert(block) {
                    return Err(bad("file chains share a data block"));
                }
            }
        }
        Ok(card)
    }

    #[must_use]
    pub fn image(&self) -> &[u8] {
        &self.image
    }

    fn entry(&self, index: usize) -> &[u8] {
        let at = self.directory + index * 0x40;
        &self.image[at..at + 0x40]
    }

    fn chain(&self, index: usize) -> Result<Vec<usize>> {
        let entry = self.entry(index);
        let count = usize::from(u16_at(entry, 0x38));
        let mut block = usize::from(u16_at(entry, 0x36));
        let total = self.image.len() / GC_CARD_BLOCK;
        if count == 0 || count > total - FIRST_DATA_BLOCK {
            return Err(bad("invalid file block count"));
        }
        let mut result = Vec::with_capacity(count);
        let mut seen = HashSet::new();
        for _ in 0..count {
            if block < FIRST_DATA_BLOCK || block >= total || !seen.insert(block) {
                return Err(bad("file chain is out of bounds or loops"));
            }
            result.push(block);
            block = usize::from(u16_at(&self.image, self.bat + 0xa + (block - 5) * 2));
        }
        if block != 0xffff {
            return Err(bad("directory length does not match allocation chain"));
        }
        Ok(result)
    }

    fn region_for_entry(&self, index: usize) -> Option<Region> {
        let e = self.entry(index);
        if !e[..4].starts_with(b"GDJ")
            || &e[4..6] != b"B2"
            || e[8..0x28].split(|b| *b == 0).next()? != b"NDMWSaveData"
        {
            return None;
        }
        match e[3] {
            b'E' => Some(Region::Usa),
            b'J' => Some(Region::Japan),
            b'P' => Some(Region::Europe),
            _ => None,
        }
    }

    pub fn save_region(&self, requested: Option<Region>) -> Result<Region> {
        let regions: Vec<_> = (0..DIRECTORY_ENTRIES)
            .filter_map(|i| self.region_for_entry(i))
            .collect();
        if let Some(region) = requested {
            if regions.contains(&region) {
                return Ok(region);
            }
            return Err(Error::NoSave(
                "requested DW4 region is absent from GameCube card".into(),
            ));
        }
        match regions.as_slice() {
            [region] => Ok(*region),
            [] => Err(Error::NoSave(
                "NDMWSaveData not found in GameCube card".into(),
            )),
            _ => Err(Error::AmbiguousRegion { regions }),
        }
    }

    fn file_index(&self, region: Region) -> Result<usize> {
        let matches: Vec<_> = (0..DIRECTORY_ENTRIES)
            .filter(|i| self.region_for_entry(*i) == Some(region))
            .collect();
        if matches.len() != 1 {
            return Err(bad("missing or duplicate DW4 file identity"));
        }
        Ok(matches[0])
    }

    /// Export the complete file as a GCI, including its existing banner.
    pub fn export_gci(&self, region: Region) -> Result<Vec<u8>> {
        let index = self.file_index(region)?;
        let mut out = self.entry(index).to_vec();
        for block in self.chain(index)? {
            out.extend_from_slice(&self.image[block * GC_CARD_BLOCK..(block + 1) * GC_CARD_BLOCK]);
        }
        Ok(out)
    }

    pub fn read_save(&self, region: Region) -> Result<Vec<u8>> {
        gci::read_save(&self.export_gci(region)?)
    }

    pub fn write_save(&mut self, region: Region, save: &[u8]) -> Result<()> {
        let index = self.file_index(region)?;
        let out = gci::write_save(&self.export_gci(region)?, save)?;
        for (block, data) in self
            .chain(index)?
            .into_iter()
            .zip(out[0x40..].as_chunks::<GC_CARD_BLOCK>().0)
        {
            self.image[block * GC_CARD_BLOCK..(block + 1) * GC_CARD_BLOCK].copy_from_slice(data);
        }
        Ok(())
    }
}

pub(crate) fn format_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(946_684_800)
}

fn put16(bytes: &mut [u8], at: usize, value: u16) {
    bytes[at..at + 2].copy_from_slice(&value.to_be_bytes());
}

fn fix_metadata(bytes: &mut [u8], start: usize, end: usize, stored: usize) {
    let (sum, inverse) = checksums(&bytes[start..end]);
    put16(bytes, stored, sum);
    put16(bytes, stored + 2, inverse);
}

/// Format a blank GameCube card, optionally containing one DW4 GCI.
/// `size_mbits` supports the six standard capacities (4 through 128 Mbit).
/// Header identity is generated from the current clock and a process counter;
/// no template memory card or console identity is required.
///
/// # Errors
/// Rejects unsupported sizes, malformed GCI data, or insufficient capacity.
pub fn format_card(gci_bytes: Option<&[u8]>, size_mbits: u16, region: Region) -> Result<Vec<u8>> {
    if !matches!(size_mbits, 4 | 8 | 16 | 32 | 64 | 128) {
        return Err(bad("unsupported card capacity"));
    }
    let blocks = usize::from(size_mbits) * 16;
    let count = if let Some(file) = gci_bytes {
        if !gci::is_gci(file) || file.len() != gci::GCI_MIN_SIZE || u16_at(file, 0x38) != 11 {
            return Err(bad("invalid DW4 GCI for new card"));
        }
        let code = match region {
            Region::Usa => b'E',
            Region::Europe => b'P',
            Region::Japan => b'J',
        };
        if file[3] != code {
            return Err(bad("GCI and card regions differ"));
        }
        11
    } else {
        0
    };
    if count > blocks - FIRST_DATA_BLOCK {
        return Err(bad("card is too small"));
    }
    let mut image = vec![0xff; blocks * GC_CARD_BLOCK];
    let time = format_time();
    // The card serial is an identity, not a security token. Include subsecond
    // time and a counter to distinguish cards created in the same second.
    static SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let mut identity = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
        ^ SEQUENCE
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            .rotate_left(32);
    let mut rand = time;
    for byte in &mut image[..12] {
        identity ^= identity << 13;
        identity ^= identity >> 7;
        identity ^= identity << 17;
        rand = rand.wrapping_mul(0x41c6_4e6d).wrapping_add(0x3039) >> 16;
        *byte = (identity as u8).wrapping_add(rand as u8);
        rand = (rand.wrapping_mul(0x41c6_4e6d).wrapping_add(0x3039) >> 16) & 0x7fff;
    }
    image[0x0c..0x14].copy_from_slice(&time.to_be_bytes());
    image[0x14..0x22].fill(0);
    put16(&mut image, 0x22, size_mbits);
    put16(&mut image, 0x24, u16::from(region == Region::Japan));
    fix_metadata(&mut image, 0, 0x1fc, 0x1fc);
    for (copy, block) in [1, 2].into_iter().enumerate() {
        let base = block * GC_CARD_BLOCK;
        if let Some(file) = gci_bytes {
            image[base..base + 0x40].copy_from_slice(&file[..0x40]);
            put16(&mut image, base + 0x36, 5);
        }
        put16(
            &mut image,
            base + 0x1ffa,
            u16::from(count != 0 && copy == 0),
        );
        fix_metadata(&mut image, base, base + 0x1ffc, base + 0x1ffc);
    }
    for (copy, block) in [3, 4].into_iter().enumerate() {
        let base = block * GC_CARD_BLOCK;
        image[base..base + GC_CARD_BLOCK].fill(0);
        put16(&mut image, base + 4, u16::from(count != 0 && copy == 0));
        put16(
            &mut image,
            base + 6,
            (blocks - FIRST_DATA_BLOCK - count) as u16,
        );
        put16(&mut image, base + 8, (4 + count) as u16);
        for i in 0..count {
            put16(
                &mut image,
                base + 0xa + i * 2,
                if i + 1 == count {
                    0xffff
                } else {
                    (6 + i) as u16
                },
            );
        }
        fix_metadata(&mut image, base + 4, base + GC_CARD_BLOCK, base);
    }
    if let Some(file) = gci_bytes {
        image[5 * GC_CARD_BLOCK..(5 + count) * GC_CARD_BLOCK].copy_from_slice(&file[0x40..]);
    }
    GameCubeMemcard::from_image(image.clone())?;
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SAVE_SIZE, SaveData};

    const CHAIN: [usize; 11] = [5, 7, 9, 11, 13, 15, 17, 19, 21, 23, 25];

    fn put(bytes: &mut [u8], at: usize, value: u16) {
        bytes[at..at + 2].copy_from_slice(&value.to_be_bytes());
    }

    fn fix(bytes: &mut [u8], start: usize, end: usize, stored: usize) {
        let (sum, inverse) = checksums(&bytes[start..end]);
        put(bytes, stored, sum);
        put(bytes, stored + 2, inverse);
    }

    fn fixture() -> Vec<u8> {
        let mut image = vec![0xff; 0x80000];
        put(&mut image, 0x22, 4);
        put(&mut image, 0x24, 0);
        fix(&mut image, 0, 0x1fc, 0x1fc);
        for (copy, counter) in [(1, 5), (2, 4)] {
            let base = copy * GC_CARD_BLOCK;
            image[base..base + 0x40].fill(0);
            image[base..base + 6].copy_from_slice(b"GDJEB2");
            image[base + 8..base + 20].copy_from_slice(b"NDMWSaveData");
            put(&mut image, base + 0x36, CHAIN[0] as u16);
            put(&mut image, base + 0x38, CHAIN.len() as u16);
            image[base + 0x40..base + 0x80].fill(0);
            image[base + 0x40..base + 0x46].copy_from_slice(b"TEST00");
            put(&mut image, base + 0x76, 6);
            put(&mut image, base + 0x78, 1);
            put(&mut image, base + 0x1ffa, counter);
            fix(&mut image, base, base + 0x1ffc, base + 0x1ffc);
        }
        for (copy, counter) in [(3, 6), (4, 5)] {
            let base = copy * GC_CARD_BLOCK;
            image[base..base + GC_CARD_BLOCK].fill(0);
            put(&mut image, base + 4, counter);
            put(&mut image, base + 6, 64 - 5 - 12);
            put(&mut image, base + 8, 25);
            for (i, block) in CHAIN.iter().enumerate() {
                let next = CHAIN.get(i + 1).copied().unwrap_or(0xffff);
                put(&mut image, base + 0xa + (block - 5) * 2, next as u16);
            }
            put(&mut image, base + 0xa + 2, 0xffff); // unrelated file at block 6
            fix(&mut image, base + 4, base + GC_CARD_BLOCK, base);
        }
        let mut raw = vec![0u8; SAVE_SIZE];
        for block in [0, crate::BLOCK] {
            raw[block + 4..block + 8].copy_from_slice(&4u32.to_be_bytes());
        }
        let mut save = SaveData::parse(&raw).unwrap();
        save.set_player_name("RAW");
        save.set_bit(500);
        let mut file = vec![0x55; 0x2000];
        file.extend(save.to_bytes());
        for (block, bytes) in CHAIN.into_iter().zip(file.as_chunks::<GC_CARD_BLOCK>().0) {
            image[block * GC_CARD_BLOCK..(block + 1) * GC_CARD_BLOCK].copy_from_slice(bytes);
        }
        image[6 * GC_CARD_BLOCK..7 * GC_CARD_BLOCK].fill(0xab);
        image
    }

    #[test]
    fn fragmented_file_read_and_write_preserve_every_other_card_byte() {
        let original = fixture();
        let mut card = GameCubeMemcard::from_image(original.clone()).unwrap();
        assert_eq!(card.directory, GC_CARD_BLOCK);
        assert_eq!(card.bat, 3 * GC_CARD_BLOCK);
        let mut save = SaveData::parse(&card.read_save(Region::Usa).unwrap()).unwrap();
        assert!(save.verify());
        assert_eq!(save.player_name(), "RAW");
        save.set_player_name("EDIT");
        save.set_bit(123456);
        card.write_save(Region::Usa, &save.to_bytes()).unwrap();
        let data = card.read_save(Region::Usa).unwrap();
        let edited = SaveData::parse(&data).unwrap();
        assert_eq!(edited.bit(), 123456);
        assert_eq!(edited.player_name(), "EDIT");
        assert!(edited.verify());
        assert_eq!(
            &original[..5 * GC_CARD_BLOCK],
            &card.image()[..5 * GC_CARD_BLOCK]
        );
        assert_eq!(
            &original[5 * GC_CARD_BLOCK..6 * GC_CARD_BLOCK],
            &card.image()[5 * GC_CARD_BLOCK..6 * GC_CARD_BLOCK]
        );
        for block in 5..64 {
            if !CHAIN.contains(&block) {
                assert_eq!(
                    &original[block * GC_CARD_BLOCK..(block + 1) * GC_CARD_BLOCK],
                    &card.image()[block * GC_CARD_BLOCK..(block + 1) * GC_CARD_BLOCK]
                );
            }
        }
        assert!(GameCubeMemcard::from_image(card.image().to_vec()).is_ok());
    }

    #[test]
    fn selects_newer_metadata_and_recovers_one_corrupt_copy() {
        let mut image = fixture();
        let dir = 2 * GC_CARD_BLOCK;
        put(&mut image, dir + 0x1ffa, 7);
        fix(&mut image, dir, dir + 0x1ffc, dir + 0x1ffc);
        assert_eq!(
            GameCubeMemcard::from_image(image.clone())
                .unwrap()
                .directory,
            dir
        );
        image[dir + 0x1ffc] ^= 1;
        assert_eq!(
            GameCubeMemcard::from_image(image.clone())
                .unwrap()
                .directory,
            GC_CARD_BLOCK
        );
        image[3 * GC_CARD_BLOCK] ^= 1;
        assert!(GameCubeMemcard::from_image(image).is_err());
    }

    #[test]
    fn rejects_header_corruption_cycles_and_length_mismatches() {
        let mut header = fixture();
        header[0x10] ^= 1;
        assert!(GameCubeMemcard::from_image(header).is_err());
        for next in [5, 0xffff, 64] {
            let mut image = fixture();
            for b in [3, 4] {
                let base = b * GC_CARD_BLOCK;
                put(&mut image, base + 0xa, next);
                fix(&mut image, base + 4, base + GC_CARD_BLOCK, base);
            }
            assert!(GameCubeMemcard::from_image(image).is_err());
        }
    }

    #[test]
    fn document_save_as_keeps_card_backup_and_can_export_gci() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("card.raw");
        let destination = dir.path().join("edited.raw");
        let exported = dir.path().join("exported.gci");
        let original = fixture();
        std::fs::write(&source, &original).unwrap();
        let mut document = crate::document::Document::load(&source).unwrap();
        document.data_mut().set_bit(765432);
        document.save(&destination).unwrap();
        assert_eq!(
            std::fs::metadata(&destination).unwrap().len(),
            original.len() as u64
        );
        assert_eq!(std::fs::read(&source).unwrap(), original);
        document.data_mut().set_bit(42);
        document.save(&destination).unwrap();
        assert!(destination.with_extension("raw.bak").exists());
        document.save(&exported).unwrap();
        assert_eq!(document.region(), Some(Region::Usa));
        assert_eq!(
            std::fs::metadata(&exported).unwrap().len(),
            gci::GCI_MIN_SIZE as u64
        );
        assert_eq!(
            crate::document::Document::load(&exported)
                .unwrap()
                .data()
                .bit(),
            42
        );
    }

    #[test]
    fn refuses_little_endian_save_data_in_gamecube_container() {
        let mut card = GameCubeMemcard::from_image(fixture()).unwrap();
        assert!(card.write_save(Region::Usa, &vec![0; SAVE_SIZE]).is_err());
    }
}
