//! Purpose: derive orphan-commit candidates and run bounded git queries.
//!
//! Responsibilities:
//! - select window commits that name no tracker id and touch no open path;
//! - attach the full commit sha as evidence;
//! - wrap the read-only git calls (`log`, `rev-parse`) shared by detectors.
//!
//! Rationale: a commit nobody can link to work is a real mismatch, but only in
//! the same fourteen-day window the health signals use. Arguments are passed
//! directly, never through a shell, so subject text is inert.

use std::path::Path;
use std::process::Command;

use super::detectors::{Vocabulary, evaluation_date, within_window};
use super::{Candidate, CandidateKind, Evidence, EvidenceKind};
use crate::brief::BriefInputs;
use crate::model::RecentEvent;

/// Window commits that name no tracker id and touch no open work path.
pub(super) fn orphan_commit_candidates(
    inputs: &BriefInputs,
    vocabulary: &Vocabulary,
) -> Vec<Candidate> {
    let Some(git) = &inputs.git else {
        return Vec::new();
    };
    let evaluation = evaluation_date(inputs);
    git.recent
        .iter()
        .filter(|event| within_window(event.committed_at.date_naive(), evaluation))
        .filter(|event| {
            !vocabulary.names_tracker(&event.subject) && !vocabulary.touches_path(&event.subject)
        })
        .map(orphan_candidate)
        .collect()
}

/// One orphan-commit candidate carrying the full sha.
fn orphan_candidate(event: &RecentEvent) -> Candidate {
    Candidate {
        kind: CandidateKind::OrphanCommit,
        summary: format!(
            "commit {} `{}` names no tracker id",
            short_sha(&event.sha),
            event.subject
        ),
        evidence: vec![Evidence {
            kind: EvidenceKind::Git,
            reference: event.sha.clone(),
        }],
    }
}

fn short_sha(sha: &str) -> &str {
    &sha[..sha.len().min(7)]
}

/// The newest commit touching `path`, read-only and bounded to one query.
pub(super) fn commit_touching(repo: &Path, path: &str) -> Option<String> {
    run_git(repo, &["log", "-1", "--format=%H", "--", path]).filter(|sha| !sha.is_empty())
}

/// Whether `token` resolves to a commit object.
pub(super) fn commit_exists(repo: &Path, token: &str) -> bool {
    Command::new("git")
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{token}^{{commit}}"),
        ])
        .current_dir(repo)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Run git in the repository without a shell, returning trimmed stdout.
pub(super) fn run_git(repo: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(repo)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
