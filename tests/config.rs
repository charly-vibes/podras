//! Purpose: integration tests for the genesis config adoption.
//!
//! Responsibilities:
//! - assert the podras config registry exposes the two human-owned files
//!   (`.podras/curation.toml` and `.podras/roadmap.toml`);
//! - assert `podras config` discovers and validates them read-only, with
//!   missing files reported as absent and broken files as validation errors;
//! - assert no config path ever writes or scaffolds a config file.
//!
//! Rationale: config files stay human-owned; the adoption is read and
//! validate only, so every test asserts the registry never mutates state.

mod common;

use predicates::prelude::*;

use assert_cmd::Command;

/// Run `podras config` in the fixture repository with extra arguments.
fn config(repo: &common::TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(["config"])
        .args(args)
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .assert()
}

#[test]
fn registry_exposes_the_two_human_owned_files() {
    let registry = podras::config::registry();

    assert_eq!(registry.registered_tools().len(), 2);
    assert_eq!(registry.marker("podras"), Some(".podras/curation.toml"));
    assert_eq!(
        registry.marker("podras-roadmap"),
        Some(".podras/roadmap.toml")
    );
}

#[test]
fn registry_reads_a_valid_curation_file() {
    let repo = common::TempRepo::new();
    repo.write(".podras/curation.toml", "purpose_lite = \"test project\"\n");

    let store = genesis_store();
    let config: podras::curation::CurationFile = store
        .get("podras", repo.root())
        .expect("valid curation reads through the registry");
    assert_eq!(config.purpose_lite.as_deref(), Some("test project"));
}

fn genesis_store() -> genesis::config::ConfigStore {
    genesis::config::ConfigStore::new(podras::config::registry())
}

#[test]
fn missing_files_are_reported_not_created() {
    let repo = common::TempRepo::new();

    config(&repo, &["--json"]).success().stdout(
        predicates::str::contains("\"found\": false")
            .and(predicates::str::contains("\"validations\": []")),
    );

    assert!(
        !repo.root().join(".podras/curation.toml").exists(),
        "config must never scaffold human-owned state"
    );
}

#[test]
fn present_files_are_discovered_and_valid() {
    let repo = common::TempRepo::new();
    repo.write(".podras/curation.toml", "purpose_lite = \"test project\"\n");
    repo.write(
        ".podras/roadmap.toml",
        "[identity]\npath = \"BRIEF.md\"\nlast_reviewed = \"2026-11-01\"\n",
    );

    config(&repo, &["--json"]).success().stdout(
        predicates::str::contains("\"found\": true")
            .and(predicates::str::contains("\"validations\": []")),
    );
}

#[test]
fn broken_toml_becomes_a_validation_error() {
    let repo = common::TempRepo::new();
    repo.write(".podras/curation.toml", "max_focus = [");

    config(&repo, &["--json"]).success().stdout(
        predicates::str::contains("\"severity\": \"error\"")
            .and(predicates::str::contains("\"podras\"")),
    );
}

#[test]
fn human_format_lists_each_config() {
    let repo = common::TempRepo::new();
    repo.write(".podras/curation.toml", "purpose_lite = \"test project\"\n");

    config(&repo, &["--human"])
        .success()
        .stdout(predicates::str::contains(".podras/curation.toml"));
}
