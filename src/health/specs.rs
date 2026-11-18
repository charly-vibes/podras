//! Purpose: derive the spec-change health signal.
//!
//! Responsibilities:
//! - flag recent openspec changes that name no tracker id in id or title;
//! - stay silent without a readable beads reading or specs reading.
//!
//! Rationale: a change written without a bead is invisible to the tracker, so
//! the signal reports exactly the changes the tracker cannot account for.

use super::SIGNAL_HORIZON_DAYS;
use super::links::{names_tracker, tracker_ids, tracker_prefixes};
use crate::brief::BriefInputs;
use crate::model::{Signal, SignalKind, WorkItem};

pub(super) fn spec_change_without_bead(inputs: &BriefInputs) -> Option<Signal> {
    let specs = inputs.specs.as_ref()?;
    // Known tracker tokens come from a readable beads reading; without one the
    // signal would be guesswork, so it stays silent.
    let beads = inputs.beads.as_ref()?;
    let known = tracker_ids(beads);
    let prefixes = tracker_prefixes(beads);
    let mut unlinked: Vec<&WorkItem> = specs
        .changes
        .iter()
        .filter(|item| item.status.as_deref() == Some("change"))
        .filter(|item| {
            item.age_days
                .is_some_and(|days| days <= SIGNAL_HORIZON_DAYS as u32)
        })
        .filter(|item| {
            !names_tracker(&item.id, &known, &prefixes)
                && !item
                    .title
                    .as_deref()
                    .is_some_and(|title| names_tracker(title, &known, &prefixes))
        })
        .collect();
    unlinked.sort_by(|left, right| left.id.cmp(&right.id));
    if unlinked.is_empty() {
        return None;
    }
    let evidence = unlinked
        .iter()
        .map(|item| {
            let age = item.age_days.unwrap_or_default();
            format!("`{}` changed {age} days ago", item.id)
        })
        .collect::<Vec<_>>()
        .join(", ");
    Some(Signal {
        kind: SignalKind::SpecChangeWithoutBead,
        reason: format!("openspec changes link no tracker id: {evidence}"),
    })
}
