//! Purpose: render the brief as the fixed-section markdown narrative.
//!
//! Responsibilities:
//! - render the nine fixed sections in cold-return order;
//! - stamp per-source freshness on the final section;
//! - render an explicit `none` line for every empty section.
//!
//! Rationale: the narrative render is the human contract; keeping it a pure
//! function of the brief guarantees regenerated renders overwrite hand edits.
//! Focus and parked content is read straight from the brief, never inferred
//! here, so curation remains the only authority for intent.

use chrono::{DateTime, SecondsFormat, Utc};

use crate::error::Result;
use crate::model::{
    Brief, HealthBasis, HealthStatus, ParkedItem, RecentEvent, RoadmapView, RoadmapWarningKind,
    SignalKind, SourceName, SourceState, Staleness, WorkItem,
};

/// The explicit placeholder rendered for an empty section or absent value.
const NONE: &str = "none";

/// Render a brief as markdown.
///
/// `reentry` is the on-demand-only next-action line; committed renders pass
/// `None` so whisper content never reaches a generated artifact.
pub fn render(brief: &Brief, reentry: Option<&str>) -> Result<String> {
    let mut out = String::new();
    out.push_str("# Project status brief\n\n");

    render_purpose(&mut out, brief, reentry);
    render_work_items(&mut out, "Focus", &brief.focus, omitted(brief, "focus"));
    render_work_items(
        &mut out,
        "In flight",
        &brief.in_flight,
        omitted(brief, "in_flight"),
    );
    render_work_items(
        &mut out,
        "Blocked / waiting",
        &brief.blocked,
        omitted(brief, "blocked"),
    );
    render_work_items(
        &mut out,
        "Next / ready",
        &brief.next,
        omitted(brief, "next"),
    );
    render_parked(&mut out, brief);
    render_recent(&mut out, brief);
    render_health(&mut out, brief);
    render_sources(&mut out, brief);

    Ok(out)
}

fn render_purpose(out: &mut String, brief: &Brief, reentry: Option<&str>) {
    push_heading(out, "## ", "Purpose / current state");
    push_text(out, &field(brief.purpose_lite.as_deref()));
    if let Some(reentry) = reentry {
        push_text(out, &format!("Re-entry: {}", oneline(reentry)));
    }
    out.push('\n');
}

fn render_work_items(out: &mut String, title: &str, items: &[WorkItem], omitted: Option<u32>) {
    push_heading(out, "## ", title);
    if items.is_empty() {
        push_text(out, NONE);
    } else {
        for item in items {
            push_text(out, &work_item_line(item));
        }
        push_omitted(out, omitted);
    }
    out.push('\n');
}

fn render_parked(out: &mut String, brief: &Brief) {
    push_heading(out, "## ", "Parked");
    if brief.parked.is_empty() {
        push_text(out, NONE);
    } else {
        for item in &brief.parked {
            push_text(out, &parked_line(item));
        }
        push_omitted(out, omitted(brief, "parked"));
    }
    out.push('\n');

    push_heading(out, "### ", "Needs re-triage");
    if brief.needs_retriage.is_empty() {
        push_text(out, NONE);
    } else {
        for item in &brief.needs_retriage {
            push_text(out, &work_item_line(item));
        }
    }
    out.push('\n');
}

fn render_recent(out: &mut String, brief: &Brief) {
    push_heading(out, "## ", "Recent");
    if brief.recent.is_empty() {
        push_text(out, NONE);
    } else {
        for event in &brief.recent {
            push_text(out, &recent_line(event));
        }
        push_omitted(out, omitted(brief, "recent"));
    }
    out.push('\n');
}

fn render_health(out: &mut String, brief: &Brief) {
    let health = &brief.health;
    push_heading(out, "## ", "Health + confidence");
    push_text(
        out,
        &format!(
            "Health: **{}** — basis: {} — owner: {} — reason: {}",
            health_status(health.status),
            health_basis(health.basis),
            field(health.owner.as_deref()),
            field(health.reason.as_deref()),
        ),
    );
    for signal in &health.signals {
        push_text(
            out,
            &format!(
                "- signal: {} — {}",
                signal_kind(signal.kind),
                oneline(&signal.reason)
            ),
        );
    }
    // The lens's advisory no-go candidate surfaces once, deterministically.
    if let Some(roadmap) = &brief.roadmap {
        push_no_go_risk(out, roadmap);
    }
    out.push('\n');
}

/// The lens's advisory no-go candidate surfaces once, deterministically: the
/// first firing warning in the lens's code-then-subject-then-detail order. It
/// stays advisory and never becomes a signal or verdict.
fn push_no_go_risk(out: &mut String, roadmap: &RoadmapView) {
    let Some(warning) = roadmap
        .warnings
        .iter()
        .find(|warning| warning.code == RoadmapWarningKind::NoGoViolationCandidate)
    else {
        return;
    };
    push_text(
        out,
        &format!(
            "- no-go risk: {} — {}",
            oneline(&warning.subject),
            oneline(&warning.detail)
        ),
    );
}

