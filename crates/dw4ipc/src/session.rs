//! The stateful editing session.

use std::path::Path;

use dw4core::document::Document;
use dw4core::{
    Difficulty, DifficultyChoice, EditSet, Mode, SaveData, SaveSpec, SaveView, Warning, build_save,
    spec_with_story,
};

use crate::error::IpcError;
use crate::payload::{NewSaveRequest, OpenResult, SourceKind};

/// A save being edited, plus where it came from.
///
/// The Tauri shell holds one behind a `Mutex`. The CLI uses it for its
/// one-shot verbs. `Document` is private on purpose: the frontend must not be
/// able to fabricate the source-card state that `save_as` needs.
#[derive(Debug, Default)]
pub struct EditorSession {
    doc: Option<Document>,
}

impl EditorSession {
    /// An empty session.
    #[must_use]
    pub fn new_session() -> Self {
        Self { doc: None }
    }

    /// Load a card or raw file, deciding by content.
    ///
    /// # Errors
    /// Whatever `Document::load` reports, mapped to [`IpcError`].
    pub fn open(&mut self, path: &Path) -> Result<OpenResult, IpcError> {
        let doc = Document::load(path)?;
        let source = source_kind(path)?;
        let view = doc.view(Mode::Normal);
        let result = OpenResult {
            path: Some(path_string(path)?),
            source,
            view,
        };
        self.doc = Some(doc);
        Ok(result)
    }

    /// Synthesise a new save in memory. Nothing is written until `save_as`.
    ///
    /// # Errors
    /// [`IpcError::Unsupported`] if `story` names no preset.
    pub fn new_save(&mut self, req: &NewSaveRequest) -> Result<OpenResult, IpcError> {
        self.build_new(req, None)
    }

    /// Synthesise a new save that will be written into a copy of `card`.
    ///
    /// The template must already contain the save file: `render_container`
    /// overwrites an existing chain and refuses to create one. This is the CLI
    /// `new --card` path; the IPC `new_save` never uses a card.
    ///
    /// # Errors
    /// As [`Self::new_save`], plus whatever loading `card` reports.
    pub fn new_save_on_card(
        &mut self,
        req: &NewSaveRequest,
        card: &Path,
    ) -> Result<OpenResult, IpcError> {
        self.build_new(req, Some(card))
    }

    fn build_new(
        &mut self,
        req: &NewSaveRequest,
        card: Option<&Path>,
    ) -> Result<OpenResult, IpcError> {
        let spec = save_spec(req)?;
        let data = build_save(&spec);

        let mut doc = match card {
            Some(path) => Document::load(path)?,
            None => Document::from_bytes(&data)?,
        };
        if card.is_some() {
            // Replace the in-memory save while keeping the source card, so a
            // later save_as copies the card and swaps in these bytes.
            *doc.data_mut() = SaveData::parse(&data)?;
        }

        let view = doc.view(Mode::Normal);
        let source = if card.is_some() {
            SourceKind::Memcard
        } else {
            SourceKind::Raw
        };
        self.doc = Some(doc);
        Ok(OpenResult {
            path: None,
            source,
            view,
        })
    }

    /// Re-project the open save for `mode`.
    ///
    /// # Errors
    /// [`IpcError::NoOpenDocument`] if nothing is open.
    pub fn view(&self, mode: Mode) -> Result<SaveView, IpcError> {
        Ok(self.doc_ref()?.view(mode))
    }

    /// Validate authoritatively. Writes nothing, even on success.
    ///
    /// # Errors
    /// [`IpcError::Validation`] carrying every rejected field, or
    /// [`IpcError::NoOpenDocument`].
    pub fn validate(&self, edits: &EditSet, mode: Mode) -> Result<Vec<Warning>, IpcError> {
        self.doc_ref()?
            .validate(edits, mode)
            .map_err(|fields| IpcError::Validation { fields })
    }

    /// Save over the path the session was opened from.
    ///
    /// # Errors
    /// [`IpcError::Unsupported`] for a pathless (`new_save`) session;
    /// otherwise whatever `Document::save` reports.
    pub fn save(&mut self) -> Result<OpenResult, IpcError> {
        let doc = self.doc.as_mut().ok_or(IpcError::NoOpenDocument)?;
        let path = doc
            .path()
            .map(Path::to_path_buf)
            .ok_or_else(|| IpcError::Unsupported {
                message: "this save has no path yet; use save_as".to_string(),
            })?;
        doc.save(&path)?;
        finish(doc, &path)
    }

    /// Save to `path`, creating it if needed.
    ///
    /// # Errors
    /// Whatever `Document::save` reports.
    pub fn save_as(&mut self, path: &Path) -> Result<OpenResult, IpcError> {
        let doc = self.doc.as_mut().ok_or(IpcError::NoOpenDocument)?;
        doc.save(path)?;
        finish(doc, path)
    }

    fn doc_ref(&self) -> Result<&Document, IpcError> {
        self.doc.as_ref().ok_or(IpcError::NoOpenDocument)
    }
}

/// Project after a write, using the card/raw source the path now resolves to.
fn finish(doc: &Document, path: &Path) -> Result<OpenResult, IpcError> {
    Ok(OpenResult {
        path: Some(path_string(path)?),
        source: source_kind(path)?,
        view: doc.view(Mode::Normal),
    })
}

/// Whether `path` holds a memory card, by content.
fn source_kind(path: &Path) -> Result<SourceKind, IpcError> {
    let bytes = std::fs::read(path).map_err(|source| IpcError::Core {
        variant: "Io".to_string(),
        message: format!("{}: {source}", path.display()),
    })?;
    Ok(if dw4core::memcard::is_memcard(&bytes) {
        SourceKind::Memcard
    } else {
        SourceKind::Raw
    })
}

/// A path that can cross IPC. Non-UTF-8 is reported, never silently lossy.
fn path_string(path: &Path) -> Result<String, IpcError> {
    path.to_str()
        .map(str::to_string)
        .ok_or_else(|| IpcError::Unsupported {
            message: format!("{}: path is not valid UTF-8", path.display()),
        })
}

/// The `SaveSpec` a request describes.
fn save_spec(req: &NewSaveRequest) -> Result<SaveSpec, IpcError> {
    let difficulty = match req.difficulty {
        DifficultyChoice::Fixed(d) => d,
        // A brand-new save has no live mirror set to infer from, so Normal is
        // the only sensible reading of Auto.
        DifficultyChoice::Auto => Difficulty::Normal,
    };
    let mut spec = match &req.story {
        Some(name) => spec_with_story(name, difficulty).ok_or_else(|| IpcError::Unsupported {
            message: format!("no story preset named {name:?}"),
        })?,
        None => SaveSpec::default(),
    };
    spec.set_species(req.species);
    spec.player_name.clone_from(&req.name);
    Ok(spec)
}
