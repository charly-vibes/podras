//! Purpose: implement `podras focus <id>`.
//!
//! Responsibilities:
//! - select-or-replace the curated focus, respecting `max_focus`;
//! - persist the change through the curation module.
//!
//! Rationale: focus is human intent, so the verb edits curation state and never
//! tracker priority.

use std::path::Path;

use crate::error::{Error, Result};

/// Select the curated focus item for the repository.
pub fn run(_repo: &Path, _id: &str) -> Result<()> {
    Err(Error::NotYetImplemented("verbs::focus::run"))
}
