//! Purpose: prove health signals fire on their evidence and verdicts layer.
//!
//! Responsibilities:
//! - trigger each of the seven D8 signals from typed brief inputs;
//! - prove silence is unknown and that only a fresh green human verdict is green;
//! - prove a fresh human green is downgraded when signals fire and expires when old.
//!
//! Rationale: health is the one place the brief is allowed to judge, so every
//! verdict must be traceable to a named signal or an unexpired human word.

use chrono::{DateTime, NaiveDate, Utc};
use podras::brief::{
    BeadsSummary, BriefInputs, CurationState, GitSummary, SpecSummary, WhisperReading,
};
use podras::curation::{CurationFile, CurationHealth, FocusEntry, ParkEntry};
use podras::health::{evaluate, signals};
use podras::model::{HealthBasis, HealthStatus, RecentEvent, Signal, SignalKind, WorkItem};

/// Pinned evaluation date shared by every fixture.
const AS_OF: &str = "2026-11-18";

fn day(raw: &str) -> NaiveDate {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").expect("fixture dates are valid")
}

fn instant(date: &str, hour: u32) -> DateTime<Utc> {
    day(date)
        .and_hms_opt(hour, 0, 0)
        .expect("fixture hours are valid")
        .and_utc()
}

fn observed_at() -> Option<DateTime<Utc>> {
    Some(instant(AS_OF, 9))
}

fn open_item(id: &str) -> WorkItem {
    WorkItem {
        id: id.to_string(),
        title: Some(format!("title {id}")),
        status: Some("open".to_string()),
        priority: Some(2),
        blocked_by: Vec::new(),
        age_days: Some(3),
        reason: None,
        confirmed_at: None,
    }
}

fn titled_open_item(id: &str, title: &str) -> WorkItem {
    WorkItem {
        title: Some(title.to_string()),
        ..open_item(id)
    }
}

fn spec_item(id: &str, title: &str, status: &str, age_days: Option<u32>) -> WorkItem {
    WorkItem {
        id: id.to_string(),
        title: Some(title.to_string()),
        status: Some(status.to_string()),
        priority: None,
        blocked_by: Vec::new(),
        age_days,
        reason: None,
        confirmed_at: None,
    }
}

fn beads(open: Vec<WorkItem>) -> Option<BeadsSummary> {
    Some(BeadsSummary {
        open,
        blocked: Vec::new(),
        ready: Vec::new(),
        observed_at: observed_at(),
    })
}

fn event(sha: &str, subject: &str, date: &str) -> RecentEvent {
    RecentEvent {
        sha: sha.to_string(),
        committed_at: instant(date, 10),
        subject: subject.to_string(),
        outcome: None,
    }
}

fn git(recent: Vec<RecentEvent>) -> Option<GitSummary> {
    Some(GitSummary {
        head_sha: Some("head123".to_string()),
        recent,
        observed_at: observed_at(),
    })
}

fn fresh_git() -> Option<GitSummary> {
    git(vec![event("head123", "initial commit", "2026-11-17")])
}

/// Git history whose newest commit is outside the 14-day window.
fn quiet_git() -> Option<GitSummary> {
    git(vec![event("old123", "initial commit", "2026-10-01")])
}

fn specs(changes: Vec<WorkItem>) -> Option<SpecSummary> {
    Some(SpecSummary {
        changes,
        stale_count: 0,
        observed_at: observed_at(),
    })
}

fn focus_entry(id: &str, focused_since: &str, confirmed_at: &str) -> FocusEntry {
    FocusEntry {
        id: id.to_string(),
        reason: Some(format!("reason for {id}")),
        focused_since: Some(focused_since.to_string()),
        confirmed_at: Some(confirmed_at.to_string()),
    }
}

fn park_date(id: &str, resurface: &str) -> ParkEntry {
    ParkEntry {
        id: id.to_string(),
        reason: format!("reason for {id}"),
        resurface: resurface.to_string(),
        parked_at: Some("2026-10-01".to_string()),
        confirmed_at: Some("2026-10-01".to_string()),
    }
}

