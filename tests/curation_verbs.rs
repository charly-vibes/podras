//! Purpose: prove the curation verbs enforce the focus contract.
//!
//! Responsibilities:
//! - `focus_cap_signal`: two curated focus rows above `max_focus` still assemble
//!   a brief and carry a `wip_over_cap` health signal;
//! - replacing the focus prints the demotion, keeps one focus, and preserves
//!   purpose, health, and parked state;
//! - the demotion picks the oldest `focused_since`, never the newest;
//! - re-focusing the same id refreshes `confirmed_at` and keeps one entry;
//! - an id unknown to the tracker fails without rewriting curation.
//!
//! Rationale: focus is human intent and the cap is advisory, never destructive.
//! The read path must keep reporting a capped file, while the verb must record
//! what it replaced so the human can see the decision it made for them.

mod common;

use assert_cmd::Command;
use chrono::Utc;
use podras::brief::{BriefInputs, CurationState, assemble};
use podras::curation::{self, CurationFile, FocusEntry};
use podras::model::{HealthStatus, SignalKind};
use podras::readers::{self, ReadContext};
use predicates::prelude::*;

fn podras_in(repo: &common::TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(args)
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .assert()
}

fn load(repo: &common::TempRepo) -> CurationFile {
    curation::load(repo.root())
        .expect("curation loads")
        .expect("curation exists")
}

fn curation_with(max_focus: usize, rows: &str) -> String {
    format!("purpose_lite = \"podras: status read path\"\nmax_focus = {max_focus}\n\n{rows}")
}

fn today() -> String {
    Utc::now().date_naive().format("%Y-%m-%d").to_string()
}

fn day(raw: &str) -> chrono::NaiveDate {
    chrono::NaiveDate::parse_from_str(raw, "%Y-%m-%d").expect("fixture date is valid")
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

/// Inputs whose curation holds two focus rows against a cap of one.
fn over_cap_inputs() -> BriefInputs {
    let as_of = day("2026-11-18");
    let curation = CurationFile {
        purpose_lite: Some("podras: status read path".to_string()),
        max_focus: Some(1),
        focus: vec![
            FocusEntry {
                id: "podras-a".to_string(),
                reason: Some("read path".to_string()),
                focused_since: Some("2026-11-16".to_string()),
                confirmed_at: Some("2026-11-18".to_string()),
            },
            FocusEntry {
                id: "podras-b".to_string(),
                reason: Some("health".to_string()),
                focused_since: Some("2026-11-17".to_string()),
                confirmed_at: Some("2026-11-18".to_string()),
            },
        ],
        ..CurationFile::default()
    };
    BriefInputs {
        curation: Some(CurationState {
            curation,
            updated_at: None,
        }),
        sources: Vec::new(),
        warnings: Vec::new(),
        as_of: Some(as_of),
        now: as_of,
        generated_at: as_of
            .and_hms_opt(9, 0, 0)
            .expect("fixture instant is valid")
            .and_utc(),
        beads: None,
        git: None,
        specs: None,
        whisper: None,
    }
}

#[test]
fn focus_cap_signal() {
    let brief = assemble(&over_cap_inputs());
    let kinds: Vec<SignalKind> = brief
        .health
        .signals
        .iter()
        .map(|signal| signal.kind)
        .collect();

    assert_eq!(kinds, vec![SignalKind::WipOverCap], "only the cap fires");
    assert_eq!(brief.health.status, HealthStatus::Amber);
}

#[test]
fn replacing_focus_demotes_oldest_and_keeps_one() {
    let repo = common::TempRepo::new();
    repo.beads_jsonl(&[
        ("podras-a", "read path", "open"),
        ("podras-b", "health", "open"),
    ]);
    repo.curation(&curation_with(
        1,
        "[health]\nstatus = \"green\"\nowner = \"@sasha\"\n\n\
         [[focus]]\nid = \"podras-a\"\nfocused_since = \"2026-11-01\"\nconfirmed_at = \"2026-11-01\"\n\n\
         [[parked]]\nid = \"podras-zz\"\nreason = \"harvest is v1.1\"\nresurface = \"2027-01-01\"\n",
    ));

    podras_in(&repo, &["focus", "podras-b"])
        .success()
        .stdout(predicate::str::contains(
            "demoted podras-a: focus replaced by podras-b",
        ))
        .stdout(predicate::str::contains("focused podras-b"));

    let curation = load(&repo);
    assert_eq!(curation.focus.len(), 1, "cap keeps one focus");
    assert_eq!(curation.focus[0].id, "podras-b");
    assert_eq!(
        curation.focus[0].focused_since.as_deref(),
        Some(today().as_str())
    );
    assert_eq!(
        curation.focus[0].confirmed_at.as_deref(),
        Some(today().as_str())
    );
    assert_eq!(
        curation.purpose_lite.as_deref(),
        Some("podras: status read path"),
        "purpose survives the focus change"
    );
    assert_eq!(
        curation
            .health
            .as_ref()
            .map(|health| health.status.as_str()),
        Some("green"),
        "health survives the focus change"
    );
    assert_eq!(curation.parked.len(), 1, "parked survives the focus change");
    assert_eq!(curation.parked[0].id, "podras-zz");
}

#[test]
fn replacement_demotes_only_the_oldest_focus() {
    let repo = common::TempRepo::new();
    repo.beads_jsonl(&[
        ("podras-a", "oldest", "open"),
        ("podras-b", "newer", "open"),
        ("podras-c", "target", "open"),
    ]);
    repo.curation(&curation_with(
        2,
        "[[focus]]\nid = \"podras-b\"\nfocused_since = \"2026-11-05\"\n\n\
         [[focus]]\nid = \"podras-a\"\nfocused_since = \"2026-11-01\"\n",
    ));

    podras_in(&repo, &["focus", "podras-c"])
        .success()
        .stdout(predicate::str::contains(
            "demoted podras-a: focus replaced by podras-c",
        ))
        .stdout(predicate::str::contains("demoted podras-b").not());

    let mut ids: Vec<String> = load(&repo)
        .focus
        .into_iter()
        .map(|entry| entry.id)
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["podras-b", "podras-c"]);
}

