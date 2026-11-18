//! Purpose: prove `scripts/guards/brief-drift.sh` detects a stale committed render
//! without writing the working tree.
//!
//! Responsibilities:
//! - `drift_gate_detects_stale_render`: a freshly written render pair passes the
//!   gate, a source change without re-rendering fails it and names the divergent
//!   artifact, and re-rendering restores a clean pass;
//! - assert `BRIEF.md` and `BRIEF.json` are byte-identical before and after every
//!   gate run (the gate is read-only).
//!
//! Rationale: a committed render is only a reviewable artifact if drift between
//! it and its inputs fails loudly; the gate must compare content, never mutate
//! the tree, and use the render's own evaluation clock instead of the wall clock.

mod common;

use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::Value;

/// Absolute path to the gate script under test.
fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/guards/brief-drift.sh")
}

/// Run the gate from the fixture repository with the binary under test.
fn run_gate(repo: &common::TempRepo) -> Output {
    Command::new(script())
        .current_dir(repo.root())
        .env("PODRAS_BIN", env!("CARGO_BIN_EXE_podras"))
        .env("HOME", repo.root())
        .env_remove("PODRAS_DRIFT_AS_OF")
        .output()
        .expect("run the brief drift gate script")
}

/// Run the built binary inside the fixture repository.
fn podras(repo: &common::TempRepo, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_podras"))
        .args(args)
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .output()
        .expect("run the podras binary under test")
}

/// Human-readable stream dump for assertion messages.
fn output_text(output: &Output) -> String {
    format!(
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The date prefix of the committed render's `generated_at` stamp.
///
/// This is the evaluation clock the gate must reuse, so re-rendering at the
/// same date is the only way to restore a matching pair.
fn committed_as_of(repo: &common::TempRepo) -> String {
    let json: Value =
        serde_json::from_str(repo.read("BRIEF.json").trim()).expect("BRIEF.json is an envelope");
    let generated_at = json["data"]["generated_at"]
        .as_str()
        .expect("BRIEF.json carries data.generated_at");
    generated_at
        .get(..10)
        .expect("generated_at starts with a YYYY-MM-DD date")
        .to_string()
}

#[test]
fn drift_gate_detects_stale_render() {
    let repo = common::TempRepo::new();
    repo.curation("purpose_lite = \"drift fixture\"\n");
    repo.beads_jsonl(&[("fx-1", "first issue", "open")]);

    let written = podras(&repo, &["brief", "--write"]);
    assert!(
        written.status.success(),
        "initial render must be written: {}",
        output_text(&written)
    );
    let committed_markdown = repo.read("BRIEF.md");
    let committed_json = repo.read("BRIEF.json");

    let clean = run_gate(&repo);
    assert!(
        clean.status.success(),
        "a freshly written render must pass the gate: {}",
        output_text(&clean)
    );
    assert_eq!(
        repo.read("BRIEF.md"),
        committed_markdown,
        "the gate never rewrites BRIEF.md"
    );
    assert_eq!(
        repo.read("BRIEF.json"),
        committed_json,
        "the gate never rewrites BRIEF.json"
    );

    // A source change without a re-render must fail the gate.
    repo.beads_jsonl(&[
        ("fx-1", "first issue", "open"),
        ("fx-2", "second issue", "open"),
    ]);

    let stale = run_gate(&repo);
    let stale_text = output_text(&stale);
    assert!(
        !stale.status.success(),
        "a stale render must fail the gate: {stale_text}"
    );
    assert!(
        stale_text.contains("BRIEF.json"),
        "the failed gate names the divergent JSON render: {stale_text}"
    );
    assert!(
        stale_text.contains("BRIEF.md"),
        "the failed gate names the divergent markdown render: {stale_text}"
    );
    assert_eq!(
        repo.read("BRIEF.md"),
        committed_markdown,
        "a failing gate still never rewrites BRIEF.md"
    );
    assert_eq!(
        repo.read("BRIEF.json"),
        committed_json,
        "a failing gate still never rewrites BRIEF.json"
    );

    // Re-rendering at the committed evaluation clock restores a clean pass.
    let as_of = committed_as_of(&repo);
    let rewritten = podras(&repo, &["brief", "--write", "--as-of", &as_of]);
    assert!(
        rewritten.status.success(),
        "re-rendering must succeed: {}",
        output_text(&rewritten)
    );

    let clean_again = run_gate(&repo);
    assert!(
        clean_again.status.success(),
        "a re-rendered pair must pass the gate again: {}",
        output_text(&clean_again)
    );
}
