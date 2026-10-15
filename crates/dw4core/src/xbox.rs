//! Xbox signed save files and FATX controller memory-unit images.
use crate::CharacterBox;
use crate::{Error, Result, SaveData};
use hmac::{Hmac, Mac};
use sha1::Sha1;
use std::collections::HashSet;

pub const SLOT_SIZE: usize = 0x2000;
pub const SIGNED_SIZE: usize = 0x8004;
pub const RECORD_SIZE: usize = 0x8018;
pub const FILE_SIZE: usize = 0x10030;
// Retail title key, independently derived from both supplied XBEs and checked
// against XboxSignatureKey and retail save HMACs. No console-specific key.
const KEY: [u8; 16] = [
    0xf1, 0x10, 0x0c, 0x5c, 0xac, 0xeb, 0x2f, 0xe8, 0x3c, 0x97, 0xa6, 0xb8, 0x92, 0x59, 0x9c, 0x13,
];

fn bad(message: impl Into<String>) -> Error {
    Error::NoSave(message.into())
}
fn word(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(
        bytes[offset..offset + 4]
            .try_into()
            .expect("checked offset"),
    )
}
#[must_use]
pub fn is_save(bytes: &[u8]) -> bool {
    bytes.len() == FILE_SIZE
}
#[must_use]
pub fn is_memory_unit(bytes: &[u8]) -> bool {
    bytes.starts_with(b"FATX")
}

pub fn verify_save(bytes: &[u8]) -> Result<[bool; 2]> {
    if bytes.len() != FILE_SIZE {
        return Err(bad("Xbox SaveData.dat must be 65,584 bytes"));
    }
    let mut valid = [false; 2];
    for (mirror, result) in valid.iter_mut().enumerate() {
        let base = mirror * RECORD_SIZE;
        let mut mac = Hmac::<Sha1>::new_from_slice(&KEY).expect("HMAC accepts 16-byte key");
        mac.update(&bytes[base..base + SIGNED_SIZE]);
        *result = word(bytes, base) == 4
            && mac
                .verify_slice(&bytes[base + SIGNED_SIZE..base + RECORD_SIZE])
                .is_ok();
    }
    Ok(valid)
}
fn resign(bytes: &mut [u8]) {
    for mirror in 0..2 {
        let base = mirror * RECORD_SIZE;
        let mut mac = Hmac::<Sha1>::new_from_slice(&KEY).expect("HMAC accepts 16-byte key");
        mac.update(&bytes[base..base + SIGNED_SIZE]);
        bytes[base + SIGNED_SIZE..base + RECORD_SIZE].copy_from_slice(&mac.finalize().into_bytes());
    }
}

#[derive(Debug, Clone)]
struct FileEntry {
    path: String,
    spans: Vec<(usize, usize)>,
    size: usize,
}
impl FileEntry {
    fn read(&self, image: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.size);
        for &(at, len) in &self.spans {
            out.extend_from_slice(&image[at..at + len]);
        }
        out
    }
    fn replace(&self, image: &mut [u8], bytes: &[u8]) -> Result<()> {
        if bytes.len() != self.size {
            return Err(bad("Xbox file size cannot change"));
        }
        let mut src = 0;
        for &(at, len) in &self.spans {
            image[at..at + len].copy_from_slice(&bytes[src..src + len]);
            src += len;
        }
        Ok(())
    }
}

