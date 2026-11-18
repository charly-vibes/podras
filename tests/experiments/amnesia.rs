//! Purpose: the amnesia experiment — `BRIEF.md` alone answers the cold-return questions.
//!
//! Responsibilities:
//! - `brief_md_answers_cold_return_questions`: every section carries the curated
//!   and observed facts, with per-source freshness and staleness;
//! - `committed_json_matches_on_demand_payload`: the committed and on-demand
//!   semantic payloads differ only in `generated_at`.
//!
//! Rationale: amnesia means the reader keeps nothing but the render, so the render
//! must answer the cold-return questions without a second source.

use serde_json::Value;

use crate::fixtures::{
    AS_OF, BLOCKED_ID, CLOSED_ID, FOCUS_ID, NEXT_ID, PARKED_ID, PARKED_TRIGGER,
    assert_exact_staleness, assert_section_order, assert_sources_table, brief_data, podras_in,
    rich_fixture, section,
};

#[test]
fn brief_md_answers_cold_return_questions() {
    let repo = rich_fixture();
    podras_in(&repo, &["brief", "--write", "--as-of", AS_OF]).success();

    let markdown = repo.read("BRIEF.md");
    assert_section_order(&markdown);

    let purpose = section(&markdown, "Purpose / current state");
    assert!(
        purpose.contains("podras: project status for a cold return"),
        "purpose answers what this repository is: {purpose}"
    );

    let focus = section(&markdown, "Focus");
    assert!(focus.contains(FOCUS_ID), "focus id is named: {focus}");
    assert!(
        focus.contains("Brief read path"),
        "focus names the curated issue title: {focus}"
    );

    let blocked = section(&markdown, "Blocked / waiting");
    assert!(
        blocked.contains(BLOCKED_ID),
        "blocked id is named: {blocked}"
    );
    assert!(
        blocked.contains("Envelope render"),
        "blocked item names its issue title: {blocked}"
    );
    assert!(
        blocked.contains(&format!("blocked by: {FOCUS_ID}")),
        "blocked item names its blocker: {blocked}"
    );

    let next = section(&markdown, "Next / ready");
    assert!(next.contains(NEXT_ID), "next id is named: {next}");
    assert!(
        !next.contains(FOCUS_ID) && !next.contains(PARKED_ID) && !next.contains(CLOSED_ID),
        "next excludes focus, parked, and closed items: {next}"
    );

    let parked = section(&markdown, "Parked");
    assert!(parked.contains(PARKED_ID), "parked id is named: {parked}");
    assert!(
        parked.contains(&format!("resurface: {PARKED_TRIGGER}")),
        "parked item names its resurface trigger: {parked}"
    );

    let health = section(&markdown, "Health + confidence");
    assert!(
        health.contains("Health: **unknown**"),
        "unknown curation health is never rendered green: {health}"
    );
    assert!(
        health.contains("basis: none") && health.contains("reason:"),
        "health verdict carries basis and reason: {health}"
    );

    let sources = section(&markdown, "Sources & freshness");
    assert!(sources.contains("| source | status | observed_at | staleness | detail |"));
    assert_sources_table(&markdown);
    assert_exact_staleness(
        &markdown,
        &[
            ("beads", "fresh"),
            ("git", "fresh"),
            ("openspec", "fresh"),
            ("whisper", "stale"),
            ("curation", "fresh"),
        ],
    );
}

#[test]
fn committed_json_matches_on_demand_payload() {
    let repo = rich_fixture();
    podras_in(&repo, &["brief", "--write", "--as-of", AS_OF]).success();

    let committed: Value =
        serde_json::from_str(repo.read("BRIEF.json").trim()).expect("BRIEF.json is JSON");
    let on_demand = brief_data(&repo);

    let mut committed_data = committed["data"].clone();
    let mut on_demand_data = on_demand;
    for payload in [&mut committed_data, &mut on_demand_data] {
        payload
            .as_object_mut()
            .expect("brief payload is an object")
            .remove("generated_at");
    }

    assert_eq!(
        committed_data, on_demand_data,
        "committed and on-demand payloads differ only in generated_at"
    );
}
