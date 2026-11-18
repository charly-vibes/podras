//! Purpose: write the generated BRIEF.md / BRIEF.json artifacts.
//!
//! Responsibilities:
//! - render and atomically write the committed brief artifacts;
//! - never merge hand edits into regenerated output.
//!
//! Rationale: committed renders are generated, never authored. One writer
//! keeps overwrite semantics in a single place.

use std::path::Path;

use crate::error::{Error, Result};
use crate::model::Brief;

/// Write `BRIEF.md` and `BRIEF.json` for the repository.
pub fn write_renders(_repo: &Path, _brief: &Brief) -> Result<()> {
    Err(Error::NotYetImplemented("write::write_renders"))
}