/// Retains the entire source, including other saves, boxes, FAT and metadata.
#[derive(Debug, Clone)]
pub struct XboxSource {
    image: Vec<u8>,
    files: Vec<FileEntry>,
    choices: Vec<CharacterBox>,
    file_index: usize,
    box_index: usize,
    memory_unit: bool,
    /// Full initialized slot until the first write; original image stays intact for conflict checks.
    created_slot: Option<Vec<u8>>,
}
impl XboxSource {
    pub fn parse(bytes: Vec<u8>, selected: Option<&str>) -> Result<Self> {
        let memory_unit = is_memory_unit(&bytes);
        let files = if memory_unit {
            Fatx::parse(&bytes)?.saves()?
        } else {
            if !is_save(&bytes) {
                return Err(bad("not an Xbox save or memory unit"));
            }
            vec![FileEntry {
                path: "SaveData.dat".into(),
                spans: vec![(0, bytes.len())],
                size: bytes.len(),
            }]
        };
        let mut choices = Vec::new();
        for file in &files {
            let save = file.read(&bytes);
            if !verify_save(&save)?.into_iter().all(|valid| valid) {
                return Err(bad(format!(
                    "{}: Xbox save signature or wrapper version is invalid",
                    file.path
                )));
            }
            for box_index in 0..4 {
                let at = 4 + box_index * SLOT_SIZE;
                if save[at] != 1 || save[RECORD_SIZE + at] != 1 {
                    continue;
                }
                let mut normalized = vec![0; crate::SAVE_SIZE];
                normalized[0x30..0x42].copy_from_slice(&save[at + 0x28..at + 0x3a]);
                let name = crate::name::decode_player_name(&normalized[0x30..0x42]);
                let id = format!("{}#{}", file.path, box_index + 1);
                let label = if files.len() > 1 {
                    format!(
                        "Save {} · Box {} · {}",
                        file.path.split('/').nth(1).unwrap_or(&file.path),
                        box_index + 1,
                        name
                    )
                } else {
                    format!("Box {} · {}", box_index + 1, name)
                };
                choices.push(CharacterBox {
                    id,
                    label,
                    box_number: (box_index + 1) as u8,
                });
            }
        }
        let chosen = match selected {
            Some(id) => choices.iter().find(|slot| slot.id == id),
            None => choices.first(),
        }
        .ok_or_else(|| bad("no occupied Xbox character box matches the selection"))?;
        let (path, number) = chosen.id.rsplit_once('#').expect("generated selection id");
        let file_index = files
            .iter()
            .position(|f| f.path == path)
            .expect("generated file path");
        let box_index = number.parse::<usize>().expect("generated box number") - 1;
        Ok(Self {
            image: bytes,
            files,
            choices,
            file_index,
            box_index,
            memory_unit,
            created_slot: None,
        })
    }
    pub(crate) fn matches_image(&self, bytes: &[u8]) -> bool {
        self.image == bytes
    }

    #[must_use]
    pub fn choices(&self) -> &[CharacterBox] {
        &self.choices
    }
    #[must_use]
    pub fn selected(&self) -> String {
        format!(
            "{}#{}",
            self.files[self.file_index].path,
            self.box_index + 1
        )
    }
    #[must_use]
    pub fn memory_unit(&self) -> bool {
        self.memory_unit
    }
    pub fn data(&self) -> Result<SaveData> {
        let save = self.files[self.file_index].read(&self.image);
        let mut raw = vec![0; crate::SAVE_SIZE];
        for mirror in 0..2 {
            let at = mirror * crate::BLOCK;
            raw[at + 4..at + 8].copy_from_slice(&4u32.to_le_bytes());
            let src = mirror * RECORD_SIZE + 4 + self.box_index * SLOT_SIZE;
            raw[at + 8..at + 8 + SLOT_SIZE].copy_from_slice(
                self.created_slot
                    .as_deref()
                    .unwrap_or(&save[src..src + SLOT_SIZE]),
            );
        }
        crate::fix_checksums(&mut raw);
        SaveData::parse(&raw)
    }
    /// Empty boxes in the currently selected save container, requiring agreement of both records.
    #[must_use]
    pub fn empty_boxes(&self) -> Vec<u8> {
        let save = self.files[self.file_index].read(&self.image);
        (0..4)
            .filter(|&index| {
                !(self.created_slot.is_some() && index == self.box_index)
                    && (0..2).all(|mirror| save[mirror * RECORD_SIZE + 4 + index * SLOT_SIZE] == 0)
            })
            .map(|index| index as u8 + 1)
            .collect()
    }

