//! Pure value semantics of the save format: the level curve, junk-shop tiers,
//! the rarity/seed model, and the caps that separate Normal from Advanced mode.
//!
//! Nothing here touches bytes. Everything is exhaustively testable.

use serde::{Deserialize, Serialize};

/// The nine technique slots, in `BASE_SKILL` order.
pub const TECHNIQUES: [&str; 9] = [
    "blunt", "slash", "stab", "bash", "shot", "crush", "blast", "heal", "force",
];

/// The eleven power-up slots, in `BASE_UPCNT` order.
///
/// Confirmed by tracing `FUN_003fa850` / `FUN_003fa3b0` / `FUN_003fab90`.
pub const POWERUP_STATS: [&str; 11] = [
    "HP max",
    "MP max",
    "Strength",
    "Defense",
    "Wisdom",
    "Spirit",
    "Speed",
    "Fire res",
    "Ice res",
    "Thunder res",
    "Dark res",
];

/// Highest level the game displays.
pub const MAX_LEVEL: u32 = 999;

/// EXP matching [`MAX_LEVEL`]: `threshold(999)`.
pub const EXP_AT_MAX_LEVEL: u32 = 1_133_652_152;

// ---------------------------------------------------------------------------
// Level curve
// ---------------------------------------------------------------------------

/// EXP needed to reach `level`: `n³ + 137n² − 77n − 61`.
///
/// Signed because `threshold(0)` is `-61`. Levels start at 1, whose threshold
/// is exactly 0.
#[must_use]
pub fn level_threshold(level: u32) -> i64 {
    let n = i64::from(level);
    n * n * n + 137 * n * n - 77 * n - 61
}

/// The largest level whose threshold is `<= exp`, minimum 1.
#[must_use]
pub fn level_from_exp(exp: u32) -> u32 {
    let exp = i64::from(exp);
    let mut level = 1u32;
    while level_threshold(level + 1) <= exp {
        level += 1;
    }
    level
}

// ---------------------------------------------------------------------------
// Junk shop
// ---------------------------------------------------------------------------

/// Cumulative donation threshold that reaches each junk-shop tier.
pub const JUNK_TIERS: [(u32, u32); 10] = [
    (0, 0),
    (1, 1_000),
    (2, 6_000),
    (3, 26_000),
    (4, 86_000),
    (5, 206_000),
    (6, 456_000),
    (7, 956_000),
    (8, 1_956_000),
    (9, 3_956_000),
];

/// The highest junk-shop tier whose threshold `counter` has reached.
#[must_use]
pub fn junk_tier_from_counter(counter: u32) -> u32 {
    JUNK_TIERS
        .iter()
        .filter(|(_, threshold)| counter >= *threshold)
        .map(|(tier, _)| *tier)
        .max()
        .unwrap_or(0)
}

/// The counter value that selects `tier`, if the tier exists.
///
/// Selecting a tier in the UI writes this value, matching the Python editor.
#[must_use]
pub fn junk_threshold(tier: u32) -> Option<u32> {
    JUNK_TIERS
        .iter()
        .find(|(t, _)| *t == tier)
        .map(|(_, th)| *th)
}

// ---------------------------------------------------------------------------
// Rarity / seed model
// ---------------------------------------------------------------------------

/// Bits of the seed that carry the additive stat bonus.
pub const SEED_BONUS_MASK: u16 = 0x7FF;

/// Rarity colour, a monotonic band of the seed's low 11 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rarity {
    White,
    Blue,
    Green,
    Yellow,
    Orange,
    Pink,
}

/// Rarity colours, ascending.
pub const RARITY_ORDER: [Rarity; 6] = [
    Rarity::White,
    Rarity::Blue,
    Rarity::Green,
    Rarity::Yellow,
    Rarity::Orange,
    Rarity::Pink,
];

