//! Purpose: integration tests for `podras park` and the expired-park path.
//!
//! Responsibilities:
//! - `park_expired_surfaces`: the deployed property contract for re-triage;
//! - assert `--reason` and `--resurface` are mandatory at the CLI boundary;
//! - assert an empty reason and a malformed trigger are domain errors;
//! - assert parking the focused item removes focus with a notice;
//! - assert event triggers, dates, and upsert semantics round-trip;
//! - assert a rejected trigger never rewrites the curation file.
//!
//! Rationale: parked state is explicit non-commitment. It is only trustworthy
//! if the reason and trigger are mandatory, the file is never half-written,
//! and an expired trigger returns the item to attention instead of hiding it.

mod common;

use assert_cmd::Command;
use chrono::Utc;
use podras::brief::assemble;
use podras::curation;
use podras::model::Resurface;
use podras::readers::{self, ReadContext};

/// Run `podras` inside the fixture repository.
fn podras_in(repo: &common::TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(args)
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .assert()
}

/// Collect every source in the fixture and assemble the brief.
fn brief_of(repo: &common::TempRepo) -> podras::model::Brief {
    let context = ReadContext {
        repo_root: repo.root().to_path_buf(),
        as_of: None,
        branch: None,
    };
    let inputs = readers::collect(&context).expect("collect sources for the brief");
    assemble(&inputs)
}

/// Curation file with one entry parked until a date that has already passed.
const EXPIRED_CURATION: &str = r#"purpose_lite = "podras: fixture repository"
max_focus = 1

[[parked]]
id = "podras-a"
reason = "experiment that ran out of time"
resurface = "2020-01-01"
parked_at = "2019-12-01"
confirmed_at = "2019-12-01"
"#;

/// Curation file whose focus is the item the tests park.
const FOCUSED_CURATION: &str = r#"purpose_lite = "podras: fixture repository"
max_focus = 1

[[focus]]
id = "podras-a"
reason = "current work"
focused_since = "2026-11-01"
confirmed_at = "2026-11-01"
"#;

fn ids(items: &[podras::model::WorkItem]) -> Vec<&str> {
    items.iter().map(|item| item.id.as_str()).collect()
}

/// Run a `podras park` invocation clap must reject and return its stderr.
fn clap_stderr(repo: &common::TempRepo, args: &[&str]) -> String {
    let output = podras_in(repo, args).code(2).get_output().stderr.clone();
    String::from_utf8(output).expect("clap errors are UTF-8")
}

fn today() -> String {
    Utc::now().date_naive().format("%Y-%m-%d").to_string()
}

#[test]
fn park_expired_surfaces() {
    let repo = common::TempRepo::new();
    repo.beads_jsonl(&[
        ("podras-a", "Expired park", "open"),
        ("podras-b", "Still active", "open"),
    ]);
    repo.curation(EXPIRED_CURATION);

    let brief = brief_of(&repo);

    assert!(
        ids(&brief.needs_retriage).contains(&"podras-a"),
        "an expired trigger moves the item to needs_retriage: {:?}",
        brief.needs_retriage
    );
    assert!(
        !brief.parked.iter().any(|item| item.id == "podras-a"),
        "an expired item is no longer parked: {:?}",
        brief.parked
    );
    assert!(
        !ids(&brief.next).contains(&"podras-a"),
        "a parked item never re-enters next: {:?}",
        brief.next
    );
    assert!(
        ids(&brief.next).contains(&"podras-b"),
        "unparked tracker work still renders in next: {:?}",
        brief.next
    );
}

#[test]
fn park_requires_reason_and_resurface() {
    let repo = common::TempRepo::new();

    let missing_reason = clap_stderr(&repo, &["park", "podras-a", "--resurface", "2026-12-01"]);
    assert!(
        missing_reason.contains("--reason <REASON>"),
        "clap must name the missing flag: {missing_reason}"
    );
    assert!(
        missing_reason.contains("required arguments"),
        "clap must report a missing required argument: {missing_reason}"
    );

    let missing_resurface = clap_stderr(&repo, &["park", "podras-a", "--reason", "later"]);
    assert!(
        missing_resurface.contains("--resurface <RESURFACE>"),
        "clap must name the missing flag: {missing_resurface}"
    );
    assert!(
        missing_resurface.contains("required arguments"),
        "clap must report a missing required argument: {missing_resurface}"
    );
}

#[test]
fn park_rejects_empty_reason() {
    let repo = common::TempRepo::new();

    podras_in(
        &repo,
        &[
            "park",
            "podras-a",
            "--reason",
            "  ",
            "--resurface",
            "2026-12-01",
        ],
    )
    .code(1)
    .stderr(predicates::str::contains("park_reason_required"));
    assert!(
        !repo.root().join(".podras/curation.toml").exists(),
        "a rejected park must not scaffold curation"
    );
}