    /// Initialize a full empty slot. Do not mutate the retained source image: it is
    /// also the baseline for detecting external writes before Save.
    pub fn create_box(&mut self, number: u8, spec: &crate::SaveSpec) -> Result<SaveData> {
        if !self.empty_boxes().contains(&number) {
            return Err(bad(
                "Choose an empty Xbox character box from 1 through 4; both records must agree",
            ));
        }
        if self.created_slot.is_some() {
            return Err(bad(
                "Save or reopen before creating another Xbox character box",
            ));
        }
        // Reuse shared validation and initialization, including a fresh identity.
        let save = self.files[self.file_index].read(&self.image);
        let mut raw = vec![0; crate::SAVE_SIZE];
        for mirror in 0..2 {
            let dest = mirror * crate::BLOCK;
            raw[dest + 4..dest + 8].copy_from_slice(&4u32.to_le_bytes());
            let src = mirror * RECORD_SIZE + 4;
            raw[dest + 8..dest + 8 + 4 * SLOT_SIZE]
                .copy_from_slice(&save[src..src + 4 * SLOT_SIZE]);
        }
        crate::fix_checksums(&mut raw);
        let mut data = SaveData::parse(&raw)?;
        data.create_box(number, spec)?;
        // Avoid identities in every container on a memory unit, including unused records.
        let mut ids = HashSet::new();
        for file in &self.files {
            let bytes = file.read(&self.image);
            for mirror in 0..2 {
                for index in 0..4 {
                    ids.insert(word(&bytes, mirror * RECORD_SIZE + 8 + index * SLOT_SIZE));
                }
            }
        }
        let mut unique = data.unique();
        while unique == 0 || ids.contains(&unique) {
            unique = unique.wrapping_add(1);
        }
        data.set_u32(crate::offsets::UNIQUE, unique);
        let at = 8 + usize::from(number - 1) * SLOT_SIZE;
        let slot = data.as_bytes()[at..at + SLOT_SIZE].to_vec();
        let index = usize::from(number - 1);
        let id = format!("{}#{}", self.files[self.file_index].path, number);
        let label = if self.files.len() > 1 {
            format!(
                "Save {} · Box {} · {}",
                self.files[self.file_index]
                    .path
                    .split('/')
                    .nth(1)
                    .unwrap_or(&self.files[self.file_index].path),
                number,
                spec.player_name
            )
        } else {
            format!("Box {} · {}", number, spec.player_name)
        };
        self.box_index = index;
        self.created_slot = Some(slot);
        self.choices.push(CharacterBox {
            id,
            label,
            box_number: number,
        });
        self.choices.sort_by(|a, b| a.id.cmp(&b.id));
        self.data()
    }

    /// Write a full newly created slot, or patch known fields while retaining existing padding.
    pub fn render(&self, data: &SaveData, export_file: bool) -> Result<Vec<u8>> {
        if data.byte_order() != crate::ByteOrder::Little {
            return Err(bad("Xbox character data must be little-endian"));
        }
        let file = &self.files[self.file_index];
        let mut save = file.read(&self.image);
        for mirror in 0..2 {
            let dest = mirror * RECORD_SIZE + 4 + self.box_index * SLOT_SIZE;
            if self.created_slot.is_some() {
                let src = mirror * crate::BLOCK + 8;
                save[dest..dest + SLOT_SIZE]
                    .copy_from_slice(&data.as_bytes()[src..src + SLOT_SIZE]);
            }
            for field in crate::offsets::FIELDS
                .iter()
                .filter(|field| field.offset >= 8)
            {
                let src = mirror * crate::BLOCK + field.offset;
                let target = dest + field.offset - 8;
                save[target..target + field.len]
                    .copy_from_slice(&data.as_bytes()[src..src + field.len]);
            }
        }
        resign(&mut save);
        if !self.memory_unit || export_file {
            return Ok(save);
        }
        let mut image = self.image.clone();
        file.replace(&mut image, &save)?;
        Ok(image)
    }
}

