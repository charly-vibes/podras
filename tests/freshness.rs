//! Purpose: property tests for per-source freshness.
//!
//! Responsibilities:
//! - assert stale and fresh observations are classified per source horizon;
//! - assert freshness never becomes a project-wide field in the brief.
//!
//! Rationale: staleness is evidence about one source. A single project-wide
//! verdict would hide which source is lagging, so the serialized contract must
//! keep the judgement inside `sources[]` only.

use std::path::PathBuf;

use chrono::{DateTime, NaiveDate, Utc};

use podras::brief::{self, BriefInputs, CurationState};
use podras::curation::{CurationFile, CurationHealth};
use podras::freshness;
use podras::model::{HealthBasis, HealthStatus, SourceName, SourceState, SourceStatus, Staleness};
use podras::readers::ReadContext;

/// Pinned evaluation date for every freshness fixture.
const AS_OF: &str = "2026-11-18";

fn date(raw: &str) -> NaiveDate {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").expect("fixture date is valid")
}

fn instant(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .expect("fixture instant is valid")
        .with_timezone(&Utc)
}

fn inputs() -> BriefInputs {
    BriefInputs {
        beads: None,
        git: None,
        specs: None,
        whisper: None,
        curation: None,
        sources: Vec::new(),
        warnings: Vec::new(),
        as_of: Some(date(AS_OF)),
        now: date(AS_OF),
        generated_at: instant("2026-11-18T12:00:00Z"),
    }
}

fn source_row(name: SourceName, observed_at: DateTime<Utc>, staleness: Staleness) -> SourceStatus {
    SourceStatus {
        name,
        status: SourceState::Present,
        observed_at: Some(observed_at),
        staleness,
        detail: None,
    }
}

#[test]
fn stale_source_flagged() {
    let as_of = date(AS_OF);
    let stale_at = instant("2026-09-01T00:00:00Z");
    let fresh_at = instant("2026-11-17T00:00:00Z");

    let stale = freshness::evaluate(SourceName::Beads, Some(stale_at), Some(as_of));
    let fresh = freshness::evaluate(SourceName::Git, Some(fresh_at), Some(as_of));
    assert_eq!(stale, Staleness::Stale, "an old beads reading is stale");
    assert_eq!(fresh, Staleness::Fresh, "a yesterday git reading is fresh");

    let mut fixture = inputs();
    fixture.sources = vec![
        source_row(SourceName::Beads, stale_at, stale),
        source_row(SourceName::Git, fresh_at, fresh),
    ];
    let brief = brief::assemble(&fixture);

    assert_eq!(brief.sources[0].staleness, Staleness::Stale);
    assert_eq!(brief.sources[1].staleness, Staleness::Fresh);

    let value = serde_json::to_value(&brief).expect("brief serializes");
    assert!(
        value.get("staleness").is_none(),
        "the brief has no top-level staleness field"
    );
    assert!(
        value["curation"].get("staleness").is_none(),
        "curation carries timestamps, never a staleness verdict"
    );
    assert_eq!(value["sources"][0]["staleness"], "stale");
    assert_eq!(value["sources"][1]["staleness"], "fresh");
    assert!(
        value["sources"]
            .as_array()
            .expect("sources is an array")
            .iter()
            .all(|source| source.get("staleness").is_some()),
        "every source row owns its own staleness"
    );
}

#[test]
fn horizons_differ_by_source() {
    let as_of = date(AS_OF);
    let ten_days_old = instant("2026-11-08T00:00:00Z");

    assert_eq!(
        freshness::evaluate(SourceName::Beads, Some(ten_days_old), Some(as_of)),
        Staleness::Stale,
        "beads horizon is seven days"
    );
    assert_eq!(
        freshness::evaluate(SourceName::Git, Some(ten_days_old), Some(as_of)),
        Staleness::Stale,
        "git horizon is seven days"
    );
    for source in [
        SourceName::Openspec,
        SourceName::Whisper,
        SourceName::Curation,
    ] {
        assert_eq!(
            freshness::evaluate(source, Some(ten_days_old), Some(as_of)),
            Staleness::Fresh,
            "{source:?} horizon is fourteen days"
        );
    }

    let seven_days_old = instant("2026-11-11T00:00:00Z");
    assert_eq!(
        freshness::evaluate(SourceName::Beads, Some(seven_days_old), Some(as_of)),
        Staleness::Fresh,
        "exactly at the horizon is not past it"
    );
    let fourteen_days_old = instant("2026-11-04T00:00:00Z");
    assert_eq!(
        freshness::evaluate(SourceName::Openspec, Some(fourteen_days_old), Some(as_of)),
        Staleness::Fresh,
        "exactly at the fourteen-day horizon is not past it"
    );
    let fifteen_days_old = instant("2026-11-03T00:00:00Z");
    assert_eq!(
        freshness::evaluate(SourceName::Openspec, Some(fifteen_days_old), Some(as_of)),
        Staleness::Stale,
        "past the fourteen-day horizon is stale"
    );

    assert_eq!(
        freshness::evaluate(SourceName::Whisper, None, Some(as_of)),
        Staleness::Unknown,
        "an unobserved source is unknown, never fresh"
    );

    let ancient = instant("2000-01-01T00:00:00Z");
    assert_eq!(
        freshness::evaluate(SourceName::Curation, Some(ancient), None),
        Staleness::Stale,
        "without --as-of the wall clock is the evaluation instant"
    );
}

