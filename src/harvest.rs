//! Purpose: harvest candidate mismatch signals from observed sources.
//!
//! Responsibilities:
//! - correlate commits, beads, and spec changes into candidate signals;
//! - report candidates only, never mutate tracker or curation state.
//!
//! Rationale: harvest proposes anomalies (orphan commits, unlinked spec
//! changes) for a human to triage; decisions stay outside the read path.

use crate::brief::BriefInputs;
use crate::error::{Error, Result};
use crate::model::Signal;

/// Harvest candidate signals from the observed inputs.
pub fn harvest(_inputs: &BriefInputs) -> Result<Vec<Signal>> {
    Err(Error::NotYetImplemented("harvest::harvest"))
}
