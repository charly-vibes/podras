//! Purpose: translate curated parked entries into parked and expired items.
//!
//! Responsibilities:
//! - parse mandatory resurface triggers, warning on invalid entries;
//! - split entries into still-parked and expired (needs re-triage);
//! - enrich expired items from the tracker when available.
//!
//! Rationale: parking is curated state, never inferred. A parked item only
//! re-enters attention when its date trigger passes, so the split here is the
//! single place that decides which queue owns an entry.

use std::collections::BTreeSet;

use chrono::NaiveDate;

use crate::curation::{CurationFile, ParkEntry};
use crate::model::{ParkedItem, Resurface, Warning, WorkItem};

use super::curation_view::{blank_work_item, curation_warning};
use super::tracker::Tracker;

/// Parked entries split into still-parked and expired (needs re-triage).
#[derive(Default)]
pub(super) struct Parking {
    pub(super) parked: Vec<ParkedItem>,
    pub(super) expired: Vec<WorkItem>,
}

impl Parking {
    /// Identifiers of every parked and expired entry.
    pub(super) fn ids(&self) -> BTreeSet<String> {
        self.parked
            .iter()
            .map(|item| item.id.clone())
            .chain(self.expired.iter().map(|item| item.id.clone()))
            .collect()
    }
}

/// One curated entry that parsed cleanly.
enum Resolved {
    Parked(ParkedItem),
    Expired(WorkItem),
}

pub(super) fn resolve_parking(
    curation: Option<&CurationFile>,
    tracker: &Tracker<'_>,
    evaluation_date: NaiveDate,
    warnings: &mut Vec<Warning>,
) -> Parking {
    let mut parking = Parking::default();
    let Some(curation) = curation else {
        return parking;
    };
    for entry in &curation.parked {
        match resolve_entry(entry, tracker, evaluation_date, warnings) {
            Some(Resolved::Parked(item)) => parking.parked.push(item),
            Some(Resolved::Expired(item)) => parking.expired.push(item),
            None => {}
        }
    }
    parking
}

fn resolve_entry(
    entry: &ParkEntry,
    tracker: &Tracker<'_>,
    evaluation_date: NaiveDate,
    warnings: &mut Vec<Warning>,
) -> Option<Resolved> {
    let resurface = parse_resurface(entry, warnings)?;
    let (parked_at, confirmed_at) = parked_dates(entry, warnings)?;
    if is_expired(&resurface, evaluation_date) {
        let item = expired_item(entry, tracker, confirmed_at);
        return Some(Resolved::Expired(item));
    }
    Some(Resolved::Parked(ParkedItem {
        id: entry.id.clone(),
        reason: entry.reason.clone(),
        parked_at,
        confirmed_at,
        resurface,
    }))
}

fn parse_resurface(entry: &ParkEntry, warnings: &mut Vec<Warning>) -> Option<Resurface> {
    match entry.resurface.parse::<Resurface>() {
        Ok(resurface) => Some(resurface),
        Err(error) => {
            warnings.push(curation_warning(
                "curation_invalid_resurface",
                error.to_string(),
            ));
            None
        }
    }
}

fn parked_dates(entry: &ParkEntry, warnings: &mut Vec<Warning>) -> Option<(NaiveDate, NaiveDate)> {
    let confirmed_at = entry.confirmed_at();
    let Some(parked_at) = entry.parked_at().or(confirmed_at) else {
        warnings.push(curation_warning(
            "curation_invalid_entry",
            format!(
                "parked entry `{}` has no usable parked_at or confirmed_at date",
                entry.id
            ),
        ));
        return None;
    };
    Some((parked_at, confirmed_at.unwrap_or(parked_at)))
}

fn is_expired(resurface: &Resurface, evaluation_date: NaiveDate) -> bool {
    matches!(resurface, Resurface::Date(date) if *date <= evaluation_date)
}

fn expired_item(entry: &ParkEntry, tracker: &Tracker<'_>, confirmed_at: NaiveDate) -> WorkItem {
    let mut item = tracker
        .get(&entry.id)
        .cloned()
        .unwrap_or_else(|| blank_work_item(&entry.id));
    item.reason = Some(entry.reason.clone());
    item.confirmed_at = Some(confirmed_at);
    item
}
