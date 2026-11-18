//! Purpose: prove the `.podras/curation.toml` shape survives a write/read cycle.
//!
//! Responsibilities:
//! - `curation_file_shape`: the deployed property contract for round-tripping;
//! - reject unknown keys, top-level and inside entries, with a `curation_parse_error`;
//! - absent file loads as `None`, save creates `.podras/` atomically;
//! - helper stamps: file mtime and the newest human confirmation date;
//! - RFC 3339 timestamps load and survive assembly into the brief.
//!
//! Rationale: curation is the single source of intent, so a lost field, a
//! reordered entry, or a silently ignored key would change what the brief says
//! the human decided.

mod common;
#[path = "curation_reader/fixtures.rs"]
mod fixtures;

use chrono::{DateTime, Utc};
use fixtures::{assemble_fixture, date, rfc3339_curation, sample};
use podras::curation;
use podras::error::Error;
use podras::model::{Brief, HealthBasis, HealthStatus, Resurface};

#[test]
fn curation_file_shape() {
    let repo = common::TempRepo::new();
    let curation = sample();

    curation::save(repo.root(), &curation).expect("save curation");
    let loaded = curation::load(repo.root())
        .expect("load curation")
        .expect("curation file is present after save");

    assert_eq!(
        curation, loaded,
        "parsed curation must equal the written file"
    );

    assert_eq!(
        loaded
            .focus
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        vec!["podras-d2e", "podras-i45"],
        "focus ordering must survive the round trip"
    );
    assert_eq!(
        loaded
            .parked
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        vec!["podras-o47", "podras-7jd"],
        "parked ordering must survive the round trip"
    );
    assert_eq!(loaded.max_focus, Some(2), "max_focus must be preserved");
    assert_eq!(
        loaded.publish_whisper,
        Some(true),
        "publish_whisper must be preserved"
    );
    assert_eq!(
        loaded.parked[0]
            .resurface
            .parse::<Resurface>()
            .expect("event trigger parses"),
        Resurface::Event("value-prop-roadmap".to_string())
    );
    assert_eq!(
        loaded.parked[1]
            .resurface
            .parse::<Resurface>()
            .expect("date trigger parses"),
        Resurface::Date(date(2026, 12, 1))
    );
    assert_eq!(
        loaded.focus[0]
            .focused_since
            .as_deref()
            .and_then(curation::parse_human_date),
        Some(date(2026, 11, 17)),
        "RFC3339 timestamps must be accepted where dates are"
    );

    let raw = repo.read(curation::CURATION_REL_PATH);
    assert!(
        raw.contains("purpose_lite = \"podras: status read path for a cold return\""),
        "serialized file must carry purpose_lite:\n{raw}"
    );
    assert!(
        raw.contains("resurface = \"event:value-prop-roadmap\""),
        "serialized file must carry event resurface triggers:\n{raw}"
    );
    assert!(
        raw.contains("resurface = \"2026-12-01\""),
        "serialized file must carry date resurface triggers:\n{raw}"
    );
}

#[test]
fn absent_file_loads_none() {
    let repo = common::TempRepo::new();
    assert_eq!(
        curation::load(repo.root()).expect("absent curation is not an error"),
        None
    );
    assert_eq!(curation::updated_at(repo.root()), None);
}

#[test]
fn unknown_key_is_a_parse_error() {
    let repo = common::TempRepo::new();
    repo.curation("purpose_lite = \"podras\"\nbogus_key = true\n");

    let error = curation::load(repo.root()).expect_err("unknown keys must be rejected");
    match error {
        Error::Domain {
            code,
            message,
            suggestion,
        } => {
            assert_eq!(code, "curation_parse_error");
            assert!(
                message.contains("bogus_key"),
                "detail must name the offending key: {message}"
            );
            assert!(suggestion.is_some(), "operators need a remediation hint");
        }
        other => panic!("expected curation_parse_error, got {other:?}"),
    }
}

#[test]
fn malformed_toml_is_a_parse_error() {
    let repo = common::TempRepo::new();
    repo.curation("purpose_lite = [\n");

    let error = curation::load(repo.root()).expect_err("malformed TOML must be rejected");
    match error {
        Error::Domain {
            code, suggestion, ..
        } => {
            assert_eq!(code, "curation_parse_error");
            assert_eq!(
                suggestion.as_deref(),
                Some(
                    "fix the TOML, or delete .podras/curation.toml and run podras init to start \
                     from the template"
                ),
                "the hint must name both the manual and the scaffold route"
            );
        }
        other => panic!("expected curation_parse_error, got {other:?}"),
    }
}

