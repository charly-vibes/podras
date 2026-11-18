//! Purpose: write the generated BRIEF.md / BRIEF.json artifacts.
//!
//! Responsibilities:
//! - render and atomically write the committed brief artifacts as one pair;
//! - write the roadmap lens pair (ROADMAP.md / ROADMAP.json) the same way;
//! - gate the whisper next-action line on curation `publish_whisper`;
//! - never merge hand edits into regenerated output.
//!
//! Rationale: committed renders are generated, never authored. One writer
//! keeps overwrite semantics in a single place, and staging both payloads
//! before replacing either target keeps a failed write from leaving the pair
//! half-updated.

use std::path::{Path, PathBuf};

use crate::curation;
use crate::error::{Error, Result};
use crate::model::Brief;

/// Committed markdown artifact name.
pub const MARKDOWN_FILE: &str = "BRIEF.md";
/// Committed JSON artifact name.
pub const JSON_FILE: &str = "BRIEF.json";
/// Roadmap lens markdown artifact name.
pub const ROADMAP_MARKDOWN_FILE: &str = "ROADMAP.md";
/// Roadmap lens JSON artifact name.
pub const ROADMAP_JSON_FILE: &str = "ROADMAP.json";

/// Write `BRIEF.md` and `BRIEF.json` for the repository.
///
/// The whisper next action is included in the markdown only when curation sets
/// `publish_whisper = true`; the JSON envelope never carries it. Both files are
/// fully regenerated, so a second write replaces hand edits rather than merging.
pub fn write_renders(repo: &Path, brief: &Brief, whisper_next_action: Option<&str>) -> Result<()> {
    write_renders_to(repo, repo, brief, whisper_next_action)
}

/// Write the render pair into an existing `dir`; never creates it.
///
/// `repo` remains the privacy authority: `publish_whisper` is read from the
/// repository's curation, never from the target directory. Staging and rename
/// semantics are the pair writer's, so the default repository-root target and a
/// temporary gate directory behave identically. A missing or non-directory
/// target is a hard error that writes nothing.
pub fn write_renders_to(
    repo: &Path,
    dir: &Path,
    brief: &Brief,
    whisper_next_action: Option<&str>,
) -> Result<()> {
    if !dir.is_dir() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "cannot write renders: {} is not an existing directory",
                dir.display()
            ),
        )));
    }

    let reentry = if publish_whisper(repo) {
        whisper_next_action
    } else {
        None
    };
    let markdown = crate::render::markdown::render(brief, reentry)?;
    let json = crate::render::envelope::to_json(brief)?;

    write_pair(
        dir,
        &[
            (MARKDOWN_FILE, markdown.as_str()),
            (JSON_FILE, json.as_str()),
        ],
    )
}

/// Write `ROADMAP.md` and `ROADMAP.json` into an existing `dir`.
///
/// `ROADMAP.md` is the lens narrative only; `ROADMAP.json` is the full brief
/// envelope with the `roadmap` object attached, so an agent reading the file
/// also sees sources, health, and truncation. The roadmap narrative carries no
/// whisper content, so no curation authority is consulted here. The directory
/// is never created, and staging plus rename semantics are the pair writer's,
/// so a failed write leaves both targets untouched. A missing or non-directory
/// target is a hard error.
pub fn write_roadmap_renders_to(dir: &Path, brief: &Brief) -> Result<()> {
    if !dir.is_dir() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!(
                "cannot write renders: {} is not an existing directory",
                dir.display()
            ),
        )));
    }

    let markdown = crate::render::roadmap::render(brief)?;
    let json = crate::render::envelope::to_json(brief)?;

    write_pair(
        dir,
        &[
            (ROADMAP_MARKDOWN_FILE, markdown.as_str()),
            (ROADMAP_JSON_FILE, json.as_str()),
        ],
    )
}

/// Whether curation permits whisper content in the committed render.
///
/// Absent, malformed, or silent curation means private: only an explicit
/// `publish_whisper = true` opts in.
fn publish_whisper(repo: &Path) -> bool {
    curation::load(repo)
        .ok()
        .flatten()
        .and_then(|file| file.publish_whisper)
        .unwrap_or(false)
}

/// Replace every named artifact as one unit.
///
/// Phase 1 refuses targets a rename could not replace, phase 2 stages every
/// payload as a sibling temp file, and only phase 3 renames the temps over
/// their targets. A failure in phase 1 or 2 therefore leaves both targets
/// untouched; a failure in phase 3 removes the temps it has not consumed.
/// Renames themselves are atomic per file, so a reader never observes a
/// half-written artifact, though no filesystem rename is transactional across
/// two names.
fn write_pair(repo: &Path, artifacts: &[(&str, &str)]) -> Result<()> {
    for (name, _) in artifacts {
        let target = repo.join(name);
        if target.exists() && !target.is_file() {
            return Err(Error::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("cannot replace {name}: target is not a regular file"),
            )));
        }
    }

    let mut staged: Vec<(PathBuf, PathBuf)> = Vec::with_capacity(artifacts.len());
    for (name, body) in artifacts {
        let temp = repo.join(format!(".{name}.{}.tmp", std::process::id()));
        if let Err(error) = std::fs::write(&temp, body) {
            let _ = std::fs::remove_file(&temp);
            remove_temps(&staged);
            return Err(Error::Io(error));
        }
        staged.push((temp, repo.join(name)));
    }

    for (index, (temp, target)) in staged.iter().enumerate() {
        if let Err(error) = std::fs::rename(temp, target) {
            remove_temps(&staged[index..]);
            return Err(Error::Io(error));
        }
    }
    Ok(())
}

/// Remove staged temp files that were never renamed into place.
fn remove_temps(staged: &[(PathBuf, PathBuf)]) {
    for (temp, _) in staged {
        let _ = std::fs::remove_file(temp);
    }
}
