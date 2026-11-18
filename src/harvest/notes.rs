//! Purpose: derive unresolved-reference candidates from the reader's latest note entry.
//!
//! Responsibilities:
//! - take the newest whisper entry already located by `readers::whisper`;
//! - report tracker ids and commit tokens in that body that do not resolve;
//! - attach notes evidence naming the path and entry timestamp.
//!
//! Rationale: a note that names work which does not exist is the cheapest
//! mismatch to detect and the easiest to prove, because the evidence is the
//! note entry itself. Discovery belongs to the reader, which also resolves the
//! global fallback store, so harvest never re-derives note paths or parses the
//! file a second time. Only commit-shaped tokens are probed, so prose can never
//! turn into a false positive.

use std::collections::BTreeSet;
use std::path::Path;

use super::detectors::Vocabulary;
use super::git::commit_exists;
use super::{Candidate, CandidateKind, Evidence, EvidenceKind};
use crate::brief::BriefInputs;
use crate::health::links::tokenize;

/// Note-derived candidates from the reader's latest branch entry.
pub(super) fn note_candidates(
    repo: &Path,
    inputs: &BriefInputs,
    vocabulary: &Vocabulary,
) -> Vec<Candidate> {
    let Some(entry) = inputs
        .whisper
        .as_ref()
        .and_then(|whisper| whisper.latest_entry.as_ref())
    else {
        return Vec::new();
    };
    let reference = format!("{}#{}", relative_path(repo, &entry.path), entry.timestamp);
    let notes_evidence = || {
        vec![Evidence {
            kind: EvidenceKind::Notes,
            reference: reference.clone(),
        }]
    };

    let mut candidates = Vec::new();
    for token in vocabulary.unresolved_tracker_tokens(&entry.body) {
        candidates.push(Candidate {
            kind: CandidateKind::NoteReferenceUnresolved,
            summary: format!("note references unresolved tracker id `{token}`"),
            evidence: notes_evidence(),
        });
    }
    for token in unresolved_commit_tokens(repo, &entry.body) {
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

fn relative_path(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}
