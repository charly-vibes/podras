//! Purpose: implement `podras park <id> --reason <text> --resurface <trigger>`.
//!
//! Responsibilities:
//! - require a reason and a resurface trigger;
//! - persist the parked entry through the curation module.
//!
//! Rationale: an item may only leave attention with a stated reason and a
//! concrete trigger, which is what keeps parked state reviewable.

use std::path::Path;

use crate::error::{Error, Result};

/// Park an item until its resurface trigger.
pub fn run(_repo: &Path, _id: &str, _reason: &str, _resurface: &str) -> Result<()> {
    Err(Error::NotYetImplemented("verbs::park::run"))
}
