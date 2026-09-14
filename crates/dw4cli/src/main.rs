//! Headless CLI over the same service layer the Tauri shell uses.

mod output;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use dw4core::document::DifficultyChoice;
use dw4core::{Category, Difficulty, Mode, Species};
use dw4ipc::{EditorSession, IpcError, NewSaveRequest};

#[derive(Parser)]
#[command(name = "dw4cli", version, about = "Edit Digimon World 4 save files")]
struct Cli {
    /// Emit the IPC payloads as JSON instead of human text.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Summarise a save without changing it.
    Info { path: PathBuf },
    /// Print every editable field.
    Dump { path: PathBuf },
    /// Synthesise a new save.
    New {
        /// Species display or model name.
        #[arg(long, value_parser = parse_species)]
        species: Option<Species>,
        /// Player name, at most 8 characters.
        #[arg(long)]
        name: Option<String>,
        /// A story preset name; omit for a storyless save.
        #[arg(long)]
        story: Option<String>,
        /// normal | hard | veryhard.
        #[arg(long, value_parser = parse_difficulty)]
        difficulty: Option<Difficulty>,
        /// Copy this card and swap in the new save (required for `-o *.ps2`).
        #[arg(long)]
        card: Option<PathBuf>,
        /// Output file.
        #[arg(short, long)]
        out: PathBuf,
    },
    /// Check a save or card for corruption.
    Verify { path: PathBuf },
    /// List or search the item catalogue.
    Items {
        /// Case-insensitive name fragment.
        #[arg(long)]
        query: Option<String>,
        /// weapon | styled | core | board | mod.
        #[arg(long, value_parser = parse_category)]
        category: Option<Category>,
        /// Maximum rows.
        #[arg(long, default_value_t = 50)]
        limit: usize,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: &Cli) -> Result<(), IpcError> {
    match &cli.command {
        Command::Info { path } => {
            let mut session = EditorSession::new_session();
            let result = session.open(path)?;
            if cli.json {
                output::json(&result);
            } else {
                output::info(&result);
            }
        }
        Command::Dump { path } => {
            let mut session = EditorSession::new_session();
            let result = session.open(path)?;
            if cli.json {
                output::json(&result.view);
            } else {
                output::dump(&result.view);
            }
        }
        Command::Items {
            query,
            category,
            limit,
        } => {
            let items = dw4ipc::catalogue_search(query.as_deref(), *category, *limit);
            if cli.json {
                output::json(&items);
            } else {
                output::items(&items);
            }
        }
        Command::New {
            species,
            name,
            story,
            difficulty,
            card,
            out,
        } => {
            if card.is_some()
                && out
                    .extension()
                    .is_none_or(|e| !e.eq_ignore_ascii_case("ps2"))
            {
                return Err(IpcError::Unsupported {
                    message: "--card only makes sense with a .ps2 output".to_string(),
                });
            }
            let req = NewSaveRequest {
                species: species.unwrap_or(Species::DEFAULT),
                name: name.clone().unwrap_or_else(|| "TST".to_string()),
                story: story.clone(),
                difficulty: match difficulty {
                    Some(d) => DifficultyChoice::Fixed(*d),
                    None => DifficultyChoice::Auto,
                },
            };
            let mut session = EditorSession::new_session();
            let opened = match card {
                Some(template) => session.new_save_on_card(&req, template)?,
                None => session.new_save(&req)?,
            };
            let edits = opened.view.to_edit_set();
            let result = session.save_as(out, &edits, Mode::Normal)?;
            if cli.json {
                output::json(&result);
            } else {
                println!("wrote {}", result.path.as_deref().unwrap_or("(unknown)"));
            }
        }
        Command::Verify { path } => {
            let report = dw4ipc::verify_path(path)?;
            if cli.json {
                output::json(&report);
            } else {
                output::verify(&report);
            }
            if !report.problems.is_empty() {
                return Err(IpcError::Unsupported {
                    message: format!("{} problem(s) found", report.problems.len()),
                });
            }
        }
    }
    Ok(())
}

fn parse_species(text: &str) -> Result<Species, String> {
    Species::ALL
        .iter()
        .copied()
        .find(|sp| {
            sp.display().eq_ignore_ascii_case(text)
                || sp.model_name().eq_ignore_ascii_case(text)
                || sp.model_stem().eq_ignore_ascii_case(text)
        })
        .ok_or_else(|| format!("unknown species {text:?}"))
}

fn parse_difficulty(text: &str) -> Result<Difficulty, String> {
    match text
        .to_ascii_lowercase()
        .replace([' ', '-', '_'], "")
        .as_str()
    {
        "normal" => Ok(Difficulty::Normal),
        "hard" => Ok(Difficulty::Hard),
        "veryhard" => Ok(Difficulty::VeryHard),
        other => Err(format!("unknown difficulty {other:?}")),
    }
}

fn parse_category(text: &str) -> Result<Category, String> {
    match text.to_ascii_lowercase().as_str() {
        "weapon" => Ok(Category::Weapon),
        "styled" => Ok(Category::Styled),
        "core" => Ok(Category::Core),
        "board" => Ok(Category::Board),
        "mod" => Ok(Category::Mod),
        other => Err(format!("unknown category {other:?}")),
    }
}
