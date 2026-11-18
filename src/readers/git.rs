//! Purpose: read git history and HEAD state.
//!
//! Responsibilities:
//! - report HEAD sha and recent commits with timestamps;
//! - report the observation time of the reading.
//!
//! Rationale: git is the mandatory evidence source for what actually changed;
//! the reader stays read-only and leaves interpretation to harvest/health.

use std::path::Path;

use crate::brief::GitSummary;
use crate::error::{Error, Result};

/// Read git HEAD and recent commits for the repository.
pub fn read(_repo: &Path) -> Result<GitSummary> {
    Err(Error::NotYetImplemented("readers::git::read"))
}
