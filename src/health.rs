//! Purpose: compute named health signals and the layered health verdict.
//!
//! Responsibilities:
//! - derive firing signals from brief inputs and curation state;
//! - layer verdicts: fresh human verdict, then signals, then unknown.
//!
//! Rationale: silence must never become green, and every verdict must carry
//! evidence. Keeping signal computation pure makes that rule testable without
//! a render.

use crate::brief::BriefInputs;
use crate::error::{Error, Result};
use crate::model::{Health, Signal};

/// Compute the named signals that fire for the observed inputs.
pub fn signals(_inputs: &BriefInputs) -> Result<Vec<Signal>> {
    Err(Error::NotYetImplemented("health::signals"))
}

/// Compute the layered health verdict for the observed inputs.
pub fn evaluate(_inputs: &BriefInputs) -> Result<Health> {
    Err(Error::NotYetImplemented("health::evaluate"))
}
