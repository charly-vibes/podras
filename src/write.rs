//! Purpose: write the generated BRIEF.md / BRIEF.json artifacts.
//!
//! Responsibilities:
//! - render and atomically write the committed brief artifacts;
//! - gate the whisper next-action line on curation `publish_whisper`;
//! - never merge hand edits into regenerated output.
//!
//! Rationale: committed renders are generated, never authored. One writer
//! keeps overwrite semantics in a single place, and temp-plus-rename keeps a
//! reader from ever observing a half-written artifact.

use std::path::Path;

use crate::curation;
use crate::error::{Error, Result};
use crate::model::Brief;

/// Committed markdown artifact name.
const MARKDOWN_FILE: &str = "BRIEF.md";
/// Committed JSON artifact name.
const JSON_FILE: &str = "BRIEF.json";

/// Write `BRIEF.md` and `BRIEF.json` for the repository.
///
/// The whisper next action is included in the markdown only when curation sets
/// `publish_whisper = true`; the JSON envelope never carries it. Both files are
/// fully regenerated, so a second write replaces hand edits rather than merging.
pub fn write_renders(repo: &Path, brief: &Brief, whisper_next_action: Option<&str>) -> Result<()> {
    let reentry = if publish_whisper(repo) {
        whisper_next_action
    } else {
        None
    };
    let markdown = crate::render::markdown::render(brief, reentry)?;
    let json = crate::render::envelope::to_json(brief)?;

    write_atomic(repo, MARKDOWN_FILE, &markdown)?;
    write_atomic(repo, JSON_FILE, &json)?;
    Ok(())
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

/// Replace one artifact atomically: write a sibling temp file, then rename.
fn write_atomic(repo: &Path, name: &str, body: &str) -> Result<()> {
    let target = repo.join(name);
    let temp = repo.join(format!(".{name}.{}.tmp", std::process::id()));
    std::fs::write(&temp, body)?;
    if let Err(error) = std::fs::rename(&temp, &target) {
        let _ = std::fs::remove_file(&temp);
        return Err(Error::Io(error));
    }
    Ok(())
}
