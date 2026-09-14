//! The 512-byte directory entries that make up the card's filesystem.

/// Size of one directory entry.
pub const ENTRY_SIZE: usize = 512;

/// Entry exists.
pub const MODE_EXISTS: u16 = 0x8000;
/// Entry is hidden.
pub const MODE_HIDDEN: u16 = 0x2000;
/// Entry is a directory.
pub const MODE_DIR: u16 = 0x0020;
/// Entry is a file.
pub const MODE_FILE: u16 = 0x0010;

/// Directory entry mode, as the reference card stores it.
///
/// `EXISTS | 0x0400 | DIR | 0x0007`: the middle bits are the permission set the
/// reference uses, which this crate does not interpret.
pub const MODE_DIR_ENTRY: u16 = 0x8427;

/// File entry mode, as the reference card stores it:
/// `EXISTS | 0x0400 | 0x0080 | FILE | 0x0007`.
pub const MODE_FILE_ENTRY: u16 = 0x8497;

/// The root directory's `..`, which the reference marks hidden.
pub const MODE_ROOT_DOTDOT: u16 = 0xa426;

/// The save directory's `..`, which the reference does not mark hidden.
pub const MODE_SAVE_DIR_DOTDOT: u16 = 0x8427;

/// Offsets within an entry, all little-endian.
mod field {
    /// `mode`, u16.
    pub const MODE: usize = 0x00;
    /// `length`, u32. For a file: bytes. For a directory: **entry count**.
    pub const LENGTH: usize = 0x04;
    /// First cluster of this entry's contents.
    pub const CLUSTER: usize = 0x10;
    /// Name, 32 bytes, NUL-terminated.
    pub const NAME: usize = 0x40;
    /// Length of the name field.
    pub const NAME_LEN: usize = 32;
}

/// One directory entry.
///
/// The raw 512-byte slot is kept so that fields this crate does not interpret —
/// timestamps, attributes, padding — survive a read/modify/write unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    slot: [u8; ENTRY_SIZE],
}

impl Entry {
    /// Build an entry from the fields this crate understands.
    ///
    /// Every other byte is zero. The reference carries timestamps and
    /// attributes we do not interpret; writing zeros there is deliberate, and
    /// the fields we *do* set are pinned against the reference by tests.
    #[must_use]
    pub fn new(mode: u16, length: u32, cluster: u32, name: &str) -> Self {
        let mut slot = [0u8; ENTRY_SIZE];
        slot[field::MODE..field::MODE + 2].copy_from_slice(&mode.to_le_bytes());
        slot[field::LENGTH..field::LENGTH + 4].copy_from_slice(&length.to_le_bytes());
        slot[field::CLUSTER..field::CLUSTER + 4].copy_from_slice(&cluster.to_le_bytes());
        let bytes = name.as_bytes();
        let take = bytes.len().min(field::NAME_LEN);
        slot[field::NAME..field::NAME + take].copy_from_slice(&bytes[..take]);
        Self { slot }
    }

    /// Decode the entry in `bytes`, which must be at least [`ENTRY_SIZE`] long.
    #[must_use]
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < ENTRY_SIZE {
            return None;
        }
        let mut slot = [0u8; ENTRY_SIZE];
        slot.copy_from_slice(&bytes[..ENTRY_SIZE]);
        Some(Self { slot })
    }

    /// The entry's mode flags.
    #[must_use]
    pub fn mode(&self) -> u16 {
        u16::from_le_bytes([self.slot[field::MODE], self.slot[field::MODE + 1]])
    }

    /// Whether the entry is present at all.
    #[must_use]
    pub fn exists(&self) -> bool {
        self.mode() & MODE_EXISTS != 0
    }

    /// Whether the entry is a directory.
    #[must_use]
    pub fn is_dir(&self) -> bool {
        self.mode() & (MODE_DIR | MODE_EXISTS) == (MODE_DIR | MODE_EXISTS)
    }

    /// Whether the entry is a file.
    #[must_use]
    pub fn is_file(&self) -> bool {
        self.mode() & (MODE_FILE | MODE_EXISTS) == (MODE_FILE | MODE_EXISTS)
    }

    /// Bytes for a file, or the **entry count** for a directory.
    ///
    /// A directory's children are bounded by *this* value on the parent entry,
    /// not by the directory's own `.` entry. That rule is why `ps2-memcard`
    /// fails on this card: the save directory's `.` holds 0 while the parent
    /// entry correctly holds 5.
    #[must_use]
    pub fn length(&self) -> u32 {
        u32::from_le_bytes([
            self.slot[field::LENGTH],
            self.slot[field::LENGTH + 1],
            self.slot[field::LENGTH + 2],
            self.slot[field::LENGTH + 3],
        ])
    }

    /// The first cluster of the entry's contents, relative to `alloc_offset`.
    #[must_use]
    pub fn cluster(&self) -> u32 {
        u32::from_le_bytes([
            self.slot[field::CLUSTER],
            self.slot[field::CLUSTER + 1],
            self.slot[field::CLUSTER + 2],
            self.slot[field::CLUSTER + 3],
        ])
    }

    /// The entry's name, up to the first NUL.
    #[must_use]
    pub fn name(&self) -> String {
        let raw = &self.slot[field::NAME..field::NAME + field::NAME_LEN];
        let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
        String::from_utf8_lossy(&raw[..end]).into_owned()
    }

    /// The raw 512-byte slot.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; ENTRY_SIZE] {
        &self.slot
    }

    /// The slot, for a read/modify/write cycle.
    #[must_use]
    pub fn encode(&self) -> [u8; ENTRY_SIZE] {
        self.slot
    }

    /// Whether this is a `.` or `..` entry.
    #[must_use]
    pub fn is_dot(&self) -> bool {
        self.name().starts_with('.')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_builds_the_fields_a_reference_entry_carries() {
        let entry = Entry::new(MODE_DIR_ENTRY, 3, 0, ".");
        assert_eq!(entry.mode(), MODE_DIR_ENTRY);
        assert_eq!(entry.length(), 3);
        assert_eq!(entry.cluster(), 0);
        assert_eq!(entry.name(), ".");
        assert!(entry.exists() && entry.is_dir() && !entry.is_file());
    }

    #[test]
    fn a_file_entry_is_a_file() {
        let entry = Entry::new(MODE_FILE_ENTRY, 81_920, 39, "BASLUS-20836savedata");
        assert!(entry.is_file() && !entry.is_dir());
        assert_eq!(entry.length(), 81_920);
        assert_eq!(entry.cluster(), 39);
    }

    #[test]
    fn a_long_name_is_truncated_to_the_field() {
        let entry = Entry::new(MODE_FILE_ENTRY, 1, 1, &"x".repeat(64));
        assert_eq!(entry.name().len(), 32);
    }

    #[test]
    fn round_trips_through_parse() {
        let built = Entry::new(MODE_FILE_ENTRY, 964, 119, "icon.sys");
        let parsed = Entry::parse(&built.encode()).expect("parses");
        assert_eq!(parsed, built);
    }
}
