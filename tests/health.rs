//! Purpose: prove health signals fire on their evidence and verdicts layer.
//!
//! Responsibilities:
//! - trigger each of the seven D8 signals from typed brief inputs;
//! - prove silence is unknown and that only a fresh green human verdict is green;
//! - prove a fresh human green is downgraded when signals fire and expires when old.
//!
//! Rationale: health is the one place the brief is allowed to judge, so every
//! verdict must be traceable to a named signal or an unexpired human word.

#[path = "health/fixtures.rs"]
mod fixtures;

use fixtures::{
    Scenario, event, firing, focus_entry, human, kinds, mandatory_row, open_item, park_date,
    park_event, spec_item, titled_open_item,
};
use podras::health::{evaluate, signals};
use podras::model::{HealthBasis, HealthStatus, SignalKind, SourceName, SourceState, Staleness};

#[test]
fn silence_is_unknown() {
    let inputs = Scenario::quiet().inputs();

    assert!(signals(&inputs).is_empty(), "quiet fixture must not fire");

    let health = evaluate(&inputs);
    assert_eq!(health.status, HealthStatus::Unknown);
    assert_eq!(health.basis, HealthBasis::None);
    assert!(health.signals.is_empty());
}

#[test]
fn fresh_human_green_without_signals_is_green() {
    let inputs = Scenario {
        verdict: Some(human("green", "2026-11-18")),
        ..Scenario::quiet()
    }
    .inputs();

    let health = evaluate(&inputs);
    assert_eq!(health.status, HealthStatus::Green);
    assert_eq!(health.basis, HealthBasis::Human);
    assert!(health.signals.is_empty());
    assert_eq!(health.owner.as_deref(), Some("@sasha"));
}

#[test]
fn human_green_downgraded_by_firing_signals() {
    let inputs = Scenario {
        verdict: Some(human("green", "2026-11-18")),
        focus: vec![
            focus_entry("podras-a", "2026-11-01", "2026-11-18"),
            focus_entry("podras-closed", "2026-11-01", "2026-11-18"),
        ],
        open: vec![open_item("podras-a")],
        ..Scenario::fresh()
    }
    .inputs();

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
    let inputs = Scenario {
        verdict: Some(human("green", "2026-10-01")),
        ..Scenario::quiet()
    }
    .inputs();

    assert!(signals(&inputs).is_empty());
    let health = evaluate(&inputs);
    assert_eq!(health.status, HealthStatus::Unknown);
    assert_eq!(health.basis, HealthBasis::None);
}

#[test]
fn stale_focus_fires_past_horizon() {
    let inputs = Scenario {
        focus: vec![focus_entry("podras-a", "2026-10-01", "2026-10-10")],
        open: vec![open_item("podras-a")],
        ..Scenario::fresh()
    }
    .inputs();

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
    let inputs = Scenario {
        focus: vec![focus_entry("podras-a", "2026-11-04", "2026-11-04")],
        open: vec![open_item("podras-a")],
        recent: vec![event("head123", "podras-a: boundary", "2026-11-17")],
        ..Scenario::default()
    }
    .inputs();

    assert!(
        signals(&inputs).is_empty(),
        "exactly 14 days is not older than 14"
    );
}

#[test]
fn wip_over_cap_fires_above_max_focus() {
    let inputs = Scenario {
        focus: vec![
            focus_entry("podras-a", "2026-11-01", "2026-11-18"),
            focus_entry("podras-b", "2026-11-01", "2026-11-18"),
        ],
        open: vec![open_item("podras-a"), open_item("podras-b")],
        recent: vec![event("head123", "podras-a: start", "2026-11-17")],
        ..Scenario::default()
    }
    .inputs();

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
    let inputs = Scenario {
        open: vec![open_item("podras-a")],
        recent: vec![event("old", "stale work", "2026-10-20")],
        ..Scenario::default()
    }
    .inputs();

    let signal = firing(&inputs, SignalKind::StuckProject);
    assert!(
        signal.reason.contains("2026-10-20"),
        "reason was: {}",
        signal.reason
    );
}

#[test]
fn focus_closed_fires_when_focused_id_is_not_open() {
    let inputs = Scenario {
        focus: vec![focus_entry("podras-gone", "2026-11-01", "2026-11-18")],
        open: vec![open_item("podras-a")],
        ..Scenario::fresh()
    }
    .inputs();

    let signal = firing(&inputs, SignalKind::FocusClosed);
    assert!(
        signal.reason.contains("podras-gone"),
        "reason was: {}",
        signal.reason
    );
}

