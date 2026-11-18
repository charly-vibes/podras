//! Purpose: prove the brief never infers focus and renders nine fixed sections.
//!
//! Responsibilities:
//! - `focus_not_inferred`: the deployed property contract for curation authority;
//! - assert the narrative render emits every section in cold-return order;
//! - assert empty sections render an explicit `none` line;
//! - assert the health line carries status, basis, reason, and owner;
//! - assert the re-entry line appears only when supplied;
//! - assert parked items never leak into next / ready.
//!
//! Rationale: curation beats inference, and the narrative is the human
//! contract. Both claims are only credible if a fixture whose tracker priority
//! disagrees with the curation file proves the brief follows the human.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use podras::brief::{
    BeadsSummary, BriefInputs, CurationState, GitSummary, SpecSummary, WhisperReading, assemble,
};
use podras::curation::{CurationFile, CurationHealth, FocusEntry, ParkEntry};
use podras::model::{
    Brief, RecentEvent, SourceName, SourceState, SourceStatus, Staleness, WorkItem,
};
use podras::render::markdown;

/// The tracker's top-priority item; the brief MUST NOT focus it by inference.
const TOP_PRIORITY_ID: &str = "podras-a";
/// The human-curated focus, deliberately lower priority than the tracker top.
const FOCUS_ID: &str = "podras-b";
/// A parked item that also appears in the tracker's ready queue.
const PARKED_ID: &str = "podras-c";

fn day(raw: &str) -> NaiveDate {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").expect("fixture dates are valid")
}

fn instant() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 18, 9, 0, 0)
        .single()
        .expect("fixture instant is valid")
}

fn item(id: &str, priority: Option<u8>) -> WorkItem {
    WorkItem {
        id: id.to_string(),
        title: Some(format!("title {id}")),
        status: Some("open".to_string()),
        priority,
        blocked_by: Vec::new(),
        age_days: Some(2),
        reason: None,
        confirmed_at: None,
    }
}

fn source(name: SourceName, status: SourceState) -> SourceStatus {
    SourceStatus {
        name,
        status,
        observed_at: Some(instant()),
        staleness: Staleness::Fresh,
        detail: None,
    }
}

fn commit(sha: &str) -> RecentEvent {
    RecentEvent {
        sha: sha.to_string(),
        committed_at: instant(),
        subject: format!("commit {sha}"),
        outcome: Some("shipped".to_string()),
    }
}

/// Inputs where tracker top priority and curation focus disagree.
fn curated_inputs() -> BriefInputs {
    let beads = BeadsSummary {
        open: vec![
            item(TOP_PRIORITY_ID, Some(0)),
            item(FOCUS_ID, Some(3)),
            item(PARKED_ID, Some(1)),
            item("podras-d", Some(5)),
        ],
        blocked: Vec::new(),
        ready: vec![
            item(TOP_PRIORITY_ID, Some(0)),
            item(FOCUS_ID, Some(3)),
            item(PARKED_ID, Some(1)),
            item("podras-d", Some(5)),
        ],
        observed_at: Some(instant()),
    };
    let git = GitSummary {
        head_sha: Some("head123".to_string()),
        recent: vec![commit("abc1234")],
        observed_at: Some(instant()),
    };
    let specs = SpecSummary {
        changes: Vec::new(),
        stale_count: 0,
        observed_at: Some(instant()),
    };
    let whisper = WhisperReading {
        observed_at: Some(instant()),
        next_action: Some("pick up podras-b".to_string()),
        content_hash: None,
    };
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
        focus: vec![FocusEntry {
            id: FOCUS_ID.to_string(),
            reason: Some("read path".to_string()),
            focused_since: Some("2026-11-01".to_string()),
            confirmed_at: Some("2026-11-18".to_string()),
        }],
        parked: vec![ParkEntry {
            id: PARKED_ID.to_string(),
            reason: "harvest is v1.1".to_string(),
            resurface: "2027-01-01".to_string(),
            parked_at: Some("2026-11-10".to_string()),
            confirmed_at: Some("2026-11-10".to_string()),
        }],
    };
    BriefInputs {
        beads: Some(beads),
        git: Some(git),
        specs: Some(specs),
        whisper: Some(whisper),
        curation: Some(CurationState {
            curation,
            updated_at: Some(instant()),
        }),
        sources: vec![
            source(SourceName::Beads, SourceState::Present),
            source(SourceName::Git, SourceState::Present),
            source(SourceName::Curation, SourceState::Present),
        ],
        warnings: Vec::new(),
        as_of: Some(day("2026-11-18")),
        now: day("2026-11-01"),
        generated_at: instant(),
    }
}

/// Inputs with no readable source and no curation: every list section is empty.
fn empty_inputs() -> BriefInputs {
    BriefInputs {
        beads: None,
        git: None,
        specs: None,
        whisper: None,
        curation: None,
        sources: Vec::new(),
        warnings: Vec::new(),
        as_of: Some(day("2026-11-18")),
        now: day("2026-11-01"),
        generated_at: instant(),
    }
}