fn park_event(id: &str, resurface: &str) -> ParkEntry {
    ParkEntry {
        id: id.to_string(),
        reason: format!("reason for {id}"),
        resurface: format!("event:{resurface}"),
        parked_at: Some("2026-10-01".to_string()),
        confirmed_at: Some("2026-10-01".to_string()),
    }
}

fn curation(
    health: Option<CurationHealth>,
    max_focus: Option<usize>,
    focus: Vec<FocusEntry>,
    parked: Vec<ParkEntry>,
) -> Option<CurationState> {
    Some(CurationState {
        curation: CurationFile {
            purpose_lite: Some("podras: status read path for a cold return".to_string()),
            max_focus,
            publish_whisper: Some(false),
            health,
            focus,
            parked,
        },
        updated_at: observed_at(),
    })
}

fn human(status: &str, confirmed_at: &str) -> CurationHealth {
    CurationHealth {
        status: status.to_string(),
        owner: Some("@sasha".to_string()),
        confirmed_at: Some(confirmed_at.to_string()),
        reason: Some("human note".to_string()),
    }
}

fn inputs(
    curation: Option<CurationState>,
    beads: Option<BeadsSummary>,
    git: Option<GitSummary>,
    specs: Option<SpecSummary>,
) -> BriefInputs {
    BriefInputs {
        beads,
        git,
        specs,
        whisper: Some(WhisperReading {
            observed_at: observed_at(),
            next_action: None,
            content_hash: None,
        }),
        curation,
        sources: Vec::new(),
        warnings: Vec::new(),
        as_of: Some(day(AS_OF)),
        now: day("2026-11-01"),
        generated_at: instant(AS_OF, 12),
    }
}

/// Inputs with no human verdict and no firing signal.
fn quiet_inputs() -> BriefInputs {
    inputs(
        curation(None, Some(1), Vec::new(), Vec::new()),
        beads(Vec::new()),
        quiet_git(),
        specs(Vec::new()),
    )
}

fn kinds(signals: &[Signal]) -> Vec<SignalKind> {
    signals.iter().map(|signal| signal.kind).collect()
}

fn firing(inputs: &BriefInputs, kind: SignalKind) -> Signal {
    signals(inputs)
        .into_iter()
        .find(|signal| signal.kind == kind)
        .unwrap_or_else(|| panic!("expected {kind:?} to fire"))
}

#[test]
fn silence_is_unknown() {
    let inputs = quiet_inputs();

    assert!(signals(&inputs).is_empty(), "quiet fixture must not fire");

    let health = evaluate(&inputs);
    assert_eq!(health.status, HealthStatus::Unknown);
    assert_eq!(health.basis, HealthBasis::None);
    assert!(health.signals.is_empty());
}

#[test]
fn fresh_human_green_without_signals_is_green() {
    let inputs = inputs(
        curation(
            Some(human("green", "2026-11-18")),
            Some(1),
            Vec::new(),
            Vec::new(),
        ),
        beads(Vec::new()),
        quiet_git(),
        specs(Vec::new()),
    );

    let health = evaluate(&inputs);
    assert_eq!(health.status, HealthStatus::Green);
    assert_eq!(health.basis, HealthBasis::Human);
    assert!(health.signals.is_empty());
    assert_eq!(health.owner.as_deref(), Some("@sasha"));
}

#[test]
fn human_green_downgraded_by_firing_signals() {
    let inputs = inputs(
        curation(
            Some(human("green", "2026-11-18")),
            Some(1),
            vec![
                focus_entry("podras-a", "2026-11-01", "2026-11-18"),
                focus_entry("podras-closed", "2026-11-01", "2026-11-18"),
            ],
            Vec::new(),
        ),
        beads(vec![open_item("podras-a")]),
        fresh_git(),
        specs(Vec::new()),
    );

    let fired = signals(&inputs);
    let fired_kinds = kinds(&fired);
    assert!(fired_kinds.contains(&SignalKind::WipOverCap));
    assert!(fired_kinds.contains(&SignalKind::FocusClosed));

    let health = evaluate(&inputs);
    assert_eq!(health.status, HealthStatus::Amber);
    assert_eq!(health.basis, HealthBasis::Human);
    assert_eq!(health.owner.as_deref(), Some("@sasha"));
    let reason = health.reason.as_deref().unwrap_or_default();
    assert!(reason.contains("wip_over_cap"), "reason was: {reason}");
    assert!(reason.contains("focus_closed"), "reason was: {reason}");
    assert!(kinds(&health.signals).contains(&SignalKind::WipOverCap));
    assert!(kinds(&health.signals).contains(&SignalKind::FocusClosed));
}