#[test]
fn same_id_refresh_keeps_one() {
    let repo = common::TempRepo::new();
    repo.beads_jsonl(&[("podras-a", "read path", "open")]);
    repo.curation(&curation_with(
        1,
        "[[focus]]\nid = \"podras-a\"\nfocused_since = \"2026-11-01\"\nconfirmed_at = \"2026-11-02\"\n",
    ));

    podras_in(&repo, &["focus", "podras-a"])
        .success()
        .stdout(predicate::str::contains("already focused podras-a"))
        .stdout(predicate::str::contains("demoted").not());

    let curation = load(&repo);
    assert_eq!(curation.focus.len(), 1, "refresh must not duplicate");
    assert_eq!(curation.focus[0].id, "podras-a");
    assert_eq!(
        curation.focus[0].focused_since.as_deref(),
        Some("2026-11-01"),
        "origin date is preserved"
    );
    assert_eq!(
        curation.focus[0].confirmed_at.as_deref(),
        Some(today().as_str())
    );
}

#[test]
fn focusing_parked_id_unparks_it() {
    let repo = common::TempRepo::new();
    repo.beads_jsonl(&[("podras-a", "expired park", "open")]);
    repo.curation(&curation_with(
        1,
        "[[parked]]\nid = \"podras-a\"\nreason = \"ran out of time\"\n\
         resurface = \"2020-01-01\"\nparked_at = \"2019-12-01\"\nconfirmed_at = \"2019-12-01\"\n",
    ));

    let before = brief_of(&repo);
    assert!(
        before
            .needs_retriage
            .iter()
            .any(|item| item.id == "podras-a"),
        "fixture starts with an expired park: {:?}",
        before.needs_retriage
    );

    podras_in(&repo, &["focus", "podras-a"])
        .success()
        .stdout(predicate::str::contains("unparked podras-a"))
        .stdout(predicate::str::contains("focused podras-a"));

    let curation = load(&repo);
    assert!(
        curation.parked.is_empty(),
        "focusing a parked id removes the parked row: {:?}",
        curation.parked
    );
    assert_eq!(curation.focus.len(), 1);
    assert_eq!(curation.focus[0].id, "podras-a");

    let after = brief_of(&repo);
    assert!(
        !after
            .needs_retriage
            .iter()
            .any(|item| item.id == "podras-a"),
        "the retriaged id no longer needs re-triage: {:?}",
        after.needs_retriage
    );
}

#[test]
fn unknown_id_fails_without_rewriting() {
    let repo = common::TempRepo::new();
    repo.beads_jsonl(&[("podras-a", "read path", "open")]);
    let original = curation_with(
        1,
        "[[focus]]\nid = \"podras-a\"\nfocused_since = \"2026-11-01\"\nconfirmed_at = \"2026-11-01\"\n",
    );
    repo.curation(&original);

    podras_in(&repo, &["focus", "podras-zzz"])
        .failure()
        .code(1)
        .stderr(predicate::str::contains("focus_unresolved"))
        .stderr(predicate::str::contains("run bd list"));

    assert_eq!(
        repo.read(".podras/curation.toml"),
        original,
        "a rejected id must not rewrite curation"
    );
}
