//! Purpose: integration tests for the `podras doctor` subcommand.
//!
//! Responsibilities:
//! - assert the checks cover the adopted surfaces (beads export, openspec
//!   directory, curation, roadmap, kaya ledger);
//! - assert missing or broken artifacts produce failing entries and the
//!   healthy repository produces an all-pass report;
//! - assert the JSON envelope on non-TTY output and the `--human` lines.
//!
//! Rationale: doctor is read-only diagnostics, so the contract is exactly
//! what the report says — no state may change and no invocation may fail
//! when the checks themselves can run.

mod common;

use predicates::prelude::*;

use assert_cmd::Command;

/// Run `podras doctor` in the fixture repository with extra arguments.
fn doctor(repo: &common::TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(["doctor"])
        .args(args)
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .assert()
}

/// A fixture repository with every adopted surface present and valid.
fn healthy_repo() -> common::TempRepo {
    let repo = common::TempRepo::new();
    repo.write(".beads/issues.jsonl", "");
    repo.write("openspec/changes/.gitkeep", "");
    repo.write(".kaya/ledger.jsonl", "");
    repo.write(".podras/curation.toml", "purpose_lite = \"test project\"\n");
    repo.write(
        ".podras/roadmap.toml",
        "[identity]\npath = \"BRIEF.md\"\nlast_reviewed = \"2026-11-01\"\n",
    );
    repo
}

#[test]
fn missing_surfaces_are_reported_as_failures() {
    let repo = common::TempRepo::new();

    doctor(&repo, &["--json"]).success().stdout(
        predicates::str::contains("\"envelope_kind\": \"doctor\"")
            .and(predicates::str::contains("\"beads-export\""))
            .and(predicates::str::contains("\"openspec-dir\""))
            .and(predicates::str::contains("\"curation\""))
            .and(predicates::str::contains("\"roadmap\""))
            .and(predicates::str::contains("\"kaya-ledger\"")),
    );
}

#[test]
fn missing_surfaces_carry_fix_hints() {
    let repo = common::TempRepo::new();

    doctor(&repo, &["--json"])
        .success()
        .stdout(predicates::str::contains("podras init"));
}

#[test]
fn healthy_repository_passes_every_check() {
    let repo = healthy_repo();

    let output = doctor(&repo, &["--json"])
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).expect("doctor emits JSON");
    let summary = &json["data"]["summary"];
    assert_eq!(summary["fail"], serde_json::json!(0));
    assert_eq!(summary["warn"], serde_json::json!(0));
    assert_eq!(summary["pass"], serde_json::json!(5));
}

#[test]
fn broken_curation_fails_with_the_parser_message() {
    let repo = healthy_repo();
    repo.write(".podras/curation.toml", "max_focus = [");

    doctor(&repo, &["--json"])
        .success()
        .stdout(predicates::str::contains("\"curation\"").and(predicates::str::contains("fail")));
}

#[test]
fn human_format_prints_check_lines() {
    let repo = common::TempRepo::new();

    doctor(&repo, &["--human"]).success().stdout(
        predicates::str::contains("podras doctor:").and(predicates::str::contains("[fail]")),
    );
}

#[test]
fn doctor_is_read_only() {
    let repo = common::TempRepo::new();

    doctor(&repo, &["--json"]).success();

    assert!(
        !repo.root().join(".podras/curation.toml").exists(),
        "doctor must never scaffold human-owned state"
    );
}

#[test]
fn doctor_reports_the_tool_name() {
    let repo = common::TempRepo::new();

    let output = doctor(&repo, &["--json"])
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).expect("doctor emits JSON");
    assert_eq!(json["data"]["tool"], serde_json::json!("podras"));
}
