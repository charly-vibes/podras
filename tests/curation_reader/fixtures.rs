//! Purpose: shared curation fixtures for the curation reader tests.
//!
//! Responsibilities:
//! - build the round-trip sample curation file;
//! - write RFC 3339 curation timestamps;
//! - assemble a brief from a repository's curation file at a fixed date.
//!
//! Rationale: every curation test speaks about the same file shape, so the
//! fixtures live in one place and the tests stay about their assertions.

use chrono::NaiveDate;

use podras::brief::{BriefInputs, CurationState, assemble};
use podras::curation::{self, CurationFile, CurationHealth, FocusEntry, ParkEntry};
use podras::model::{Brief, SourceName, SourceState, SourceStatus, Staleness};

use super::common::TempRepo;

pub(super) fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("valid fixture date")
}

pub(super) fn sample() -> CurationFile {
    CurationFile {
        purpose_lite: Some("podras: status read path for a cold return".to_string()),
        max_focus: Some(2),
        publish_whisper: Some(true),
        health: Some(CurationHealth {
            status: "amber".to_string(),
            owner: Some("@sasha".to_string()),
            confirmed_at: Some("2026-11-01".to_string()),
            reason: Some("v1 kickoff".to_string()),
        }),
        focus: vec![
            FocusEntry {
                id: "podras-d2e".to_string(),
                reason: Some("proposal phase".to_string()),
                focused_since: Some("2026-11-17T09:30:00Z".to_string()),
                confirmed_at: Some("2026-11-17".to_string()),
            },
            FocusEntry {
                id: "podras-i45".to_string(),
                reason: None,
                focused_since: Some("2026-11-18".to_string()),
                confirmed_at: None,
            },
        ],
        parked: vec![
            ParkEntry {
                id: "podras-o47".to_string(),
                reason: "harvest is v1.1".to_string(),
                resurface: "event:value-prop-roadmap".to_string(),
                parked_at: Some("2026-11-18".to_string()),
                confirmed_at: Some("2026-11-18T12:00:00Z".to_string()),
            },
            ParkEntry {
                id: "podras-7jd".to_string(),
                reason: "freshness hardening is v1.1".to_string(),
                resurface: "2026-12-01".to_string(),
                parked_at: Some("2026-11-18".to_string()),
                confirmed_at: None,
            },
        ],
    }
}

/// Assemble a brief from the repository's curation file at a fixed date.
pub(super) fn assemble_fixture(repo: &TempRepo) -> Brief {
    let loaded = curation::load(repo.root())
        .expect("load curation")
        .expect("curation present");
    let updated_at = curation::updated_at(repo.root());
    let inputs = BriefInputs {
        beads: None,
        git: None,
        specs: None,
        whisper: None,
        curation: Some(CurationState {
            curation: loaded,
            updated_at,
        }),
        sources: vec![SourceStatus {
            name: SourceName::Curation,
            status: SourceState::Present,
            observed_at: updated_at,
            staleness: Staleness::Fresh,
            detail: None,
        }],
        warnings: Vec::new(),
        as_of: Some(date(2026, 11, 18)),
        now: date(2026, 11, 18),
        generated_at: date(2026, 11, 18)
            .and_hms_opt(9, 0, 0)
            .expect("valid fixture hour")
            .and_utc(),
    };
    assemble(&inputs)
}

pub(super) fn rfc3339_curation(repo: &TempRepo) {
    repo.curation(
        r#"purpose_lite = "podras: RFC 3339 fixture"

[health]
status = "green"
owner = "@sasha"
confirmed_at = "2026-11-18T07:00:00Z"

[[parked]]
id = "podras-o47"
reason = "harvest is v1.1"
resurface = "event:value-prop-roadmap"
parked_at = "2026-11-17T22:15:00+02:00"
confirmed_at = "2026-11-18T07:00:00Z"
"#,
    );
}
