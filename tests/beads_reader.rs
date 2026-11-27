//! Purpose: prove the beads export reader derives open, ready, and blocked sets.
//!
//! Responsibilities:
//! - exercise JSONL parsing, dependency direction, age, and observation time;
//! - prove malformed lines degrade to one counted warning without failing;
//! - prove a missing export is reported as missing, never as a panic.
//!
//! Rationale: beads owns issue lifecycle, so the reader must report exactly what
//! the export says while staying tolerant of partial or corrupt input.

mod common;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use common::TempRepo;
use podras::brief::BeadsSummary;
use podras::model::{SourceName, SourceState, Staleness, Warning, WorkItem};
use podras::readers::{ReadContext, SourceRead, beads};
use serde_json::{Value, json};

fn day(raw: &str) -> NaiveDate {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").expect("fixture date is valid")
}

fn context(repo: &TempRepo) -> ReadContext {
    ReadContext {
        repo_root: repo.root().to_path_buf(),
        as_of: Some(day("2026-11-18")),
        branch: Some("main".to_string()),
    }
}

fn ids(items: &[WorkItem]) -> Vec<String> {
    let mut ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
    ids.sort();
    ids
}

/// Rows that exercise the ready, blocked, in-progress, and closed states.
fn open_rows() -> Vec<Value> {
    vec![
        json!({
            "_type": "issue",
            "id": "ready-open",
            "title": "ready work",
            "status": "open",
            "priority": 2,
            "created_at": "2026-11-10T00:00:00Z",
            "updated_at": "2026-11-12T00:00:00Z",
            "dependencies": [],
            "metadata": { "files": ["src/ready.rs"] }
        }),
        json!({
            "_type": "issue",
            "id": "blocked-open",
            "title": "blocked work",
            "status": "open",
            "priority": 1,
            "created_at": "2026-11-11T00:00:00Z",
            "updated_at": "2026-11-13T00:00:00Z",
            "dependencies": [
                { "issue_id": "blocked-open", "depends_on_id": "blocker-open", "type": "blocks" },
                { "issue_id": "blocked-open", "depends_on_id": "closed-blocker", "type": "blocks" }
            ],
            "metadata": {}
        }),
        json!({
            "_type": "issue",
            "id": "blocker-open",
            "title": "open blocker",
            "status": "in_progress",
            "priority": 0,
            "created_at": "2026-11-09T00:00:00Z",
            "updated_at": "2026-11-16T12:00:00Z",
            "dependencies": [],
            "metadata": {}
        }),
        json!({
            "_type": "issue",
            "id": "closed-blocker",
            "title": "closed blocker",
            "status": "closed",
            "priority": 2,
            "created_at": "2026-11-01T00:00:00Z",
            "updated_at": "2026-11-14T00:00:00Z",
            "dependencies": [],
            "metadata": {}
        }),
    ]
}

/// Remaining rows: a satisfied blocker, a status-less issue, and a non-issue.
fn tail_rows() -> Vec<Value> {
    vec![
        json!({
            "_type": "issue",
            "id": "open-on-done",
            "title": "done blocker does not block",
            "status": "open",
            "priority": 3,
            "created_at": "2026-11-15T00:00:00Z",
            "updated_at": "2026-11-15T00:00:00Z",
            "dependencies": [
                { "issue_id": "open-on-done", "depends_on_id": "done-blocker", "type": "blocks" },
                { "issue_id": "open-on-done", "depends_on_id": "ready-open", "type": "relates_to" }
            ],
            "metadata": {}
        }),
        json!({
            "_type": "issue",
            "id": "done-blocker",
            "title": "done item",
            "status": "done",
            "priority": null,
            "created_at": "2026-11-02T00:00:00Z",
            "updated_at": "2026-11-03T00:00:00Z",
            "dependencies": [],
            "metadata": {}
        }),
        json!({
            "_type": "issue",
            "id": "status-missing",
            "title": "no status yet",
            "priority": 4,
            "created_at": "2026-11-14T00:00:00Z",
            "updated_at": "2026-11-14T00:00:00Z",
            "dependencies": [],
            "metadata": {}
        }),
        json!({
            "_type": "note",
            "id": "ignored-note",
            "body": "not an issue"
        }),
    ]
}

