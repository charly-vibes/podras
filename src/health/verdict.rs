//! Purpose: layer a fresh human verdict over computed health signals.
//!
//! Responsibilities:
//! - accept only fresh, parseable human verdicts;
//! - warn on status values that are neither valid nor `unknown`;
//! - fall back to signals, then unknown, when no fresh human word exists;
//! - name every firing signal in the verdict reason.
//!
//! Rationale: this is the only place a green verdict can be produced, and it
//! always needs a fresh human word; a green human verdict with any firing
//! signal is downgraded to amber so silence never reads as green.

use chrono::NaiveDate;

use crate::curation::{CurationHealth, parse_human_date};
use crate::model::{Health, HealthBasis, HealthStatus, Signal, SignalKind, SourceName, Warning};

/// A human verdict older than this many days is treated as absent.
const HUMAN_VERDICT_HORIZON_DAYS: i64 = 14;

/// Layer a fresh human verdict over firing signals.
///
/// D8 order: a fresh human verdict stands (green is downgraded to amber when
/// any signal fires); otherwise firing signals make the project amber; with
/// neither the verdict is unknown.
pub(crate) fn layer(
    curation_health: Option<&CurationHealth>,
    evaluation_date: NaiveDate,
    signals: Vec<Signal>,
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
    fresh_human_verdict(health, status, signals, owner)
}

/// Build the verdict from a fresh human word, downgrading green under signals.
fn fresh_human_verdict(
    health: &CurationHealth,
    status: Option<HealthStatus>,
    signals: Vec<Signal>,
    owner: Option<String>,
) -> Health {
    match status {
        Some(HealthStatus::Green) if signals.is_empty() => Health {
            status: HealthStatus::Green,
            basis: HealthBasis::Human,
            signals,
            owner,
            reason: health.reason.clone(),
        },
        Some(HealthStatus::Green) => Health {
            status: HealthStatus::Amber,
            basis: HealthBasis::Human,
            reason: Some(format!(
                "human green downgraded by signals: {}",
                named_signals(&signals)
            )),
            signals,
            owner,
        },
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