#[test]
fn stale_human_verdict_is_absent() {
    let inputs = inputs(
        curation(
            Some(human("green", "2026-10-01")),
            Some(1),
            Vec::new(),
            Vec::new(),
        ),
        beads(Vec::new()),
        quiet_git(),
        specs(Vec::new()),
    );

    assert!(signals(&inputs).is_empty());
    let health = evaluate(&inputs);
    assert_eq!(health.status, HealthStatus::Unknown);
    assert_eq!(health.basis, HealthBasis::None);
}

#[test]
fn stale_focus_fires_past_horizon() {
    let inputs = inputs(
        curation(
            None,
            Some(1),
            vec![focus_entry("podras-a", "2026-10-01", "2026-10-10")],
            Vec::new(),
        ),
        beads(vec![open_item("podras-a")]),
        fresh_git(),
        specs(Vec::new()),
    );

    let signal = firing(&inputs, SignalKind::StaleFocus);
    assert!(
        signal.reason.contains("podras-a"),
        "reason was: {}",
        signal.reason
    );
    assert_eq!(evaluate(&inputs).status, HealthStatus::Amber);
    assert_eq!(evaluate(&inputs).basis, HealthBasis::Signals);
}

#[test]
fn stale_focus_boundary_of_fourteen_days_is_not_stale() {
    let inputs = inputs(
        curation(
            None,
            Some(1),
            vec![focus_entry("podras-a", "2026-11-04", "2026-11-04")],
            Vec::new(),
        ),
        beads(vec![open_item("podras-a")]),
        git(vec![event("head123", "podras-a: boundary", "2026-11-17")]),
        specs(Vec::new()),
    );

    assert!(
        signals(&inputs).is_empty(),
        "exactly 14 days is not older than 14"
    );
}

#[test]
fn wip_over_cap_fires_above_max_focus() {
    let inputs = inputs(
        curation(
            None,
            Some(1),
            vec![
                focus_entry("podras-a", "2026-11-01", "2026-11-18"),
                focus_entry("podras-b", "2026-11-01", "2026-11-18"),
            ],
            Vec::new(),
        ),
        beads(vec![open_item("podras-a"), open_item("podras-b")]),
        git(vec![event("head123", "podras-a: start", "2026-11-17")]),
        specs(Vec::new()),
    );

    let signal = firing(&inputs, SignalKind::WipOverCap);
    assert!(
        signal.reason.contains("max_focus 1"),
        "reason was: {}",
        signal.reason
    );
    assert_eq!(kinds(&signals(&inputs)), vec![SignalKind::WipOverCap]);
}

#[test]
fn stuck_project_fires_when_in_flight_and_history_is_old() {
    let inputs = inputs(
        curation(None, Some(1), Vec::new(), Vec::new()),
        beads(vec![open_item("podras-a")]),
        git(vec![event("old", "stale work", "2026-10-20")]),
        specs(Vec::new()),
    );

    let signal = firing(&inputs, SignalKind::StuckProject);
    assert!(
        signal.reason.contains("2026-10-20"),
        "reason was: {}",
        signal.reason
    );
}