/// Serialize every fixture row as JSONL and append one malformed line.
fn export_body() -> String {
    let mut rows = open_rows();
    rows.extend(tail_rows());
    let mut body: String = rows
        .iter()
        .map(|row| serde_json::to_string(row).expect("serialize fixture row"))
        .collect::<Vec<_>>()
        .join("\n");
    body.push_str("\nnot valid json\n");
    body
}

fn observed_instant() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 16, 12, 0, 0)
        .single()
        .expect("fixture instant is valid")
}

fn assert_present_status(read: &SourceRead<BeadsSummary>, data: &BeadsSummary) {
    assert_eq!(read.status.name, SourceName::Beads);
    assert_eq!(read.status.status, SourceState::Present);
    assert_eq!(read.status.staleness, Staleness::Unknown);
    assert_eq!(data.observed_at, Some(observed_instant()));
}

fn assert_blocked_details(item: &WorkItem) {
    assert_eq!(item.blocked_by, vec!["blocker-open"]);
    assert_eq!(item.priority, Some(1));
    assert_eq!(item.age_days, Some(7));
    assert_eq!(item.reason, None);
    assert_eq!(item.confirmed_at, None);
}

fn assert_ready_details(data: &BeadsSummary) {
    let ready = data
        .open
        .iter()
        .find(|item| item.id == "ready-open")
        .expect("ready item is open");
    assert_eq!(ready.title.as_deref(), Some("ready work"));
    assert_eq!(ready.status.as_deref(), Some("open"));
    assert_eq!(ready.priority, Some(2));
    assert_eq!(ready.age_days, Some(8));
    assert!(ready.blocked_by.is_empty());
}

fn assert_parse_warning(warning: &Warning) {
    assert_eq!(warning.source, SourceName::Beads);
    assert_eq!(warning.code, "beads_parse_error");
    assert!(warning.detail.contains('1'), "detail: {}", warning.detail);
}

#[test]
fn beads_reader_derives_sets_and_warnings() {
    let repo = TempRepo::new();
    repo.write(".beads/issues.jsonl", &export_body());

    let read = beads::read(&context(&repo), day("2026-11-18"));
    let data = read.data.as_ref().expect("present export yields data");

    assert_present_status(&read, data);
    assert_eq!(
        ids(&data.open),
        vec![
            "blocked-open",
            "blocker-open",
            "open-on-done",
            "ready-open",
            "status-missing",
        ]
    );
    // Only status `open` is claimable: the in-progress blocker and the
    // status-less row stay in `open` but must never appear in `ready`.
    assert_eq!(ids(&data.ready), vec!["open-on-done", "ready-open"]);
    assert!(
        data.ready
            .iter()
            .all(|item| item.status.as_deref() == Some("open")),
        "ready items must be exactly open: {:?}",
        ids(&data.ready)
    );
    assert_eq!(ids(&data.blocked), vec!["blocked-open"]);

    assert_blocked_details(&data.blocked[0]);
    assert_ready_details(data);

    assert_eq!(read.warnings.len(), 1);
    assert_parse_warning(&read.warnings[0]);
}

#[test]
fn empty_export_falls_back_to_file_mtime() {
    let repo = TempRepo::new();
    repo.write(".beads/issues.jsonl", "");

    let read = beads::read(&context(&repo), day("2026-11-18"));
    let data = read.data.as_ref().expect("empty export is still present");

    assert_eq!(read.status.status, SourceState::Present);
    assert!(read.warnings.is_empty(), "warnings: {:?}", read.warnings);
    assert!(data.open.is_empty());
    assert!(data.ready.is_empty());
    assert!(data.blocked.is_empty());

    let mtime: DateTime<Utc> = std::fs::metadata(repo.root().join(".beads/issues.jsonl"))
        .expect("stat export")
        .modified()
        .expect("export has an mtime")
        .into();
    assert_eq!(data.observed_at, Some(mtime));
    assert_eq!(read.status.observed_at, Some(mtime));
}

