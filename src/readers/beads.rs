//! Purpose: read the beads issue tracker export.
//!
//! Responsibilities:
//! - parse `.beads/issues.jsonl` into open, blocked, and ready work items;
//! - report the observation time of the export.
//!
//! Rationale: beads owns issue lifecycle. The brief consumes it read-only and
//! never writes tracker state, keeping lifecycle authority in one tool.

use std::path::Path;

use crate::brief::BeadsSummary;
use crate::error::{Error, Result};

/// Read the beads export for the repository.
pub fn read(_repo: &Path) -> Result<BeadsSummary> {
    Err(Error::NotYetImplemented("readers::beads::read"))
}
