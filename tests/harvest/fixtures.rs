//! Purpose: fixtures and envelope assertions shared by the harvest scenario tests.
//!
//! Responsibilities:
//! - `fixture` seeds an orphan commit, an unlinked spec change, and a dangling
//!   note reference; `quiet_fixture` seeds a repository with nothing to harvest;
//! - `podras_in` and `harvest_json` drive the real binary against a fixture;
//! - `snapshot` images every repository file so a run can prove it wrote nothing;
//! - candidate assertions: shape, decision-freedom, human render, and evidence
//!   resolution.
//!
//! Rationale: harvest is the one verb whose entire value depends on being
//! read-only, so its fixtures and its honesty assertions are shared by every
//! scenario instead of being re-derived per test.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::Command as AssertCommand;
use serde_json::Value;

use crate::common;

/// Evaluation date used by both scenarios; the orphan commit is 3 days old.
pub(crate) const AS_OF: &str = "2026-11-18";
/// Fixture commit instant inside the 14-day harvest window.
const IN_WINDOW: &str = "2026-11-15T00:00:00Z";

/// Run the binary inside a fixture repository.
pub(crate) fn podras_in(repo: &common::TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    AssertCommand::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(args)
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .assert()
}

/// Run `harvest --json` and return the genesis envelope.
pub(crate) fn harvest_json(repo: &common::TempRepo) -> Value {
    let output = podras_in(repo, &["harvest", "--json", "--as-of", AS_OF])
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("harvest envelope is UTF-8");
    serde_json::from_str(stdout.trim()).expect("harvest --json emits a JSON envelope")
}

/// Run `harvest` and return its narrative render.
fn narrate(repo: &common::TempRepo) -> String {
    let output = podras_in(repo, &["harvest", "--as-of", AS_OF])
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(output).expect("harvest narrative is UTF-8")
}

/// A fixture with an orphan commit, an unlinked spec change, and a note
/// reference to a tracker id absent from the export.
pub(crate) fn fixture() -> common::TempRepo {
    let repo = common::TempRepo::new();
    repo.beads_jsonl(&[("podras-abc", "Harvest visibility", "open")]);
    repo.commit_at("2026-11-01T00:00:00Z", "podras-abc seed tracker");

    repo.spec_change("no-bead-change", "# Unlinked change\n");
    repo.set_mtime("openspec/changes/no-bead-change/proposal.md", IN_WINDOW);
    repo.commit_at(IN_WINDOW, "podras-abc draft spec change");

    repo.write("src/orphan.rs", "// no tracker id names this file\n");
    repo.commit_at(IN_WINDOW, "add orphan module");

    repo.whisper_branch("main", "See podras-zzz for background.");
    repo.curation("purpose_lite = \"harvest fixture\"\npublish_whisper = false\n");
    repo
}

/// A fixture whose only work is already tracker-linked, so harvest finds nothing.
fn quiet_fixture() -> common::TempRepo {
    let repo = common::TempRepo::new();
    repo.beads_jsonl(&[("podras-abc", "Only linked work", "open")]);
    repo.commit_at("2026-11-01T00:00:00Z", "podras-abc seed tracker");
    repo
}

/// Every regular file below the repository root, excluding `.git`.
pub(crate) fn snapshot(repo: &common::TempRepo) -> BTreeMap<String, Vec<u8>> {
    let mut files = BTreeMap::new();
    collect_files(repo.root(), Path::new(""), &mut files);
    files
}

/// Collect every regular file directly below `relative`, recursing into subdirectories.
fn collect_files(root: &Path, relative: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
    for path in child_paths(root, relative) {
        if root.join(&path).is_dir() {
            collect_files(root, &path, out);
            continue;
        }
        insert_file(root, &path, out);
    }
}

/// Repository-relative paths directly below `relative`, skipping `.git`.
fn child_paths(root: &Path, relative: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root.join(relative)) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| relative.join(entry.file_name()))
        .filter(|path| path != Path::new(".git"))
        .collect()
}

/// Insert one file's bytes under its repository-relative path.
fn insert_file(root: &Path, path: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
    if let Ok(bytes) = fs::read(root.join(path)) {
        out.insert(path.to_string_lossy().into_owned(), bytes);
    }
}

/// The `data.candidates` array of a harvest envelope.
pub(crate) fn candidates(envelope: &Value) -> &Vec<Value> {
    envelope["data"]["candidates"]
        .as_array()
        .unwrap_or_else(|| panic!("data.candidates is an array: {envelope}"))
}

