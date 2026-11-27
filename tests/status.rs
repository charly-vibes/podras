//! Purpose: integration tests for the `podras status` aggregation.
//!
//! Responsibilities:
//! - assert `podras status` aggregates the podras health surfaces (doctor
//!   bridge and config) into sections;
//! - assert the JSON envelope on non-TTY output and the `--human` lines;
//! - assert a degraded repository still exits successfully with error
//!   sections instead of failing.
//!
//! Rationale: status is a read-only dashboard over the same surfaces doctor
//! checks, so a degraded repository is data, not a failure.

mod common;

use assert_cmd::Command;

/// Run `podras status` in the fixture repository with extra arguments.
fn status(repo: &common::TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(["status"])
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
fn healthy_repository_reports_all_sections_healthy() {
    let repo = healthy_repo();

    let output = status(&repo, &["--json"])
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).expect("status emits JSON");
    assert_eq!(json["envelope_kind"], serde_json::json!("ok"));
    assert_eq!(json["data"]["sections"].as_array().map(Vec::len), Some(2));
    assert_eq!(json["data"]["summary"]["healthy"], serde_json::json!(2));
}

#[test]
fn sections_are_named_after_their_surface() {
    let repo = healthy_repo();

    let output = status(&repo, &["--json"])
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).expect("status emits JSON");
    let tools: Vec<&str> = json["data"]["sections"]
        .as_array()
        .expect("sections array")
        .iter()
        .map(|section| section["tool"].as_str().expect("tool name"))
        .collect();
    assert!(
        tools.contains(&"podras"),
        "doctor bridge section: {tools:?}"
    );
    assert!(
        tools.contains(&"podras-config"),
        "config section: {tools:?}"
    );
}

#[test]
fn degraded_repository_still_succeeds_with_error_sections() {
    let repo = common::TempRepo::new();

    let output = status(&repo, &["--json"])
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).expect("status emits JSON");
    assert_eq!(json["data"]["summary"]["errors"], serde_json::json!(1));
}

#[test]
fn human_format_prints_section_lines() {
    let repo = healthy_repo();

    status(&repo, &["--human"])
        .success()
        .stdout(predicates::str::contains("podras status:"));
}

#[test]
fn quiet_suppresses_human_output() {
    let repo = healthy_repo();

    status(&repo, &["--human", "-q"]).success().stdout("");
}
