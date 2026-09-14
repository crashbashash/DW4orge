//! Assembly of the static UI tables.

use dw4core::{
    BOSS_FLAGS, CAP_BIT, CAP_DISK_COUNT, CAP_EXP, CAP_ITEM_BONUS, CAP_ITEM_MODS, CAP_LEVEL,
    CAP_TECH, CAP_UPCNT, CAP_XDATA, CHAPTER_FLAGS, FOLDER_LABELS, INTRO_FLAGS, LOBBY_FLAGS,
    MIRRORS, POWERUP_STATS, QUEST_FLAGS, STORY_PRESETS, get_catalogue, upcnt_safe_cap,
};

use crate::payload::{AppInfo, NamedCap, PowerupLimit, UiData};

/// Every field limit the validator can cite, keyed by its path prefix.
fn caps() -> Vec<NamedCap> {
    [
        ("bit", CAP_BIT),
        ("xdata", CAP_XDATA),
        ("level", CAP_LEVEL),
        ("exp", CAP_EXP),
        ("bank_bit", CAP_BIT),
        ("tech", CAP_TECH),
        ("upcnt", CAP_UPCNT),
        ("disks", CAP_DISK_COUNT),
        ("item_bonus", CAP_ITEM_BONUS),
        ("item_mods", CAP_ITEM_MODS),
    ]
    .into_iter()
    .map(|(field, cap)| NamedCap {
        field: field.to_string(),
        cap,
    })
    .collect()
}

/// Every flag label, in the order the Story tab shows them.
fn flag_labels() -> Vec<dw4core::FlagLabel> {
    let mut out = Vec::new();
    for group in [
        INTRO_FLAGS,
        CHAPTER_FLAGS,
        BOSS_FLAGS,
        QUEST_FLAGS,
        LOBBY_FLAGS,
    ] {
        out.extend(group.iter().copied());
    }
    out
}

/// Build the static tables.
#[must_use]
pub fn ui_data() -> UiData {
    UiData {
        catalogue: get_catalogue().iter().cloned().collect(),
        caps: caps(),
        powerups: POWERUP_STATS
            .iter()
            .enumerate()
            .map(|(slot, stat)| PowerupLimit {
                slot: slot as u8,
                stat: (*stat).to_string(),
                normal_max: upcnt_safe_cap(slot),
            })
            .collect(),
        mirrors: MIRRORS.to_vec(),
        folder_labels: FOLDER_LABELS.iter().map(|s| (*s).to_string()).collect(),
        flag_labels: flag_labels(),
        story_presets: STORY_PRESETS.to_vec(),
    }
}

/// Application identity plus the static tables.
///
/// `schema_version` is bumped whenever a payload changes shape, so a frontend
/// built against older bindings can refuse rather than mis-render.
#[must_use]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        core_version: dw4core::VERSION.to_string(),
        save_size: dw4core::SAVE_SIZE,
        block_size: dw4core::BLOCK,
        schema_version: 1,
        ui: ui_data(),
    }
}
