//! Purpose: judge per-source freshness.
//!
//! Responsibilities:
//! - classify one source's observation instant as fresh, stale, or unknown;
//! - compare against the requested `--as-of` date when pinned, else today;
//! - own the per-source freshness horizons.
//!
//! Rationale: staleness is per source, never project-wide. A single small
//! classifier keeps every caller from inventing its own horizon logic.

use chrono::{DateTime, NaiveDate, Utc};

use crate::model::{SourceName, Staleness};

/// Beads export is expected to move at least weekly.
const BEADS_HORIZON_DAYS: i64 = 7;
/// Git history is expected to move at least weekly.
const GIT_HORIZON_DAYS: i64 = 7;
/// OpenSpec changes are expected to move at least fortnightly.
const OPENSPEC_HORIZON_DAYS: i64 = 14;
/// Whisper notes are expected to move at least fortnightly.
const WHISPER_HORIZON_DAYS: i64 = 14;
/// Human curation is expected to move at least fortnightly.
const CURATION_HORIZON_DAYS: i64 = 14;

/// Classify one source reading.
///
/// Freshness is judged on calendar dates: the age in days is the evaluation
/// date minus the observation date. An observation exactly on the horizon is
/// not yet past it, so it stays fresh. `None` observed_at is always unknown.
pub fn evaluate(
    source: SourceName,
    observed_at: Option<DateTime<Utc>>,
    as_of: Option<NaiveDate>,
) -> Staleness {
    let Some(observed_at) = observed_at else {
        return Staleness::Unknown;
    };
    let evaluation_date = as_of.unwrap_or_else(|| Utc::now().date_naive());
    let age_days = (evaluation_date - observed_at.date_naive()).num_days();
    if age_days > horizon_days(source) {
        Staleness::Stale
    } else {
        Staleness::Fresh
    }
}

/// Days a source reading may age before it counts as stale.
fn horizon_days(source: SourceName) -> i64 {
    match source {
        SourceName::Beads => BEADS_HORIZON_DAYS,
        SourceName::Git => GIT_HORIZON_DAYS,
        SourceName::Openspec => OPENSPEC_HORIZON_DAYS,
        SourceName::Whisper => WHISPER_HORIZON_DAYS,
        SourceName::Curation => CURATION_HORIZON_DAYS,
    }
}
