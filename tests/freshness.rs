//! Purpose: property tests for per-source freshness.
//!
//! Responsibilities:
//! - assert stale and fresh observations are classified per source horizon;
//! - assert freshness never becomes a project-wide field in the brief.
//!
//! Rationale: staleness is evidence about one source. A single project-wide
//! verdict would hide which source is lagging, so the serialized contract must
//! keep the judgement inside `sources[]` only.

use chrono::{DateTime, NaiveDate, Utc};

use podras::brief::{self, BriefInputs};
use podras::freshness;
use podras::model::{SourceName, SourceState, SourceStatus, Staleness};

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