#[test]
fn midnight_observations_use_one_evaluation_date() {
    let context = ReadContext {
        repo_root: PathBuf::from("."),
        as_of: None,
        branch: None,
    };
    let generated_at = instant("2026-11-18T00:00:01Z");
    let evaluation_date = context.evaluation_date(generated_at.date_naive());

    // Two sources observed seconds apart, on either side of midnight. A
    // wall-clock read per source could roll the second row onto the next date
    // mid-run; one threaded date keeps both on the same run day.
    let before_midnight = instant("2026-11-10T23:59:59Z");
    let after_midnight = instant("2026-11-11T00:00:01Z");
    assert_eq!(
        freshness::evaluate_at(SourceName::Beads, Some(before_midnight), evaluation_date),
        Staleness::Stale,
        "eight days before the run date is past the seven-day horizon"
    );
    assert_eq!(
        freshness::evaluate_at(SourceName::Git, Some(after_midnight), evaluation_date),
        Staleness::Fresh,
        "seven days before the run date is exactly at the horizon"
    );

    // `--as-of` pins the run date regardless of when the brief is generated.
    let pinned = ReadContext {
        repo_root: PathBuf::from("."),
        as_of: Some(evaluation_date),
        branch: None,
    };
    assert_eq!(
        pinned.evaluation_date(instant("2026-12-25T00:00:00Z").date_naive()),
        evaluation_date,
        "--as-of wins over the generation date"
    );
}

/// Every declared source and the horizon the module documentation promises.
const DOCUMENTED_HORIZONS: [(SourceName, i64); 5] = [
    (SourceName::Beads, 7),
    (SourceName::Git, 7),
    (SourceName::Openspec, 14),
    (SourceName::Whisper, 14),
    (SourceName::Curation, 14),
];

fn impaired_row(name: SourceName, status: SourceState, staleness: Staleness) -> SourceStatus {
    SourceStatus {
        name,
        status,
        observed_at: None,
        staleness,
        detail: None,
    }
}

/// The number of days the module documentation promises for `source`.
fn documented_days(source: SourceName) -> i64 {
    DOCUMENTED_HORIZONS
        .iter()
        .find_map(|(name, days)| (*name == source).then_some(*days))
        .expect("every declared source documents its horizon")
}

/// The horizon table declares every source exactly once, with the documented days.
fn assert_horizon_table() {
    assert_eq!(
        freshness::HORIZONS.len(),
        SourceName::ALL.len(),
        "the table declares exactly one row per declared source"
    );
    for source in SourceName::ALL {
        let rows: Vec<_> = freshness::HORIZONS
            .iter()
            .filter(|horizon| horizon.source == source)
            .collect();
        assert_eq!(rows.len(), 1, "{source:?} is declared exactly once");
        assert_eq!(
            rows[0].days,
            documented_days(source),
            "{source:?} horizon matches its docs"
        );
    }
}

/// Each probed horizon is inclusive at its boundary and stale one day past it.
fn assert_horizon_boundaries(as_of: NaiveDate) {
    assert_eq!(
        freshness::evaluate_at(
            SourceName::Beads,
            Some(instant("2026-11-11T00:00:00Z")),
            as_of
        ),
        Staleness::Fresh,
        "beads exactly on its seven-day horizon is fresh"
    );
    assert_eq!(
        freshness::evaluate_at(
            SourceName::Beads,
            Some(instant("2026-11-10T00:00:00Z")),
            as_of
        ),
        Staleness::Stale,
        "beads one day past its seven-day horizon is stale"
    );
    assert_eq!(
        freshness::evaluate_at(
            SourceName::Openspec,
            Some(instant("2026-11-04T00:00:00Z")),
            as_of
        ),
        Staleness::Fresh,
        "openspec exactly on its fourteen-day horizon is fresh"
    );
    assert_eq!(
        freshness::evaluate_at(
            SourceName::Openspec,
            Some(instant("2026-11-03T00:00:00Z")),
            as_of
        ),
        Staleness::Stale,
        "openspec one day past its fourteen-day horizon is stale"
    );
    assert_eq!(
        freshness::evaluate_at(SourceName::Git, None, as_of),
        Staleness::Unknown,
        "an unobserved source stays unknown"
    );
}

/// A fresh human green whose mandatory beads row is stale.
fn human_green_over_stale_beads(updated_at: DateTime<Utc>) -> BriefInputs {
    let mut fixture = inputs();
    fixture.curation = Some(CurationState {
        curation: CurationFile {
            health: Some(CurationHealth {
                status: "green".to_string(),
                owner: Some("@sasha".to_string()),
                confirmed_at: Some(AS_OF.to_string()),
                reason: Some("fresh human green".to_string()),
            }),
            ..CurationFile::default()
        },
        updated_at: Some(updated_at),
    });
    fixture.sources = vec![
        impaired_row(SourceName::Beads, SourceState::Present, Staleness::Stale),
        impaired_row(SourceName::Git, SourceState::Present, Staleness::Fresh),
    ];
    fixture
}

#[test]
fn freshness_thresholds_declared() {
    assert_horizon_table();
    assert_horizon_boundaries(date(AS_OF));

    // Fault injection: a fresh human green cannot paint over a stale
    // mandatory source.
    let brief = brief::assemble(&human_green_over_stale_beads(instant(
        "2026-11-04T00:00:00Z",
    )));
    assert_eq!(
        brief.health.status,
        HealthStatus::Amber,
        "a stale mandatory source blocks green: {:?}",
        brief.health
    );
    assert_eq!(brief.health.basis, HealthBasis::Human);
    assert_eq!(
        brief.health.reason.as_deref(),
        Some("human green downgraded by unusable mandatory sources: beads (stale)")
    );
}