fn ids(items: &[WorkItem]) -> Vec<&str> {
    items.iter().map(|item| item.id.as_str()).collect()
}

fn headings(output: &str) -> Vec<&str> {
    output
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .collect()
}

fn section<'a>(output: &'a str, heading: &str) -> &'a str {
    let marker = format!("## {heading}\n");
    let start = output
        .find(&marker)
        .unwrap_or_else(|| panic!("rendered output has section `{heading}`: {output}"))
        + marker.len();
    let rest = &output[start..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    &rest[..end]
}

fn assert_none_line(body: &str, heading: &str) {
    assert!(
        body.lines().any(|line| line.trim() == "none"),
        "section `{heading}` must carry an explicit none line, got: {body:?}"
    );
}

#[test]
fn focus_not_inferred() {
    let brief = assemble(&curated_inputs());

    assert_eq!(brief.focus.len(), 1, "curation declares exactly one focus");
    assert_eq!(brief.focus[0].id, FOCUS_ID, "focus must follow curation");
    assert_ne!(
        brief.focus[0].id, TOP_PRIORITY_ID,
        "focus must not follow tracker priority"
    );
    assert!(
        ids(&brief.next).contains(&TOP_PRIORITY_ID),
        "the top-priority tracker item belongs in next, not focus"
    );
    assert!(
        !ids(&brief.next).contains(&PARKED_ID),
        "a parked item must never appear in next"
    );
    assert!(
        brief.parked.iter().any(|item| item.id == PARKED_ID),
        "the curated parked item must render in parked"
    );

    let rendered = markdown::render(&brief, None).expect("render the narrative");
    let focus = section(&rendered, "Focus");
    assert!(
        focus.contains(FOCUS_ID),
        "focus section shows the curated id"
    );
    assert!(
        !focus.contains(TOP_PRIORITY_ID),
        "focus section never shows the inferred top-priority id"
    );
    assert!(
        !section(&rendered, "Next / ready").contains(PARKED_ID),
        "parked items are absent from the rendered next section"
    );
    assert!(
        section(&rendered, "Parked").contains(PARKED_ID),
        "parked items render in the parked section"
    );
}

#[test]
fn render_nine_sections_in_cold_return_order() {
    let brief = assemble(&curated_inputs());
    let rendered = markdown::render(&brief, None).expect("render the narrative");

    assert_eq!(
        headings(&rendered),
        [
            "Purpose / current state",
            "Focus",
            "In flight",
            "Blocked / waiting",
            "Next / ready",
            "Parked",
            "Recent",
            "Health + confidence",
            "Sources & freshness",
        ],
        "the nine fixed sections render once each in cold-return order"
    );
    assert!(
        rendered.contains("### Needs re-triage"),
        "parked carries the needs-re-triage subsection: {rendered}"
    );
}

#[test]
fn empty_sections_render_none_lines() {
    let brief: Brief = assemble(&empty_inputs());
    let rendered = markdown::render(&brief, None).expect("render the narrative");

    for heading in [
        "Purpose / current state",
        "Focus",
        "In flight",
        "Blocked / waiting",
        "Next / ready",
        "Parked",
        "Recent",
    ] {
        assert_none_line(section(&rendered, heading), heading);
    }

    let parked = section(&rendered, "Parked");
    let retriage = parked
        .split_once("### Needs re-triage")
        .expect("needs-re-triage subsection is always present")
        .1;
    assert_none_line(retriage, "Needs re-triage");
}

#[test]
fn health_line_carries_status_basis_reason_and_owner() {
    let brief = assemble(&curated_inputs());
    let rendered = markdown::render(&brief, None).expect("render the narrative");
    let rendered_health = section(&rendered, "Health + confidence");

    let status = format!("{:?}", brief.health.status).to_lowercase();
    let basis = format!("{:?}", brief.health.basis).to_lowercase();
    assert!(
        rendered_health.contains(&status),
        "status renders: {rendered_health}"
    );
    assert!(
        rendered_health.contains(&basis),
        "basis renders: {rendered_health}"
    );
    assert!(
        rendered_health.contains("@sasha"),
        "owner renders: {rendered_health}"
    );
    let reason = brief
        .health
        .reason
        .as_deref()
        .expect("assembled health carries a reason");
    assert!(
        rendered_health.contains(reason),
        "reason renders: {rendered_health}"
    );
}

#[test]
fn reentry_line_only_when_supplied() {
    let brief = assemble(&curated_inputs());

    let committed = markdown::render(&brief, None).expect("render the narrative");
    assert!(
        !committed.contains("Re-entry"),
        "committed render carries no re-entry line"
    );

    let on_demand =
        markdown::render(&brief, Some("pick up podras-b")).expect("render the narrative");
    assert!(
        on_demand.contains("Re-entry: pick up podras-b"),
        "on-demand render carries the supplied re-entry line: {on_demand}"
    );
}
