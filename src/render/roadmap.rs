//! Purpose: render the roadmap lens as a standalone markdown narrative.
//!
//! Responsibilities:
//! - render identity, bets grouped by horizon/status, parked items (including
//!   those whose resurface trigger expired), no-gos, and advisory warnings in
//!   one fixed order;
//! - keep every value on one line and every empty section an explicit `none`;
//! - report truncation for bets and parked items.
//!
//! Rationale: the lens is a separate render path from the brief (design
//! decision 16): it shows WHO and WHY, never what is happening now. It is a
//! pure function of the brief's `roadmap` value, so a present lens with zero
//! bets is rendered as present rather than collapsing into the absent case.

use chrono::NaiveDate;

use crate::error::{Error, Result};
use crate::model::{
    BetStatus, BetView, Brief, Enforcement, Horizon, ParkedItem, RoadmapView, RoadmapWarningKind,
    Staleness,
};

/// The explicit placeholder rendered for an empty section or absent value.
const NONE: &str = "none";

/// Render the roadmap lens as markdown.
///
/// A brief without a roadmap value is a domain error: the lens render is only
/// requested when `.podras/roadmap.toml` exists, so absence here is a wiring
/// bug, not a degraded source.
pub fn render(brief: &Brief) -> Result<String> {
    let Some(view) = brief.roadmap.as_ref() else {
        return Err(Error::Domain {
            code: "roadmap_absent",
            message: "the brief carries no roadmap lens".into(),
            suggestion: None,
        });
    };

    let mut out = String::new();
    out.push_str("# Roadmap lens\n\n");
    render_identity(&mut out, view);
    render_bets(&mut out, view);
    render_parked(&mut out, brief);
    render_no_gos(&mut out, view);
    render_warnings(&mut out, view);

    Ok(out)
}

fn render_identity(out: &mut String, view: &RoadmapView) {
    push_heading(out, "## ", "Identity");
    push_text(out, &format!("- path: {}", oneline(&view.identity.path)));
    push_text(
        out,
        &format!("- last reviewed: {}", date(view.identity.last_reviewed)),
    );
    push_text(
        out,
        &format!("- review due: {}", opt_date(view.review_due.identity)),
    );
    push_text(
        out,
        &format!("- staleness: {}", staleness(view.identity.staleness)),
    );
    out.push('\n');
}

fn render_bets(out: &mut String, view: &RoadmapView) {
    for group in [
        Group::Now,
        Group::Next,
        Group::Later,
        Group::Deferred,
        Group::Shipped,
    ] {
        render_bet_group(out, view, group);
        out.push('\n');
    }

    // The bets bound is lens-wide, so the omitted count is stated once, after
    // the final group, even when that group is empty.
    render_bet_group(out, view, Group::Killed);
    push_omitted(out, omitted(view, "bets"));
    out.push('\n');
}

/// Render one horizon/status group: its bets, or an explicit `none`.
fn render_bet_group(out: &mut String, view: &RoadmapView, group: Group) {
    push_heading(out, "## ", group.title());
    let bets: Vec<&BetView> = view
        .bets
        .iter()
        .filter(|bet| group_of(bet) == group)
        .collect();
    if bets.is_empty() {
        push_text(out, NONE);
    } else {
        for bet in bets {
            for line in bet_lines(bet) {
                push_text(out, &line);
            }
        }
    }
}

/// Render the parked section: curated parked items first, then the entries
/// whose resurface trigger expired and that curation therefore wants
/// re-triaged.
///
/// The omission marker belongs to the bounded parked list (`parked` in the
/// brief's `truncation`); expired entries are not bounded. Empty means `none`
/// only when both lists are empty.
fn render_parked(out: &mut String, brief: &Brief) {
    push_heading(out, "## ", "Not now (parked)");
    if brief.parked.is_empty() && brief.needs_retriage.is_empty() {
        push_text(out, NONE);
    } else {
        for item in &brief.parked {
            push_text(out, &parked_line(item));
        }
        if !brief.parked.is_empty() {
            push_omitted(out, brief.truncation.omitted_counts.get("parked").copied());
        }
        for item in &brief.needs_retriage {
            let line = format!(
                "- {} — {} (needs re-triage: resurface trigger expired)",
                oneline(&item.id),
                field(item.title.as_deref())
            );
            push_text(out, &line);
        }
    }
    out.push('\n');
}

fn render_no_gos(out: &mut String, view: &RoadmapView) {
    push_heading(out, "## ", "Boundaries (no-gos)");
    if view.no_gos.is_empty() {
        push_text(out, NONE);
    } else {
        for no_go in &view.no_gos {
            push_text(
                out,
                &format!(
                    "- {} — {} (scope: {}; enforcement: {}; revoke by: {})",
                    oneline(&no_go.id),
                    oneline(&no_go.reason),
                    oneline(&no_go.scope),
                    enforcement(&no_go.enforcement),
                    oneline(&no_go.revoke_by),
                ),
            );
        }
    }
    out.push('\n');
}

fn render_warnings(out: &mut String, view: &RoadmapView) {
    push_heading(out, "## ", "Warnings");
    if view.warnings.is_empty() {
        push_text(out, NONE);
    } else {
        for warning in &view.warnings {
            push_text(
                out,
                &format!(
                    "- {} — {} — {}",
                    warning_code(&warning.code),
                    oneline(&warning.subject),
                    oneline(&warning.detail),
                ),
            );
        }
    }
}