impl Rarity {
    /// The inclusive `+N` bonus range this colour spans.
    #[must_use]
    pub fn range(self) -> (u16, u16) {
        match self {
            Rarity::White => (0x000, 0x000),
            Rarity::Blue => (0x001, 0x00F),
            Rarity::Green => (0x010, 0x07F),
            Rarity::Yellow => (0x080, 0x1FF),
            Rarity::Orange => (0x200, 0x3FF),
            Rarity::Pink => (0x400, 0x7FF),
        }
    }

    /// Lower-case colour name, as shown in the UI.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Rarity::White => "white",
            Rarity::Blue => "blue",
            Rarity::Green => "green",
            Rarity::Yellow => "yellow",
            Rarity::Orange => "orange",
            Rarity::Pink => "pink",
        }
    }

    /// A representative seed inside this colour's range.
    #[must_use]
    pub fn seed(self) -> u16 {
        match self {
            Rarity::White => 0x000,
            Rarity::Blue => 0x006,
            Rarity::Green => 0x020,
            Rarity::Yellow => 0x0A9,
            Rarity::Orange => 0x205,
            Rarity::Pink => 0x530,
        }
    }
}

/// The rarity colour a seed falls into.
#[must_use]
pub fn color_for_seed(seed: u16) -> Rarity {
    let effective = seed & SEED_BONUS_MASK;
    RARITY_ORDER
        .into_iter()
        .rev()
        .find(|r| effective >= r.range().0)
        .unwrap_or(Rarity::White)
}

/// Pull a `+N` bonus into `rarity`'s inclusive range.
#[must_use]
pub fn clamp_bonus_to_rarity(bonus: u16, rarity: Rarity) -> u16 {
    let (lo, hi) = rarity.range();
    bonus.clamp(lo, hi)
}

/// A human label for a seed: the colour plus the raw `+N` bonus.
///
/// `rarity_name(0x530) == "pink (+1328)"`.
#[must_use]
pub fn rarity_name(seed: u16) -> String {
    format!(
        "{} (+{})",
        color_for_seed(seed).label(),
        seed & SEED_BONUS_MASK
    )
}

// ---------------------------------------------------------------------------
// Caps
// ---------------------------------------------------------------------------

/// A field's limits: what the game tolerates, and what the type can hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Cap {
    /// Maximum accepted in Normal mode. Values are never negative there.
    pub normal_max: i64,
    /// Minimum accepted in Advanced mode.
    pub dtype_min: i64,
    /// Maximum accepted in Advanced mode.
    pub dtype_max: i64,
}

impl Cap {
    /// Whether Normal mode accepts `value`.
    #[must_use]
    pub fn allows_normal(self, value: i64) -> bool {
        (0..=self.normal_max).contains(&value)
    }

    /// Whether Advanced mode accepts `value`.
    #[must_use]
    pub fn allows_advanced(self, value: i64) -> bool {
        (self.dtype_min..=self.dtype_max).contains(&value)
    }

    /// Clamp into the Normal range.
    #[must_use]
    pub fn clamp_normal(self, value: i64) -> i64 {
        value.clamp(0, self.normal_max)
    }
}

