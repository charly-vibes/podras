//! Purpose: integration tests for the `podras brief` CLI surface.
//!
//! Responsibilities:
//! - assert `brief --json` emits a genesis success envelope with the typed data;
//! - assert two runs differ only in `generated_at`;
//! - assert a missing mandatory source still exits 0 with a warning.
//!
//! Rationale: the CLI is the only public entry point for the read path, so its
//! envelope wrapper, determinism, and degradation guarantees are tested there
//! rather than inferred from library functions.

mod common;

use assert_cmd::Command;
use serde_json::Value;

fn podras_in(repo: &common::TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(args)
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .assert()
}

/// Run `brief --json` and return the parsed envelope.
fn brief_json(repo: &common::TempRepo) -> Value {
    let output = podras_in(repo, &["brief", "--json"])
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("brief envelope is UTF-8");
    serde_json::from_str(stdout.trim()).expect("brief --json emits a JSON envelope")
}

/// A fixture with every declared source present and readable.
fn populated() -> common::TempRepo {
    let repo = common::TempRepo::new();
    repo.beads_jsonl(&[
        ("podras-a", "Read path", "open"),
        ("podras-b", "Envelope", "open"),
    ]);
    repo.curation(
        r#"purpose_lite = "podras: fixture repository"
max_focus = 1
publish_whisper = false

[health]
status = "unknown"

[[focus]]
id = "podras-a"
reason = "fixture focus"
focused_since = "2026-11-01"
confirmed_at = "2026-11-01"
"#,
    );
    repo.spec_change("fixture-change", "# Fixture change\n");
    repo.write("openspec/specs/brief-v1/spec.md", "# Brief v1\n");
    repo.whisper_branch("main", "Ship the brief read path.");
    repo.commit("fixture sources");
    repo
}

#[test]
fn brief_json_emits_genesis_envelope() {
    let repo = populated();

    let envelope = brief_json(&repo);
    assert_eq!(
        envelope["ok"].as_bool(),
        Some(true),
        "success envelope: {envelope}"
    );
    assert_eq!(
        envelope["envelope_kind"], "ok",
        "brief is a success envelope: {envelope}"
    );
    let data = envelope["data"]
        .as_object()
        .expect("data carries the typed brief payload");
    assert_eq!(
        data.get("schema_version").and_then(Value::as_str),
        Some("brief.v1"),
        "the payload declares the brief schema version: {envelope}"
    );
}

#[test]
fn brief_json_is_deterministic_except_generated_at() {
    let repo = populated();

    let first = brief_json(&repo);
    let second = brief_json(&repo);

    let mut first_data = first["data"].clone();
    let mut second_data = second["data"].clone();
    assert!(
        first_data.get("generated_at").is_some(),
        "the data carries a generation stamp: {first}"
    );
    first_data
        .as_object_mut()
        .expect("brief payload is an object")
        .remove("generated_at");
    second_data
        .as_object_mut()
        .expect("brief payload is an object")
        .remove("generated_at");

    assert_eq!(
        first_data, second_data,
        "two runs over identical inputs differ only in generated_at"
    );
}

#[test]
fn missing_beads_still_exits_zero_with_warning() {
    let repo = common::TempRepo::new();

    let envelope = brief_json(&repo);

    let data = &envelope["data"];
    let beads = data["sources"]
        .as_array()
        .expect("data.sources is an array")
        .iter()
        .find(|source| source["name"] == "beads")
        .expect("beads source row is declared");
    assert_eq!(
        beads["status"], "missing",
        "a fixture without .beads/ reports beads missing: {beads}"
    );
    let warnings = data["warnings"]
        .as_array()
        .expect("data.warnings is an array");
    assert!(
        warnings.iter().any(|warning| warning["source"] == "beads"),
        "missing beads must emit a warning: {warnings:?}"
    );
}
