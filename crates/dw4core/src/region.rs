//! Regional PS2 card identities. The raw save layout is shared by all releases.
use serde::{Deserialize, Serialize};

/// PS2 release whose card directory and icon metadata should be used.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "lowercase")]
pub enum Region {
    #[default]
    Usa,
    Europe,
    Japan,
}

impl Region {
    pub const ALL: [Self; 3] = [Self::Usa, Self::Europe, Self::Japan];

    #[must_use]
    pub const fn save_name(self) -> &'static str {
        match self {
            Self::Usa => "BASLUS-20836savedata",
            Self::Europe => "BESLES-53405savedata",
            Self::Japan => "BISLPS-25453savedata",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Usa => "USA",
            Self::Europe => "Europe",
            Self::Japan => "Japan",
        }
    }
}