/// `BIT`. The game clamps its display at 9,999,999.
pub const CAP_BIT: Cap = Cap {
    normal_max: 9_999_999,
    dtype_min: 0,
    dtype_max: 0xFFFF_FFFF,
};
/// Level. The field is a u32 but the game caps and displays 999.
pub const CAP_LEVEL: Cap = Cap {
    normal_max: MAX_LEVEL as i64,
    dtype_min: 0,
    dtype_max: 0xFFFF_FFFF,
};
/// EXP, capped at the level-999 threshold.
pub const CAP_EXP: Cap = Cap {
    normal_max: EXP_AT_MAX_LEVEL as i64,
    dtype_min: 0,
    dtype_max: 0xFFFF_FFFF,
};
/// Technique level. The only **signed** field: `0xFFFFFFFF` reads as `-1`.
pub const CAP_TECH: Cap = Cap {
    normal_max: 9_999,
    dtype_min: -0x8000_0000,
    dtype_max: 0x7FFF_FFFF,
};
/// The `XDATA` counter.
pub const CAP_XDATA: Cap = Cap {
    normal_max: 9_999,
    dtype_min: 0,
    dtype_max: 0xFFFF_FFFF,
};
/// A power-up slot. Use [`upcnt_safe_cap`] for the per-slot Normal limit.
pub const CAP_UPCNT: Cap = Cap {
    normal_max: 9_999,
    dtype_min: 0,
    dtype_max: 0xFFFF_FFFF,
};
/// An item's `+N` bonus, which is the seed's low 11 bits.
pub const CAP_ITEM_BONUS: Cap = Cap {
    normal_max: 0x7FF,
    dtype_min: 0,
    dtype_max: 0x7FF,
};
/// An item's mod count, 4 bits.
pub const CAP_ITEM_MODS: Cap = Cap {
    normal_max: 15,
    dtype_min: 0,
    dtype_max: 15,
};
/// A disk count, stored in the high 16 bits of its u32.
pub const CAP_DISK_COUNT: Cap = Cap {
    normal_max: 0xFFFF,
    dtype_min: 0,
    dtype_max: 0xFFFF,
};

