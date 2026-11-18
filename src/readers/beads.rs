//! Purpose: read the beads issue tracker export.
//!
//! Responsibilities:
//! - parse `.beads/issues.jsonl` into open, blocked, and ready work items;
//! - count malformed lines into one warning while keeping the source present;
//! - report the observation time of the export.
//!
//! Rationale: beads owns issue lifecycle. The brief consumes it read-only and
//! never writes tracker state, keeping lifecycle authority in one tool. Partial
//! or corrupt exports degrade to warnings, never to an aborted brief.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;

use crate::brief::BeadsSummary;
use crate::model::{SourceName, SourceState, SourceStatus, Staleness, Warning, WorkItem};
use crate::readers::{ReadContext, SourceRead};

/// Repository-relative path of the beads export.
const ISSUES_PATH: &str = ".beads/issues.jsonl";

/// One JSONL row; only fields the brief consumes are decoded.
#[derive(Debug, Deserialize)]
struct RawIssue {
    #[serde(rename = "_type", default)]
    kind: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    priority: Option<u8>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
    #[serde(default)]
    dependencies: Vec<RawDependency>,
    /// Recorded for the brief contract; `WorkItem` exposes no file list yet.
    #[serde(rename = "metadata", default)]
    _metadata: Option<RawMetadata>,
}

/// One dependency edge as exported by beads.
#[derive(Debug, Deserialize)]
struct RawDependency {
    #[serde(default)]
    depends_on_id: Option<String>,
    #[serde(rename = "type", default)]
    kind: Option<String>,
}

/// Optional issue metadata.
#[derive(Debug, Deserialize)]
struct RawMetadata {
    #[serde(rename = "files", default)]
    _files: Option<Vec<String>>,
}

/// Read the beads export for the repository.
///
/// A missing export is reported as `missing` with a `beads_missing` warning;
/// unparseable rows are skipped and counted into one `beads_parse_error`
/// warning while the source stays `present`.
pub fn read(context: &ReadContext) -> SourceRead<BeadsSummary> {
    let path = context.repo_root.join(ISSUES_PATH);
    let text = match load_export(&path) {
        Ok(text) => text,
        Err(reading) => return *reading,
    };

    let evaluation_date = context.as_of.unwrap_or_else(|| Utc::now().date_naive());
    let (issues, skipped) = parse_issues(&text);
    let classified = classify(&issues, evaluation_date);
    // An empty or timestamp-free export still has an observation instant: the
    // export file's own mtime is the evidence of when beads last wrote it.
    let observed_at = classified.observed_at.or_else(|| modified_at(&path));
    let (detail, warnings) = parse_warnings(skipped);

    SourceRead {
        data: Some(BeadsSummary {
            open: classified.open,
            blocked: classified.blocked,
            ready: classified.ready,
            observed_at,
        }),
        status: SourceStatus {
            name: SourceName::Beads,
            status: SourceState::Present,
            observed_at,
            staleness: Staleness::Unknown,
            detail,
        },
        warnings,
    }
}

/// Open issues after one classification pass, plus the newest update seen.
struct Classified {
    open: Vec<WorkItem>,
    ready: Vec<WorkItem>,
    blocked: Vec<WorkItem>,
    observed_at: Option<DateTime<Utc>>,
}

/// Load the export, mapping absence to `missing` and other IO errors to `unreadable`.
fn load_export(path: &Path) -> std::result::Result<String, Box<SourceRead<BeadsSummary>>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(Box::new(missing())),
        Err(error) => Err(Box::new(unreadable(&error))),
    }
}

/// Export file mtime, used when no row carries a parseable update time.
fn modified_at(path: &Path) -> Option<DateTime<Utc>> {
    fs::metadata(path)
        .ok()?
        .modified()
        .ok()
        .map(DateTime::<Utc>::from)
}

/// Report an export that exists but could not be read.
fn unreadable(error: &std::io::Error) -> SourceRead<BeadsSummary> {
    SourceRead {
        data: None,
        status: SourceStatus {
            name: SourceName::Beads,
            status: SourceState::Unreadable,
            observed_at: None,
            staleness: Staleness::Unknown,
            detail: Some(format!("{ISSUES_PATH}: {error}")),
        },
        warnings: vec![Warning {
            source: SourceName::Beads,
            code: "beads_unreadable".to_string(),
            detail: format!("could not read {ISSUES_PATH}: {error}"),
        }],
    }
}

/// Split open issues into open/ready/blocked views, tracking the newest update.
fn classify(issues: &[RawIssue], evaluation_date: NaiveDate) -> Classified {
    let statuses = statuses(issues);
    let mut classified = Classified {
        open: Vec::new(),
        ready: Vec::new(),
        blocked: Vec::new(),
        observed_at: None,
    };

    for issue in issues {
        let Some(id) = issue.id.as_deref() else {
            continue;
        };
        if let Some(updated_at) = parse_instant(issue.updated_at.as_deref()) {
            classified.observed_at = Some(
                classified
                    .observed_at
                    .map_or(updated_at, |newest| newest.max(updated_at)),
            );
        }
        if !is_open(issue.status.as_deref()) {
            continue;
        }

        let item = work_item(issue, id, evaluation_date, blocked_by(issue, &statuses));
        if !item.blocked_by.is_empty() {
            classified.blocked.push(item.clone());
        } else if is_claimable(issue.status.as_deref()) {
            classified.ready.push(item.clone());
        }
        classified.open.push(item);
    }

    classified
}