/// One bet as its three fixed lines.
fn bet_lines(bet: &BetView) -> [String; 3] {
    let mut lifecycle = format!(
        "  - admitted: {}; last reviewed: {}; review or kill: {}",
        date(bet.admitted_at),
        date(bet.last_reviewed),
        opt_date(bet.review_or_kill),
    );
    if let Some(revive_when) = bet.revive_when.as_deref() {
        lifecycle.push_str(&format!("; revive when: {}", oneline(revive_when)));
    }
    if let Some(killed_reason) = bet.killed_reason.as_deref() {
        lifecycle.push_str(&format!("; killed reason: {}", oneline(killed_reason)));
    }

    [
        format!(
            "- {} — {} (status: {}; horizon: {}; appetite: {}w; owner: {}; epic: {})",
            oneline(&bet.id),
            oneline(&bet.title),
            bet_status(&bet.status),
            horizon(&bet.horizon),
            bet.appetite_weeks,
            oneline(&bet.owner),
            field(bet.epic.as_deref()),
        ),
        format!("  - outcome: {}", oneline(&bet.outcome)),
        lifecycle,
    ]
}

fn parked_line(item: &ParkedItem) -> String {
    format!(
        "- {} — {} (parked: {}; confirmed: {}; resurface: {})",
        oneline(&item.id),
        oneline(&item.reason),
        item.parked_at,
        item.confirmed_at,
        oneline(&item.resurface.to_string()),
    )
}

/// The six rendered bet groups, in render order.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Group {
    Now,
    Next,
    Later,
    Deferred,
    Shipped,
    Killed,
}

impl Group {
    fn title(self) -> &'static str {
        match self {
            Group::Now => "Now",
            Group::Next => "Next",
            Group::Later => "Later",
            Group::Deferred => "Deferred",
            Group::Shipped => "Shipped",
            Group::Killed => "Killed",
        }
    }
}

/// Group a bet by status first, horizon second.
///
/// A bet appears once: deferred, shipped, and killed bets always render under
/// their status group regardless of the stored horizon.
fn group_of(bet: &BetView) -> Group {
    match bet.status {
        BetStatus::Deferred => Group::Deferred,
        BetStatus::Shipped => Group::Shipped,
        BetStatus::Killed => Group::Killed,
        _ => match bet.horizon {
            Horizon::Now => Group::Now,
            Horizon::Next => Group::Next,
            Horizon::Later => Group::Later,
        },
    }
}

fn push_heading(out: &mut String, marker: &str, title: &str) {
    out.push_str(marker);
    out.push_str(title);
    out.push_str("\n\n");
}

fn push_text(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}

fn push_omitted(out: &mut String, omitted: Option<u32>) {
    if let Some(count) = omitted {
        push_text(out, &format!("({count} more omitted)"));
    }
}

fn omitted(view: &RoadmapView, section: &str) -> Option<u32> {
    view.truncation.omitted_counts.get(section).copied()
}

/// Collapse a multi-line value so it cannot break the surrounding list.
fn oneline(value: &str) -> String {
    value.replace(['\n', '\r'], " ")
}

/// Render a scalar field, falling back to `none` when absent or blank.
fn field(value: Option<&str>) -> String {
    match value {
        None => NONE.to_string(),
        Some(value) if value.trim().is_empty() => NONE.to_string(),
        Some(value) => oneline(value),
    }
}

fn date(value: NaiveDate) -> String {
    value.format("%Y-%m-%d").to_string()
}

fn opt_date(value: Option<NaiveDate>) -> String {
    value.map_or_else(|| NONE.to_string(), date)
}

fn bet_status(status: &BetStatus) -> &'static str {
    match status {
        BetStatus::Shaping => "shaping",
        BetStatus::Committed => "committed",
        BetStatus::Building => "building",
        BetStatus::Deferred => "deferred",
        BetStatus::Shipped => "shipped",
        BetStatus::Killed => "killed",
    }
}

fn horizon(horizon: &Horizon) -> &'static str {
    match horizon {
        Horizon::Now => "now",
        Horizon::Next => "next",
        Horizon::Later => "later",
    }
}

fn enforcement(tier: &Enforcement) -> &'static str {
    match tier {
        Enforcement::Automated => "automated",
        Enforcement::HumanReview => "human_review",
    }
}

fn staleness(value: Staleness) -> &'static str {
    match value {
        Staleness::Fresh => "fresh",
        Staleness::Stale => "stale",
        Staleness::Unknown => "unknown",
    }
}

fn warning_code(kind: &RoadmapWarningKind) -> &'static str {
    match kind {
        RoadmapWarningKind::BetAgeExceedsAppetite => "bet_age_exceeds_appetite",
        RoadmapWarningKind::CircuitBreakerDue => "circuit_breaker_due",
        RoadmapWarningKind::BetEpicMissing => "bet_epic_missing",
        RoadmapWarningKind::BetEpicClosedNotRetired => "bet_epic_closed_not_retired",
        RoadmapWarningKind::BetNoTransition => "bet_no_transition",
        RoadmapWarningKind::HorizonEpicMismatch => "horizon_epic_mismatch",
        RoadmapWarningKind::NoGoViolationCandidate => "no_go_violation_candidate",
        RoadmapWarningKind::IdentityReviewOverdue => "identity_review_overdue",
        RoadmapWarningKind::RoadmapReviewOverdue => "roadmap_review_overdue",
    }
}
