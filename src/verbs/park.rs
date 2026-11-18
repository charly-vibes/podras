//! Purpose: implement `podras park <id> --reason <text> --resurface <trigger>`.
//!
//! Responsibilities:
//! - reject an empty reason and an unparseable resurface trigger;
//! - demote the item when it is focused, printing an unpark notice;
//! - upsert the parked entry (id, reason, trigger, parked/confirmed today);
//! - persist the change atomically through the curation module.
//!
//! Rationale: an item may only leave attention with a stated reason and a
//! concrete trigger, and parking is a demotion, so the item cannot stay
//! focused. Both rules are enforced before curation is written, which keeps
//! parked state reviewable and the file untouched on rejection.

use std::path::Path;

use chrono::Utc;

use crate::curation::{self, CurationFile, ParkEntry};
use crate::error::{Error, Result};
use crate::model::Resurface;

/// Park an item until its resurface trigger.
pub fn run(repo: &Path, id: &str, reason: &str, resurface: &str) -> Result<()> {
    let reason = required_reason(reason)?;
    let trigger = parse_trigger(resurface)?;
    super::resolve_open_item(repo, id, "park_unresolved")?;
    let mut curation = curation::load(repo)?.unwrap_or_default();
    let unfocused = unpark_focused(&mut curation, id);
    upsert(&mut curation, id, reason, &trigger, &today_stamp());
    curation::save(repo, &curation)?;
    if unfocused {
        println!("unfocused {id}: a parked item never stays focused");
    }
    println!("parked {id} until {trigger}");
    Ok(())
}

/// Trimmed, non-empty park reason.
fn required_reason(raw: &str) -> Result<&str> {
    let reason = raw.trim();
    if reason.is_empty() {
        return Err(Error::Domain {
            code: "park_reason_required",
            message: "parking requires a non-empty --reason".to_string(),
            suggestion: Some(
                "state why the item leaves attention, e.g. --reason \"harvest is v1.1\""
                    .to_string(),
            ),
        });
    }
    Ok(reason)
}

/// Parse the mandatory date-or-event resurface trigger.
fn parse_trigger(raw: &str) -> Result<Resurface> {
    raw.trim()
        .parse::<Resurface>()
        .map_err(|error| Error::Domain {
            code: "park_invalid_resurface",
            message: format!(
                "resurface trigger `{}` is not accepted; use `YYYY-MM-DD` or `event:<name>`",
                error.raw
            ),
            suggestion: Some(
                "pass --resurface 2026-12-01 or --resurface event:value-prop-roadmap".to_string(),
            ),
        })
}

/// Remove every focus entry for `id`, reporting whether anything was removed.
fn unpark_focused(curation: &mut CurationFile, id: &str) -> bool {
    let before = curation.focus.len();
    curation.focus.retain(|entry| entry.id != id);
    curation.focus.len() != before
}

/// Replace the entry for `id`, or append it when the item is not parked yet.
fn upsert(curation: &mut CurationFile, id: &str, reason: &str, trigger: &Resurface, today: &str) {
    let entry = ParkEntry {
        id: id.to_string(),
        reason: reason.to_string(),
        resurface: trigger.to_string(),
        parked_at: Some(today.to_string()),
        confirmed_at: Some(today.to_string()),
    };
    match curation
        .parked
        .iter_mut()
        .find(|existing| existing.id == id)
    {
        Some(existing) => *existing = entry,
        None => curation.parked.push(entry),
    }
}

/// Calendar date of this run, in the `YYYY-MM-DD` form humans hand-edit.
fn today_stamp() -> String {
    Utc::now().date_naive().format("%Y-%m-%d").to_string()
}
