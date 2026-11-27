//! Purpose: integration tests for the `podras feedback` subcommand.
//!
//! Responsibilities:
//! - assert a dry run previews the issue body and the filing command
//!   against podras's own repository without invoking `gh`;
//! - assert the machine envelope reports the dry-run outcome;
//! - assert an unknown kind fails with a stable domain code.
//!
//! Rationale: feedback delegates to `genesis::feedback::handle_feedback`
//! with podras's repo context; the tests pin only the podras-side wiring
//! (target repo, kinds, envelope), never the genesis internals.

mod common;

use predicates::prelude::*;

use assert_cmd::Command;

/// Run `podras feedback` in the fixture repository with extra arguments.
fn feedback(repo: &common::TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    feedback_stdin(repo, "", args)
}

/// Run `podras feedback` with content piped into stdin.
fn feedback_stdin(
    repo: &common::TempRepo,
    stdin: &str,
    args: &[&str],
) -> assert_cmd::assert::Assert {
    Command::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(["feedback"])
        .args(args)
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .write_stdin(stdin.to_string())
        .assert()
}

#[test]
fn dry_run_previews_without_filing() {
    let repo = common::TempRepo::new();

    feedback_stdin(
        &repo,
        "the report body\n",
        &["bug", "--title", "boom", "--dry-run"],
    )
    .success()
    .stderr(
        predicates::str::contains("boom").and(predicates::str::contains(
            "gh issue create --repo charly-vibes/podras",
        )),
    );
}

#[test]
fn dry_run_emits_a_success_envelope() {
    let repo = common::TempRepo::new();

    let output = feedback_stdin(&repo, "the report body\n", &["bug", "--dry-run"])
        .success()
        .get_output()
        .stdout
        .clone();
    let json: serde_json::Value = serde_json::from_slice(&output).expect("feedback emits JSON");
    assert_eq!(json["envelope_kind"], serde_json::json!("ok"));
    // genesis 0.13.2 maps dry-run onto the fallback ladder: no gh call, the
    // prefilled filing URL rides back as FallbackUrl.
    assert_eq!(json["data"]["outcome"], serde_json::json!("fallback_url"));
}

#[test]
fn unknown_kind_fails_with_a_stable_code() {
    let repo = common::TempRepo::new();

    feedback(&repo, &["nonsense", "--dry-run"])
        .failure()
        .stderr(
            predicates::str::contains("feedback_failed").and(predicates::str::contains("nonsense")),
        );
}
