//! Item ids as stored in the device folder and bank.
//!
//! A stored u32 is `base_id | ((seed << 4 | mod_count) << 16)`, where
//! `base_id` is `category << 8 | index`. The 12-bit `seed` is a **direct
//! additive stat bonus** in its low 11 bits (bit 11 is masked off), and also
//! determines the rarity colour.

use serde::{Deserialize, Serialize};

use crate::EMPTY;

/// Graded weapons: 50 models × 5 grades + 6 unique.
pub const CAT_WEAPON: u8 = 0x00;
/// Styled weapons, flat ids.
pub const CAT_STYLED: u8 = 0x05;
/// Armor, which the game calls cores.
pub const CAT_CORE: u8 = 0x10;
/// Sub-slot equipment, which the game calls boards.
pub const CAT_BOARD: u8 = 0x20;
/// Mod chips in the inventory.
pub const CAT_MOD: u8 = 0x30;
/// The "equipped" mod category byte. Game-managed; never a legal inventory item.
pub const CAT_MOD_EQUIPPED: u8 = 0x34;

/// The item category encoded in a base id's second byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub enum Category {
    /// Graded weapons.
    Weapon,
    /// Styled weapons.
    Styled,
    /// Cores, used in the armor slot.
    Core,
    /// Boards, used in the sub slot.
    Board,
    /// Mod chips.
    Mod,
    /// The game-managed "equipped" mod variant.
    ModEquipped,
    /// Any other category byte.
    Unknown(u8),
}

impl Category {
    /// The category for a base id.
    #[must_use]
    pub fn from_base_id(base_id: u32) -> Category {
        match ((base_id >> 8) & 0xFF) as u8 {
            CAT_WEAPON => Category::Weapon,
            CAT_STYLED => Category::Styled,
            CAT_CORE => Category::Core,
            CAT_BOARD => Category::Board,
            CAT_MOD => Category::Mod,
            CAT_MOD_EQUIPPED => Category::ModEquipped,
            other => Category::Unknown(other),
        }
    }

    /// The category byte.
    #[must_use]
    pub fn byte(self) -> u8 {
        match self {
            Category::Weapon => CAT_WEAPON,
            Category::Styled => CAT_STYLED,
            Category::Core => CAT_CORE,
            Category::Board => CAT_BOARD,
            Category::Mod => CAT_MOD,
            Category::ModEquipped => CAT_MOD_EQUIPPED,
            Category::Unknown(b) => b,
        }
    }

    /// A human label for the category.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Category::Weapon => "Weapon",
            Category::Styled => "Weapon (styled)",
            Category::Core => "Core (armor)",
            Category::Board => "Board (sub)",
            Category::Mod => "Mod (chip)",
            Category::ModEquipped => "Mod (equipped)",
            Category::Unknown(_) => "Unknown category",
        }
    }
}

/// Pack a device-folder u32.
///
/// `seed` is masked to 12 bits and `mod_count` to 4, so out-of-range inputs
/// cannot corrupt the base id.
#[must_use]
pub fn build_item_id(base_id: u32, seed: u16, mod_count: u8) -> u32 {
    let instance = (u32::from(seed & 0x0FFF) << 4) | u32::from(mod_count & 0x0F);
    base_id | (instance << 16)
}

/// Unpack a device-folder u32 into `(base_id, seed, mod_count)`.
#[must_use]
pub fn split_item_id(full: u32) -> (u32, u16, u8) {
    let base_id = full & 0xFFFF;
    let seed = ((full >> 20) & 0x0FFF) as u16;
    let mod_count = ((full >> 16) & 0x0F) as u8;
    (base_id, seed, mod_count)
}

/// The category of a stored item id.
#[must_use]
pub fn category_of(full: u32) -> Category {
    Category::from_base_id(split_item_id(full).0)
}

/// Whether a stored u32 means "this slot is empty".
#[must_use]
pub fn is_empty(full: u32) -> bool {
    full == EMPTY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_base_id_alone_round_trips() {
        assert_eq!(build_item_id(0x050D, 0, 0), 0x050D);
        assert_eq!(split_item_id(0x050D), (0x050D, 0, 0));
    }

    #[test]
    fn the_real_card_ids_decode_to_the_expected_parts() {
        // The three devices in the fixture save: Bash Katana, Shot Pistol, Crush Arm.
        for (full, base) in [(0x050Du32, 0x050Du32), (0x0513, 0x0513), (0x0512, 0x0512)] {
            assert_eq!(split_item_id(full), (base, 0, 0));
            assert_eq!(category_of(full), Category::Styled);
        }
    }

    #[test]
    fn seed_and_mods_land_in_the_upper_half() {
        let full = build_item_id(0x1010, 0x123, 0x7);
        assert_eq!(full, 0x1010 | (0x1237 << 16));
        assert_eq!(split_item_id(full), (0x1010, 0x123, 0x7));
    }

    #[test]
    fn a_full_seed_does_not_bleed_into_the_base_id() {
        let full = build_item_id(0xFFFF, 0xFFF, 0xF);
        assert_eq!(split_item_id(full), (0xFFFF, 0xFFF, 0xF));
    }

    #[test]
    fn seed_and_mods_are_masked_to_their_widths() {
        let full = build_item_id(0x0000, 0xFFFF, 0xFF);
        assert_eq!(split_item_id(full), (0x0000, 0xFFF, 0xF));
    }

    #[test]
    fn categories_come_from_the_second_byte() {
        assert_eq!(category_of(0x0010), Category::Weapon);
        // Graded weapons are category 0x00, so they span 0x0000..=0x00FF.
        assert_eq!(category_of(0x00FF), Category::Weapon);
        assert_eq!(category_of(0x0500), Category::Styled);
        assert_eq!(category_of(0x1020), Category::Core);
        assert_eq!(category_of(0x2020), Category::Board);
        assert_eq!(category_of(0x3000), Category::Mod);
        assert_eq!(category_of(0x3400), Category::ModEquipped);
        assert_eq!(category_of(0x44FF), Category::Unknown(0x44));
        // 0x03FF is category byte 0x03, which is not a known category.
        assert_eq!(category_of(0x03FF), Category::Unknown(0x03));
    }

    #[test]
    fn the_empty_sentinel_is_not_an_item() {
        assert!(is_empty(crate::EMPTY));
        assert!(!is_empty(0));
    }

    #[test]
    fn category_bytes_round_trip() {
        for cat in [
            Category::Weapon,
            Category::Styled,
            Category::Core,
            Category::Board,
            Category::Mod,
            Category::ModEquipped,
            Category::Unknown(0x44),
        ] {
            assert_eq!(Category::from_base_id(u32::from(cat.byte()) << 8), cat);
        }
    }
}
