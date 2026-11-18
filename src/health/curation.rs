//! Purpose: derive health signals from the curation state alone.
//!
//! Responsibilities:
//! - flag focus entries whose latest focus or confirmation is stale;
//! - flag focus lists over the WIP cap;
//! - flag focused items that are no longer open in the tracker;
//! - flag parked items whose resurface date has arrived.
//!
//! Rationale: curation is the human's stated intent, so these signals compare
//! it against the tracker and the pinned evaluation date only, never the clock.

use std::collections::BTreeSet;

use chrono::NaiveDate;

use super::{SIGNAL_HORIZON_DAYS, older_than_horizon};
use crate::brief::BriefInputs;
use crate::curation::{CurationFile, FocusEntry, ParkEntry};
use crate::model::{Resurface, Signal, SignalKind};

/// Focus cap used when curation does not declare one.
const DEFAULT_MAX_FOCUS: usize = 1;

/// The freshest date a focus entry can offer, if any.
fn focus_latest(entry: &FocusEntry) -> Option<NaiveDate> {
    entry
        .focused_since()
        .into_iter()
        .chain(entry.confirmed_at())
        .max()
}

pub(super) fn stale_focus(
    curation: Option<&CurationFile>,
    evaluation: NaiveDate,
) -> Option<Signal> {
    let curation = curation?;
    let mut stale: Vec<&FocusEntry> = curation
        .focus
        .iter()
        .filter(|entry| {
            focus_latest(entry).is_some_and(|date| older_than_horizon(date, evaluation))
        })
        .collect();
    stale.sort_by(|left, right| left.id.cmp(&right.id));
    if stale.is_empty() {
        return None;
    }
    let evidence = stale
        .iter()
        .map(|entry| {
            let latest = focus_latest(entry).expect("stale focus has a date");
            format!("`{}` last focused/confirmed {latest}", entry.id)
        })
        .collect::<Vec<_>>()
        .join(", ");
    Some(Signal {
        kind: SignalKind::StaleFocus,
        reason: format!("focus older than {SIGNAL_HORIZON_DAYS} days: {evidence}"),
    })
}

pub(super) fn wip_over_cap(curation: Option<&CurationFile>) -> Option<Signal> {
    let curation = curation?;
    let max_focus = curation.max_focus.unwrap_or(DEFAULT_MAX_FOCUS);
    if curation.focus.len() <= max_focus {
        return None;
    }
    let mut ids: Vec<&str> = curation
        .focus
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    ids.sort_unstable();
    Some(Signal {
        kind: SignalKind::WipOverCap,
        reason: format!(
            "{} focused items exceed max_focus {max_focus}: {}",
            ids.len(),
            ids.join(", ")
        ),
    })
}

pub(super) fn focus_closed(
    inputs: &BriefInputs,
    curation: Option<&CurationFile>,
) -> Option<Signal> {
    let curation = curation?;
    if curation.focus.is_empty() {
        return None;
    }
    // Without a readable tracker there is no evidence that a focus closed.
    let beads = inputs.beads.as_ref()?;
    let open: BTreeSet<&str> = beads.open.iter().map(|item| item.id.as_str()).collect();
    let mut closed: Vec<&str> = curation
        .focus
        .iter()
        .map(|entry| entry.id.as_str())
        .filter(|id| !open.contains(id))
        .collect();
    closed.sort_unstable();
    if closed.is_empty() {
        return None;
    }
    Some(Signal {
        kind: SignalKind::FocusClosed,
        reason: format!(
            "focused items are not open in the tracker: {}",
            closed.join(", ")
        ),
    })
}

pub(super) fn parked_expired(
    curation: Option<&CurationFile>,
    evaluation: NaiveDate,
) -> Option<Signal> {
    let curation = curation?;
    let mut expired: Vec<(&ParkEntry, NaiveDate)> = curation
        .parked
        .iter()
        .filter_map(|entry| match entry.resurface.parse::<Resurface>() {
            Ok(Resurface::Date(date)) if date <= evaluation => Some((entry, date)),
            // Event triggers have no read-path source that can say they fired.
            _ => None,
        })
        .collect();
    expired.sort_by(|left, right| left.0.id.cmp(&right.0.id));
    if expired.is_empty() {
        return None;
    }
    let evidence = expired
        .iter()
        .map(|(entry, date)| format!("`{}` reached {date}", entry.id))
        .collect::<Vec<_>>()
        .join(", ");
    Some(Signal {
        kind: SignalKind::ParkedExpired,
        reason: format!("parked items passed their resurface date: {evidence}"),
    })
}
