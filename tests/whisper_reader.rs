//! Purpose: prove the whisper reader extracts only the latest branch entry.
//!
//! Responsibilities:
//! - read the newest entry header timestamp and its next-action line;
//! - report a missing note as a status row plus warning, never an error;
//! - resolve the global fallback through an explicit home root.
//!
//! Rationale: whisper content is private and unbounded, so the reader must be
//! line-precise. Tests pin the exact timestamp, one action line, and the hash
//! of the observed bytes so a parser that returns whole notes fails loudly.

mod common;

use std::fs;

use chrono::{DateTime, Utc};
use podras::model::{SourceName, SourceState, Staleness};
use podras::readers::ReadContext;
use podras::readers::whisper::{read, read_with_home};
use sha2::{Digest, Sha256};

use common::TempRepo;

/// Two entries: the first must never leak into the reading.
const TWO_ENTRIES: &str = "\
- 2026-11-17T10:00:00Z [id:old] (#topic) Format: 1
  ## Next action
  old action that must not surface

- 2026-11-18T12:34:56Z [id:new] (#topic) Format: 1
  ## Next action
  ship the whisper reader

  ## Validation
  cargo test --test whisper_reader
";

fn context(repo: &TempRepo, branch: Option<&str>) -> ReadContext {
    ReadContext {
        repo_root: repo.root().to_path_buf(),
        as_of: None,
        branch: branch.map(str::to_string),
    }
}

fn instant(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .expect("fixture timestamps are RFC3339")
        .with_timezone(&Utc)
}

fn sha256_hex(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[test]
fn latest_entry_supplies_observed_at_and_one_next_action() {
    let repo = TempRepo::new();
    repo.write(".whisper/branches/main/notes.md", TWO_ENTRIES);

    let result = read(&context(&repo, Some("main")));

    assert_eq!(result.status.name, SourceName::Whisper);
    assert_eq!(result.status.status, SourceState::Present);
    assert_eq!(result.status.staleness, Staleness::Unknown);
    assert!(result.warnings.is_empty());

    let reading = result.data.expect("present whisper reading");
    assert_eq!(reading.observed_at, Some(instant("2026-11-18T12:34:56Z")));
    assert_eq!(
        reading.next_action.as_deref(),
        Some("ship the whisper reader")
    );
    assert_eq!(
        reading.content_hash.as_deref(),
        Some(sha256_hex(TWO_ENTRIES).as_str())
    );
    let entry = reading
        .latest_entry
        .as_ref()
        .expect("the newest timestamped entry is retained for harvest");
    assert_eq!(
        entry.path,
        repo.root().join(".whisper/branches/main/notes.md")
    );
    assert_eq!(entry.timestamp, "2026-11-18T12:34:56Z");
    assert!(
        entry.body.contains("ship the whisper reader"),
        "body holds the newest entry: {entry:?}"
    );
    assert!(
        !entry.body.contains("old action that must not surface"),
        "body never includes an older entry: {entry:?}"
    );
}

#[test]
fn next_action_never_returns_a_heading() {
    let repo = TempRepo::new();
    let note = "\
- 2026-11-18T12:34:56Z [id:headed] (#topic) Format: 1
  ## Next action

  ## Validation
  cargo test --test whisper_reader
";
    repo.write(".whisper/branches/main/notes.md", note);

    let result = read(&context(&repo, Some("main")));

    let reading = result.data.expect("present whisper reading");
    assert_eq!(reading.observed_at, Some(instant("2026-11-18T12:34:56Z")));
    assert_eq!(
        reading.next_action, None,
        "a following heading ends the action section"
    );
}

#[test]
fn missing_note_is_a_status_row_and_warning() {
    let repo = TempRepo::new();

    let result = read(&context(&repo, Some("main")));

    assert!(result.data.is_none());
    assert_eq!(result.status.name, SourceName::Whisper);
    assert_eq!(result.status.status, SourceState::Missing);
    assert_eq!(result.warnings.len(), 1);
    assert_eq!(result.warnings[0].source, SourceName::Whisper);
    assert_eq!(result.warnings[0].code, "whisper_missing");
}

#[test]
fn global_fallback_resolves_origin_and_slugs_branch() {
    let repo = TempRepo::new();
    repo.git(&[
        "remote",
        "add",
        "origin",
        "git@example.invalid:org/repo.git",
    ]);

    let home = tempfile::tempdir().expect("create temporary home");
    let note = "\
- 2026-11-18T09:00:00Z [id:global] (#feature) Format: 1
  ## Next action
  global fallback action
";
    let notes = home
        .path()
        .join(".whisper/repos/example.invalid/org/repo/branches/feature--x/notes.md");
    fs::create_dir_all(notes.parent().expect("note parent")).expect("create global note dir");
    fs::write(&notes, note).expect("write global note");

    let result = read_with_home(&context(&repo, Some("feature/x")), Some(home.path()));

    assert_eq!(result.status.status, SourceState::Present);
    let reading = result.data.expect("global whisper reading");
    assert_eq!(reading.observed_at, Some(instant("2026-11-18T09:00:00Z")));
    assert_eq!(
        reading.next_action.as_deref(),
        Some("global fallback action")
    );
    assert_eq!(
        reading.content_hash.as_deref(),
        Some(sha256_hex(note).as_str())
    );
    let entry = reading
        .latest_entry
        .as_ref()
        .expect("global fallback retains the latest entry");
    assert_eq!(entry.path, notes);
    assert_eq!(entry.timestamp, "2026-11-18T09:00:00Z");
    assert!(entry.body.contains("global fallback action"));
}