struct Fatx<'a> {
    bytes: &'a [u8],
    cluster_size: usize,
    data_offset: usize,
    fat_width: usize,
    clusters: usize,
    root: u32,
}
impl<'a> Fatx<'a> {
    fn parse(bytes: &'a [u8]) -> Result<Self> {
        if !is_memory_unit(bytes)
            || bytes.len() < 0x3000
            || bytes.len() > 128 * 1024 * 1024
            || !bytes.len().is_multiple_of(4096)
        {
            return Err(bad(
                "unsupported Xbox memory-unit image size (maximum 128 MiB)",
            ));
        }
        let sectors = word(bytes, 8) as usize;
        if !sectors.is_power_of_two() || sectors > 128 {
            return Err(bad("invalid memory-unit cluster geometry"));
        }
        let cluster_size = sectors * 4096;
        let entries = bytes.len() / cluster_size + 1;
        let fat_width = if entries < 0xfff0 { 2 } else { 4 };
        let data_offset = 4096 + (entries * fat_width).div_ceil(4096) * 4096;
        if data_offset >= bytes.len() {
            return Err(bad("memory-unit FAT exceeds image"));
        }
        let clusters = (bytes.len() - data_offset) / cluster_size;
        let fs = Self {
            bytes,
            cluster_size,
            data_offset,
            fat_width,
            clusters,
            root: word(bytes, 12),
        };
        fs.chain(fs.root)?;
        Ok(fs)
    }
    fn chain(&self, start: u32) -> Result<Vec<u32>> {
        let mut seen = HashSet::new();
        let mut chain = Vec::new();
        let mut cluster = start;
        loop {
            if cluster == 0 || cluster as usize > self.clusters || !seen.insert(cluster) {
                return Err(bad(format!(
                    "invalid or looping memory-unit cluster chain at {cluster}"
                )));
            }
            chain.push(cluster);
            let at = 4096 + cluster as usize * self.fat_width;
            if at + self.fat_width > self.data_offset {
                return Err(bad("memory-unit FAT index exceeds table"));
            }
            let next = if self.fat_width == 2 {
                u16::from_le_bytes(self.bytes[at..at + 2].try_into().expect("checked range")) as u32
            } else {
                word(self.bytes, at)
            };
            let end = if self.fat_width == 2 {
                0xfff8
            } else {
                0xfffffff8
            };
            if next >= end {
                break;
            }
            cluster = next;
        }
        Ok(chain)
    }
    fn cluster_offset(&self, cluster: u32) -> usize {
        self.data_offset + (cluster as usize - 1) * self.cluster_size
    }
    fn saves(&self) -> Result<Vec<FileEntry>> {
        let mut queue = vec![(String::new(), self.root, 0usize)];
        let mut allocated = HashSet::new();
        let mut files = Vec::new();
        while let Some((path, first, size)) = queue.pop() {
            let chain = self.chain(first)?;
            for cluster in &chain {
                if !allocated.insert(*cluster) {
                    return Err(bad("memory-unit files or directories share clusters"));
                }
            }
            if size > 0 {
                if size > chain.len() * self.cluster_size {
                    return Err(bad("memory-unit file has a truncated cluster chain"));
                }
                let mut left = size;
                let spans = chain
                    .iter()
                    .take(size.div_ceil(self.cluster_size))
                    .map(|&cluster| {
                        let len = left.min(self.cluster_size);
                        left -= len;
                        (self.cluster_offset(cluster), len)
                    })
                    .collect();
                let parts: Vec<_> = path.split('/').collect();
                if parts.len() == 3
                    && parts[0].eq_ignore_ascii_case("42410002")
                    && parts[2].eq_ignore_ascii_case("SaveData.dat")
                {
                    if size != FILE_SIZE {
                        return Err(bad(format!("{path}: incorrect Xbox save size")));
                    }
                    files.push(FileEntry { path, spans, size });
                }
                continue;
            }
            let mut ended = false;
            let mut names = HashSet::new();
            for cluster in chain {
                let at = self.cluster_offset(cluster);
                for entry in self.bytes[at..at + self.cluster_size].as_chunks::<64>().0 {
                    let len = entry[0] as usize;
                    if len == 0 || len == 0xff {
                        ended = true;
                        break;
                    }
                    if len == 0xe5 {
                        continue;
                    }
                    if len > 42 {
                        return Err(bad("invalid memory-unit filename length"));
                    }
                    let name = std::str::from_utf8(&entry[2..2 + len])
                        .map_err(|_| bad("invalid memory-unit filename"))?;
                    if name.contains(['/', '\\']) || !names.insert(name.to_ascii_lowercase()) {
                        return Err(bad("invalid or duplicate memory-unit filename"));
                    }
                    let child = if path.is_empty() {
                        name.into()
                    } else {
                        format!("{path}/{name}")
                    };
                    let start = word(entry, 44);
                    let size = word(entry, 48) as usize;
                    if entry[1] & 0x10 != 0 {
                        queue.push((child, start, 0));
                    } else if size > 0 {
                        queue.push((child, start, size));
                    } else if start != 0 {
                        return Err(bad("unsupported allocated empty memory-unit file"));
                    }
                }
                if ended {
                    break;
                }
            }
        }
        files.sort_by(|a, b| a.path.cmp(&b.path));
        if files.is_empty() {
            return Err(bad("no Digimon World 4 save found on the Xbox memory unit"));
        }
        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn put(bytes: &mut [u8], at: usize, value: u32) {
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn fixture() -> Vec<u8> {
        let mut file = vec![0; FILE_SIZE];
        for mirror in 0..2 {
            put(&mut file, mirror * RECORD_SIZE, 4);
            for box_index in 0..4 {
                let spec = crate::SaveSpec {
                    player_name: format!("BOX{}", box_index + 1),
                    ..Default::default()
                };
                let raw = crate::build_save(&spec);
                let at = mirror * RECORD_SIZE + 4 + box_index * SLOT_SIZE;
                file[at..at + SLOT_SIZE].copy_from_slice(&raw[8..8 + SLOT_SIZE]);
                file[at + SLOT_SIZE - 1] = (box_index + mirror + 10) as u8;
            }
        }
        resign(&mut file);
        file
    }
    fn entry(image: &mut [u8], at: usize, name: &str, directory: bool, cluster: u32, size: usize) {
        image[at] = name.len() as u8;
        image[at + 1] = if directory { 0x10 } else { 0x20 };
        image[at + 2..at + 2 + name.len()].copy_from_slice(name.as_bytes());
        put(image, at + 44, cluster);
        put(image, at + 48, size as u32);
    }
    fn memory_unit(file: &[u8]) -> Vec<u8> {
        let mut image = vec![0; 8 * 1024 * 1024];
        image[..4].copy_from_slice(b"FATX");
        put(&mut image, 8, 4);
        put(&mut image, 12, 1);
        for cluster in 0..12 {
            image[4096 + cluster * 2..4098 + cluster * 2].copy_from_slice(&0xffffu16.to_le_bytes());
        }
        let offset = |cluster: usize| 8192 + (cluster - 1) * 16384;
        entry(&mut image, offset(1), "42410002", true, 2, 0);
        entry(&mut image, offset(1) + 64, "unrelated.bin", false, 4, 32);
        image[offset(4)..offset(4) + 32].fill(0x67);
        entry(&mut image, offset(2), "ABCDEF012345", true, 3, 0);
        entry(&mut image, offset(3), "SaveData.dat", false, 5, FILE_SIZE);
        let chain = [5, 8, 6, 9, 7];
        for (i, cluster) in chain.into_iter().enumerate() {
            if i + 1 < chain.len() {
                image[4096 + cluster * 2..4098 + cluster * 2]
                    .copy_from_slice(&(chain[i + 1] as u16).to_le_bytes());
            }
            let start = i * 16384;
            let len = (FILE_SIZE - start).min(16384);
            image[offset(cluster)..offset(cluster) + len]
                .copy_from_slice(&file[start..start + len]);
        }
        image
    }
    fn with_empty(number: usize) -> Vec<u8> {
        let mut bytes = fixture();
        for mirror in 0..2 {
            let at = mirror * RECORD_SIZE + 4 + (number - 1) * SLOT_SIZE;
            bytes[at..at + SLOT_SIZE].fill(0xcc);
            bytes[at] = 0;
        }
        resign(&mut bytes);
        bytes
    }

    #[test]
    fn creates_each_empty_box_with_new_identity_and_full_initialization() {
        for number in 1..=4 {
            let original = with_empty(number);
            let mut source = XboxSource::parse(original.clone(), None).unwrap();
            assert_eq!(source.empty_boxes(), [number as u8]);
            let spec = crate::SaveSpec {
                player_name: "NEWあ".into(),
                ..Default::default()
            };
            let mut data = source.create_box(number as u8, &spec).unwrap();
            assert_ne!(data.unique(), 0);
            assert_ne!(data.unique(), crate::SaveSpec::default().unique);
            assert_eq!(data.player_name(), "NEWあ");
            assert!(source.empty_boxes().is_empty());
            assert!(source.matches_image(&original));
            data.set_bit(456);
            let output = source.render(&data, false).unwrap();
            assert_eq!(verify_save(&output).unwrap(), [true, true]);
            for mirror in 0..2 {
                let at = mirror * RECORD_SIZE + 4 + (number - 1) * SLOT_SIZE;
                assert_eq!(
                    output[at + SLOT_SIZE - 1],
                    0,
                    "dirty empty padding initialized"
                );
                let signature = mirror * RECORD_SIZE + SIGNED_SIZE..(mirror + 1) * RECORD_SIZE;
                for i in mirror * RECORD_SIZE..(mirror + 1) * RECORD_SIZE {
                    if !(at..at + SLOT_SIZE).contains(&i) && !signature.contains(&i) {
                        assert_eq!(original[i], output[i], "other bytes at {i:x}");
                    }
                }
            }
            let check = XboxSource::parse(output, Some(&source.selected())).unwrap();
            assert_eq!(check.data().unwrap().bit(), 456);
            assert_eq!(check.data().unwrap().unique(), data.unique());
        }
    }

    #[test]
    fn creation_rejects_bad_targets_and_names_without_changing_source_or_selection() {
        let original = with_empty(3);
        let mut source = XboxSource::parse(original.clone(), None).unwrap();
        let selected = source.selected();
        for number in [0, 1, 5] {
            assert!(
                source
                    .create_box(number, &crate::SaveSpec::default())
                    .is_err()
            );
        }
        for name in ["", "  ", "123456789", "bad\n", "😀"] {
            assert!(
                source
                    .create_box(
                        3,
                        &crate::SaveSpec {
                            player_name: name.into(),
                            ..Default::default()
                        }
                    )
                    .is_err()
            );
            assert_eq!(source.selected(), selected);
            assert!(source.matches_image(&original));
            assert_eq!(source.empty_boxes(), [3]);
        }
        for flag in [1, 2, 255] {
            let mut disagree = original.clone();
            disagree[RECORD_SIZE + 4 + 2 * SLOT_SIZE] = flag;
            resign(&mut disagree);
            let mut source = XboxSource::parse(disagree, None).unwrap();
            assert!(source.empty_boxes().is_empty());
            assert!(source.create_box(3, &crate::SaveSpec::default()).is_err());
        }
    }

    #[test]
    fn creation_on_multi_container_fragmented_mu_preserves_other_saves_and_metadata() {
        let file = with_empty(2);
        let mut original = memory_unit(&file);
        let offset = |cluster: usize| 8192 + (cluster - 1) * 16384;
        entry(&mut original, offset(2) + 64, "SECOND", true, 10, 0);
        entry(
            &mut original,
            offset(10),
            "SaveData.dat",
            false,
            11,
            FILE_SIZE,
        );
        for cluster in 11..=15 {
            let next = if cluster == 15 { 0xffff } else { cluster + 1 } as u16;
            original[4096 + cluster * 2..4098 + cluster * 2].copy_from_slice(&next.to_le_bytes());
            let start = (cluster - 11) * 16384;
            let len = (FILE_SIZE - start).min(16384);
            original[offset(cluster)..offset(cluster) + len]
                .copy_from_slice(&file[start..start + len]);
        }
        let id = "42410002/SECOND/SaveData.dat#4";
        let mut source = XboxSource::parse(original.clone(), Some(id)).unwrap();
        let data = source.create_box(2, &crate::SaveSpec::default()).unwrap();
        assert_eq!(source.selected(), "42410002/SECOND/SaveData.dat#2");
        assert!(
            source
                .choices()
                .iter()
                .any(|choice| choice.label.starts_with("Save SECOND · Box 2"))
        );
        let output = source.render(&data, false).unwrap();
        let file = &source.files[source.file_index];
        for (i, (before, after)) in original.iter().zip(&output).enumerate() {
            if !file
                .spans
                .iter()
                .any(|&(at, len)| (at..at + len).contains(&i))
            {
                assert_eq!(before, after, "unrelated image byte {i:x}");
            }
        }
        let exported = source.render(&data, true).unwrap();
        assert_eq!(verify_save(&exported).unwrap(), [true, true]);
        assert!(
            XboxSource::parse(output, Some(&source.selected()))
                .unwrap()
                .empty_boxes()
                .is_empty()
        );
    }

    #[test]
    fn created_box_document_writes_backup_and_rejects_external_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("save.dat");
        let original = with_empty(4);
        std::fs::write(&path, &original).unwrap();
        let mut doc = crate::document::Document::load(&path).unwrap();
        doc.create_box(4, &crate::SaveSpec::default()).unwrap();
        let mut external = original.clone();
        external[100] ^= 1;
        resign(&mut external);
        std::fs::write(&path, &external).unwrap();
        assert!(doc.save(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), external);
        assert!(!path.with_extension("dat.bak").exists());
        std::fs::write(&path, &original).unwrap();
        doc.save(&path).unwrap();
        assert_eq!(
            std::fs::read(path.with_extension("dat.bak")).unwrap(),
            original
        );
        assert_eq!(doc.character_selection().as_deref(), Some("SaveData.dat#4"));
    }

    #[test]
    fn every_box_maps_and_edits_without_touching_other_boxes_or_padding() {
        let original = fixture();
        for box_index in 0..4 {
            let id = format!("SaveData.dat#{}", box_index + 1);
            let source = XboxSource::parse(original.clone(), Some(&id)).unwrap();
            let mut data = source.data().unwrap();
            assert_eq!(data.player_name(), format!("BOX{}", box_index + 1));
            data.set_bit(123456);
            data.set_bank_bit(654321);
            data.set_disk_count(2, 7);
            data.set_level(crate::Species::Dorumon, 17);
            data.set_skill(crate::Species::Dorumon, 8, 99);
            let edited = source.render(&data, false).unwrap();
            assert_eq!(verify_save(&edited).unwrap(), [true, true]);
            for mirror in 0..2 {
                let at = mirror * RECORD_SIZE + 4 + box_index * SLOT_SIZE;
                let allowed = at..at + crate::offsets::PAD_END - 8;
                let signature = mirror * RECORD_SIZE + SIGNED_SIZE..(mirror + 1) * RECORD_SIZE;
                for (i, (a, b)) in original.iter().zip(&edited).enumerate() {
                    if i / RECORD_SIZE == mirror && !allowed.contains(&i) && !signature.contains(&i)
                    {
                        assert_eq!(a, b, "unexpected change at {i:x}");
                    }
                }
            }
            let check = XboxSource::parse(edited, Some(&id))
                .unwrap()
                .data()
                .unwrap();
            assert_eq!(check.bit(), 123456);
            assert_eq!(check.bank_bit(), 654321);
            assert_eq!(check.disk_count(2), 7);
            assert_eq!(check.level(crate::Species::Dorumon), 17);
            assert_eq!(check.skill(crate::Species::Dorumon, 8), 99);
        }
    }
    #[test]
    fn fragmented_memory_unit_roundtrip_preserves_filesystem_and_unrelated_data() {
        let original = memory_unit(&fixture());
        let source = XboxSource::parse(
            original.clone(),
            Some("42410002/ABCDEF012345/SaveData.dat#4"),
        )
        .unwrap();
        assert_eq!(source.choices().len(), 4);
        let mut data = source.data().unwrap();
        data.set_bit(888);
        let edited = source.render(&data, false).unwrap();
        let check = XboxSource::parse(edited.clone(), Some(&source.selected())).unwrap();
        assert_eq!(check.data().unwrap().bit(), 888);
        assert_eq!(&original[..8192], &edited[..8192]);
        let file = &source.files[0];
        for (i, (a, b)) in original.iter().zip(&edited).enumerate() {
            if !file
                .spans
                .iter()
                .any(|&(at, len)| (at..at + len).contains(&i))
            {
                assert_eq!(a, b, "image change at {i:x}");
            }
        }
        let exported = source.render(&data, true).unwrap();
        assert_eq!(exported.len(), FILE_SIZE);
        assert_eq!(
            XboxSource::parse(exported, Some("SaveData.dat#4"))
                .unwrap()
                .data()
                .unwrap()
                .bit(),
            888
        );
    }
    #[test]
    fn document_save_keeps_selection_backup_and_exports_memory_units() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("save.dat");
        let original = fixture();
        std::fs::write(&path, &original).unwrap();
        let mut doc =
            crate::document::Document::load_with_selection(&path, None, Some("SaveData.dat#2"))
                .unwrap();
        doc.data_mut().set_bit(222);
        doc.save(&path).unwrap();
        assert_eq!(doc.character_selection().as_deref(), Some("SaveData.dat#2"));
        assert_eq!(
            std::fs::read(temp.path().join("save.dat.bak")).unwrap(),
            original
        );
        let reopened =
            crate::document::Document::load_with_selection(&path, None, Some("SaveData.dat#2"))
                .unwrap();
        assert_eq!(reopened.data().bit(), 222);
        doc.data_mut().set_bit(333);
        doc.save(&path).unwrap();
        assert_eq!(
            std::fs::read(temp.path().join("save.dat.bak")).unwrap(),
            original
        );
        let mu = temp.path().join("card.img");
        std::fs::write(&mu, memory_unit(&original)).unwrap();
        let mut doc = crate::document::Document::load_with_selection(
            &mu,
            None,
            Some("42410002/ABCDEF012345/SaveData.dat#4"),
        )
        .unwrap();
        doc.data_mut().set_bit(444);
        let output = temp.path().join("export.dat");
        doc.save(&output).unwrap();
        assert_eq!(doc.character_selection().as_deref(), Some("SaveData.dat#4"));
        assert_eq!(doc.data().bit(), 444);
        let source =
            XboxSource::parse(std::fs::read(&output).unwrap(), Some("SaveData.dat#1")).unwrap();
        assert_eq!(source.data().unwrap().bit(), 0);
        assert_eq!(std::fs::read(mu).unwrap(), memory_unit(&original));
    }