#[test]
fn parked_row_without_resurface_is_excluded_not_fatal() {
    let repo = common::TempRepo::new();
    repo.curation(
        r#"purpose_lite = "podras: partial park"

[health]
status = "green"
owner = "@sasha"
confirmed_at = "2026-11-18"

[[focus]]
id = "podras-f"
reason = "still focused"
focused_since = "2026-11-17"
confirmed_at = "2026-11-17"

[[parked]]
id = "podras-ok"
reason = "valid park"
resurface = "event:release"
parked_at = "2026-11-17"
confirmed_at = "2026-11-17"

[[parked]]
id = "podras-broken"
reason = "missing trigger"
parked_at = "2026-11-17"
confirmed_at = "2026-11-17"

[[parked]]
id = "podras-empty"
reason = ""
resurface = "event:release"
parked_at = "2026-11-17"
confirmed_at = "2026-11-17"
"#,
    );

    let brief = assemble_fixture(&repo);

    let parked: Vec<&str> = brief.parked.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(
        parked,
        vec!["podras-ok"],
        "only the entry with a resurface trigger and a reason stays parked: {parked:?}"
    );
    assert_invalid_parked_rows_are_reported(&brief);
}

fn assert_invalid_parked_rows_are_reported(brief: &Brief) {
    for broken in ["podras-broken", "podras-empty"] {
        assert!(
            !brief.needs_retriage.iter().any(|item| item.id == broken),
            "an invalid entry never becomes expired: {:?}",
            brief.needs_retriage
        );
    }
    assert!(
        brief
            .warnings
            .iter()
            .any(|warning| warning.code == "curation_invalid_resurface"),
        "the entry without a trigger is reported: {:?}",
        brief.warnings
    );
    assert!(
        brief
            .warnings
            .iter()
            .any(|warning| warning.code == "curation_invalid_entry"),
        "the entry without a reason is reported: {:?}",
        brief.warnings
    );
    assert_eq!(
        brief
            .focus
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        vec!["podras-f"],
        "a sibling parked entry cannot erase focus"
    );
    assert_eq!(brief.health.status, HealthStatus::Green);
    assert_eq!(brief.health.basis, HealthBasis::Human);
    assert_eq!(
        brief.purpose_lite.as_deref(),
        Some("podras: partial park"),
        "purpose survives the invalid parked row"
    );
}

#[test]
fn save_creates_podras_and_leaves_no_temp_file() {
    let repo = common::TempRepo::new();
    let dir = repo.root().join(".podras");
    assert!(!dir.exists(), "fixture starts without .podras/");

    curation::save(repo.root(), &sample()).expect("save curation");

    assert!(dir.is_dir(), "save must create .podras/");
    let names = std::fs::read_dir(&dir)
        .expect("read .podras/")
        .map(|entry| entry.expect("dir entry").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec!["curation.toml".to_string()],
        "atomic write must leave only curation.toml"
    );
}

#[test]
fn unknown_key_inside_a_parked_entry_is_a_parse_error() {
    let repo = common::TempRepo::new();
    repo.curation(
        r#"[[parked]]
id = "podras-o47"
reason = "harvest is v1.1"
resurface = "event:value-prop-roadmap"
parked_at = "2026-11-18"
confirmd_at = "2026-11-18"
"#,
    );

    let error = curation::load(repo.root()).expect_err("unknown entry keys must be rejected");
    match error {
        Error::Domain {
            code,
            message,
            suggestion,
        } => {
            assert_eq!(code, "curation_parse_error");
            assert!(
                message.contains("confirmd_at"),
                "detail must name the offending entry key: {message}"
            );
            assert!(suggestion.is_some(), "operators need a remediation hint");
        }
        other => panic!("expected curation_parse_error, got {other:?}"),
    }
}

#[test]
fn rfc3339_curation_timestamps_survive_assemble() {
    let repo = common::TempRepo::new();
    rfc3339_curation(&repo);

    let brief = assemble_fixture(&repo);

    assert_eq!(brief.parked.len(), 1, "RFC 3339 park dates must resolve");
    assert_eq!(brief.parked[0].id, "podras-o47");
    assert_eq!(
        brief.parked[0].parked_at,
        date(2026, 11, 17),
        "the offset's local date is kept"
    );
    assert_eq!(
        brief.parked[0].confirmed_at,
        date(2026, 11, 18),
        "RFC 3339 confirmed_at must resolve"
    );
    assert_eq!(
        brief.curation.confirmed_at.map(|stamp| stamp.date_naive()),
        Some(date(2026, 11, 18)),
        "the brief stamp must see RFC 3339 health and park confirmations"
    );
    assert_eq!(brief.health.basis, HealthBasis::Human);
    assert_eq!(brief.health.status, HealthStatus::Green);
}

#[test]
fn helper_stamps_mtime_and_newest_confirmation() {
    let repo = common::TempRepo::new();
    curation::save(repo.root(), &sample()).expect("save curation");

    let loaded = curation::load(repo.root())
        .expect("load curation")
        .expect("curation present");
    assert_eq!(
        curation::confirmed_at(&loaded),
        Some(date(2026, 11, 18)),
        "newest confirmation wins across health, focus, and parked"
    );

    let stamp: DateTime<Utc> = curation::updated_at(repo.root()).expect("file mtime");
    assert!(stamp.timestamp() > 0, "mtime must be a real UTC instant");
}