#[test]
fn parking_focused_item_removes_focus() {
    let repo = common::TempRepo::new();
    repo.curation(FOCUSED_CURATION);

    let output = podras_in(
        &repo,
        &[
            "park",
            "podras-a",
            "--reason",
            "handed off",
            "--resurface",
            "2026-12-01",
        ],
    )
    .success()
    .get_output()
    .stdout
    .clone();
    let stdout = String::from_utf8(output).expect("park notice is UTF-8");
    assert!(
        stdout.contains("unfocused podras-a"),
        "parking the focus prints a notice: {stdout}"
    );
    assert!(
        stdout.contains("parked podras-a until 2026-12-01"),
        "park confirms id and trigger: {stdout}"
    );

    let loaded = curation::load(repo.root())
        .expect("load curation")
        .expect("curation is present after park");
    assert!(
        loaded.focus.is_empty(),
        "a parked item must not stay focused: {:?}",
        loaded.focus
    );
    assert_eq!(loaded.parked.len(), 1, "one parked entry is written");
    assert_eq!(loaded.parked[0].id, "podras-a");
    assert_eq!(loaded.parked[0].reason, "handed off");
}

#[test]
fn park_event_trigger_round_trips() {
    let repo = common::TempRepo::new();

    podras_in(
        &repo,
        &[
            "park",
            "podras-a",
            "--reason",
            "waits for the roadmap",
            "--resurface",
            "event:value-prop-roadmap",
        ],
    )
    .success();

    let loaded = curation::load(repo.root())
        .expect("load curation")
        .expect("park scaffolds curation when absent");
    assert_eq!(loaded.parked.len(), 1, "one parked entry is written");
    let entry = &loaded.parked[0];
    assert_eq!(entry.id, "podras-a");
    assert_eq!(
        entry.resurface(),
        Ok(Resurface::Event("value-prop-roadmap".to_string())),
        "the event trigger round-trips"
    );
    assert_eq!(
        entry.parked_at.as_deref(),
        Some(today().as_str()),
        "parked_at is today"
    );
    assert_eq!(
        entry.confirmed_at.as_deref(),
        Some(today().as_str()),
        "confirmed_at is today"
    );
}

#[test]
fn park_rejects_malformed_trigger_without_rewriting() {
    let repo = common::TempRepo::new();
    repo.curation(FOCUSED_CURATION);
    let before = repo.read(".podras/curation.toml");

    podras_in(
        &repo,
        &[
            "park",
            "podras-a",
            "--reason",
            "later",
            "--resurface",
            "2026-13-45",
        ],
    )
    .code(1)
    .stderr(predicates::str::contains("park_invalid_resurface"))
    .stderr(predicates::str::contains("YYYY-MM-DD"));

    assert_eq!(
        repo.read(".podras/curation.toml"),
        before,
        "a rejected trigger leaves the curation bytes untouched"
    );
}

#[test]
fn park_rejects_unresolved_ids_without_rewriting() {
    let repo = common::TempRepo::new();
    repo.beads_jsonl(&[
        ("podras-a", "open work", "open"),
        ("podras-b", "closed work", "closed"),
    ]);
    repo.curation(FOCUSED_CURATION);

    for id in ["podras-zzz", "podras-b"] {
        podras_in(
            &repo,
            &["park", id, "--reason", "later", "--resurface", "2026-12-01"],
        )
        .code(1)
        .stderr(predicates::str::contains("park_unresolved"))
        .stderr(predicates::str::contains("run bd list"));

        assert_eq!(
            repo.read(".podras/curation.toml"),
            FOCUSED_CURATION,
            "parking `{id}` must fail before mutating curation"
        );
    }
}

#[test]
fn park_replaces_existing_entry() {
    let repo = common::TempRepo::new();
    repo.curation(
        r#"purpose_lite = "podras: fixture repository"

[[parked]]
id = "podras-a"
reason = "old reason"
resurface = "2026-01-01"
parked_at = "2025-12-01"
confirmed_at = "2025-12-01"
"#,
    );

    podras_in(
        &repo,
        &[
            "park",
            "podras-a",
            "--reason",
            "new reason",
            "--resurface",
            "event:release",
        ],
    )
    .success();

    let loaded = curation::load(repo.root())
        .expect("load curation")
        .expect("curation is present after park");
    assert_eq!(
        loaded.parked.len(),
        1,
        "the entry is replaced, not duplicated"
    );
    assert_eq!(loaded.parked[0].reason, "new reason");
    assert_eq!(
        loaded.parked[0].resurface(),
        Ok(Resurface::Event("release".to_string()))
    );
    assert_eq!(
        loaded.parked[0].parked_at.as_deref(),
        Some(today().as_str())
    );
}
