//! Purpose: shared brief fixtures for the determinism tests.
//!
//! Responsibilities:
//! - build the deterministic input set every semantic-payload test compares;
//! - build the pinned-clock repository whose horizons land on `2026-11-18`;
//! - run the real binary and normalize the one volatile envelope field.
//!
//! Rationale: determinism is proven by comparing identical fixtures, so all
//! fixture construction lives here and stays visibly free of wall-clock reads.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use serde_json::Value;

use podras::brief::{
    BeadsSummary, BriefInputs, CurationState, GitSummary, SpecSummary, WhisperReading,
};
use podras::curation::{CurationFile, CurationHealth, FocusEntry, ParkEntry};
use podras::model::{
    RecentEvent, SourceName, SourceState, SourceStatus, Staleness, Warning, WorkItem,
};

use super::common::TempRepo;

/// Pinned evaluation date for the reproducible-render scenario.
pub(super) const BOUNDARY_AS_OF: &str = "2026-11-18";

pub(super) fn day(raw: &str) -> NaiveDate {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").expect("fixture dates are valid")
}

pub(super) fn instant(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 18, hour, 0, 0)
        .single()
        .expect("fixture instants are valid")
}

fn item(id: &str, priority: Option<u8>) -> WorkItem {
    WorkItem {
        id: id.to_string(),
        title: Some(format!("title {id}")),
        status: Some("open".to_string()),
        priority,
        blocked_by: Vec::new(),
        age_days: Some(4),
        reason: None,
        confirmed_at: None,
    }
}

fn source(name: SourceName, status: SourceState) -> SourceStatus {
    SourceStatus {
        name,
        status,
        observed_at: Some(instant(9)),
        staleness: Staleness::Fresh,
        detail: None,
    }
}

fn commit(sha: &str, hour: u32) -> RecentEvent {
    RecentEvent {
        sha: sha.to_string(),
        committed_at: instant(hour),
        subject: format!("commit {sha}"),
        outcome: None,
    }
}

fn beads() -> BeadsSummary {
    let mut blocked = item("podras-blocked", Some(1));
    blocked.blocked_by = vec!["podras-a".to_string()];

    let open = vec![
        item("podras-b", Some(2)),
        item("podras-a", None),
        item("podras-c", Some(1)),
        item("podras-d", Some(2)),
        item("podras-parked-event", Some(0)),
        item("podras-parked-old", Some(3)),
        item("podras-parked-date", None),
    ];

    let ready = vec![
        item("podras-parked-event", Some(0)),
        item("podras-ready-late", Some(7)),
        item("podras-c", Some(1)),
        item("podras-ready-none", None),
    ];

    BeadsSummary {
        open,
        blocked: vec![blocked],
        ready,
        observed_at: Some(instant(9)),
    }
}

fn git() -> GitSummary {
    let recent = vec![
        commit("b2", 10),
        commit("a1", 10),
        commit("c0", 11),
        commit("d9", 8),
        commit("e1", 9),
        commit("f1", 7),
        commit("a0", 6),
    ];
    GitSummary {
        head_sha: Some("head123".to_string()),
        recent,
        observed_at: Some(instant(9)),
    }
}

fn specs() -> SpecSummary {
    SpecSummary {
        changes: Vec::new(),
        stale_count: 0,
        observed_at: Some(instant(9)),
    }
}

fn whisper() -> WhisperReading {
    WhisperReading {
        observed_at: Some(instant(9)),
        next_action: Some("pick up podras-a".to_string()),
        content_hash: None,
    }
}

fn curation_health() -> CurationHealth {
    CurationHealth {
        status: "green".to_string(),
        owner: Some("@sasha".to_string()),
        confirmed_at: Some("2026-11-18".to_string()),
        reason: Some("v1 kickoff".to_string()),
    }
}

fn curation_focus() -> Vec<FocusEntry> {
    vec![
        FocusEntry {
            id: "podras-c".to_string(),
            reason: Some("read path".to_string()),
            focused_since: Some("2026-11-01".to_string()),
            confirmed_at: Some("2026-11-18".to_string()),
        },
        FocusEntry {
            id: "podras-second".to_string(),
            reason: Some("older focus".to_string()),
            focused_since: Some("2026-10-01".to_string()),
            confirmed_at: Some("2026-11-10".to_string()),
        },
    ]
}

fn curation_parked() -> Vec<ParkEntry> {
    vec![
        ParkEntry {
            id: "podras-parked-event".to_string(),
            reason: "harvest is v1.1".to_string(),
            resurface: "event:value-prop-roadmap".to_string(),
            parked_at: Some("2026-11-10".to_string()),
            confirmed_at: Some("2026-11-10".to_string()),
        },
        ParkEntry {
            id: "podras-parked-old".to_string(),
            reason: "revisit after release".to_string(),
            resurface: "2026-11-01".to_string(),
            parked_at: Some("2026-10-01".to_string()),
            confirmed_at: Some("2026-10-01".to_string()),
        },
        ParkEntry {
            id: "podras-parked-date".to_string(),
            reason: "triggered while away".to_string(),
            resurface: "2026-11-17".to_string(),
            parked_at: Some("2026-11-02".to_string()),
            confirmed_at: Some("2026-11-02".to_string()),
        },
    ]
}

