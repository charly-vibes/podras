//! Purpose: prove declared sources degrade independently instead of aborting.
//!
//! Responsibilities:
//! - `missing_source_warns`: the deployed property contract;
//! - assert every declared source still appears in the envelope;
//! - assert a missing source is loud (status + warning) and exits 0.
//!
//! Rationale: a cold return must never depend on every source existing. The
//! brief declares what it could not read instead of hiding or aborting on it.

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

#[test]
fn missing_source_warns() {
    let repo = common::TempRepo::new();

    let output = podras_in(&repo, &["brief", "--json"])
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("brief envelope is UTF-8");
    let envelope: Value =
        serde_json::from_str(stdout.trim()).expect("brief --json emits a JSON envelope");
    let data = &envelope["data"];

    let sources = data["sources"]
        .as_array()
        .expect("data.sources is an array");
    let names: Vec<&str> = sources
        .iter()
        .map(|source| source["name"].as_str().expect("source name is a string"))
        .collect();
    assert_eq!(
        names,
        ["beads", "git", "openspec", "whisper", "curation"],
        "every declared source must appear in fixed declaration order"
    );

    let openspec = sources
        .iter()
        .find(|source| source["name"] == "openspec")
        .expect("openspec source row is declared");
    assert_eq!(
        openspec["status"], "missing",
        "a repository without openspec/ reports the source as missing: {openspec}"
    );

    let warnings = data["warnings"]
        .as_array()
        .expect("data.warnings is an array");
    assert!(
        !warnings.is_empty(),
        "a missing source must emit a warning, got {warnings:?}"
    );
    assert!(
        warnings.iter().any(|warning| {
            warning["source"] == "openspec" && warning["code"] == "openspec_missing"
        }),
        "the openspec source warning must name the source and a stable code: {warnings:?}"
    );
}
