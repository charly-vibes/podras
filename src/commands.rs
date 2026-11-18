//! Purpose: implement the CLI commands shared by the binary and tests.
//!
//! Responsibilities:
//! - own one function per subcommand, operating on a resolved repository root;
//! - emit the JSON Schema for the brief model.
//!
//! Rationale: keeping commands in the library lets integration tests and the
//! binary share exactly one dispatch layer; the binary only maps errors to
//! miette and exit codes.

use std::path::Path;

use chrono::NaiveDate;

use crate::error::{Error, Result};
use crate::model::Brief;

/// Render the brief for the repository.
pub fn brief(_repo: &Path, _json: bool, _write: bool, _as_of: Option<NaiveDate>) -> Result<()> {
    Err(Error::NotYetImplemented("commands::brief"))
}

/// Select the curated focus item for the repository.
pub fn focus(repo: &Path, id: &str) -> Result<()> {
    crate::verbs::focus::run(repo, id)
}

/// Park an item until its resurface trigger.
pub fn park(repo: &Path, id: &str, reason: &str, resurface: &str) -> Result<()> {
    crate::verbs::park::run(repo, id, reason, resurface)
}

/// Initialize podras state for the repository.
pub fn init(repo: &Path) -> Result<()> {
    crate::verbs::init::run(repo)
}

/// Emit the brief JSON Schema (2020-12) to stdout.
pub fn schema() -> Result<()> {
    let schema = schemars::schema_for!(Brief);
    let rendered = serde_json::to_string_pretty(&schema)?;
    println!("{rendered}");
    Ok(())
}
