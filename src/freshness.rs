//! Purpose: judge per-source freshness.
//!
//! Responsibilities:
//! - classify one source's observation instant as fresh, stale, or unknown;
//! - compare against the requested `--as-of` instant when pinned.
//!
//! Rationale: staleness is per source, never project-wide. A single small
//! classifier keeps every caller from inventing its own horizon logic.

use crate::error::{Error, Result};
use crate::model::{SourceName, Staleness};

/// Classify one source reading.
pub fn evaluate(
    _source: SourceName,
    _observed_at: Option<&str>,
    _as_of: Option<&str>,
) -> Result<Staleness> {
    Err(Error::NotYetImplemented("freshness::evaluate"))
}
