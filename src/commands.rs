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

use crate::error::{Error, Result};
use crate::model::Brief;

/// Render the brief for the repository.
pub fn brief(_repo: &Path, _json: bool, _write: bool, _as_of: Option<&str>) -> Result<()> {
    Err(Error::NotYetImplemented("commands::brief"))
}

/// Select the curated focus item for the repository.
pub fn focus(_repo: &Path, _id: &str) -> Result<()> {
    Err(Error::NotYetImplemented("commands::focus"))
}

/// Park an item until its resurface trigger.
pub fn park(_repo: &Path, _id: &str, _reason: &str, _resurface: &str) -> Result<()> {
    Err(Error::NotYetImplemented("commands::park"))
}

/// Initialize podras state for the repository.
pub fn init(_repo: &Path) -> Result<()> {
    Err(Error::NotYetImplemented("commands::init"))
}

/// Emit the brief JSON Schema (2020-12) to stdout.
pub fn schema() -> Result<()> {
    let schema = schemars::schema_for!(Brief);
    let rendered = serde_json::to_string_pretty(&schema)?;
    println!("{rendered}");
    Ok(())
}
