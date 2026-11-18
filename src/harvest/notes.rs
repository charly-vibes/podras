//! Purpose: derive unresolved-reference candidates from the latest note entry.
//!
//! Responsibilities:
//! - locate the newest note entry for the current branch, falling back to
//!   `main`, and parse its header timestamp and body;
//! - report tracker ids and commit tokens in that body that do not resolve;
//! - attach notes evidence naming the path and entry timestamp.
//!
//! Rationale: a note that names work which does not exist is the cheapest
//! mismatch to detect and the easiest to prove, because the evidence is the
//! note entry itself. Only commit-shaped tokens are probed, so prose can never
//! turn into a false positive.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::detectors::{Vocabulary, tokenize};
use super::git::{commit_exists, run_git};
use super::{Candidate, CandidateKind, Evidence, EvidenceKind};
use crate::brief::BriefInputs;

/// Repository-relative notes directory.
const NOTES_DIR: &str = ".whisper/branches";

/// Note-derived candidates from the latest branch entry.
pub(super) fn note_candidates(
    repo: &Path,
    inputs: &BriefInputs,
    vocabulary: &Vocabulary,
) -> Vec<Candidate> {
    if inputs.whisper.is_none() {
        return Vec::new();
    }
    let Some((path, timestamp, body)) = latest_note_entry(repo) else {
        return Vec::new();
    };
    let reference = format!("{}#{}", relative_path(repo, &path), timestamp);
    let notes_evidence = || {
        vec![Evidence {
            kind: EvidenceKind::Notes,
            reference: reference.clone(),
        }]
    };

    let mut candidates = Vec::new();
    for token in vocabulary.unresolved_tracker_tokens(&body) {
        candidates.push(Candidate {
            kind: CandidateKind::NoteReferenceUnresolved,
            summary: format!("note references unresolved tracker id `{token}`"),
            evidence: notes_evidence(),
        });
    }
    for token in unresolved_commit_tokens(repo, &body) {
        candidates.push(Candidate {
            kind: CandidateKind::NoteReferenceUnresolved,
            summary: format!("note references unresolved commit `{token}`"),
            evidence: notes_evidence(),
        });
    }
    candidates
}

/// Commit-shaped tokens in a note body that no git object resolves.
fn unresolved_commit_tokens(repo: &Path, body: &str) -> BTreeSet<String> {
    tokenize(body)
        .map(str::to_ascii_lowercase)
        .filter(|token| commit_shaped(token) && !commit_exists(repo, token))
        .collect()
}

/// A conservative commit token: 7–40 hex characters with at least one digit.
fn commit_shaped(token: &str) -> bool {
    (7..=40).contains(&token.len())
        && token.chars().all(|character| character.is_ascii_hexdigit())
        && token.chars().any(|character| character.is_ascii_digit())
}

/// The newest note entry on the current branch (falling back to `main`).
fn latest_note_entry(repo: &Path) -> Option<(PathBuf, String, String)> {
    let branch = current_branch(repo);
    let mut paths = vec![repo.join(NOTES_DIR).join(&branch).join("notes.md")];
    if branch != "main" {
        paths.push(repo.join(NOTES_DIR).join("main").join("notes.md"));
    }
    let path = paths.into_iter().find(|path| path.is_file())?;
    let content = std::fs::read_to_string(&path).ok()?;
    let (timestamp, body) = parse_latest_entry(&content)?;
    Some((path, timestamp, body))
}

/// Parse the last `- <RFC3339>` entry header and the body after it.
fn parse_latest_entry(content: &str) -> Option<(String, String)> {
    let lines: Vec<&str> = content.lines().collect();
    let (index, timestamp) = lines.iter().enumerate().rev().find_map(|(index, line)| {
        let rest = line.strip_prefix("- ")?;
        let timestamp = rest.split_whitespace().next()?;
        chrono::DateTime::parse_from_rfc3339(timestamp).ok()?;
        Some((index, timestamp.to_string()))
    })?;
    let body = lines[index + 1..].join("\n");
    Some((timestamp, body))
}

/// The branch whose notes are read, defaulting to `main`.
fn current_branch(repo: &Path) -> String {
    run_git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
        .filter(|branch| branch != "HEAD")
        .unwrap_or_else(|| "main".to_string())
}

fn relative_path(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}
