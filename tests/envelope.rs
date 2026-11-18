//! Purpose: property tests for the typed agent envelope.
//!
//! Responsibilities:
//! - assert the declared section bounds and truncation accounting;
//! - assert an emitted brief validates against the model's JSON Schema;
//! - assert the committed schema bytes are exactly the regenerated model schema.
//!
//! Rationale: the envelope is a machine contract. These tests fail loudly when
//! a bound, an enum, or the schema silently drifts away from the model.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde_json::Value;

use podras::brief::{self, BeadsSummary, BriefInputs, CurationState, GitSummary};
use podras::curation::{CurationFile, FocusEntry, ParkEntry};
use podras::model::{
    Brief, RecentEvent, Signal, SignalKind, SourceName, SourceState, SourceStatus, Staleness,
    Warning, WorkItem,
};
use podras::render::envelope;

#[path = "envelope/fixtures.rs"]
mod fixtures;

use fixtures::{schema_curation, schema_sources, schema_warnings};

/// Pinned evaluation date for every envelope fixture.
const AS_OF: &str = "2026-11-18";

fn date(raw: &str) -> NaiveDate {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").expect("fixture date is valid")
}

fn instant(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .expect("fixture instant is valid")
        .with_timezone(&Utc)
}

fn work_item(id: &str, priority: u8) -> WorkItem {
    WorkItem {
        id: id.to_string(),
        title: Some(format!("title {id}")),
        status: Some("open".to_string()),
        priority: Some(priority),
        blocked_by: Vec::new(),
        age_days: Some(1),
        reason: None,
        confirmed_at: None,
    }
}

fn recent_event(index: usize) -> RecentEvent {
    RecentEvent {
        sha: format!("{index:040x}"),
        committed_at: instant("2026-11-17T00:00:00Z") - Duration::minutes(index as i64),
        subject: format!("commit {index}"),
        outcome: None,
    }
}

fn source_status(
    name: SourceName,
    status: SourceState,
    observed_at: Option<DateTime<Utc>>,
    staleness: Staleness,
    detail: Option<&str>,
) -> SourceStatus {
    SourceStatus {
        name,
        status,
        observed_at,
        staleness,
        detail: detail.map(str::to_string),
    }
}

/// A deterministic input set; each test fills only the sources it needs.
fn inputs() -> BriefInputs {
    BriefInputs {
        beads: None,
        git: None,
        specs: None,
        whisper: None,
        curation: None,
        sources: Vec::new(),
        warnings: Vec::new(),
        as_of: Some(date(AS_OF)),
        now: date(AS_OF),
        generated_at: instant("2026-11-18T12:00:00Z"),
    }
}

#[test]
fn truncation_flagged() {
    let total_recent = 8usize;
    let mut fixture = inputs();
    fixture.git = Some(GitSummary {
        head_sha: Some("0123456789abcdef".to_string()),
        recent: (0..total_recent).map(recent_event).collect(),
        observed_at: Some(instant("2026-11-17T00:00:00Z")),
    });

    let brief = brief::assemble(&fixture);

    assert!(brief.truncation.applied, "truncation must be declared");
    assert_eq!(brief.recent.len(), 5, "recent section is bounded to five");
    assert_eq!(
        brief.truncation.omitted_counts.get("recent").copied(),
        Some((total_recent - 5) as u32),
        "omitted_counts.recent must equal total - bound"
    );
    assert_eq!(
        brief.truncation.omitted_counts.len(),
        1,
        "only the truncated section appears in omitted_counts"
    );
}

fn bounds_focus() -> Vec<FocusEntry> {
    (0..3)
        .map(|index| FocusEntry {
            id: format!("open-{index}"),
            reason: Some("fixture focus".to_string()),
            focused_since: Some("2026-11-01".to_string()),
            confirmed_at: Some("2026-11-01".to_string()),
        })
        .collect()
}

fn bounds_parked() -> Vec<ParkEntry> {
    (0..7)
        .map(|index| ParkEntry {
            id: format!("parked-{index}"),
            reason: "fixture park".to_string(),
            resurface: "2027-01-01".to_string(),
            parked_at: Some("2026-11-01".to_string()),
            confirmed_at: Some("2026-11-01".to_string()),
        })
        .collect()
}

fn bounds_beads() -> BeadsSummary {
    BeadsSummary {
        open: (0..7)
            .map(|index| work_item(&format!("open-{index}"), 1))
            .collect(),
        blocked: (0..7)
            .map(|index| work_item(&format!("blocked-{index}"), 2))
            .collect(),
        ready: (0..7)
            .map(|index| work_item(&format!("ready-{index}"), 3))
            .collect(),
        observed_at: Some(instant("2026-11-17T00:00:00Z")),
    }
}

fn bounds_git() -> GitSummary {
    GitSummary {
        head_sha: Some("0123456789abcdef".to_string()),
        recent: (0..8).map(recent_event).collect(),
        observed_at: Some(instant("2026-11-17T00:00:00Z")),
    }
}

fn bounds_curation() -> CurationState {
    CurationState {
        curation: CurationFile {
            purpose_lite: Some("bounds fixture".to_string()),
            max_focus: Some(1),
            publish_whisper: Some(false),
            health: None,
            focus: bounds_focus(),
            parked: bounds_parked(),
        },
        updated_at: Some(instant("2026-11-01T00:00:00Z")),
    }
}

fn bounds_warnings() -> Vec<Warning> {
    (0..7)
        .map(|index| Warning {
            source: SourceName::Beads,
            code: format!("fixture_warning_{index}"),
            detail: format!("fixture warning {index}"),
        })
        .collect()
}

