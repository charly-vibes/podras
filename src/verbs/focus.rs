//! Purpose: implement `podras focus <id>`.
//!
//! Responsibilities:
//! - resolve the requested id against the readable beads export;
//! - refresh an existing focus or replace the oldest one within `max_focus`;
//! - persist through the curation module and report each decision on stdout.
//!
//! Rationale: focus is human intent, so the verb edits curation state and never
//! tracker priority. The cap is enforced by demoting the oldest entry with a
//! visible notice, never by silently dropping a human decision.

use std::path::Path;

use chrono::NaiveDate;

use crate::curation::{self, CurationFile, FocusEntry};
use crate::error::Result;

/// Focus cap used when curation does not declare one.
const DEFAULT_MAX_FOCUS: usize = 1;

/// Select the curated focus item for the repository.
pub fn run(repo: &Path, id: &str) -> Result<()> {
    super::resolve_open_item(repo, id, "focus_unresolved")?;
    let mut curation = curation::load(repo)?.unwrap_or_default();
    let unparked = drop_parked(&mut curation, id);
    let today = today();

    if let Some(entry) = curation.focus.iter_mut().find(|entry| entry.id == id) {
        entry.confirmed_at = Some(today.clone());
        curation::save(repo, &curation)?;
        report_unparked(unparked, id);
        println!("already focused {id}; confirmed {today}");
        return Ok(());
    }

    let demoted = make_room(&mut curation);
    curation.focus.push(FocusEntry {
        id: id.to_string(),
        reason: None,
        focused_since: Some(today.clone()),
        confirmed_at: Some(today),
    });
    curation::save(repo, &curation)?;
    for old in demoted {
        println!("demoted {old}: focus replaced by {id}");
    }
    report_unparked(unparked, id);
    println!("focused {id}");
    Ok(())
}

/// Drop the parked entry for `id`, reporting whether one was removed.
///
/// Focusing is the re-triage transition: a human bringing an expired park back
/// into attention is unparking it, so the stale row must not keep feeding
/// `needs_retriage` alongside the focus entry.
fn drop_parked(curation: &mut CurationFile, id: &str) -> bool {
    let before = curation.parked.len();
    curation.parked.retain(|entry| entry.id != id);
    curation.parked.len() != before
}

/// Print the unpark notice after the file write already succeeded.
fn report_unparked(unparked: bool, id: &str) {
    if unparked {
        println!("unparked {id}");
    }
}

/// Demote focus entries until one more fits under the cap, returning their ids.
///
/// Purpose, health, and parked state are untouched: only focus competes for the
/// cap, and a demotion is a visible, reversible curation edit.
fn make_room(curation: &mut CurationFile) -> Vec<String> {
    let max_focus = curation.max_focus.unwrap_or(DEFAULT_MAX_FOCUS);
    let mut demoted = Vec::new();
    while !curation.focus.is_empty() && curation.focus.len() >= max_focus {
        let index = oldest_index(&curation.focus);
        demoted.push(curation.focus.remove(index).id);
    }
    demoted
}

/// Index of the focus entry with the earliest `focused_since`, undated first.
fn oldest_index(focus: &[FocusEntry]) -> usize {
    let mut oldest = 0;
    for (index, entry) in focus.iter().enumerate() {
        if since_key(entry) < since_key(&focus[oldest]) {
            oldest = index;
        }
    }
    oldest
}

/// Ordering key: entries without a parseable origin date count as oldest.
fn since_key(entry: &FocusEntry) -> NaiveDate {
    entry.focused_since().unwrap_or(NaiveDate::MIN)
}

/// Today's UTC date in the `YYYY-MM-DD` form curation stores.
fn today() -> String {
    chrono::Utc::now().date_naive().to_string()
}
