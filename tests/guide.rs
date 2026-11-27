//! Purpose: integration tests for the genesis guide wiring in the podras CLI.
//!
//! Responsibilities:
//! - assert the genesis verbosity and format flags exist at the top level
//!   (`-v`, `-q`, `--json`, `--human`) and reach the new subcommands;
//! - assert existing subcommands still parse and run with their own flags
//!   when none of the new flags are present (byte-identical behavior is
//!   covered by the pre-existing command tests).
//!
//! Rationale: the guide adoption is additive: new flags may appear, but no
//! existing invocation may change meaning.

mod common;

use assert_cmd::Command;

/// Run the `podras` binary in the fixture repository with the given arguments.
fn podras(repo: &common::TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(args)
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .assert()
}

#[test]
fn top_level_json_flag_reaches_new_subcommands() {
    let repo = common::TempRepo::new();

    podras(&repo, &["--json", "status"])
        .success()
        .stdout(predicates::str::contains("\"envelope_kind\""));
}

#[test]
fn format_flag_after_subcommand_is_accepted() {
    let repo = common::TempRepo::new();

    podras(&repo, &["status", "--json"]).success();
    podras(&repo, &["doctor", "--human"]).success();
}

#[test]
fn verbosity_flags_are_accepted_everywhere() {
    let repo = common::TempRepo::new();

    podras(&repo, &["-v", "doctor"]).success();
    podras(&repo, &["doctor", "-vv"]).success();
    podras(&repo, &["-q", "status", "--human"]).success();
}

#[test]
fn existing_subcommands_still_parse_with_their_own_flags() {
    let repo = common::TempRepo::new();

    podras(&repo, &["brief", "--json"]).success();
    podras(&repo, &["brief"]).success();
    podras(&repo, &["schema"]).success();
}
