//! The stateful editing session.

use std::path::Path;

use dw4core::document::Document;
use dw4core::{
    Difficulty, DifficultyChoice, EditSet, Mode, SaveData, SaveSpec, SaveView, Species, Warning,
    build_save_with_order, level_threshold, spec_with_story,
};

use crate::error::IpcError;
use crate::payload::{NewSaveRequest, OpenResult, SourceKind, SpeciesStats};

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
        self.open_with_region(path, None)
    }

    pub fn open_with_region(
        &mut self,
        path: &Path,
        region: Option<dw4core::Region>,
    ) -> Result<OpenResult, IpcError> {
        self.open_with_selection(path, region, None)
    }

    pub fn open_with_selection(
        &mut self,
        path: &Path,
        region: Option<dw4core::Region>,
        selection: Option<&str>,
    ) -> Result<OpenResult, IpcError> {
        let doc = Document::load_with_selection(path, region, selection)?;
        let source = source_kind(path)?;
        let view = doc.view(Mode::Normal);
        let result = OpenResult {
            empty_boxes: doc.empty_boxes(),
            pending_write: false,
            region: doc.region(),
            path: Some(path_string(path)?),
            source,
            view,
            character_boxes: (!doc.character_boxes().is_empty()).then(|| doc.character_boxes()),
            character_selection: doc.character_selection(),
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
        let format = req.format.unwrap_or(SourceKind::Memcard);
        let order =
            match format {
                SourceKind::Gamecube | SourceKind::GamecubeCard => dw4core::ByteOrder::Big,
                SourceKind::Memcard | SourceKind::Raw => dw4core::ByteOrder::Little,
                SourceKind::Xbox | SourceKind::XboxMu => return Err(IpcError::Unsupported {
                    message:
                        "Open an existing Xbox save to edit it; Xbox save creation is not supported"
                            .into(),
                }),
            };
        if card.is_some() && order == dw4core::ByteOrder::Big {
            return Err(IpcError::Unsupported {
                message: "GameCube creation cannot use a PS2 donor card".into(),
            });
        }
        if let Some(path) = card
            && source_kind(path)? != SourceKind::Memcard
        {
            return Err(IpcError::Unsupported {
                message: "New-save donors must be PS2 memory-card images".into(),
            });
        }
        let data = build_save_with_order(&spec, order);

        let mut doc = match card {
            Some(path) => Document::load_with_region(path, req.region)?,
            None => Document::from_bytes(&data)?,
        };
        if card.is_some() {
            // Replace only the selected character in the donor save. The
            // other three boxes and complete mirror headers must survive.
            doc.data_mut()
                .set_bytes(8, &data[8..8 + SaveData::PS2_BOX_SIZE]);
        }

        if card.is_none() {
            doc.set_region(req.region.unwrap_or_default())?;
        }
        let view = doc.view(Mode::Normal);
        let source = if card.is_some() {
            SourceKind::Memcard
        } else {
            match format {
                SourceKind::Memcard => SourceKind::Raw,
                other => other,
            }
        };
        let region = doc.region();
        self.doc = Some(doc);
        Ok(OpenResult {
            empty_boxes: self.doc_ref()?.empty_boxes(),
            pending_write: true,
            region,
            path: None,
            source,
            view,
            character_boxes: None,
            character_selection: None,
        })
    }

    /// Create a character in an empty box of the currently opened PS2 or GameCube save.
    /// Reloads the source before creation; nothing is written until Save/Save As.
    /// A failed request keeps the previous editing session intact.
    pub fn create_box(&mut self, number: u8, req: &NewSaveRequest) -> Result<OpenResult, IpcError> {
        if matches!(req.format, Some(SourceKind::Xbox | SourceKind::XboxMu)) {
            return Err(IpcError::Unsupported {
                message: "Xbox box creation is not supported yet".into(),
            });
        }
        let old = self.doc_ref()?;
        let path = old
            .path()
            .ok_or_else(|| IpcError::Unsupported {
                message: "Save the new file before creating another character box".into(),
            })?
            .to_path_buf();
        if old.region().is_some() && req.region.is_some() && req.region != old.region() {
            return Err(IpcError::Unsupported {
                message: "An existing card keeps its original region".into(),
            });
        }
        let mut doc = Document::load_with_region(&path, old.region().or(req.region))?;
        if doc.empty_boxes().is_none() {
            return Err(IpcError::Unsupported {
                message: "Box creation supports PS2 and GameCube saves only".into(),
            });
        }
        let gamecube = doc.data().byte_order() == dw4core::ByteOrder::Big;
        if matches!(req.format, Some(SourceKind::Memcard)) && gamecube
            || matches!(
                req.format,
                Some(SourceKind::Gamecube | SourceKind::GamecubeCard)
            ) && !gamecube
        {
            return Err(IpcError::Unsupported {
                message: "Creation cannot change the opened save's platform".into(),
            });
        }
        // A new character needs the initial story state and usable starter gear.
        let mut request = req.clone();
        if request.story.is_none() {
            request.story = Some("Fresh (tutorial)".into());
        }
        let mut spec = save_spec(&request)?;
        spec.device = vec![0x050d, 0x0513, 0x0512];
        spec.weapons = vec![0, 1, 2];
        doc.create_box(number, &spec)?;
        let mut result = finish(&doc, &path)?;
        result.pending_write = true;
        self.doc = Some(doc);
        Ok(result)
    }

    /// PS2-only entry point retained for clients of 0.7.0.
    pub fn create_ps2_box(
        &mut self,
        number: u8,
        req: &NewSaveRequest,
    ) -> Result<OpenResult, IpcError> {
        if self.doc_ref()?.empty_ps2_boxes().is_none() {
            return Err(IpcError::Unsupported {
                message: "This creation entry point supports PS2 only".into(),
            });
        }
        self.create_box(number, req)
    }

    /// Re-project the open save for `mode`.
    ///
    /// # Errors
    /// [`IpcError::NoOpenDocument`] if nothing is open.
    pub fn view(&self, mode: Mode) -> Result<SaveView, IpcError> {
        Ok(self.doc_ref()?.view(mode))
    }

    /// The stored progression for one species.
    ///
    /// Applies the same Normal-mode EXP lift as [`Self::view`], so a species
    /// switch shows what the view would show if that species were active.
    ///
    /// # Errors
    /// [`IpcError::NoOpenDocument`] if nothing is open.
    pub fn species_stats(&self, species: Species, mode: Mode) -> Result<SpeciesStats, IpcError> {
        let data = self.doc_ref()?.data();
        let level = data.level(species);
        let mut exp = data.exp(species);
        if !mode.is_advanced() {
            let threshold = level_threshold(level);
            if i64::from(exp) < threshold && threshold <= i64::from(u32::MAX) {
                exp = threshold as u32;
            }
        }
        Ok(SpeciesStats {
            level,
            exp,
            tech: std::array::from_fn(|i| data.skill(species, i)),
            upcnt: std::array::from_fn(|i| data.upcnt(species, i)),
        })
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
    /// [`IpcError::Validation`] if any field is rejected (nothing is written),
    /// [`IpcError::Unsupported`] for a pathless (`new_save`) session, or
    /// whatever `Document::save` reports.
    pub fn save(&mut self, edits: &EditSet, mode: Mode) -> Result<OpenResult, IpcError> {
        let doc = self.doc.as_mut().ok_or(IpcError::NoOpenDocument)?;
        let path = doc
            .path()
            .map(Path::to_path_buf)
            .ok_or_else(|| IpcError::Unsupported {
                message: "this save has no path yet; use save_as".to_string(),
            })?;
        // `apply` validates first and writes nothing on rejection, so a save
        // can never persist state the validator did not accept.
        doc.apply(edits, mode)
            .map_err(|fields| IpcError::Validation { fields })?;
        doc.save(&path)?;
        finish(doc, &path)
    }

    /// Save the draft to `path`, creating it if needed.
    ///
    /// # Errors
    /// As [`Self::save`], plus whatever `Document::save` reports.
    pub fn save_as(
        &mut self,
        path: &Path,
        edits: &EditSet,
        mode: Mode,
    ) -> Result<OpenResult, IpcError> {
        self.save_as_with_region(path, edits, mode, None)
    }

    pub fn save_as_with_region(
        &mut self,
        path: &Path,
        edits: &EditSet,
        mode: Mode,
        region: Option<dw4core::Region>,
    ) -> Result<OpenResult, IpcError> {
        let doc = self.doc.as_mut().ok_or(IpcError::NoOpenDocument)?;
        if let Some(region) = region {
            doc.set_region(region)?;
        }
        doc.apply(edits, mode)
            .map_err(|fields| IpcError::Validation { fields })?;
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
        empty_boxes: doc.empty_boxes(),
        pending_write: false,
        region: doc.region(),
        path: Some(path_string(path)?),
        source: source_kind(path)?,
        view: doc.view(Mode::Normal),
        character_boxes: (!doc.character_boxes().is_empty()).then(|| doc.character_boxes()),
        character_selection: doc.character_selection(),
    })
}

