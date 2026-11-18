//! Purpose: resolve the human health verdict layer.
//!
//! Responsibilities:
//! - accept only fresh, valid human verdicts;
//! - warn on unparseable status values;
//! - fall back through signals, then unknown, when no fresh verdict exists.
//!
//! Rationale: silence is not green. This hook keeps the human layer explicit
//! while `crate::health` owns the approved D8 order: fresh human verdict,
//! then computed signals, then unknown. Signals always travel with the verdict
//! that they influenced.

use chrono::NaiveDate;

use crate::curation::CurationHealth;
use crate::model::{Health, Signal, Warning};

pub(super) fn resolve_health(
    health: Option<&CurationHealth>,
    evaluation_date: NaiveDate,
    signals: Vec<Signal>,
    warnings: &mut Vec<Warning>,
) -> Health {
    crate::health::layer(health, evaluation_date, signals, warnings)
}