    #[test]
    fn refuses_to_overwrite_external_changes_and_wrong_export_formats() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("save.dat");
        let original = fixture();
        std::fs::write(&path, &original).unwrap();
        let mut doc = crate::document::Document::load(&path).unwrap();
        doc.data_mut().set_bit(100);
        let mut changed = original.clone();
        changed[123] ^= 1;
        std::fs::write(&path, &changed).unwrap();
        assert!(doc.save(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), changed);
        assert!(!temp.path().join("save.dat.bak").exists());
        assert!(doc.save(&temp.path().join("wrong.ps2")).is_err());
        assert!(!temp.path().join("wrong.ps2").exists());
    }

    #[test]
    fn normal_xbox_bank_edits_pack_holes_without_losing_items() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("save.dat");
        std::fs::write(&path, fixture()).unwrap();
        let mut doc = crate::document::Document::load(&path).unwrap();
        let mut edits = doc.view(crate::Mode::Normal).to_edit_set();
        edits.bank_items[0] = 0x0019;
        edits.bank_items[95] = 0x1000;
        doc.apply(&edits, crate::Mode::Normal).unwrap();
        assert_eq!(doc.data().bank_device(0), 0x0019);
        assert_eq!(doc.data().bank_device(1), 0x1000);
        assert_eq!(doc.data().bank_device(95), crate::EMPTY);
        doc.apply(&edits, crate::Mode::Advanced).unwrap();
        assert_eq!(doc.data().bank_device(1), crate::EMPTY);
        assert_eq!(doc.data().bank_device(95), 0x1000);
        edits.bank_items.push(crate::EMPTY);
        assert!(doc.apply(&edits, crate::Mode::Normal).is_err());
    }

    #[test]
    fn rejects_bad_signatures_geometry_loops_and_shared_clusters() {
        let mut file = fixture();
        file[123] ^= 1;
        assert!(XboxSource::parse(file, None).is_err());
        let mut image = memory_unit(&fixture());
        put(&mut image, 8, u32::MAX);
        assert!(XboxSource::parse(image, None).is_err());
        let mut image = memory_unit(&fixture());
        image[4096 + 5 * 2..4098 + 5 * 2].copy_from_slice(&5u16.to_le_bytes());
        assert!(XboxSource::parse(image, None).is_err());
        let mut image = memory_unit(&fixture());
        put(&mut image, 8192 + 64 + 44, 5);
        assert!(XboxSource::parse(image, None).is_err());
    }
}