/// Open/closed state per issue id, used to decide which blockers count.
fn statuses(issues: &[RawIssue]) -> BTreeMap<&str, bool> {
    issues
        .iter()
        .filter_map(|issue| {
            let id = issue.id.as_deref()?;
            Some((id, is_open(issue.status.as_deref())))
        })
        .collect()
}

/// Blocking dependency ids that still refer to open issues.
fn blocked_by(issue: &RawIssue, statuses: &BTreeMap<&str, bool>) -> Vec<String> {
    issue
        .dependencies
        .iter()
        .filter(|dependency| dependency.kind.as_deref() == Some("blocks"))
        .filter_map(|dependency| dependency.depends_on_id.as_deref())
        .filter(|blocker| statuses.get(blocker).copied().unwrap_or(false))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(str::to_string)
        .collect()
}

/// The parse warning and its status detail for malformed lines.
fn parse_warnings(skipped: usize) -> (Option<String>, Vec<Warning>) {
    if skipped == 0 {
        return (None, Vec::new());
    }
    let detail = format!("{skipped} malformed line(s) skipped in {ISSUES_PATH}");
    let warnings = vec![Warning {
        source: SourceName::Beads,
        code: "beads_parse_error".to_string(),
        detail: detail.clone(),
    }];
    (Some(detail), warnings)
}

/// Read every decodable issue row, counting rows that cannot be decoded.
fn parse_issues(text: &str) -> (Vec<RawIssue>, usize) {
    let mut issues = Vec::new();
    let mut skipped = 0;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(raw) = serde_json::from_str::<RawIssue>(line) else {
            skipped += 1;
            continue;
        };
        if raw.kind.as_deref() != Some("issue") {
            continue;
        }
        match raw.id.as_deref() {
            Some(id) if !id.trim().is_empty() => issues.push(raw),
            _ => skipped += 1,
        }
    }

    (issues, skipped)
}

/// Build the brief's view of one open issue.
fn work_item(
    issue: &RawIssue,
    id: &str,
    evaluation_date: NaiveDate,
    blocked_by: Vec<String>,
) -> WorkItem {
    WorkItem {
        id: id.to_string(),
        title: issue.title.clone(),
        status: issue.status.clone(),
        priority: issue.priority,
        blocked_by,
        age_days: age_days(issue.created_at.as_deref(), evaluation_date),
        reason: None,
        confirmed_at: None,
    }
}

/// Closed statuses remove an issue from open/ready/blocked.
fn is_open(status: Option<&str>) -> bool {
    !matches!(status, Some("closed" | "done"))
}

/// Only an issue whose status is exactly `open` is claimable work.
///
/// `bd ready` excludes in-progress, blocked, deferred, hooked, and status-less
/// rows even though they remain open issues for the tracker views.
fn is_claimable(status: Option<&str>) -> bool {
    status == Some("open")
}

/// Whole days between creation and the evaluation date, never negative.
fn age_days(created_at: Option<&str>, evaluation_date: NaiveDate) -> Option<u32> {
    let created_at = parse_instant(created_at)?;
    let days = (evaluation_date - created_at.date_naive()).num_days();
    Some(days.max(0) as u32)
}

/// Parse an RFC 3339 timestamp into UTC.
fn parse_instant(raw: Option<&str>) -> Option<DateTime<Utc>> {
    raw.and_then(|raw| DateTime::parse_from_rfc3339(raw.trim()).ok())
        .map(|instant| instant.with_timezone(&Utc))
}

/// Report an absent export: no data, missing status, one stable warning.
fn missing() -> SourceRead<BeadsSummary> {
    SourceRead {
        data: None,
        status: SourceStatus {
            name: SourceName::Beads,
            status: SourceState::Missing,
            observed_at: None,
            staleness: Staleness::Unknown,
            detail: Some(format!("{ISSUES_PATH} does not exist")),
        },
        warnings: vec![Warning {
            source: SourceName::Beads,
            code: "beads_missing".to_string(),
            detail: format!("missing {ISSUES_PATH}"),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_and_done_are_not_open() {
        assert!(!is_open(Some("closed")));
        assert!(!is_open(Some("done")));
        assert!(is_open(Some("open")));
        assert!(is_open(None));
    }

    #[test]
    fn only_open_status_is_claimable() {
        assert!(is_claimable(Some("open")));
        for status in [
            "in_progress",
            "blocked",
            "deferred",
            "hooked",
            "closed",
            "done",
        ] {
            assert!(!is_claimable(Some(status)), "{status} is not claimable");
        }
        assert!(!is_claimable(None));
    }

    #[test]
    fn age_floors_and_clamps() {
        let evaluation = NaiveDate::from_ymd_opt(2026, 11, 18).expect("valid date");
        assert_eq!(age_days(Some("2026-11-10T23:59:59Z"), evaluation), Some(8));
        assert_eq!(age_days(Some("2026-11-20T00:00:00Z"), evaluation), Some(0));
        assert_eq!(age_days(Some("not a date"), evaluation), None);
        assert_eq!(age_days(None, evaluation), None);
    }
}
