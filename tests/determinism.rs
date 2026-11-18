//! Purpose: prove the brief payload is deterministic and deterministically ordered.
//!
//! Responsibilities:
//! - assemble twin briefs from identical inputs and compare semantic payloads;
//! - prove `generated_at` is excluded from the payload and from `source_hash`;
//! - prove computed sections, focus, parked, recent, warnings, and sources order.
//!
//! Rationale: determinism is what lets a generated render be regenerated and
//! diffed. Comparing semantic payloads drops the one volatile field while
//! keeping every other field under assertion.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use podras::brief::{
    BeadsSummary, BriefInputs, CurationState, GitSummary, SpecSummary, WhisperReading, assemble,
};
use podras::curation::{CurationFile, CurationHealth, FocusEntry, ParkEntry};
use podras::model::{
    HealthBasis, HealthStatus, RecentEvent, SourceName, SourceState, SourceStatus, Staleness,
    Warning, WorkItem, semantic_payload, source_hash,
};

fn day(raw: &str) -> NaiveDate {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").expect("fixture dates are valid")
}

fn instant(hour: u32) -> DateTime<Utc> {
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

fn inputs(generated_at: DateTime<Utc>) -> BriefInputs {
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

    let recent = vec![
        commit("b2", 10),
        commit("a1", 10),
        commit("c0", 11),
        commit("d9", 8),
        commit("e1", 9),
        commit("f1", 7),
        commit("a0", 6),
    ];

    let curation = CurationFile {
        purpose_lite: Some("podras: status read path for a cold return".to_string()),
        max_focus: Some(1),
        publish_whisper: Some(false),
        health: Some(CurationHealth {
            status: "green".to_string(),
            owner: Some("@sasha".to_string()),
            confirmed_at: Some("2026-11-18".to_string()),
            reason: Some("v1 kickoff".to_string()),
        }),
        focus: vec![
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
        ],
        parked: vec![
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
        ],
    };

    BriefInputs {
        beads: Some(BeadsSummary {
            open,
            blocked: vec![blocked],
            ready,
            observed_at: Some(instant(9)),
        }),
        git: Some(GitSummary {
            head_sha: Some("head123".to_string()),
            recent,
            observed_at: Some(instant(9)),
        }),
        specs: Some(SpecSummary {
            changes: Vec::new(),
            stale_count: 0,
            observed_at: Some(instant(9)),
        }),
        whisper: Some(WhisperReading {
            observed_at: Some(instant(9)),
            next_action: Some("pick up podras-a".to_string()),
            content_hash: None,
        }),
        curation: Some(CurationState {
            curation,
            updated_at: Some(instant(8)),
        }),
        sources: vec![
            source(SourceName::Curation, SourceState::Present),
            source(SourceName::Whisper, SourceState::Missing),
            source(SourceName::Beads, SourceState::Present),
            source(SourceName::Openspec, SourceState::Missing),
            source(SourceName::Git, SourceState::Present),
        ],
        warnings: vec![
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
        ],
        as_of: Some(day("2026-11-18")),
        now: day("2026-11-01"),
        generated_at,
    }
}

fn ids(items: &[WorkItem]) -> Vec<&str> {
    items.iter().map(|item| item.id.as_str()).collect()
}

#[test]
fn determinism_payload() {
    let first = assemble(&inputs(instant(1)));
    let second = assemble(&inputs(instant(2)));

    assert_eq!(first.generated_at, instant(1));
    assert_ne!(first.generated_at, second.generated_at);

    let payload_a = semantic_payload(&first);
    let payload_b = semantic_payload(&second);
    assert_eq!(payload_a, payload_b, "semantic payloads must match");
    assert!(payload_a.get("generated_at").is_none());
    assert!(payload_a.get("source_hash").is_none());
    assert_eq!(
        source_hash(&first, None),
        source_hash(&second, None),
        "source hash must ignore generated_at"
    );

    assert_eq!(
        payload_a["recent"][0]["committed_at"],
        serde_json::json!("2026-11-18T11:00:00Z")
    );
    assert_eq!(
        payload_a["needs_retriage"][0]["confirmed_at"],
        serde_json::json!("2026-10-01")
    );
    assert_eq!(
        payload_a["parked"][0]["resurface"],
        serde_json::json!("event:value-prop-roadmap")
    );

    assert_eq!(
        ids(&first.in_flight),
        ["podras-c", "podras-b", "podras-d", "podras-a"]
    );
    assert_eq!(ids(&first.blocked), ["podras-blocked"]);
    assert_eq!(ids(&first.next), ["podras-ready-late", "podras-ready-none"]);
    assert!(!ids(&first.next).contains(&"podras-parked-event"));
    assert_eq!(
        first
            .recent
            .iter()
            .map(|event| event.sha.as_str())
            .collect::<Vec<_>>(),
        ["c0", "a1", "b2", "e1", "d9"]
    );
    assert_eq!(first.parked.len(), 1);
    assert_eq!(first.parked[0].id, "podras-parked-event");
    assert_eq!(
        ids(&first.needs_retriage),
        ["podras-parked-old", "podras-parked-date"]
    );

    assert_eq!(first.focus.len(), 1);
    assert_eq!(first.focus[0].id, "podras-second");
    assert_eq!(first.focus[0].confirmed_at, Some(day("2026-11-10")));
    assert!(first.truncation.applied);
    assert_eq!(first.truncation.omitted_counts.get("focus"), Some(&1));
    assert_eq!(first.truncation.omitted_counts.get("recent"), Some(&2));
    assert!(!first.truncation.omitted_counts.contains_key("blocked"));

    assert_eq!(
        first.sources.iter().map(|row| row.name).collect::<Vec<_>>(),
        [
            SourceName::Beads,
            SourceName::Git,
            SourceName::Openspec,
            SourceName::Whisper,
            SourceName::Curation,
        ]
    );
    assert_eq!(
        first
            .warnings
            .iter()
            .map(|warning| (warning.source, warning.detail.as_str()))
            .collect::<Vec<_>>(),
        [
            (SourceName::Beads, "line a"),
            (SourceName::Beads, "line z"),
            (SourceName::Git, "no HEAD"),
        ]
    );

    assert_eq!(first.health.status, HealthStatus::Green);
    assert_eq!(first.health.basis, HealthBasis::Human);
    assert_eq!(first.health.owner.as_deref(), Some("@sasha"));
    assert_eq!(first.head_sha.as_deref(), Some("head123"));
    assert!(first.source_hash.is_some());
}
