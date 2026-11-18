//! Purpose: layer a fresh human verdict over computed health signals.
//!
//! Responsibilities:
//! - accept only fresh, parseable human verdicts;
//! - warn on status values that are neither valid nor `unknown`;
//! - fall back to signals, then unknown, when no fresh human word exists;
//! - name every firing signal in the verdict reason;
//! - refuse green while a mandatory source is unusable.
//!
//! Rationale: this is the only place a green verdict can be produced, and it
//! always needs a fresh human word plus usable mandatory evidence; a green
//! human verdict with any firing signal or impaired mandatory source is
//! downgraded to amber so silence never reads as green.

use chrono::NaiveDate;

use crate::curation::{CurationHealth, parse_human_date};
use crate::model::{
    Health, HealthBasis, HealthStatus, Signal, SignalKind, SourceName, SourceState, SourceStatus,
    Staleness, Warning,
};

/// A human verdict older than this many days is treated as absent.
const HUMAN_VERDICT_HORIZON_DAYS: i64 = 14;

/// Layer a fresh human verdict over firing signals and impaired sources.
///
/// D8 order: a fresh human verdict stands (green is downgraded to amber when
/// any signal fires or a mandatory source is unusable); otherwise firing
/// signals make the project amber; with neither the verdict is unknown.
pub(crate) fn layer(
    curation_health: Option<&CurationHealth>,
    evaluation_date: NaiveDate,
    signals: Vec<Signal>,
    impaired_mandatory: &[SourceName],
    sources: &[SourceStatus],
    warnings: &mut Vec<Warning>,
) -> Health {
    let owner = curation_health.and_then(|health| health.owner.clone());
    let Some(health) = curation_health else {
        return signals_or_unknown(signals, owner);
    };
    let status = human_status(health, warnings);
    if !verdict_is_fresh(health, evaluation_date) {
        return signals_or_unknown(signals, owner);
    }
    fresh_human_verdict(health, status, signals, impaired_mandatory, sources, owner)
}

/// Build the verdict from a fresh human word, downgrading green under signals
/// or impaired mandatory evidence.
fn fresh_human_verdict(
    health: &CurationHealth,
    status: Option<HealthStatus>,
    signals: Vec<Signal>,
    impaired_mandatory: &[SourceName],
    sources: &[SourceStatus],
    owner: Option<String>,
) -> Health {
    match status {
        Some(HealthStatus::Green) => {
            green_verdict(health, signals, impaired_mandatory, sources, owner)
        }
        Some(human_status @ (HealthStatus::Amber | HealthStatus::Red)) => Health {
            status: human_status,
            basis: HealthBasis::Human,
            signals,
            owner,
            reason: health.reason.clone(),
        },
        _ => signals_or_unknown(signals, owner),
    }
}

/// A green human word stands only while no signal fires and every mandatory
/// source is usable; otherwise it is downgraded to amber with the cause named.
fn green_verdict(
    health: &CurationHealth,
    signals: Vec<Signal>,
    impaired_mandatory: &[SourceName],
    sources: &[SourceStatus],
    owner: Option<String>,
) -> Health {
    if signals.is_empty() && impaired_mandatory.is_empty() {
        return Health {
            status: HealthStatus::Green,
            basis: HealthBasis::Human,
            signals,
            owner,
            reason: health.reason.clone(),
        };
    }
    let reason = if signals.is_empty() {
        impaired_reason(impaired_mandatory, sources)
    } else {
        format!(
            "human green downgraded by signals: {}",
            named_signals(&signals)
        )
    };
    Health {
        status: HealthStatus::Amber,
        basis: HealthBasis::Human,
        reason: Some(reason),
        signals,
        owner,
    }
}

/// Parse the human status, warning on a value that is neither valid nor `unknown`.
fn human_status(health: &CurationHealth, warnings: &mut Vec<Warning>) -> Option<HealthStatus> {
    let status = parse_health_status(&health.status);
    if status.is_none() && !health.status.eq_ignore_ascii_case("unknown") {
        warnings.push(Warning {
            source: SourceName::Curation,
            code: "curation_invalid_health".to_string(),
            detail: format!(
                "health status `{}` is not green, amber, red, or unknown",
                health.status
            ),
        });
    }
    status
}

/// True when the human verdict was confirmed inside its freshness horizon.
fn verdict_is_fresh(health: &CurationHealth, evaluation_date: NaiveDate) -> bool {
    health
        .confirmed_at
        .as_deref()
        .and_then(parse_human_date)
        .is_some_and(|date| (evaluation_date - date).num_days() <= HUMAN_VERDICT_HORIZON_DAYS)
}

/// Verdict when no fresh human word exists: signals amber, otherwise unknown.
fn signals_or_unknown(signals: Vec<Signal>, owner: Option<String>) -> Health {
    if signals.is_empty() {
        return Health {
            status: HealthStatus::Unknown,
            basis: HealthBasis::None,
            signals,
            owner,
            reason: None,
        };
    }
    Health {
        status: HealthStatus::Amber,
        basis: HealthBasis::Signals,
        reason: Some(named_signals(&signals)),
        signals,
        owner,
    }
}

/// Name every impaired mandatory source with its state for a verdict reason.
fn impaired_reason(impaired: &[SourceName], sources: &[SourceStatus]) -> String {
    let named = impaired
        .iter()
        .map(|name| {
            let state = sources
                .iter()
                .find(|row| row.name == *name)
                .map_or("unknown", source_state);
            format!("{} ({state})", source_label(*name))
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("human green downgraded by unusable mandatory sources: {named}")
}

/// The evidence state of one source row, as verdict text.
fn source_state(row: &SourceStatus) -> &'static str {
    match (row.status, row.staleness) {
        (SourceState::Missing, _) => "missing",
        (SourceState::Unreadable, _) => "unreadable",
        (SourceState::Present, Staleness::Stale) => "stale",
        (SourceState::Present, Staleness::Unknown) => "unknown",
        (SourceState::Present, Staleness::Fresh) => "fresh",
    }
}

fn source_label(name: SourceName) -> &'static str {
    match name {
        SourceName::Beads => "beads",
        SourceName::Git => "git",
        SourceName::Openspec => "openspec",
        SourceName::Whisper => "whisper",
        SourceName::Curation => "curation",
    }
}

/// Name every firing signal, with its evidence, for a verdict reason.
fn named_signals(signals: &[Signal]) -> String {
    signals
        .iter()
        .map(|signal| format!("{}: {}", kind_name(signal.kind), signal.reason))
        .collect::<Vec<_>>()
        .join("; ")
}

fn kind_name(kind: SignalKind) -> &'static str {
    match kind {
        SignalKind::StaleFocus => "stale_focus",
        SignalKind::WipOverCap => "wip_over_cap",
        SignalKind::StuckProject => "stuck_project",
        SignalKind::FocusClosed => "focus_closed",
        SignalKind::ParkedExpired => "parked_expired",
        SignalKind::OrphanCommits => "orphan_commits",
        SignalKind::SpecChangeWithoutBead => "spec_change_without_bead",
    }
}

fn parse_health_status(raw: &str) -> Option<HealthStatus> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "green" => Some(HealthStatus::Green),
        "amber" => Some(HealthStatus::Amber),
        "red" => Some(HealthStatus::Red),
        "unknown" => Some(HealthStatus::Unknown),
        _ => None,
    }
}
