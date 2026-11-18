//! Purpose: prove the C6 no-go risk line renders from the lens, advisory only.
//!
//! Responsibilities:
//! - exactly one `- no-go risk:` line when the lens fires `no_go_violation_candidate`;
//! - byte-identical Health when no candidate fires;
//! - the candidate never becomes a signal.
//!
//! Rationale: the brief's Health section is the one judging voice; the no-go
//! line must appear only on the named candidate code (decision 16, wj1).

use super::fixtures::Scenario;
use super::health_section;

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
