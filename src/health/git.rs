//! Purpose: derive health signals from git evidence and tracker coverage.
//!
//! Responsibilities:
//! - flag in-flight work whose repository has gone quiet;
//! - flag window commits whose subjects link no tracker id or open path.
//!
//! Rationale: git answers what actually changed, while beads answers what was
//! planned. A project is stuck when open work exists but no commit landed in
//! the horizon; commits are orphans when their subjects name neither an id nor
//! an open work-item path.

use std::collections::BTreeSet;

use chrono::NaiveDate;

use super::SIGNAL_HORIZON_DAYS;
use super::links::{names_tracker, open_work_paths, touches_path, tracker_ids, tracker_prefixes};
use super::{older_than_horizon, within_window};
use crate::brief::{BeadsSummary, BriefInputs, GitSummary};
use crate::curation::CurationFile;
use crate::model::{RecentEvent, Signal, SignalKind, WorkItem};

pub(super) fn stuck_project(
    inputs: &BriefInputs,
    curation: Option<&CurationFile>,
    evaluation: NaiveDate,
) -> Option<Signal> {
    let in_flight = in_flight_items(inputs, curation);
    if in_flight.is_empty() {
        return None;
    }
    let newest = inputs
        .git
        .as_ref()?
        .recent
        .iter()
        .map(|event| event.committed_at)
        .max()?;
    if !older_than_horizon(newest.date_naive(), evaluation) {
        return None;
    }
    Some(Signal {
        kind: SignalKind::StuckProject,
        reason: format!(
            "{} in-flight items but the newest commit was {} (older than {} days)",
            in_flight.len(),
            newest.date_naive(),
            SIGNAL_HORIZON_DAYS
        ),
    })
}

/// Open tracker items that are not parked, mirroring the in-flight section.
fn in_flight_items<'a>(
    inputs: &'a BriefInputs,
    curation: Option<&'a CurationFile>,
) -> Vec<&'a WorkItem> {
    let parked: BTreeSet<&str> = curation
        .map(|curation| {
            curation
                .parked
                .iter()
                .map(|entry| entry.id.as_str())
                .collect()
        })
        .unwrap_or_default();
    inputs
        .beads
        .as_ref()
        .map(|beads| {
            beads
                .open
                .iter()
                .filter(|item| !parked.contains(item.id.as_str()))
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn orphan_commits(inputs: &BriefInputs, evaluation: NaiveDate) -> Option<Signal> {
    let beads = inputs.beads.as_ref()?;
    let git = inputs.git.as_ref()?;
    let orphans = unlinked_commits(beads, git, evaluation);
    if orphans.is_empty() {
        return None;
    }
    let evidence = orphans
        .iter()
        .map(|event| format!("{} ({})", event.sha, event.subject))
        .collect::<Vec<_>>()
        .join("; ");
    Some(Signal {
        kind: SignalKind::OrphanCommits,
        reason: format!(
            "{} commits in the {SIGNAL_HORIZON_DAYS}-day window link no tracker id or open work-item path: {evidence}",
            orphans.len()
        ),
    })
}

/// Window commits whose subject names no tracker id or open work-item path.
fn unlinked_commits<'a>(
    beads: &BeadsSummary,
    git: &'a GitSummary,
    evaluation: NaiveDate,
) -> Vec<&'a RecentEvent> {
    let known = tracker_ids(beads);
    let prefixes = tracker_prefixes(beads);
    let paths = open_work_paths(beads);
    let mut orphans: Vec<&RecentEvent> = git
        .recent
        .iter()
        .filter(|event| within_window(event.committed_at.date_naive(), evaluation))
        .filter(|event| {
            !names_tracker(&event.subject, &known, &prefixes)
                && !touches_path(&event.subject, &paths)
        })
        .collect();
    orphans.sort_by(|left, right| left.sha.cmp(&right.sha));
    orphans
}