#[test]
fn beads_reader_accepts_stringified_file_lists() {
    let repo = TempRepo::new();
    // Live-export shapes: one row carries `metadata.files` as a real array,
    // the other as the JSON-encoded string `bd export` sometimes writes, plus
    // a dependency edge carrying the extra keys the live file includes.
    let rows = [
        json!({
            "_type": "issue",
            "id": "array-files",
            "title": "files as an array",
            "status": "open",
            "priority": 2,
            "created_at": "2026-11-10T00:00:00Z",
            "updated_at": "2026-11-11T00:00:00Z",
            "dependencies": [],
            "metadata": { "files": ["docs/"] }
        }),
        json!({
            "_type": "issue",
            "id": "stringified-files",
            "title": "files as a string",
            "status": "open",
            "priority": 2,
            "created_at": "2026-11-10T00:00:00Z",
            "updated_at": "2026-11-11T00:00:00Z",
            "dependencies": [
                { "issue_id": "stringified-files", "depends_on_id": "array-files", "type": "relates_to", "created_at": "2026-11-10T01:00:00Z", "created_by": "kaya", "metadata": "{}" }
            ],
            "metadata": { "files": "[\"docs/\",\".kaya/\"]", "base_commit": "7d7d286ac5eaf6c51c6fa4514de19c51d159a000" }
        }),
        json!({
            "_type": "issue",
            "id": "unusable-files",
            "title": "stringified files that are not an array",
            "status": "open",
            "priority": 2,
            "created_at": "2026-11-10T00:00:00Z",
            "updated_at": "2026-11-11T00:00:00Z",
            "dependencies": [],
            "metadata": { "files": "not a json array" }
        }),
        json!({
            "_type": "note",
            "id": "ignored-note",
            "body": "not an issue"
        }),
    ];
    let body = rows
        .iter()
        .map(|row| serde_json::to_string(row).expect("serialize fixture row"))
        .collect::<Vec<_>>()
        .join("\n");
    repo.write(".beads/issues.jsonl", &format!("{body}\n"));

    let read = beads::read(&context(&repo), day("2026-11-18"));
    let data = read.data.as_ref().expect("present export yields data");

    assert_eq!(read.status.status, SourceState::Present);
    // Both file-list shapes parse: no row is skipped as malformed.
    assert!(
        read.warnings.is_empty(),
        "stringified file lists must not count as malformed: {:?}",
        read.warnings
    );
    assert_eq!(
        ids(&data.open),
        vec!["array-files", "stringified-files", "unusable-files"]
    );
    // The malformed file list degrades to no files, never to a dropped row.
    assert_eq!(
        data.observed_at,
        Some(
            Utc.with_ymd_and_hms(2026, 11, 11, 0, 0, 0)
                .single()
                .expect("fixture instant is valid")
        )
    );
}

#[test]
fn beads_reader_missing_export_reports_missing() {
    let repo = TempRepo::new();

    let read = beads::read(&context(&repo), day("2026-11-18"));

    assert!(read.data.is_none());
    assert_eq!(read.status.name, SourceName::Beads);
    assert_eq!(read.status.status, SourceState::Missing);
    assert_eq!(read.status.staleness, Staleness::Unknown);
    assert_eq!(read.status.observed_at, None);
    assert_eq!(read.warnings.len(), 1);
    assert_eq!(read.warnings[0].source, SourceName::Beads);
    assert_eq!(read.warnings[0].code, "beads_missing");
    assert!(!read.warnings[0].detail.is_empty());
}
