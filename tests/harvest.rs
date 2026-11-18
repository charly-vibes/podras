//! Purpose: prove `podras harvest` proposes candidates without deciding anything.
//!
//! Responsibilities:
//! - `harvest_candidates_never_decisions`: typed candidates with evidence, but
//!   curation, tracker, focus, and parked state are byte-identical afterwards;
//! - `harvest_idempotent_provenance`: two runs at one `--as-of` emit the same
//!   candidates in the same order, every evidence ref resolves, and no
//!   repository file changes.
//!
//! Rationale: discovery is not commitment. Harvest is the one verb whose entire
//! value depends on being read-only, so these properties are exercised through
//! the binary rather than inferred from the assembler.

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::Command as AssertCommand;
use serde_json::Value;

/// Evaluation date used by both scenarios; the orphan commit is 3 days old.
const AS_OF: &str = "2026-11-18";
/// Fixture commit instant inside the 14-day harvest window.
const IN_WINDOW: &str = "2026-11-15T00:00:00Z";

fn podras_in(repo: &common::TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    AssertCommand::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(args)
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .assert()
}

/// Run `harvest --json` and return the genesis envelope.
fn harvest_json(repo: &common::TempRepo) -> Value {
    let output = podras_in(repo, &["harvest", "--json", "--as-of", AS_OF])
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("harvest envelope is UTF-8");
    serde_json::from_str(stdout.trim()).expect("harvest --json emits a JSON envelope")
}

/// A fixture with an orphan commit, an unlinked spec change, and a note
/// reference to a tracker id absent from the export.
fn fixture() -> common::TempRepo {
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

/// Every regular file below the repository root, excluding `.git`.
fn snapshot(repo: &common::TempRepo) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, relative: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let directory = root.join(relative);
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let path = relative.join(&name);
            if path == Path::new(".git") {
                continue;
            }
            let absolute = root.join(&path);
            if absolute.is_dir() {
                walk(root, &path, out);
            } else if let Ok(bytes) = fs::read(&absolute) {
                out.insert(path.to_string_lossy().into_owned(), bytes);
            }
        }
    }

    let mut files = BTreeMap::new();
    walk(repo.root(), Path::new(""), &mut files);
    files
}

fn candidates(envelope: &Value) -> &Vec<Value> {
    envelope["data"]["candidates"]
        .as_array()
        .unwrap_or_else(|| panic!("data.candidates is an array: {envelope}"))
}

#[test]
fn harvest_candidates_never_decisions() {
    let repo = fixture();
    let curation_before = repo.read(".podras/curation.toml");
    let beads_before = repo.read(".beads/issues.jsonl");

    let envelope = harvest_json(&repo);

    assert_eq!(envelope["ok"].as_bool(), Some(true), "{envelope}");
    assert_eq!(envelope["envelope_kind"], "ok", "{envelope}");
    assert_eq!(
        envelope["data"]["schema_version"].as_str(),
        Some("harvest.v1"),
        "{envelope}"
    );

    let candidates = candidates(&envelope);
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
    }

    // Decisions stayed outside the read path: no curation, tracker, or focus
    // change, and no committed artifact was produced.
    assert_eq!(repo.read(".podras/curation.toml"), curation_before);
    assert_eq!(repo.read(".beads/issues.jsonl"), beads_before);
    let curation: toml::Value =
        toml::from_str(&curation_before).expect("curation fixture is valid TOML");
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

    // The human render groups by kind and states empty groups explicitly.
    let human = podras_in(&repo, &["harvest", "--as-of", AS_OF])
        .success()
        .get_output()
        .stdout
        .clone();
    let human = String::from_utf8(human).expect("harvest narrative is UTF-8");
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

    let quiet = common::TempRepo::new();
    quiet.beads_jsonl(&[("podras-abc", "Only linked work", "open")]);
    quiet.commit_at("2026-11-01T00:00:00Z", "podras-abc seed tracker");
    let quiet_human = podras_in(&quiet, &["harvest", "--as-of", AS_OF])
        .success()
        .get_output()
        .stdout
        .clone();
    let quiet_human = String::from_utf8(quiet_human).expect("harvest narrative is UTF-8");
    assert_eq!(
        quiet_human.matches("  none").count(),
        3,
        "empty groups are explicit, never omitted: {quiet_human}"
    );
}

#[test]
fn brief_out_dir_targets_a_directory() {
    let repo = fixture();
    let target = repo.root().join("renders/nested");

    podras_in(
        &repo,
        &["brief", "--write", "--out-dir", target.to_str().unwrap()],
    )
    .success();

    assert!(target.join("BRIEF.md").is_file(), "markdown pair member");
    assert!(target.join("BRIEF.json").is_file(), "json pair member");
    assert!(
        !repo.root().join("BRIEF.md").exists() && !repo.root().join("BRIEF.json").exists(),
        "an explicit target leaves the repository root untouched"
    );

    // The default remains the repository root.
    podras_in(&repo, &["brief", "--write"]).success();
    assert!(repo.root().join("BRIEF.md").is_file());
    assert!(repo.root().join("BRIEF.json").is_file());

    // `--out-dir` is meaningless without `--write`: a usage error, no write.
    let unwritten = repo.root().join("unwritten");
    podras_in(&repo, &["brief", "--out-dir", unwritten.to_str().unwrap()]).code(2);
    assert!(!unwritten.exists(), "a usage error writes nothing");
}

#[test]
fn harvest_idempotent_provenance() {
    let repo = fixture();
    let before = snapshot(&repo);

    let first = harvest_json(&repo);
    let second = harvest_json(&repo);

    let mut first_data = first["data"].clone();
    let mut second_data = second["data"].clone();
    first_data
        .as_object_mut()
        .expect("harvest payload is an object")
        .remove("generated_at");
    second_data
        .as_object_mut()
        .expect("harvest payload is an object")
        .remove("generated_at");
    assert_eq!(
        first_data, second_data,
        "identical inputs at one --as-of produce the same candidates"
    );

    let candidates = candidates(&second);
    assert_eq!(candidates.len(), 3, "{second}");
    assert_eq!(
        snapshot(&repo),
        before,
        "harvest changes no repository file between runs"
    );

    for candidate in candidates {
        for evidence in candidate["evidence"]
            .as_array()
            .expect("candidate carries evidence")
        {
            match evidence["kind"].as_str() {
                Some("git") => {
                    let sha = evidence["ref"].as_str().expect("git evidence ref");
                    let status = Command::new("git")
                        .args(["cat-file", "-e", &format!("{sha}^{{commit}}")])
                        .current_dir(repo.root())
                        .status()
                        .expect("run git in fixture");
                    assert!(status.success(), "git evidence `{sha}` resolves");
                }
                Some("notes") => {
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
                other => panic!("unexpected evidence kind: {other:?}"),
            }
        }
    }
}