/// The Normal-mode cap for a power-up slot.
///
/// The game clamps the *derived* stat after adding the power-up: HP max and MP
/// max (slots 0 and 1) clamp at 99,999, every other stat at 9,999
/// (`FUN_003fa850` / `FUN_003fa3b0`). Above those the value has no effect.
#[must_use]
pub fn upcnt_safe_cap(slot: usize) -> i64 {
    if slot == 0 || slot == 1 {
        99_999
    } else {
        9_999
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_curve_has_the_documented_anchors() {
        assert_eq!(level_threshold(1), 0);
        assert_eq!(level_threshold(2), 341);
        assert_eq!(level_threshold(999), 1_133_652_152);
    }

    #[test]
    fn the_curve_is_strictly_increasing_from_level_one() {
        let mut prev = level_threshold(1);
        for n in 2..=1200 {
            let cur = level_threshold(n);
            assert!(cur > prev, "threshold({n}) = {cur} is not above {prev}");
            prev = cur;
        }
    }

    #[test]
    fn level_from_exp_inverts_the_curve() {
        for n in 1..=999u32 {
            let exp = level_threshold(n) as u32;
            assert_eq!(level_from_exp(exp), n, "exp for level {n}");
        }
    }

    #[test]
    fn level_from_exp_is_one_before_the_second_threshold() {
        assert_eq!(level_from_exp(0), 1);
        assert_eq!(level_from_exp(340), 1);
        assert_eq!(level_from_exp(341), 2);
    }

    #[test]
    fn level_from_exp_tolerates_the_data_type_maximum() {
        // Must terminate rather than loop forever at the u32 ceiling.
        let n = level_from_exp(u32::MAX);
        assert!((1500..=1700).contains(&n), "got {n}");
    }

    #[test]
    fn the_exp_cap_is_exactly_the_level_999_threshold() {
        assert_eq!(CAP_EXP.normal_max, level_threshold(MAX_LEVEL));
        assert_eq!(MAX_LEVEL, 999);
    }

    #[test]
    fn junk_tiers_are_ascending_and_end_at_3956000() {
        let mut prev = 0;
        for (tier, threshold) in JUNK_TIERS {
            assert!(threshold >= prev, "tier {tier} is not ascending");
            prev = threshold;
        }
        assert_eq!(JUNK_TIERS[0], (0, 0));
        assert_eq!(JUNK_TIERS[9], (9, 3_956_000));
    }

    #[test]
    fn junk_tier_lookup_picks_the_highest_reached_tier() {
        assert_eq!(junk_tier_from_counter(0), 0);
        assert_eq!(junk_tier_from_counter(999), 0);
        assert_eq!(junk_tier_from_counter(1_000), 1);
        assert_eq!(junk_tier_from_counter(5_999), 1);
        assert_eq!(junk_tier_from_counter(6_000), 2);
        assert_eq!(junk_tier_from_counter(3_955_999), 8);
        assert_eq!(junk_tier_from_counter(3_956_000), 9);
        assert_eq!(junk_tier_from_counter(u32::MAX), 9);
    }

    #[test]
    fn junk_threshold_is_the_tier_table_written_as_a_counter_value() {
        assert_eq!(junk_threshold(0), Some(0));
        assert_eq!(junk_threshold(9), Some(3_956_000));
        assert_eq!(junk_threshold(10), None);
    }

    #[test]
    fn rarity_bands_are_contiguous_and_cover_the_bonus_range() {
        let mut expected_lo = 0u16;
        for r in RARITY_ORDER {
            let (lo, hi) = r.range();
            assert_eq!(
                lo, expected_lo,
                "{r:?} does not start where the previous ended"
            );
            assert!(hi >= lo, "{r:?} has an inverted range");
            expected_lo = hi + 1;
        }
        assert_eq!(expected_lo, SEED_BONUS_MASK + 1);
    }

    #[test]
    fn rarity_lookup_uses_the_low_eleven_bits() {
        assert_eq!(color_for_seed(0x000), Rarity::White);
        assert_eq!(color_for_seed(0x001), Rarity::Blue);
        assert_eq!(color_for_seed(0x00F), Rarity::Blue);
        assert_eq!(color_for_seed(0x010), Rarity::Green);
        assert_eq!(color_for_seed(0x07F), Rarity::Green);
        assert_eq!(color_for_seed(0x080), Rarity::Yellow);
        assert_eq!(color_for_seed(0x1FF), Rarity::Yellow);
        assert_eq!(color_for_seed(0x200), Rarity::Orange);
        assert_eq!(color_for_seed(0x3FF), Rarity::Orange);
        assert_eq!(color_for_seed(0x400), Rarity::Pink);
        assert_eq!(color_for_seed(0x7FF), Rarity::Pink);
    }

    #[test]
    fn bit_eleven_of_the_seed_is_masked_off() {
        for seed in 0..=0xFFFu16 {
            assert_eq!(color_for_seed(seed), color_for_seed(seed & SEED_BONUS_MASK));
        }
    }

    #[test]
    fn clamp_pulls_a_bonus_into_its_band() {
        assert_eq!(clamp_bonus_to_rarity(0x7FF, Rarity::Blue), 0x00F);
        assert_eq!(clamp_bonus_to_rarity(0, Rarity::Pink), 0x400);
        assert_eq!(clamp_bonus_to_rarity(0x0A9, Rarity::Yellow), 0x0A9);
    }

    #[test]
    fn rarity_name_shows_the_colour_and_the_bonus() {
        assert_eq!(rarity_name(0x530), "pink (+1328)");
        assert_eq!(rarity_name(0x000), "white (+0)");
    }

    #[test]
    fn normal_and_advanced_ranges_differ_only_where_the_game_exceeds_the_type() {
        assert!(CAP_BIT.allows_normal(9_999_999));
        assert!(!CAP_BIT.allows_normal(10_000_000));
        assert!(CAP_BIT.allows_advanced(0xFFFF_FFFF));
        assert!(!CAP_BIT.allows_advanced(-1));

        // Techniques are the one signed field.
        assert!(CAP_TECH.allows_advanced(-1));
        assert!(CAP_TECH.allows_advanced(0x7FFF_FFFF));
        assert!(!CAP_TECH.allows_advanced(0x8000_0000));
        assert!(!CAP_TECH.allows_normal(-1));
    }

    #[test]
    fn hp_and_mp_power_ups_have_the_higher_safe_cap() {
        assert_eq!(upcnt_safe_cap(0), 99_999);
        assert_eq!(upcnt_safe_cap(1), 99_999);
        for slot in 2..11 {
            assert_eq!(upcnt_safe_cap(slot), 9_999, "slot {slot}");
        }
    }
}
