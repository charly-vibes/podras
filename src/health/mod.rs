//! Purpose: compute named health signals and the layered health verdict.
//!
//! Responsibilities:
//! - expose `signals` and `evaluate`, the crate-internal `layer` hook, and the
//!   shared signal horizon;
//! - derive the seven D8 signals, one submodule per signal family: curation,
//!   git, specs, plus the shared tracker-linking vocabulary;
//! - keep the date-window predicates used by every signals submodule.
//!
//! Rationale: silence must never become green, and every verdict must carry
//! evidence. Keeping signal computation pure makes that rule testable without
//! a render. Signals compare a date window against the pinned `--as-of` date
//! (falling back to the caller's wall-clock date), so health is deterministic.

use chrono::NaiveDate;

use crate::brief::BriefInputs;
use crate::model::{Health, Signal};

mod curation;
mod git;
mod links;
mod specs;
mod verdict;

use curation::{focus_closed, parked_expired, stale_focus, wip_over_cap};
use git::{orphan_commits, stuck_project};
use specs::spec_change_without_bead;

pub(crate) use verdict::layer;

/// A focus, commit, or change older than this many days is stale evidence.
pub(super) const SIGNAL_HORIZON_DAYS: i64 = 14;

/// Compute the named signals that fire for the observed inputs.
pub fn signals(inputs: &BriefInputs) -> Vec<Signal> {
    let evaluation_date = inputs.as_of.unwrap_or(inputs.now);
    let curation = inputs.curation.as_ref().map(|state| &state.curation);
    let mut firing = Vec::new();

    if let Some(signal) = stale_focus(curation, evaluation_date) {
        firing.push(signal);
    }
    if let Some(signal) = wip_over_cap(curation) {
        firing.push(signal);
    }
    if let Some(signal) = stuck_project(inputs, curation, evaluation_date) {
        firing.push(signal);
    }
    if let Some(signal) = focus_closed(inputs, curation) {
        firing.push(signal);
    }
    if let Some(signal) = parked_expired(curation, evaluation_date) {
        firing.push(signal);
    }
    if let Some(signal) = orphan_commits(inputs, evaluation_date) {
        firing.push(signal);
    }
    if let Some(signal) = spec_change_without_bead(inputs) {
        firing.push(signal);
    }

    firing
}

/// Compute the layered health verdict for the observed inputs.
pub fn evaluate(inputs: &BriefInputs) -> Health {
    let evaluation_date = inputs.as_of.unwrap_or(inputs.now);
    let curation_health = inputs
        .curation
        .as_ref()
        .and_then(|state| state.curation.health.as_ref());
    let signals = signals(inputs);
    let mut warnings = Vec::new();
    let impaired_mandatory = crate::freshness::impaired_mandatory(&inputs.sources);
    layer(
        curation_health,
        evaluation_date,
        signals,
        &impaired_mandatory,
        &inputs.sources,
        &mut warnings,
    )
}

/// True when `date` is strictly more than the signal horizon before `evaluation`.
pub(super) fn older_than_horizon(date: NaiveDate, evaluation: NaiveDate) -> bool {
    (evaluation - date).num_days() > SIGNAL_HORIZON_DAYS
}

/// True when `date` falls inside the inclusive 14-day window ending at `evaluation`.
pub(super) fn within_window(date: NaiveDate, evaluation: NaiveDate) -> bool {
    let age = (evaluation - date).num_days();
    (0..=SIGNAL_HORIZON_DAYS).contains(&age)
}