fn curation() -> CurationFile {
    CurationFile {
        purpose_lite: Some("podras: status read path for a cold return".to_string()),
        max_focus: Some(1),
        publish_whisper: Some(false),
        health: Some(curation_health()),
        focus: curation_focus(),
        parked: curation_parked(),
    }
}

fn sources() -> Vec<SourceStatus> {
    vec![
        source(SourceName::Curation, SourceState::Present),
        source(SourceName::Whisper, SourceState::Missing),
        source(SourceName::Beads, SourceState::Present),
        source(SourceName::Openspec, SourceState::Missing),
        source(SourceName::Git, SourceState::Present),
    ]
}

fn warnings() -> Vec<Warning> {
    vec![
        Warning {
            source: SourceName::Git,
            code: "git_unreadable".to_string(),
            detail: "no HEAD".to_string(),
        },
        Warning {
            source: SourceName::Beads,
            code: "beads_parse_error".to_string(),
            detail: "line z".to_string(),
        },
        Warning {
            source: SourceName::Beads,
            code: "beads_parse_error".to_string(),
            detail: "line a".to_string(),
        },
    ]
}

pub(super) fn inputs(generated_at: DateTime<Utc>) -> BriefInputs {
    BriefInputs {
        beads: Some(beads()),
        git: Some(git()),
        specs: Some(specs()),
        whisper: Some(whisper()),
        curation: Some(CurationState {
            curation: curation(),
            updated_at: Some(instant(8)),
        }),
        sources: sources(),
        warnings: warnings(),
        as_of: Some(day("2026-11-18")),
        now: day("2026-11-01"),
        generated_at,
    }
}

/// Run the real binary inside a fixture repository.
pub(super) fn podras_in(repo: &TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    assert_cmd::Command::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(args)
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .assert()
}

/// Run `brief --json --as-of <date>` and return the parsed envelope.
pub(super) fn brief_json_at(repo: &TempRepo, as_of: &str) -> Value {
    let output = podras_in(repo, &["brief", "--json", "--as-of", as_of])
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_str(
        String::from_utf8(output)
            .expect("brief output is UTF-8")
            .trim(),
    )
    .expect("brief --json emits a JSON envelope")
}

/// Drop the only volatile payload field so two runs can be compared.
pub(super) fn strip_generated_at(mut envelope: Value) -> Value {
    envelope["data"]
        .as_object_mut()
        .expect("brief payload is an object")
        .remove("generated_at");
    envelope
}

/// Replace the volatile value inside the `Freshness:` stamp line.
pub(super) fn normalize_freshness_stamp(markdown: &str) -> String {
    markdown
        .lines()
        .map(|line| {
            line.strip_prefix("Freshness: generated_at ")
                .and_then(|rest| rest.find(';').map(|end| &rest[end..]))
                .map_or_else(
                    || line.to_string(),
                    |rest| format!("Freshness: generated_at <volatile>{rest}"),
                )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn source_row<'a>(data: &'a Value, name: &str) -> &'a Value {
    data["sources"]
        .as_array()
        .expect("sources is an array")
        .iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("source `{name}` is declared: {data}"))
}

pub(super) fn read_json(repo: &TempRepo, rel: &str) -> Value {
    serde_json::from_str(&repo.read(rel)).expect("fixture render is JSON")
}

/// Fixture whose human verdict and parked resurface sit exactly on their
/// freshness boundaries for `2026-11-18`.
pub(super) fn boundary_fixture() -> TempRepo {
    let repo = TempRepo::new();
    // Exactly seven days before the pinned date: on the beads horizon.
    repo.beads_jsonl_at(
        &[("podras-a", "Boundary work", "open")],
        &[],
        "2026-11-11T00:00:00Z",
    );
    repo.curation(
        r#"purpose_lite = "podras: pinned clock fixture"
max_focus = 1
publish_whisper = false

[health]
status = "green"
owner = "@sasha"
confirmed_at = "2026-11-04"
reason = "boundary human green"

[[focus]]
id = "podras-a"
reason = "boundary focus"
focused_since = "2026-11-04"
confirmed_at = "2026-11-04"

[[parked]]
id = "podras-parked"
reason = "boundary resurface"
resurface = "2026-11-18"
parked_at = "2026-11-01"
confirmed_at = "2026-11-01"
"#,
    );
    repo.write(
        ".whisper/branches/main/notes.md",
        "- 2026-11-04T00:00:00Z [id:fixture] (#main) Format: 1\n  boundary note\n",
    );
    repo.spec_change("boundary-change", "# podras-a: boundary change\n");
    repo.write("openspec/specs/brief-v1/spec.md", "# Brief v1\n");

    // Every fourteen-day horizon lands exactly on the pinned date.
    repo.set_mtime(".podras/curation.toml", "2026-11-04T00:00:00Z");
    repo.set_mtime(
        "openspec/changes/boundary-change/proposal.md",
        "2026-11-04T00:00:00Z",
    );
    repo.set_mtime("openspec/specs/brief-v1/spec.md", "2026-11-04T00:00:00Z");

    // Exactly seven days before the pinned date: on the git horizon.
    repo.commit_at("2026-11-11T00:00:00Z", "podras-a: boundary sources");
    repo
}
