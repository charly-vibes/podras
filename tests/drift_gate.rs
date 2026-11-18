//! Purpose: prove `scripts/guards/brief-drift.sh` detects a stale committed render
//! without writing the working tree.
//!
//! Responsibilities:
//! - `drift_gate_detects_stale_render`: a freshly written render pair passes the
//!   gate, a source change without re-rendering fails it and names the divergent
//!   artifact, and re-rendering restores a clean pass;
//! - `drift_gate_ignores_clone_local_provenance`: a byte-identical clone with
//!   fresh mtimes and a new absolute path still passes, while content drift fails;
//! - `drift_gate_uses_recorded_as_of`: the gate evaluates at the render's
//!   recorded `as_of`, not at the wall clock in `generated_at`;
//! - assert `BRIEF.md` and `BRIEF.json` are byte-identical before and after every
//!   gate run (the gate is read-only).
//!
//! Rationale: a committed render is only a reviewable artifact if drift between
//! it and its inputs fails loudly; the gate must compare content, never mutate
//! the tree, and use the render's own evaluation clock instead of the wall clock.

mod common;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// Absolute path to the gate script under test.
fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/guards/brief-drift.sh")
}

/// Run the gate from the fixture repository with the binary under test.
fn run_gate(repo: &common::TempRepo) -> Output {
    run_gate_in(repo.root())
}

/// Run the gate from an arbitrary repository root with the binary under test.
fn run_gate_in(root: &Path) -> Output {
    Command::new(script())
        .current_dir(root)
        .env("PODRAS_BIN", env!("CARGO_BIN_EXE_podras"))
        .env("HOME", root)
        .env_remove("PODRAS_DRIFT_AS_OF")
        .output()
        .expect("run the brief drift gate script")
}

/// Run the built binary inside the fixture repository.
fn podras(repo: &common::TempRepo, args: &[&str]) -> Output {
    podras_at(repo.root(), args)
}

/// Run the built binary inside an arbitrary repository root.
fn podras_at(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_podras"))
        .args(args)
        .current_dir(root)
        .env("HOME", root)
        .output()
        .expect("run the podras binary under test")
}

/// Copy a repository tree, including `.git`, with fresh filesystem mtimes.
///
/// This is a second clone of the same bytes: paths and mtimes change while the
/// content the brief derives from does not.
fn copy_tree(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("create clone directory");
    for entry in std::fs::read_dir(src).expect("read source tree") {
        copy_entry(&entry.expect("read source entry"), dst);
    }
}

