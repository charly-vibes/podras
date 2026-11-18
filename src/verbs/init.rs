//! Purpose: implement `podras init`.
//!
//! Responsibilities:
//! - scaffold `.podras/curation.toml` when absent without overwriting content;
//! - inject or refresh the AGENTS.md managed block idempotently.
//!
//! Rationale: init is the only onboarding mutation; idempotence is the property
//! that lets humans and agents run it unconditionally.

use std::path::Path;

use crate::error::{Error, Result};

/// Initialize podras state for the repository.
pub fn run(_repo: &Path) -> Result<()> {
    Err(Error::NotYetImplemented("verbs::init::run"))
}
