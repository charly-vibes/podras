//! Purpose: view curated focus and curation stamps through deterministic order.
//!
//! Responsibilities:
//! - order focus entries by `focused_since` then id;
//! - enrich curated focus entries from the tracker;
//! - derive the curation timestamps exposed by the brief;
//! - build curation warnings for the other steps.
//!
//! Rationale: curation beats inference. Focus membership comes only from the
//! curation file, so this module owns every curated-focus decision. Dates come
//! from the curation accessors, which accept both `YYYY-MM-DD` and RFC 3339,
//! so every brief consumer shares one timestamp policy.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use chrono::NaiveTime;

use crate::curation::{self, CurationFile, FocusEntry};
use crate::model::{CurationStamp, SourceName, Warning, WorkItem};

use super::CurationState;
use super::tracker::Tracker;

/// Curated focus ids and their enriched, deterministically ordered items.
pub(super) fn lead_focus(
    curation: Option<&CurationFile>,
    tracker: &Tracker<'_>,
) -> (BTreeSet<String>, Vec<WorkItem>) {
    let focus_entries = sorted_focus_entries(curation);
    let focus_ids: BTreeSet<String> = curation
        .map(|curation| {
            curation
                .focus
                .iter()
                .map(|entry| entry.id.clone())
                .collect()
        })
        .unwrap_or_default();
    let focus = focus_entries
        .iter()
        .map(|entry| curated_work_item(entry, tracker.get(&entry.id)))
        .collect();
    (focus_ids, focus)
}

fn sorted_focus_entries(curation: Option<&CurationFile>) -> Vec<&FocusEntry> {
    let mut entries: Vec<&FocusEntry> = curation
        .map(|curation| curation.focus.iter().collect())
        .unwrap_or_default();
    entries.sort_by(
        |left, right| match (left.focused_since(), right.focused_since()) {
            (Some(left_since), Some(right_since)) => left_since
                .cmp(&right_since)
                .then_with(|| left.id.cmp(&right.id)),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => left.id.cmp(&right.id),
        },
    );
    entries
}

fn curated_work_item(entry: &FocusEntry, tracker_item: Option<&WorkItem>) -> WorkItem {
    let mut item = tracker_item
        .cloned()
        .unwrap_or_else(|| blank_work_item(&entry.id));
    item.reason = entry.reason.clone();
    item.confirmed_at = entry.confirmed_at();
    item
}

pub(super) fn blank_work_item(id: &str) -> WorkItem {
    WorkItem {
        id: id.to_string(),
        title: None,
        status: None,
        priority: None,
        blocked_by: Vec::new(),
        age_days: None,
        reason: None,
        confirmed_at: None,
    }
}

pub(super) fn curation_warning(code: &str, detail: String) -> Warning {
    Warning {
        source: SourceName::Curation,
        code: code.to_string(),
        detail,
    }
}

pub(super) fn curation_stamp(state: Option<&CurationState>) -> CurationStamp {
    let Some(state) = state else {
        return CurationStamp {
            updated_at: None,
            confirmed_at: None,
        };
    };
    CurationStamp {
        updated_at: state.updated_at,
        confirmed_at: curation::confirmed_at(&state.curation)
            .map(|date| date.and_time(NaiveTime::MIN).and_utc()),
    }
}
