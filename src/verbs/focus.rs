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
use crate::error::{Error, Result};
use crate::readers::{self, ReadContext};

/// Focus cap used when curation does not declare one.
const DEFAULT_MAX_FOCUS: usize = 1;

/// Select the curated focus item for the repository.
pub fn run(repo: &Path, id: &str) -> Result<()> {
    let mut curation = curation::load(repo)?.unwrap_or_default();
    resolve(repo, id)?;
    let today = today();

    if let Some(entry) = curation.focus.iter_mut().find(|entry| entry.id == id) {
        entry.confirmed_at = Some(today.clone());
        curation::save(repo, &curation)?;
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
    println!("focused {id}");
    Ok(())
}

/// Reject ids the readable beads export does not list as open.
///
/// An absent or unreadable export cannot answer the question, so the verb
/// accepts the id instead of inventing a tracker condition the repo never made.
fn resolve(repo: &Path, id: &str) -> Result<()> {
    let context = ReadContext {
        repo_root: repo.to_path_buf(),
        as_of: None,
        branch: None,
    };
    let Some(beads) = readers::beads::read(&context).data else {
        return Ok(());
    };
    if beads.open.iter().any(|item| item.id == id) {
        return Ok(());
    }
    Err(Error::Domain {
        code: "focus_unresolved",
        message: format!("`{id}` is not an open issue in the beads export"),
        suggestion: Some("run bd list".to_string()),
    })
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
