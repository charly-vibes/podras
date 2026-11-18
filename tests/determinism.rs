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

mod common;
#[path = "determinism/fixtures.rs"]
mod fixtures;

use fixtures::{
    BOUNDARY_AS_OF, boundary_fixture, brief_json_at, day, inputs, instant,
    normalize_freshness_stamp, podras_in, read_json, source_row, strip_generated_at,
};
use podras::brief::assemble;
use podras::model::{
    Brief, HealthBasis, HealthStatus, SignalKind, SourceName, WorkItem, semantic_payload,
    source_hash,
};
use serde_json::Value;

fn ids(items: &[WorkItem]) -> Vec<&str> {
    items.iter().map(|item| item.id.as_str()).collect()
}

fn assert_payload_ignores_volatile(first: &Brief, second: &Brief) {
    let payload_a = semantic_payload(first);
    let payload_b = semantic_payload(second);
    assert_eq!(payload_a, payload_b, "semantic payloads must match");
    assert!(payload_a.get("generated_at").is_none());
    assert!(payload_a.get("source_hash").is_none());
    assert_eq!(
        source_hash(first, None),
        source_hash(second, None),
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
}

fn assert_section_order(first: &Brief) {
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
}

fn assert_focus_and_truncation(first: &Brief) {
    assert_eq!(first.focus.len(), 1);
    assert_eq!(first.focus[0].id, "podras-second");
    assert_eq!(first.focus[0].confirmed_at, Some(day("2026-11-10")));
    assert!(first.truncation.applied);
    assert_eq!(first.truncation.omitted_counts.get("focus"), Some(&1));
    assert_eq!(first.truncation.omitted_counts.get("recent"), Some(&2));
    assert!(!first.truncation.omitted_counts.contains_key("blocked"));
}

fn assert_evidence(first: &Brief) {
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

    let health_kinds: Vec<SignalKind> = first
        .health
        .signals
        .iter()
        .map(|signal| signal.kind)
        .collect();
    assert_eq!(first.health.status, HealthStatus::Amber);
    assert_eq!(first.health.basis, HealthBasis::Human);
    assert!(health_kinds.contains(&SignalKind::WipOverCap));
    assert!(health_kinds.contains(&SignalKind::FocusClosed));
    let health_reason = first.health.reason.as_deref().unwrap_or_default();
    assert!(health_reason.contains("wip_over_cap"));
    assert!(health_reason.contains("focus_closed"));
    assert_eq!(first.health.owner.as_deref(), Some("@sasha"));
    assert_eq!(first.head_sha.as_deref(), Some("head123"));
    assert!(first.source_hash.is_some());
}

#[test]
fn determinism_payload() {
    let first = assemble(&inputs(instant(1)));
    let second = assemble(&inputs(instant(2)));

    assert_eq!(first.generated_at, instant(1));
    assert_ne!(first.generated_at, second.generated_at);

    assert_payload_ignores_volatile(&first, &second);
    assert_section_order(&first);
    assert_focus_and_truncation(&first);
    assert_evidence(&first);
}

/// Every source sits exactly on its freshness horizon, so all stay fresh.
fn assert_boundary_freshness(data: &Value) {
    for source in ["beads", "git", "openspec", "whisper", "curation"] {
        assert_eq!(
            source_row(data, source)["staleness"],
            "fresh",
            "{source} sits exactly on its horizon: {}",
            source_row(data, source)
        );
    }
}

/// The human verdict is fresh and the boundary resurface trigger fires.
fn assert_boundary_health(data: &Value) {
    assert_eq!(
        data["health"]["basis"], "human",
        "a human verdict exactly on its horizon is fresh: {}",
        data["health"]
    );
    assert_eq!(
        data["health"]["signals"]
            .as_array()
            .expect("signals is an array")
            .iter()
            .map(|signal| signal["kind"].clone())
            .collect::<Vec<_>>(),
        vec![Value::String("parked_expired".to_string())],
        "the resurface trigger exactly on the pinned date fires: {}",
        data["health"]
    );
    assert_eq!(
        data["needs_retriage"][0]["id"], "podras-parked",
        "the trigger moves the item out of parked: {}",
        data["needs_retriage"]
    );
    assert!(
        data["parked"]
            .as_array()
            .expect("parked is an array")
            .is_empty(),
        "the boundary trigger is expired, not parked: {}",
        data["parked"]
    );
}

/// Two `--write` runs at the same `--as-of` differ only in `generated_at`.
fn assert_writes_are_reproducible(repo: &common::TempRepo) {
    podras_in(repo, &["brief", "--write", "--as-of", BOUNDARY_AS_OF]).success();
    let first_markdown = repo.read("BRIEF.md");
    let first_render: Value = read_json(repo, "BRIEF.json");

    podras_in(repo, &["brief", "--write", "--as-of", BOUNDARY_AS_OF]).success();
    let second_markdown = repo.read("BRIEF.md");
    let second_render: Value = read_json(repo, "BRIEF.json");

    assert_eq!(
        strip_generated_at(first_render),
        strip_generated_at(second_render),
        "two --write runs at the same --as-of differ only in generated_at"
    );
    assert!(
        first_markdown.contains("Freshness: generated_at "),
        "the render carries the freshness stamp: {first_markdown}"
    );
    assert_eq!(
        normalize_freshness_stamp(&first_markdown),
        normalize_freshness_stamp(&second_markdown),
        "two BRIEF.md renders differ only in the stamp generated_at"
    );
}

/// An unparseable `--as-of` exits 2 and writes neither render.
fn assert_unparseable_as_of_writes_nothing() {
    let invalid = common::TempRepo::new();
    podras_in(&invalid, &["brief", "--write", "--as-of", "2026-13-40"])
        .failure()
        .code(2);
    assert!(
        !invalid.root().join("BRIEF.md").exists(),
        "an unparseable --as-of writes no BRIEF.md"
    );
    assert!(
        !invalid.root().join("BRIEF.json").exists(),
        "an unparseable --as-of writes no BRIEF.json"
    );
}

#[test]
fn as_of_reproducible_render() {
    let repo = boundary_fixture();

    let first = brief_json_at(&repo, BOUNDARY_AS_OF);
    assert_eq!(first["ok"].as_bool(), Some(true), "{first}");
    let data = &first["data"];

    // The pinned date decides freshness, not the wall clock.
    assert_boundary_freshness(data);
    assert_boundary_health(data);

    let second = brief_json_at(&repo, BOUNDARY_AS_OF);
    assert_eq!(
        strip_generated_at(first.clone()),
        strip_generated_at(second),
        "two --json runs at the same --as-of differ only in generated_at"
    );

    assert_writes_are_reproducible(&repo);
    assert_unparseable_as_of_writes_nothing();
}