#[test]
fn bounds_declared() {
    let mut fixture = inputs();
    fixture.beads = Some(bounds_beads());
    fixture.git = Some(bounds_git());
    fixture.curation = Some(bounds_curation());
    fixture.warnings = bounds_warnings();

    let brief = brief::assemble(&fixture);

    assert_eq!(brief.focus.len(), 1, "focus bound");
    assert_eq!(brief.recent.len(), 5, "recent bound");
    assert_eq!(brief.blocked.len(), 5, "blocked bound");
    assert_eq!(brief.next.len(), 5, "next bound");
    assert_eq!(brief.parked.len(), 5, "parked bound");
    assert_eq!(brief.warnings.len(), 7, "warnings stay unbounded");

    let expected = BTreeMap::from([
        ("blocked".to_string(), 2u32),
        ("focus".to_string(), 2u32),
        ("next".to_string(), 2u32),
        ("parked".to_string(), 2u32),
        ("recent".to_string(), 3u32),
    ]);
    assert_eq!(brief.truncation.omitted_counts, expected);
}

fn assert_envelope_wrapper(wrapped: &genesis::envelope::Envelope<Brief>, brief: &Brief) {
    assert!(wrapped.ok);
    assert_eq!(
        wrapped.envelope_kind,
        genesis::envelope::EnvelopeKind::Ok,
        "briefs ship as an Ok envelope"
    );
    assert!(
        wrapped.warnings.is_empty(),
        "warnings live inside the brief, not the genesis wrapper"
    );
    assert_eq!(wrapped.data, *brief);
}

fn assert_source_enums(instance: &Value) {
    let source_names = ["beads", "git", "openspec", "whisper", "curation"];
    let source_states = ["present", "missing", "unreadable"];
    let stalenesses = ["fresh", "stale", "unknown"];
    for index in 0..source_names.len() {
        assert_enum_member(instance, &format!("/sources/{index}/name"), &source_names);
        assert_enum_member(
            instance,
            &format!("/sources/{index}/status"),
            &source_states,
        );
        assert_enum_member(
            instance,
            &format!("/sources/{index}/staleness"),
            &stalenesses,
        );
    }
}

fn assert_health_enums(instance: &Value, brief: &Brief) {
    let health_statuses = ["green", "amber", "red", "unknown"];
    let health_bases = ["human", "signals", "none"];
    let signal_kinds = [
        "stale_focus",
        "wip_over_cap",
        "stuck_project",
        "focus_closed",
        "parked_expired",
        "orphan_commits",
        "spec_change_without_bead",
    ];
    assert_enum_member(instance, "/health/status", &health_statuses);
    assert_enum_member(instance, "/health/basis", &health_bases);
    for index in 0..brief.health.signals.len() {
        assert_enum_member(
            instance,
            &format!("/health/signals/{index}/kind"),
            &signal_kinds,
        );
    }
}

fn assert_warning_enums(instance: &Value, brief: &Brief) {
    let source_names = ["beads", "git", "openspec", "whisper", "curation"];
    for index in 0..brief.warnings.len() {
        assert_enum_member(
            instance,
            &format!("/warnings/{index}/source"),
            &source_names,
        );
    }
}

fn assert_enum_members(instance: &Value, brief: &Brief) {
    assert_source_enums(instance);
    assert_health_enums(instance, brief);
    assert_warning_enums(instance, brief);
}

#[test]
fn envelope_schema_conform() {
    let mut fixture = inputs();
    fixture.sources = schema_sources();
    fixture.curation = Some(schema_curation());
    fixture.warnings = schema_warnings();

    let mut brief = brief::assemble(&fixture);
    brief.health.signals.push(Signal {
        kind: SignalKind::StaleFocus,
        reason: "focus unconfirmed for 15 days".to_string(),
    });

    let wrapped = envelope::envelope(&brief).expect("envelope builds");
    assert_envelope_wrapper(&wrapped, &brief);

    let instance = serde_json::to_value(&wrapped.data).expect("brief serializes");
    let schema = schemars::schema_for!(Brief);
    let schema_value = serde_json::to_value(&schema).expect("schema serializes");
    let validator = jsonschema::validator_for(&schema_value).expect("schema compiles");
    validator
        .validate(&instance)
        .expect("the emitted brief conforms to its own schema");

    let rendered = envelope::to_json(&brief).expect("envelope renders as JSON");
    let parsed: Value = serde_json::from_str(&rendered).expect("rendered envelope is JSON");
    assert_eq!(
        parsed.get("data"),
        Some(&instance),
        "the JSON render carries the same brief payload"
    );

    assert_enum_members(&instance, &brief);
}

#[test]
fn schema_file_matches_committed_bytes() {
    let regenerated = envelope::schema().expect("schema renders");
    let committed = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schema/brief-envelope.schema.json"
    ))
    .expect("committed brief schema exists");

    assert_eq!(
        regenerated, committed,
        "regenerate schema/brief-envelope.schema.json from schemars::schema_for!(Brief)"
    );
}

fn assert_enum_member(instance: &Value, pointer: &str, allowed: &[&str]) {
    let field = instance
        .pointer(pointer)
        .unwrap_or_else(|| panic!("{pointer} is missing"));
    let raw = field
        .as_str()
        .unwrap_or_else(|| panic!("{pointer} is not a string"));
    assert!(
        allowed.contains(&raw),
        "{pointer} = {raw:?} is not one of {allowed:?}"
    );
}
