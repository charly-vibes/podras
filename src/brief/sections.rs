//! Purpose: compute the brief's sections and assemble the final envelope.
//!
//! Responsibilities:
//! - derive in-flight, blocked, next, needs-retriage, and recent sections;
//! - exclude parked and focused items from computed queues;
//! - declare every source row in fixed order;
//! - build the `Brief` from the assembled parts.
//!
//! Rationale: every section here is computed, never curated. Keeping queue
//! construction together makes the curation-beats-inference rule visible.

use std::collections::BTreeSet;

use chrono::NaiveDate;

use crate::model::{
    Brief, Health, ParkedItem, RecentEvent, SCHEMA_VERSION, SourceName, SourceState, SourceStatus,
    Staleness, Truncation, Warning, WorkItem,
};

use super::BriefInputs;
use super::curation_view::curation_stamp;
use super::ordering;
use super::parking::Parking;

/// The brief's ordered sections before truncation.
pub(super) struct Sections {
    pub(super) focus: Vec<WorkItem>,
    pub(super) in_flight: Vec<WorkItem>,
    pub(super) blocked: Vec<WorkItem>,
    pub(super) next: Vec<WorkItem>,
    pub(super) parked: Vec<ParkedItem>,
    pub(super) needs_retriage: Vec<WorkItem>,
    pub(super) recent: Vec<RecentEvent>,
}

/// Build every section from the readings and curated parking split.
pub(super) fn computed(
    focus: Vec<WorkItem>,
    inputs: &BriefInputs,
    parking: Parking,
    focus_ids: &BTreeSet<String>,
) -> Sections {
    let parked_ids = parking.ids();
    let mut in_flight = open_items(inputs, &parked_ids);
    let mut blocked = blocked_items(inputs, &parked_ids);
    let mut next = ready_items(inputs, &parked_ids, focus_ids);
    let mut needs_retriage = parking.expired;
    let mut parked = parking.parked;
    let mut recent = recent_events(inputs);

    ordering::sort_work_items(&mut in_flight);
    ordering::sort_work_items(&mut blocked);
    ordering::sort_work_items(&mut next);
    ordering::sort_work_items(&mut needs_retriage);
    ordering::sort_parked(&mut parked);
    ordering::sort_recent(&mut recent);

    Sections {
        focus,
        in_flight,
        blocked,
        next,
        parked,
        needs_retriage,
        recent,
    }
}

fn open_items(inputs: &BriefInputs, parked_ids: &BTreeSet<String>) -> Vec<WorkItem> {
    inputs
        .beads
        .as_ref()
        .map(|beads| {
            beads
                .open
                .iter()
                .filter(|item| !parked_ids.contains(&item.id))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn blocked_items(inputs: &BriefInputs, parked_ids: &BTreeSet<String>) -> Vec<WorkItem> {
    inputs
        .beads
        .as_ref()
        .map(|beads| {
            beads
                .blocked
                .iter()
                .filter(|item| !parked_ids.contains(&item.id))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn ready_items(
    inputs: &BriefInputs,
    parked_ids: &BTreeSet<String>,
    focus_ids: &BTreeSet<String>,
) -> Vec<WorkItem> {
    inputs
        .beads
        .as_ref()
        .map(|beads| {
            beads
                .ready
                .iter()
                .filter(|item| !parked_ids.contains(&item.id) && !focus_ids.contains(&item.id))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn recent_events(inputs: &BriefInputs) -> Vec<RecentEvent> {
    inputs
        .git
        .as_ref()
        .map(|git| git.recent.clone())
        .unwrap_or_default()
}

/// Build the typed envelope, leaving `source_hash` for the caller to stamp.
pub(super) fn envelope(
    inputs: &BriefInputs,
    evaluation_date: NaiveDate,
    sections: Sections,
    truncation: Truncation,
    health: Health,
    warnings: Vec<Warning>,
) -> Brief {
    let curation = inputs.curation.as_ref().map(|state| &state.curation);
    let Sections {
        focus,
        in_flight,
        blocked,
        next,
        parked,
        needs_retriage,
        recent,
    } = sections;
    Brief {
        schema_version: SCHEMA_VERSION.to_string(),
        as_of: evaluation_date,
        generated_at: inputs.generated_at,
        sources: declared_sources(&inputs.sources),
        curation: curation_stamp(inputs.curation.as_ref()),
        purpose_lite: curation.and_then(|curation| curation.purpose_lite.clone()),
        focus,
        in_flight,
        blocked,
        next,
        parked,
        needs_retriage,
        recent,
        health,
        truncation,
        head_sha: inputs.git.as_ref().and_then(|git| git.head_sha.clone()),
        source_hash: None,
        warnings,
        roadmap: None,
    }
}

fn declared_sources(rows: &[SourceStatus]) -> Vec<SourceStatus> {
    SourceName::DECLARED
        .iter()
        .map(|name| {
            rows.iter()
                .find(|row| row.name == *name)
                .cloned()
                .unwrap_or(SourceStatus {
                    name: *name,
                    status: SourceState::Missing,
                    observed_at: None,
                    staleness: Staleness::Unknown,
                    detail: None,
                })
        })
        .collect()
}