#[test]
fn parked_expired_fires_on_past_date_but_not_on_event() {
    let inputs = Scenario {
        parked: vec![
            park_date("podras-old", "2026-11-01"),
            park_event("podras-event", "value-prop-roadmap"),
        ],
        ..Scenario::fresh()
    }
    .inputs();

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
    let inputs = Scenario {
        open: vec![titled_open_item("podras-a", "wire src/health.rs")],
        recent: vec![
            event("b2", "commit b2", "2026-11-10"),
            event("a1", "podras-a: fix health", "2026-11-11"),
            event("c0", "src/health.rs: tighten rule", "2026-11-12"),
            event("d9", "chore outside window", "2026-10-01"),
        ],
        ..Scenario::default()
    }
    .inputs();

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
    let inputs = Scenario {
        open: vec![open_item("podras-a")],
        changes: vec![
            spec_item("add-brief", "Brief read path", "change", Some(3)),
            spec_item("podras-d2e", "linked change", "change", Some(2)),
            spec_item("old-change", "old change", "change", Some(20)),
            spec_item("read-path", "spec document", "spec", Some(1)),
        ],
        ..Scenario::fresh()
    }
    .inputs();

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
    let inputs = Scenario {
        focus: vec![
            focus_entry("podras-a", "2026-11-01", "2026-11-18"),
            focus_entry("podras-b", "2026-11-01", "2026-11-18"),
        ],
        open: vec![open_item("podras-a"), open_item("podras-b")],
        ..Scenario::fresh()
    }
    .inputs();

    let health = evaluate(&inputs);
    assert_eq!(health.status, HealthStatus::Amber);
    assert_eq!(health.basis, HealthBasis::Signals);
    let reason = health.reason.as_deref().unwrap_or_default();
    assert!(reason.contains("wip_over_cap"), "reason was: {reason}");
}

/// A fresh human green over missing or stale mandatory sources is amber.
#[test]
fn fresh_human_green_downgraded_by_impaired_mandatory_sources() {
    let inputs = Scenario {
        verdict: Some(human("green", "2026-11-18")),
        sources: vec![
            mandatory_row(SourceName::Beads, SourceState::Missing, Staleness::Unknown),
            mandatory_row(SourceName::Git, SourceState::Present, Staleness::Fresh),
        ],
        ..Scenario::quiet()
    }
    .inputs();

    assert!(
        signals(&inputs).is_empty(),
        "the mandatory guard must work without a firing signal"
    );
    let health = evaluate(&inputs);
    assert_eq!(health.status, HealthStatus::Amber);
    assert_eq!(health.basis, HealthBasis::Human);
    assert_eq!(health.owner.as_deref(), Some("@sasha"));
    assert_eq!(
        health.reason.as_deref(),
        Some("human green downgraded by unusable mandatory sources: beads (missing)")
    );
}

/// A stale mandatory source is named with its state in the downgrade reason.
#[test]
fn fresh_human_green_downgrade_names_a_stale_mandatory_source() {
    let inputs = Scenario {
        verdict: Some(human("green", "2026-11-18")),
        sources: vec![
            mandatory_row(SourceName::Beads, SourceState::Present, Staleness::Fresh),
            mandatory_row(SourceName::Git, SourceState::Present, Staleness::Stale),
        ],
        ..Scenario::quiet()
    }
    .inputs();

    let health = evaluate(&inputs);
    assert_eq!(health.status, HealthStatus::Amber);
    assert_eq!(health.basis, HealthBasis::Human);
    assert_eq!(
        health.reason.as_deref(),
        Some("human green downgraded by unusable mandatory sources: git (stale)")
    );
}