#[test]
fn focus_closed_fires_when_focused_id_is_not_open() {
    let inputs = inputs(
        curation(
            None,
            Some(1),
            vec![focus_entry("podras-gone", "2026-11-01", "2026-11-18")],
            Vec::new(),
        ),
        beads(vec![open_item("podras-a")]),
        fresh_git(),
        specs(Vec::new()),
    );

    let signal = firing(&inputs, SignalKind::FocusClosed);
    assert!(
        signal.reason.contains("podras-gone"),
        "reason was: {}",
        signal.reason
    );
}

#[test]
fn parked_expired_fires_on_past_date_but_not_on_event() {
    let inputs = inputs(
        curation(
            None,
            Some(1),
            Vec::new(),
            vec![
                park_date("podras-old", "2026-11-01"),
                park_event("podras-event", "value-prop-roadmap"),
            ],
        ),
        beads(Vec::new()),
        fresh_git(),
        specs(Vec::new()),
    );

    let signal = firing(&inputs, SignalKind::ParkedExpired);
    assert!(
        signal.reason.contains("podras-old"),
        "reason was: {}",
        signal.reason
    );
    assert!(
        !signal.reason.contains("podras-event"),
        "event triggers must not fire: {}",
        signal.reason
    );
}

#[test]
fn orphan_commits_fire_only_for_unlinked_window_commits() {
    let inputs = inputs(
        curation(None, Some(1), Vec::new(), Vec::new()),
        beads(vec![titled_open_item("podras-a", "wire src/health.rs")]),
        git(vec![
            event("b2", "commit b2", "2026-11-10"),
            event("a1", "podras-a: fix health", "2026-11-11"),
            event("c0", "src/health.rs: tighten rule", "2026-11-12"),
            event("d9", "chore outside window", "2026-10-01"),
        ]),
        specs(Vec::new()),
    );

    let signal = firing(&inputs, SignalKind::OrphanCommits);
    assert!(
        signal.reason.contains("b2"),
        "reason was: {}",
        signal.reason
    );
    assert!(
        !signal.reason.contains("a1"),
        "tracker-linked commit fired: {}",
        signal.reason
    );
    assert!(
        !signal.reason.contains("c0"),
        "path-linked commit fired: {}",
        signal.reason
    );
    assert!(
        !signal.reason.contains("d9"),
        "old commit fired: {}",
        signal.reason
    );
}

#[test]
fn spec_change_without_bead_fires_only_for_young_unlinked_changes() {
    let inputs = inputs(
        curation(None, Some(1), Vec::new(), Vec::new()),
        beads(vec![open_item("podras-a")]),
        fresh_git(),
        specs(vec![
            spec_item("add-brief", "Brief read path", "change", Some(3)),
            spec_item("podras-d2e", "linked change", "change", Some(2)),
            spec_item("old-change", "old change", "change", Some(20)),
            spec_item("read-path", "spec document", "spec", Some(1)),
        ]),
    );

    let signal = firing(&inputs, SignalKind::SpecChangeWithoutBead);
    assert!(
        signal.reason.contains("add-brief"),
        "reason was: {}",
        signal.reason
    );
    assert!(
        !signal.reason.contains("podras-d2e"),
        "linked change fired: {}",
        signal.reason
    );
    assert!(
        !signal.reason.contains("old-change"),
        "old change fired: {}",
        signal.reason
    );
    assert!(
        !signal.reason.contains("read-path"),
        "spec item fired: {}",
        signal.reason
    );
}

#[test]
fn signals_without_a_human_verdict_are_amber_with_signal_basis() {
    let inputs = inputs(
        curation(
            None,
            Some(1),
            vec![
                focus_entry("podras-a", "2026-11-01", "2026-11-18"),
                focus_entry("podras-b", "2026-11-01", "2026-11-18"),
            ],
            Vec::new(),
        ),
        beads(vec![open_item("podras-a"), open_item("podras-b")]),
        fresh_git(),
        specs(Vec::new()),
    );

    let health = evaluate(&inputs);
    assert_eq!(health.status, HealthStatus::Amber);
    assert_eq!(health.basis, HealthBasis::Signals);
    let reason = health.reason.as_deref().unwrap_or_default();
    assert!(reason.contains("wip_over_cap"), "reason was: {reason}");
}