fn render_sources(out: &mut String, brief: &Brief) {
    push_heading(out, "## ", "Sources & freshness");
    push_text(
        out,
        "| source | status | observed_at | staleness | detail |",
    );
    push_text(out, "| --- | --- | --- | --- | --- |");
    for row in &brief.sources {
        push_text(
            out,
            &format!(
                "| {} | {} | {} | {} | {} |",
                source_name(row.name),
                source_state(row.status),
                timestamp(row.observed_at),
                staleness(row.staleness),
                table_safe(row.detail.as_deref()),
            ),
        );
    }
    out.push('\n');
    push_text(
        out,
        &format!(
            "Curation: updated_at {}; confirmed_at {}.",
            timestamp(brief.curation.updated_at),
            timestamp(brief.curation.confirmed_at),
        ),
    );
    out.push('\n');
    push_text(
        out,
        &format!(
            "Freshness: generated_at {}; per-source observed_at and staleness are listed above.",
            timestamp(Some(brief.generated_at)),
        ),
    );
}

fn work_item_line(item: &WorkItem) -> String {
    let mut line = format!("- {}", oneline(&item.id));
    if let Some(title) = item.title.as_deref() {
        line.push_str(&format!(" — {}", oneline(title)));
    }

    let mut facts: Vec<String> = Vec::new();
    if let Some(status) = item.status.as_deref() {
        facts.push(format!("status: {}", oneline(status)));
    }
    if let Some(priority) = item.priority {
        facts.push(format!("priority: {priority}"));
    }
    if let Some(age) = item.age_days {
        facts.push(format!("age: {age}d"));
    }
    if !item.blocked_by.is_empty() {
        let blockers: Vec<String> = item.blocked_by.iter().map(|id| oneline(id)).collect();
        facts.push(format!("blocked by: {}", blockers.join(", ")));
    }
    if let Some(reason) = item.reason.as_deref() {
        facts.push(format!("reason: {}", oneline(reason)));
    }
    if let Some(confirmed_at) = item.confirmed_at {
        facts.push(format!("confirmed: {confirmed_at}"));
    }
    if !facts.is_empty() {
        line.push_str(&format!(" ({})", facts.join("; ")));
    }
    line
}

fn parked_line(item: &ParkedItem) -> String {
    format!(
        "- {} — {} (parked: {}; confirmed: {}; resurface: {})",
        oneline(&item.id),
        oneline(&item.reason),
        item.parked_at,
        item.confirmed_at,
        item.resurface,
    )
}

fn recent_line(event: &RecentEvent) -> String {
    let mut line = format!(
        "- {} {} ({})",
        oneline(&event.sha),
        oneline(&event.subject),
        timestamp(Some(event.committed_at)),
    );
    if let Some(outcome) = event.outcome.as_deref() {
        line.push_str(&format!(" — {}", oneline(outcome)));
    }
    line
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

fn omitted(brief: &Brief, section: &str) -> Option<u32> {
    brief.truncation.omitted_counts.get(section).copied()
}

fn timestamp(value: Option<DateTime<Utc>>) -> String {
    value.map_or_else(
        || NONE.to_string(),
        |value| value.to_rfc3339_opts(SecondsFormat::Secs, true),
    )
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

/// Render a scalar field as a markdown table cell, escaping only `|`.
fn table_safe(value: Option<&str>) -> String {
    field(value).replace('|', "\\|")
}

fn source_name(name: SourceName) -> &'static str {
    match name {
        SourceName::Beads => "beads",
        SourceName::Git => "git",
        SourceName::Openspec => "openspec",
        SourceName::Whisper => "whisper",
        SourceName::Curation => "curation",
    }
}

fn source_state(state: SourceState) -> &'static str {
    match state {
        SourceState::Present => "present",
        SourceState::Missing => "missing",
        SourceState::Unreadable => "unreadable",
    }
}

fn staleness(value: Staleness) -> &'static str {
    match value {
        Staleness::Fresh => "fresh",
        Staleness::Stale => "stale",
        Staleness::Unknown => "unknown",
    }
}

fn health_status(status: HealthStatus) -> &'static str {
    match status {
        HealthStatus::Green => "green",
        HealthStatus::Amber => "amber",
        HealthStatus::Red => "red",
        HealthStatus::Unknown => "unknown",
    }
}

fn health_basis(basis: HealthBasis) -> &'static str {
    match basis {
        HealthBasis::Human => "human",
        HealthBasis::Signals => "signals",
        HealthBasis::None => "none",
    }
}

fn signal_kind(kind: SignalKind) -> &'static str {
    match kind {
        SignalKind::StaleFocus => "stale focus",
        SignalKind::WipOverCap => "WIP over cap",
        SignalKind::StuckProject => "stuck project",
        SignalKind::FocusClosed => "focus closed",
        SignalKind::ParkedExpired => "parked expired",
        SignalKind::OrphanCommits => "orphan commits",
        SignalKind::SpecChangeWithoutBead => "spec change without bead",
    }
}