/// Assert the envelope names one candidate per item in kind declaration order,
/// each with a deterministic summary and resolvable evidence.
pub(crate) fn assert_candidate_shape(envelope: &Value) {
    assert_eq!(envelope["ok"].as_bool(), Some(true), "{envelope}");
    assert_eq!(envelope["envelope_kind"], "ok", "{envelope}");
    assert_eq!(
        envelope["data"]["schema_version"].as_str(),
        Some("harvest.v1"),
        "{envelope}"
    );

    let candidates = candidates(envelope);
    let kinds: Vec<&str> = candidates
        .iter()
        .map(|candidate| candidate["kind"].as_str().expect("candidate kind"))
        .collect();
    assert_eq!(
        kinds,
        vec![
            "note_reference_unresolved",
            "orphan_commit",
            "unlinked_spec_change"
        ],
        "one candidate per item, in kind declaration order: {envelope}"
    );

    for candidate in candidates {
        assert_candidate_evidence(candidate);
    }
}

/// Assert one candidate carries a summary and at least one evidence item.
fn assert_candidate_evidence(candidate: &Value) {
    assert!(
        candidate["summary"]
            .as_str()
            .is_some_and(|summary| !summary.is_empty()),
        "candidate carries a deterministic summary: {candidate}"
    );
    let evidence = candidate["evidence"]
        .as_array()
        .expect("candidate carries evidence");
    assert!(!evidence.is_empty(), "evidence-free candidates are dropped");
    for item in evidence {
        assert_evidence_item(item);
    }
}

/// Assert one evidence item names a `git` or `notes` source and a non-empty ref.
fn assert_evidence_item(item: &Value) {
    assert!(
        matches!(item["kind"].as_str(), Some("git" | "notes")),
        "evidence kind is git or notes: {item}"
    );
    assert!(
        item["ref"]
            .as_str()
            .is_some_and(|reference| !reference.is_empty()),
        "evidence carries a resolvable ref: {item}"
    );
}

/// Assert the read path decided nothing: no curation, tracker, focus, park, or artifact.
pub(crate) fn assert_no_decisions(
    repo: &common::TempRepo,
    curation_before: &str,
    beads_before: &str,
) {
    assert_eq!(repo.read(".podras/curation.toml"), curation_before);
    assert_eq!(repo.read(".beads/issues.jsonl"), beads_before);
    let curation: toml::Value =
        toml::from_str(curation_before).expect("curation fixture is valid TOML");
    assert!(
        curation
            .get("focus")
            .and_then(toml::Value::as_array)
            .is_none_or(Vec::is_empty),
        "harvest never promotes a focus item: {curation}"
    );
    assert!(
        curation
            .get("parked")
            .and_then(toml::Value::as_array)
            .is_none_or(Vec::is_empty),
        "harvest never parks anything: {curation}"
    );
    assert!(
        !repo.root().join("BRIEF.md").exists() && !repo.root().join("BRIEF.json").exists(),
        "harvest has no committed artifact"
    );
}

/// Assert the human render groups by kind and states empty groups explicitly.
pub(crate) fn assert_human_render(repo: &common::TempRepo) {
    let human = narrate(repo);
    for kind in [
        "note_reference_unresolved",
        "orphan_commit",
        "unlinked_spec_change",
    ] {
        assert!(
            human.contains(kind),
            "human render names every candidate kind `{kind}`: {human}"
        );
    }

    let quiet_human = narrate(&quiet_fixture());
    assert_eq!(
        quiet_human.matches("  none").count(),
        3,
        "empty groups are explicit, never omitted: {quiet_human}"
    );
}

/// Assert every evidence ref of one candidate resolves against the fixture.
pub(crate) fn assert_evidence_resolves(repo: &common::TempRepo, candidate: &Value) {
    for evidence in candidate["evidence"]
        .as_array()
        .expect("candidate carries evidence")
    {
        match evidence["kind"].as_str() {
            Some("git") => assert_git_ref_resolves(repo, evidence),
            Some("notes") => assert_note_ref_resolves(repo, evidence),
            other => panic!("unexpected evidence kind: {other:?}"),
        }
    }
}

/// Assert a commit ref names a commit present in the fixture repository.
fn assert_git_ref_resolves(repo: &common::TempRepo, evidence: &Value) {
    let sha = evidence["ref"].as_str().expect("git evidence ref");
    let status = Command::new("git")
        .args(["cat-file", "-e", &format!("{sha}^{{commit}}")])
        .current_dir(repo.root())
        .status()
        .expect("run git in fixture");
    assert!(status.success(), "git evidence `{sha}` resolves");
}

/// Assert a notes ref names a present entry in a readable note file.
fn assert_note_ref_resolves(repo: &common::TempRepo, evidence: &Value) {
    let reference = evidence["ref"].as_str().expect("notes evidence ref");
    let (path, timestamp) = reference
        .rsplit_once('#')
        .expect("notes evidence is `<path>#<entry timestamp>`");
    let path = repo.root().join(path);
    assert!(path.is_file(), "notes evidence `{reference}` names a file");
    let body = fs::read_to_string(&path).expect("read notes evidence");
    assert!(
        body.contains(timestamp),
        "notes evidence `{reference}` names a present entry"
    );
}
