//! Purpose: give every brief section a deterministic order and render bound.
//!
//! Responsibilities:
//! - sort work items, parked items, recent events, and warnings;
//! - truncate bounded sections and record omitted counts.
//!
//! Rationale: determinism is what lets a generated render be regenerated and
//! diffed. Ordering keys and bounds live together because both decide which
//! items a reader sees and in what order.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::model::{
    BLOCKED_BOUND, FOCUS_BOUND, NEXT_BOUND, PARKED_BOUND, ParkedItem, RECENT_BOUND, RecentEvent,
    Resurface, Truncation, Warning, WorkItem,
};

use super::sections::Sections;

pub(super) fn sort_work_items(items: &mut [WorkItem]) {
    items.sort_by(|left, right| match (left.priority, right.priority) {
        (Some(left_priority), Some(right_priority)) => left_priority
            .cmp(&right_priority)
            .then_with(|| left.id.cmp(&right.id)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left.id.cmp(&right.id),
    });
}

pub(super) fn sort_parked(items: &mut [ParkedItem]) {
    items.sort_by(|left, right| match (&left.resurface, &right.resurface) {
        (Resurface::Date(left_date), Resurface::Date(right_date)) => left_date
            .cmp(right_date)
            .then_with(|| left.id.cmp(&right.id)),
        (Resurface::Date(_), Resurface::Event(_)) => Ordering::Less,
        (Resurface::Event(_), Resurface::Date(_)) => Ordering::Greater,
        (Resurface::Event(left_event), Resurface::Event(right_event)) => left_event
            .cmp(right_event)
            .then_with(|| left.id.cmp(&right.id)),
    });
}

pub(super) fn sort_recent(items: &mut [RecentEvent]) {
    items.sort_by(|left, right| {
        right
            .committed_at
            .cmp(&left.committed_at)
            .then_with(|| left.sha.cmp(&right.sha))
    });
}

pub(super) fn sort_warnings(warnings: &mut [Warning]) {
    warnings.sort_by(|left, right| {
        left.source
            .declaration_order()
            .cmp(&right.source.declaration_order())
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.detail.cmp(&right.detail))
    });
}

/// Truncate bounded sections in place and describe what was omitted.
pub(super) fn apply_bounds(mut sections: Sections) -> (Sections, Truncation) {
    let mut omitted_counts: BTreeMap<String, u32> = BTreeMap::new();
    sections.focus = truncate("focus", sections.focus, FOCUS_BOUND, &mut omitted_counts);
    sections.recent = truncate("recent", sections.recent, RECENT_BOUND, &mut omitted_counts);
    sections.blocked = truncate(
        "blocked",
        sections.blocked,
        BLOCKED_BOUND,
        &mut omitted_counts,
    );
    sections.next = truncate("next", sections.next, NEXT_BOUND, &mut omitted_counts);
    sections.parked = truncate("parked", sections.parked, PARKED_BOUND, &mut omitted_counts);
    let truncation = Truncation {
        applied: !omitted_counts.is_empty(),
        omitted_counts,
    };
    (sections, truncation)
}

fn truncate<T>(
    section: &str,
    items: Vec<T>,
    bound: usize,
    omitted_counts: &mut BTreeMap<String, u32>,
) -> Vec<T> {
    if items.len() <= bound {
        return items;
    }
    let dropped = items.len() - bound;
    omitted_counts.insert(
        section.to_string(),
        u32::try_from(dropped).unwrap_or(u32::MAX),
    );
    items.into_iter().take(bound).collect()
}