/// Fresh mandatory sources keep a signal-free fresh human green green.
#[test]
fn fresh_human_green_with_fresh_mandatory_sources_is_green() {
    let inputs = Scenario {
        verdict: Some(human("green", "2026-11-18")),
        sources: vec![
            mandatory_row(SourceName::Beads, SourceState::Present, Staleness::Fresh),
            mandatory_row(SourceName::Git, SourceState::Present, Staleness::Fresh),
            mandatory_row(SourceName::Openspec, SourceState::Present, Staleness::Fresh),
            mandatory_row(SourceName::Whisper, SourceState::Present, Staleness::Fresh),
            mandatory_row(SourceName::Curation, SourceState::Present, Staleness::Fresh),
        ],
        ..Scenario::quiet()
    }
    .inputs();

    let health = evaluate(&inputs);
    assert_eq!(health.status, HealthStatus::Green, "{health:?}");
    assert_eq!(health.basis, HealthBasis::Human);
    assert!(health.signals.is_empty());
    assert_eq!(health.reason.as_deref(), Some("human note"));
}

/// Extract the markdown Health section, including its heading.
fn health_section(markdown: &str) -> &str {
    let start = markdown
        .find("## Health + confidence")
        .expect("the narrative renders a Health section");
    let tail = &markdown[start..];
    let end = tail["## Health + confidence".len()..]
        .find("\n## ")
        .map(|offset| offset + "## Health + confidence".len() + 1)
        .unwrap_or(tail.len());
    &tail[..end]
}

/// A lens view whose only warning is one no-go violation candidate.
fn candidate_lens() -> podras::model::RoadmapView {
    use podras::model::{
        IdentityView, ReviewDue, RoadmapView, RoadmapWarning, Staleness, Truncation,
    };

    RoadmapView {
        identity: IdentityView {
            path: "README.md#why".to_string(),
            last_reviewed: chrono::NaiveDate::from_ymd_opt(2026, 11, 1).expect("fixture date"),
            next_review: Some(chrono::NaiveDate::from_ymd_opt(2027, 5, 18).expect("fixture date")),
            staleness: Staleness::Fresh,
        },
        bets: Vec::new(),
        no_gos: Vec::new(),
        review_due: ReviewDue {
            identity: Some(chrono::NaiveDate::from_ymd_opt(2027, 5, 18).expect("fixture date")),
            roadmap: None,
        },
        warnings: vec![RoadmapWarning {
            code: podras::model::RoadmapWarningKind::NoGoViolationCandidate,
            subject: "no-open-tracker-writes".to_string(),
            detail: "podras-a matched `brief`".to_string(),
        }],
        truncation: Truncation {
            applied: false,
            omitted_counts: std::collections::BTreeMap::new(),
        },
    }
}

/// The Health section surfaces exactly one advisory no-go line when the lens
/// carries a `no_go_violation_candidate`, and nothing when it does not.
#[test]
fn health_no_go_line() {
    use podras::brief::assemble;
    use podras::model::RoadmapWarningKind;
    use podras::render::{RenderFormat, render};

    let mut candidate = assemble(&Scenario::quiet().inputs());
    candidate.roadmap = Some(candidate_lens());
    let candidate_render = render(&candidate, RenderFormat::Markdown).expect("markdown renders");
    let candidate_health = health_section(&candidate_render);
    let matches: Vec<&str> = candidate_health
        .lines()
        .filter(|line| line.starts_with("- no-go risk:"))
        .collect();
    assert_eq!(
        matches,
        vec!["- no-go risk: no-open-tracker-writes — podras-a matched `brief`"],
        "exactly one advisory line, after the signals: {candidate_health}"
    );
    let signal_lines = candidate_health
        .lines()
        .filter(|line| line.starts_with("- signal:"))
        .count();
    assert_eq!(signal_lines, 0, "the quiet fixture fires no signals");
    let health_line = candidate_health
        .find("Health: **")
        .expect("the status line renders");
    assert!(
        candidate_health[health_line..]
            .find("- no-go risk:")
            .is_some(),
        "the advisory line follows the status line: {candidate_health}"
    );
    assert!(
        !candidate_health.contains("signal: no_go_violation_candidate"),
        "the candidate never becomes a signal: {candidate_health}"
    );

    // A different advisory and no candidate at all: Health is byte-identical
    // to the no-roadmap render.
    let mut other = assemble(&Scenario::quiet().inputs());
    let mut lens = candidate_lens();
    lens.warnings[0].code = RoadmapWarningKind::IdentityReviewOverdue;
    other.roadmap = Some(lens);
    let plain = assemble(&Scenario::quiet().inputs());
    assert_eq!(
        health_section(&render(&other, RenderFormat::Markdown).expect("markdown renders")),
        health_section(&render(&plain, RenderFormat::Markdown).expect("markdown renders")),
        "no candidate means a byte-identical Health section"
    );
}
