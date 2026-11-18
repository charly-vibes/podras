//! Purpose: judge per-source freshness.
//!
//! Responsibilities:
//! - classify one source's observation instant as fresh, stale, or unknown;
//! - judge against the run's single evaluation date (`--as-of` when pinned,
//!   else the brief generation date);
//! - own the per-source freshness horizons and the mandatory-source guard.
//!
//! Rationale: staleness is per source, never project-wide. One declared table
//! keeps every caller from inventing its own horizon logic, and a stale or
//! missing mandatory source can never be painted over by a fresh human word.

use chrono::{DateTime, NaiveDate, Utc};

use crate::model::{SourceName, SourceState, SourceStatus, Staleness};

/// One declared per-source freshness horizon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Horizon {
    /// Declared source this horizon applies to.
    pub source: SourceName,
    /// Whole days the source reading may age before it counts as stale.
    pub days: i64,
}

/// The only declaration of per-source freshness horizons.
///
/// Beads and git are expected to move at least weekly; openspec, whisper, and
/// curation at least fortnightly. The envelope never serializes this table: it
/// keeps per-source `staleness` only.
pub const HORIZONS: [Horizon; 5] = [
    Horizon {
        source: SourceName::Beads,
        days: 7,
    },
    Horizon {
        source: SourceName::Git,
        days: 7,
    },
    Horizon {
        source: SourceName::Openspec,
        days: 14,
    },
    Horizon {
        source: SourceName::Whisper,
        days: 14,
    },
    Horizon {
        source: SourceName::Curation,
        days: 14,
    },
];

/// Classify one source reading against an explicit evaluation date.
///
/// Freshness is judged on calendar dates: the age in days is the evaluation
/// date minus the observation date. An observation exactly on the horizon is
/// not yet past it, so it stays fresh. `None` observed_at is always unknown.
///
/// `evaluation_date` is the run's single date. The brief read path computes it
/// once and threads it here, so sources observed minutes apart are judged
/// against the same day even when a run crosses midnight.
pub fn evaluate_at(
    source: SourceName,
    observed_at: Option<DateTime<Utc>>,
    evaluation_date: NaiveDate,
) -> Staleness {
    let Some(observed_at) = observed_at else {
        return Staleness::Unknown;
    };
    let age_days = (evaluation_date - observed_at.date_naive()).num_days();
    if age_days > horizon_days(source) {
        Staleness::Stale
    } else {
        Staleness::Fresh
    }
}

/// Classify one source reading against `--as-of`, else today's UTC date.
///
/// Thin wrapper over [`evaluate_at`] for callers that have no run-level date
/// yet. The brief read path uses [`evaluate_at`] directly with the date
/// computed once per run.
pub fn evaluate(
    source: SourceName,
    observed_at: Option<DateTime<Utc>>,
    as_of: Option<NaiveDate>,
) -> Staleness {
    evaluate_at(
        source,
        observed_at,
        as_of.unwrap_or_else(|| Utc::now().date_naive()),
    )
}

/// Mandatory sources that are missing, unreadable, or not fresh.
///
/// Beads and git are mandatory evidence: a fresh human green cannot stand
/// while one of them is unusable. Openspec and whisper are best-effort (they
/// report their own staleness and warn), and a missing curation row already
/// forces `unknown`, so neither vetoes a human verdict.
pub fn impaired_mandatory(sources: &[SourceStatus]) -> Vec<SourceName> {
    sources
        .iter()
        .filter(|row| matches!(row.name, SourceName::Beads | SourceName::Git))
        .filter(|row| row.status != SourceState::Present || row.staleness != Staleness::Fresh)
        .map(|row| row.name)
        .collect()
}

/// Days a source reading may age before it counts as stale.
fn horizon_days(source: SourceName) -> i64 {
    HORIZONS
        .iter()
        .find(|horizon| horizon.source == source)
        .map(|horizon| horizon.days)
        .expect("every declared source has exactly one horizon")
}
