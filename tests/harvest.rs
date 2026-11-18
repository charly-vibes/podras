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

use common::TempRepo;

#[path = "harvest/fixtures.rs"]
mod fixtures;

use std::fs;
use std::path::Path;

use podras::readers::{ReadContext, collect_with_home};

use fixtures::{
    assert_candidate_shape, assert_evidence_resolves, assert_human_render, assert_no_decisions,
    candidates, fixture, harvest_json, podras_in, snapshot,
};

#[test]
fn harvest_candidates_never_decisions() {
    let (repo, curation_before, beads_before) = fixture_with_snapshots();
    let envelope = harvest_json(&repo);
    let candidates = envelope["data"]["candidates"]
        .as_array()
        .expect("harvest envelope carries a candidates array");
    assert!(!candidates.is_empty(), "the fixture yields candidates");
    assert_candidate_shape(&envelope);
    assert_no_decisions(&repo, &curation_before, &beads_before);
    assert_human_render(&repo);
}

fn fixture_with_snapshots() -> (TempRepo, String, String) {
    let repo = fixture();
    let curation = repo.read(".podras/curation.toml");
    let beads = repo.read(".beads/issues.jsonl");
    (repo, curation, beads)
}

#[test]
fn brief_out_dir_targets_a_directory() {
    let repo = fixture();

    // `--out-dir` targets an existing directory: a missing target is a hard
    // error that writes nothing, here or anywhere else.
    let missing = repo.root().join("renders/nested");
    podras_in(
        &repo,
        &["brief", "--write", "--out-dir", missing.to_str().unwrap()],
    )
    .code(1);
    assert!(
        !missing.exists(),
        "a missing target directory is never created"
    );
    assert!(
        !repo.root().join("BRIEF.md").exists() && !repo.root().join("BRIEF.json").exists(),
        "a rejected target writes nothing at all"
    );

    let target = repo.root().join("renders");
    std::fs::create_dir(&target).expect("create the render target directory");

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

/// Notes that live only in the global whisper store must still become
/// candidates: harvest consumes the reader's latest entry instead of
/// re-deriving a repository-local note.
#[test]
fn global_note_store_supplies_note_candidates() {
    let (repo, home, context) = repo_with_global_note_only();
    let inputs = collect_with_home(&context, Some(home.path())).expect("collect fixture inputs");
    assert!(
        inputs
            .whisper
            .as_ref()
            .and_then(|whisper| whisper.latest_entry.as_ref())
            .is_some(),
        "the reader resolved the global note store"
    );

    let harvest = podras::harvest::assemble(repo.root(), &inputs);
    let notes_candidates: Vec<_> = harvest
        .candidates
        .iter()
        .filter(|candidate| candidate.kind.as_str() == "note_reference_unresolved")
        .collect();

    assert_eq!(
        notes_candidates.len(),
        1,
        "the global note reference yields one candidate: {:?}",
        harvest.candidates
    );
    let candidate = notes_candidates[0];
    assert!(
        candidate.summary.contains("podras-zzz"),
        "summary names the unresolved id: {}",
        candidate.summary
    );
    assert_eq!(candidate.evidence.len(), 1);
    assert_eq!(candidate.evidence[0].kind.as_str(), "notes");
    assert!(
        candidate.evidence[0]
            .reference
            .ends_with("notes.md#2026-11-18T09:00:00Z"),
        "notes evidence names the entry: {}",
        candidate.evidence[0].reference
    );
    let (path, _) = candidate.evidence[0]
        .reference
        .rsplit_once('#')
        .expect("notes evidence is `<path>#<entry timestamp>`");
    assert!(
        Path::new(path).is_file(),
        "notes evidence resolves to the global note file: {path}"
    );
}

#[test]
fn harvest_idempotent_provenance() {
    let repo = fixture();
    let before = snapshot(&repo);

    let first = harvest_json(&repo);
    let second = harvest_json(&repo);

    let mut first_data = first["data"].clone();
    let mut second_data = second["data"].clone();
    for payload in [&mut first_data, &mut second_data] {
        payload
            .as_object_mut()
            .expect("harvest payload is an object")
            .remove("generated_at");
    }
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
        assert_evidence_resolves(&repo, candidate);
    }
}

/// Fixture whose notes exist only under a temporary global whisper store.
fn repo_with_global_note_only() -> (TempRepo, tempfile::TempDir, ReadContext) {
    let repo = fixture();
    repo.git(&[
        "remote",
        "add",
        "origin",
        "git@example.invalid:org/repo.git",
    ]);
    fs::remove_file(repo.root().join(".whisper/branches/main/notes.md"))
        .expect("the fixture note is removed so only the global store remains");
    let home = tempfile::tempdir().expect("create temporary home");
    let notes = home
        .path()
        .join(".whisper/repos/example.invalid/org/repo/branches/main/notes.md");
    fs::create_dir_all(notes.parent().expect("note parent")).expect("create global note dir");
    fs::write(
        &notes,
        "- 2026-11-18T09:00:00Z [id:global] (#main) Format: 1\n  See podras-zzz for background.\n",
    )
    .expect("write global note");
    let context = ReadContext {
        repo_root: repo.root().to_path_buf(),
        as_of: Some(
            chrono::NaiveDate::from_ymd_opt(2026, 11, 18).expect("valid fixture evaluation date"),
        ),
        branch: Some("main".to_string()),
    };
    (repo, home, context)
}
