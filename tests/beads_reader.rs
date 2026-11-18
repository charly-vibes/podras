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
use podras::model::{SourceName, SourceState, Staleness};
use podras::readers::{ReadContext, beads};
use serde_json::json;

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

fn ids(items: &[podras::model::WorkItem]) -> Vec<String> {
    let mut ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
    ids.sort();
    ids
}

#[test]
fn beads_reader_derives_sets_and_warnings() {
    let repo = TempRepo::new();
    let lines = [
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
    ];

    let mut body: String = lines
        .iter()
        .map(|line| serde_json::to_string(line).expect("serialize fixture row"))
        .collect::<Vec<_>>()
        .join("\n");
    body.push_str("\nnot valid json\n");
    repo.write(".beads/issues.jsonl", &body);

    let read = beads::read(&context(&repo));
    let data = read.data.as_ref().expect("present export yields data");

    assert_eq!(read.status.name, SourceName::Beads);
    assert_eq!(read.status.status, SourceState::Present);
    assert_eq!(read.status.staleness, Staleness::Unknown);
    assert_eq!(
        data.observed_at,
        Some(
            Utc.with_ymd_and_hms(2026, 11, 16, 12, 0, 0)
                .single()
                .expect("fixture instant is valid")
        )
    );

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

    let blocked = &data.blocked[0];
    assert_eq!(blocked.blocked_by, vec!["blocker-open"]);
    assert_eq!(blocked.priority, Some(1));
    assert_eq!(blocked.age_days, Some(7));
    assert_eq!(blocked.reason, None);
    assert_eq!(blocked.confirmed_at, None);

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

    assert_eq!(read.warnings.len(), 1);
    let warning = &read.warnings[0];
    assert_eq!(warning.source, SourceName::Beads);
    assert_eq!(warning.code, "beads_parse_error");
    assert!(warning.detail.contains('1'), "detail: {}", warning.detail);
}

#[test]
fn empty_export_falls_back_to_file_mtime() {
    let repo = TempRepo::new();
    repo.write(".beads/issues.jsonl", "");

    let read = beads::read(&context(&repo));
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
fn beads_reader_missing_export_reports_missing() {
    let repo = TempRepo::new();

    let read = beads::read(&context(&repo));

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
