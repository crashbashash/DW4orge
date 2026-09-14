//! The player name: a `0xFFFF` marker then up to eight fullwidth code points.
//!
//! The save stores `PLAYERNAME` as UTF-16 in which the printable ASCII range has
//! been shifted into the fullwidth forms block. `A` is `0xFF21`, `!` is
//! `0xFF01`, and a space is `U+3000` (ideographic space) rather than `0x0020`.
//! Anything outside `0x21..=0x7E` is stored verbatim, matching the Python
//! editor.

use crate::offsets;
use crate::save::SaveData;

/// How many characters a player name can hold.
///
/// Measured from the save writer `FUN_003ee6d0`: the copy loop fills at most
/// eight `u16` slots, NUL-padding the rest, then writes the `0xFFFF` marker.
pub const NAME_CHARS: usize = 8;

/// Bytes of the `PLAYERNAME` field: the marker plus [`NAME_CHARS`] UTF-16 units.
pub const NAME_FIELD_LEN: usize = 2 + NAME_CHARS * 2;

/// The marker occupying the first `u16` of the field.
const NAME_MARKER: u16 = 0xFFFF;

/// Encode one character as the fullwidth code unit the game stores.
#[must_use]
pub fn char_to_fullwidth(ch: char) -> u16 {
    let o = ch as u32;
    if (0x21..=0x7E).contains(&o) {
        // Shift ASCII into the fullwidth forms block.
        (o + 0xFEE0) as u16
    } else if ch == ' ' {
        0x3000
    } else {
        // Everything else is stored as-is; only ASCII is remapped.
        u16::try_from(o).unwrap_or(0xFFFD)
    }
}

/// Decode one stored code unit back to a character.
#[must_use]
pub fn fullwidth_to_char(unit: u16) -> char {
    let o = u32::from(unit);
    let o = if (0xFF01..=0xFF5E).contains(&o) {
        o - 0xFEE0
    } else if o == 0x3000 {
        0x20
    } else {
        o
    };
    char::from_u32(o).unwrap_or('\u{FFFD}')
}

/// Encode a name into the field: marker, up to [`NAME_CHARS`] chars, NUL padding.
#[must_use]
pub fn encode_player_name(text: &str) -> [u8; NAME_FIELD_LEN] {
    let mut out = [0u8; NAME_FIELD_LEN];
    out[..2].copy_from_slice(&NAME_MARKER.to_le_bytes());
    for (i, ch) in text.chars().take(NAME_CHARS).enumerate() {
        let at = 2 + i * 2;
        out[at..at + 2].copy_from_slice(&char_to_fullwidth(ch).to_le_bytes());
    }
    out
}

/// Decode the 8-byte field, skipping the marker and any NUL slots.
#[must_use]
pub fn decode_player_name(field: &[u8]) -> String {
    let mut out = String::new();
    for i in 0..NAME_CHARS {
        let at = 2 + i * 2;
        if at + 2 > field.len() {
            break;
        }
        let unit = u16::from_le_bytes([field[at], field[at + 1]]);
        if unit == 0 || unit == NAME_MARKER {
            continue;
        }
        out.push(fullwidth_to_char(unit));
    }
    out
}

impl SaveData {
    /// The player name, decoded from its fullwidth encoding.
    #[must_use]
    pub fn player_name(&self) -> String {
        decode_player_name(self.get_bytes(offsets::PLAYER_NAME, NAME_FIELD_LEN))
    }

    /// Write the player name, truncating to 8 characters.
    pub fn set_player_name(&mut self, text: &str) {
        self.set_bytes(offsets::PLAYER_NAME, &encode_player_name(text));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_letters_become_fullwidth() {
        assert_eq!(char_to_fullwidth('A'), 0xFF21);
        assert_eq!(char_to_fullwidth('a'), 0xFF41);
        assert_eq!(char_to_fullwidth('0'), 0xFF10);
        assert_eq!(char_to_fullwidth('!'), 0xFF01);
    }

    #[test]
    fn space_is_ideographic_not_a_fullwidth_space() {
        // The game stores a space as U+3000, not U+0020 and not U+FF00.
        assert_eq!(char_to_fullwidth(' '), 0x3000);
        assert_eq!(fullwidth_to_char(0x3000), ' ');
    }

    #[test]
    fn a_non_ascii_character_passes_through_unchanged() {
        // The Python editor does not remap anything outside 0x21..=0x7E.
        assert_eq!(char_to_fullwidth('\u{3c0}'), 0x03C0);
        assert_eq!(fullwidth_to_char(0x03C0), '\u{3c0}');
    }

    #[test]
    fn code_points_round_trip() {
        for ch in "Az09!~ ".chars() {
            assert_eq!(fullwidth_to_char(char_to_fullwidth(ch)), ch, "char {ch:?}");
        }
    }

    #[test]
    fn the_encoding_starts_with_the_ffff_marker() {
        let field = encode_player_name("abc");
        assert_eq!(&field[..2], &[0xFF, 0xFF]);
        assert_eq!(&field[2..4], &0xFF41u16.to_le_bytes());
        assert_eq!(&field[4..6], &0xFF42u16.to_le_bytes());
        assert_eq!(&field[6..8], &0xFF43u16.to_le_bytes());
    }

    #[test]
    fn eight_characters_fill_the_whole_field() {
        let field = encode_player_name("abcdefgh");
        assert_eq!(field.len(), NAME_FIELD_LEN);
        assert_eq!(&field[..2], &[0xFF, 0xFF]);
        assert_eq!(&field[2..4], &0xFF41u16.to_le_bytes());
        assert_eq!(&field[16..18], &0xFF48u16.to_le_bytes());
        assert_eq!(decode_player_name(&field), "abcdefgh");
    }

    #[test]
    fn names_longer_than_eight_are_truncated() {
        assert_eq!(
            decode_player_name(&encode_player_name("abcdefghij")),
            "abcdefgh"
        );
    }

    #[test]
    fn the_field_length_matches_the_offset_table() {
        // The offset map names the padding after the field; it must not claim
        // bytes the codec writes, or the two would silently disagree.
        assert_eq!(
            crate::offsets::PLAYER_NAME_PAD - crate::offsets::PLAYER_NAME,
            NAME_FIELD_LEN
        );
    }

    #[test]
    fn short_names_are_nul_padded() {
        let field = encode_player_name("a");
        assert_eq!(&field[4..8], &[0u8; 4]);
        assert_eq!(decode_player_name(&field), "a");
    }

    #[test]
    fn decoding_skips_the_marker_and_nul_slots() {
        assert_eq!(decode_player_name(&[0xFF, 0xFF, 0, 0, 0, 0, 0, 0]), "");
    }
}
