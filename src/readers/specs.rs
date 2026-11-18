//! Purpose: read the OpenSpec change store.
//!
//! Responsibilities:
//! - enumerate open spec changes as work items;
//! - count changes whose proposals look stale.
//!
//! Rationale: the spec store is best-effort; its absence must only produce a
//! missing-source warning, so this reader has no authority to abort a brief.

use std::path::Path;

use crate::brief::SpecSummary;
use crate::error::{Error, Result};

/// Read the OpenSpec change store for the repository.
pub fn read(_repo: &Path) -> Result<SpecSummary> {
    Err(Error::NotYetImplemented("readers::specs::read"))
}