/// Copy one directory entry into `dst`, recursing through subdirectories.
fn copy_entry(entry: &std::fs::DirEntry, dst: &Path) {
    let target = dst.join(entry.file_name());
    let kind = entry.file_type().expect("read source entry kind");
    if kind.is_dir() {
        copy_tree(&entry.path(), &target);
    }
    if kind.is_file() {
        std::fs::copy(entry.path(), &target).expect("copy fixture file");
    }
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

/// A fixture whose render records a path detail and a pinned curation mtime.
fn clone_fixture() -> common::TempRepo {
    let repo = common::TempRepo::new();
    repo.curation("purpose_lite = \"clone fixture\"\n");
    repo.beads_jsonl(&[("fx-1", "first issue", "open")]);
    // Pin the curation mtime so a clone necessarily reports a different one.
    repo.set_mtime(".podras/curation.toml", "2026-11-01T00:00:00Z");
    // An unreadable note makes the source row carry an absolute path detail.
    std::fs::create_dir_all(repo.root().join(".whisper/branches/main/notes.md"))
        .expect("block the whisper note with a directory");
    repo
}

/// Render, commit the renders, then re-render at the render's own clock.
///
/// Committing the renders moves HEAD past their `head_sha`; re-rendering at the
/// same clock makes the pair match a fresh render of HEAD itself.
fn commit_render_at_recorded_clock(repo: &common::TempRepo) {
    let written = podras(repo, &["brief", "--write"]);
    assert!(
        written.status.success(),
        "the source render must be written: {}",
        output_text(&written)
    );
    repo.commit("commit the brief renders");

    let as_of = committed_as_of(repo);
    let rewritten = podras(repo, &["brief", "--write", "--as-of", &as_of]);
    assert!(
        rewritten.status.success(),
        "re-rendering at the recorded clock must succeed: {}",
        output_text(&rewritten)
    );
}

/// Assert the gate passes in a clone and rewrites neither render.
fn assert_clone_passes_gate(root: &Path) {
    let committed_markdown =
        std::fs::read_to_string(root.join("BRIEF.md")).expect("the clone carries BRIEF.md");
    let committed_json =
        std::fs::read_to_string(root.join("BRIEF.json")).expect("the clone carries BRIEF.json");

    let clean = run_gate_in(root);
    assert!(
        clean.status.success(),
        "clone-local mtimes and paths are not content drift: {}",
        output_text(&clean)
    );
    assert_eq!(
        std::fs::read_to_string(root.join("BRIEF.md")).expect("read clone BRIEF.md"),
        committed_markdown,
        "the gate never rewrites the clone's BRIEF.md"
    );
    assert_eq!(
        std::fs::read_to_string(root.join("BRIEF.json")).expect("read clone BRIEF.json"),
        committed_json,
        "the gate never rewrites the clone's BRIEF.json"
    );
}

/// Append a second tracker issue to the clone: real content drift.
fn append_tracker_issue(root: &Path) {
    let export = std::fs::read_to_string(root.join(".beads/issues.jsonl"))
        .expect("read clone tracker export");
    let issue = serde_json::json!({
        "_type": "issue",
        "id": "fx-2",
        "title": "second issue",
        "description": "",
        "status": "open",
        "priority": 1,
        "issue_type": "task",
        "owner": "podras@tests.invalid",
        "created_at": "2026-11-01T00:00:00Z",
        "created_by": "Podras Tests",
        "updated_at": "2026-11-01T00:00:00Z",
        "metadata": {},
        "labels": [],
        "dependencies": [],
        "comments": []
    });
    std::fs::write(
        root.join(".beads/issues.jsonl"),
        format!("{}\n{issue}", export.trim_end()),
    )
    .expect("append a tracker issue in the clone");
}

#[test]
fn drift_gate_ignores_clone_local_provenance() {
    let repo = clone_fixture();
    commit_render_at_recorded_clock(&repo);

    let clone = tempfile::tempdir().expect("create clone directory");
    copy_tree(repo.root(), clone.path());
    assert_clone_passes_gate(clone.path());

    // A real content change in the clone must still fail.
    append_tracker_issue(clone.path());
    let stale = run_gate_in(clone.path());
    let stale_text = output_text(&stale);
    assert!(
        !stale.status.success(),
        "content drift in a clone must fail the gate: {stale_text}"
    );
    assert!(
        stale_text.contains("BRIEF.json"),
        "the failed gate names the divergent JSON render: {stale_text}"
    );
}

#[test]
fn drift_gate_uses_recorded_as_of() {
    let repo = common::TempRepo::new();
    repo.curation("purpose_lite = \"as-of fixture\"\n");
    repo.beads_jsonl(&[("fx-1", "first issue", "open")]);

    let written = podras(&repo, &["brief", "--write", "--as-of", "2026-11-01"]);
    assert!(
        written.status.success(),
        "the pinned render must be written: {}",
        output_text(&written)
    );

    let envelope: Value =
        serde_json::from_str(repo.read("BRIEF.json").trim()).expect("BRIEF.json is an envelope");
    assert_eq!(
        envelope["data"]["as_of"].as_str(),
        Some("2026-11-01"),
        "the envelope records the evaluation clock, not the wall clock: {envelope}"
    );

    // PODRAS_DRIFT_AS_OF is unset, so the gate must reuse the recorded clock
    // instead of the wall-clock date in `generated_at`.
    let clean = run_gate(&repo);
    let clean_text = output_text(&clean);
    assert!(
        clean.status.success(),
        "the gate evaluates at the recorded as_of: {clean_text}"
    );
    assert!(
        clean_text.contains("2026-11-01"),
        "the gate reports the recorded evaluation date: {clean_text}"
    );
}
