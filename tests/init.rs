//! Purpose: integration tests for the `podras init` onboarding verb.
//!
//! Responsibilities:
//! - assert the first run scaffolds `.podras/curation.toml` and the PODRAS
//!   managed block with markers and provenance;
//! - assert a second run is byte-identical and reports `already initialized`;
//! - assert existing curation bytes and unrelated AGENTS.md blocks survive.
//!
//! Rationale: init is the only onboarding mutation and must be safe to run
//! unconditionally, so idempotence and non-destruction are the contract.

mod common;

use assert_cmd::Command;

const PODRAS_START: &str = "<!-- PODRAS:START -->";
const PODRAS_END: &str = "<!-- PODRAS:END -->";

/// Run `podras init` in the fixture repository.
fn init(repo: &common::TempRepo) -> assert_cmd::assert::Assert {
    Command::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(["init"])
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .assert()
}

#[test]
fn first_run_scaffolds_curation_and_managed_block() {
    let repo = common::TempRepo::new();

    init(&repo).success();

    let curation = repo.read(".podras/curation.toml");
    assert!(
        curation.contains(".podras/curation.toml"),
        "the scaffolded curation template explains its own path: {curation}"
    );
    assert!(
        curation.starts_with("#"),
        "the scaffolded curation file is a commented template: {curation}"
    );
    assert!(
        podras::curation::load(repo.root())
            .expect("scaffolded curation parses")
            .is_some(),
        "init writes a parseable curation file"
    );

    let agents = repo.read("AGENTS.md");
    assert!(
        agents.starts_with(PODRAS_START),
        "block leads the new file: {agents}"
    );
    assert!(
        agents.contains(PODRAS_END),
        "block has an end marker: {agents}"
    );
    assert!(
        agents.contains("<!-- provenance: generator=podras "),
        "block carries the podras provenance footer: {agents}"
    );
    assert!(
        agents.contains("source=PODRAS"),
        "provenance names the managed block: {agents}"
    );
    assert!(
        agents.contains("podras brief --json"),
        "block tells agents to start from the JSON brief: {agents}"
    );
    assert!(
        agents.contains("freshness"),
        "block requires a freshness check before inference: {agents}"
    );
    assert!(
        agents.contains("Beads owns") || agents.contains("beads owns"),
        "block names beads as lifecycle owner: {agents}"
    );
    assert!(
        agents.contains("silently change focus"),
        "block forbids silent focus changes: {agents}"
    );
    assert!(
        agents.contains("committed renders"),
        "block forbids hand-editing committed renders: {agents}"
    );
}

#[test]
fn second_run_is_byte_identical_and_reports_already_initialized() {
    let repo = common::TempRepo::new();

    init(&repo).success();
    let curation_first = repo.read(".podras/curation.toml");
    let agents_first = repo.read("AGENTS.md");

    let second = init(&repo).success();
    let stdout =
        String::from_utf8(second.get_output().stdout.clone()).expect("init output is UTF-8");

    assert_eq!(
        repo.read(".podras/curation.toml"),
        curation_first,
        "second run must not rewrite curation bytes"
    );
    assert_eq!(
        repo.read("AGENTS.md"),
        agents_first,
        "second run must not rewrite the AGENTS.md block"
    );
    assert!(
        stdout.contains("already initialized"),
        "second run reports a no-op: {stdout}"
    );
}

#[test]
fn existing_curation_bytes_are_preserved() {
    let repo = common::TempRepo::new();
    let original = "# hand-written\npurpose_lite = \"keep me\"\n\n[[focus]]\nid = \"podras-a\"\n";
    repo.curation(original);

    init(&repo).success();

    assert_eq!(
        repo.read(".podras/curation.toml"),
        original,
        "init never overwrites an existing curation file"
    );
    assert!(
        repo.read("AGENTS.md").contains(PODRAS_START),
        "init still injects the block alongside existing curation"
    );
}

#[test]
fn existing_agents_content_and_other_blocks_are_preserved() {
    let repo = common::TempRepo::new();
    let existing = "# AGENTS.md\n\n<!-- OTHER:START -->\nother tool rules\n<!-- OTHER:END -->\n\nHand-written tail.\n";
    repo.write("AGENTS.md", existing);

    init(&repo).success();
    let agents = repo.read("AGENTS.md");

    assert!(
        agents.contains(existing.trim_start_matches("# AGENTS.md\n\n")),
        "init preserves unrelated content and blocks: {agents}"
    );
    assert!(
        agents.contains(PODRAS_START),
        "PODRAS block is injected: {agents}"
    );
    assert!(
        agents.contains("<!-- OTHER:START -->"),
        "other block survives: {agents}"
    );

    let first = repo.read("AGENTS.md");
    init(&repo).success();
    assert_eq!(
        repo.read("AGENTS.md"),
        first,
        "a repeat run leaves a file with other blocks byte-identical"
    );
}
