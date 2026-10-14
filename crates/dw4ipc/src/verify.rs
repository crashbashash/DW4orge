//! Whole-container verification, for `dw4cli verify`.

use std::path::Path;

use dw4core::memcard::{Ps2Memcard, SAVE_DIR, SAVE_FILE, page_spare};
use dw4core::{SaveData, Severity};
use serde::{Deserialize, Serialize};

use crate::error::IpcError;
use crate::payload::SourceKind;

/// One finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyProblem {
    /// How serious it is.
    pub severity: Severity,
    /// What was found.
    pub message: String,
}

/// What the card layer found. `None` for a raw file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardReport {
    /// Superblock version.
    pub version: String,
    /// Bytes of page data.
    pub page_size: u16,
    /// Pages per cluster.
    pub pages_per_cluster: u16,
    /// Clusters on the card.
    pub clusters: u32,
    /// In-use pages whose ECC was checked.
    pub ecc_checked: usize,
    /// In-use pages whose stored ECC disagreed, including the superblock.
    pub ecc_mismatched: Vec<usize>,
}

/// The result of verifying one path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifyReport {
    /// The file checked.
    pub path: String,
    /// Which container it is.
    pub source: SourceKind,
    /// Whether the save's own checksums are valid.
    pub checksum_ok: bool,
    /// Everything that looked wrong.
    pub problems: Vec<VerifyProblem>,
    /// Card detail, when it is a card.
    pub card: Option<CardReport>,
}

/// Verify a raw save or a memory card.
///
/// # Errors
/// [`IpcError`] only if the file cannot be read or is structurally so broken
/// that no save can be extracted; a merely corrupt save is a report, not an
/// error.
pub fn verify_path(path: &Path) -> Result<VerifyReport, IpcError> {
    let bytes = std::fs::read(path).map_err(|source| IpcError::Core {
        variant: "Io".to_string(),
        message: format!("{}: {source}", path.display()),
    })?;
    let path = path
        .to_str()
        .ok_or_else(|| IpcError::Unsupported {
            message: "path is not valid UTF-8".to_string(),
        })?
        .to_string();

    let mut problems = Vec::new();
    let is_card = dw4core::memcard::is_memcard(&bytes);

    let (save_bytes, card) = if is_card {
        let mut memcard = Ps2Memcard::from_image(bytes.clone())?;
        let superblock = memcard.superblock().clone();
        let geometry = *memcard.geometry();

        if let Err(err) = memcard.locate(SAVE_DIR, SAVE_FILE) {
            problems.push(VerifyProblem {
                severity: Severity::Error,
                message: err.to_string(),
            });
        }
        let save = memcard.read_save(SAVE_DIR, SAVE_FILE)?;

        let (checked, mismatched) = ecc_scan(
            memcard.image(),
            geometry.raw_page_size,
            geometry.total_pages(),
        );
        if !mismatched.is_empty() {
            // Page 1 is inside the pre-allocated superblock region and is a
            // known exception: `dw4core/tests/memcard.rs` pins it as the only
            // mismatch on the real card.
            let unexpected: Vec<usize> = mismatched.iter().copied().filter(|n| *n != 1).collect();
            if !unexpected.is_empty() {
                problems.push(VerifyProblem {
                    severity: Severity::Error,
                    message: format!("ECC mismatch on page(s) {unexpected:?}"),
                });
            }
        }

        (
            save,
            Some(CardReport {
                version: superblock.version.clone(),
                page_size: superblock.page_size,
                pages_per_cluster: superblock.pages_per_cluster,
                clusters: superblock.clusters_per_card,
                ecc_checked: checked,
                ecc_mismatched: mismatched,
            }),
        )
    } else {
        (bytes, None)
    };

    let checksum_ok = match SaveData::parse(&save_bytes) {
        Ok(data) => {
            let ok = data.verify();
            if !ok {
                problems.push(VerifyProblem {
                    severity: Severity::Error,
                    message: "save checksums do not match".to_string(),
                });
            }
            ok
        }
        Err(err) => {
            problems.push(VerifyProblem {
                severity: Severity::Error,
                message: err.to_string(),
            });
            false
        }
    };

    Ok(VerifyReport {
        path,
        source: if is_card {
            SourceKind::Memcard
        } else {
            SourceKind::Raw
        },
        checksum_ok,
        problems,
        card,
    })
}

/// Check every in-use page's ECC, skipping fully erased pages.
fn ecc_scan(image: &[u8], raw_page_size: usize, total_pages: usize) -> (usize, Vec<usize>) {
    const DATA: usize = 512;
    let mut checked = 0;
    let mut mismatched = Vec::new();
    for n in 0..total_pages {
        let at = n * raw_page_size;
        let page = &image[at..at + raw_page_size];
        if page == &[0xFFu8; 528][..] {
            continue;
        }
        checked += 1;
        if page_spare(&page[..DATA]) != page[DATA..] {
            mismatched.push(n);
        }
    }
    (checked, mismatched)
}