/// Whether `path` holds a memory card, by content.
fn source_kind(path: &Path) -> Result<SourceKind, IpcError> {
    let bytes = std::fs::read(path).map_err(|source| IpcError::Core {
        variant: "Io".to_string(),
        message: format!("{}: {source}", path.display()),
    })?;
    Ok(if dw4core::xbox::is_save(&bytes) {
        SourceKind::Xbox
    } else if dw4core::xbox::is_memory_unit(&bytes) {
        SourceKind::XboxMu
    } else if dw4core::memcard::is_memcard(&bytes) {
        SourceKind::Memcard
    } else if dw4core::memcard::is_gamecube_card(&bytes) {
        SourceKind::GamecubeCard
    } else if dw4core::memcard::is_gamecube_gci(&bytes) {
        SourceKind::Gamecube
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
        None if matches!(
            req.format,
            Some(SourceKind::Gamecube | SourceKind::GamecubeCard)
        ) =>
        {
            spec_with_story("Fresh (tutorial)", difficulty).expect("built-in fresh preset")
        }
        None => SaveSpec::default(),
    };
    if matches!(
        req.format,
        Some(SourceKind::Gamecube | SourceKind::GamecubeCard)
    ) {
        // A generated character needs a usable starter inventory. Equipment
        // fields reference device-folder indices, rather than item IDs.
        spec.device = vec![0x050d, 0x0513, 0x0512];
        spec.weapons = vec![0, 1, 2];
    }
    spec.set_species(req.species);
    spec.player_name.clone_from(&req.name);
    Ok(spec)
}
