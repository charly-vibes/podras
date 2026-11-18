//! Purpose: prove the git reader reports HEAD, recent commits, and observation time.
//!
//! Responsibilities:
//! - assert HEAD sha matches `git rev-parse HEAD` in a fixture repository;
//! - assert commits parse and sort newest first, independent of walk order;
//! - assert `observed_at` is the HEAD commit time;
//! - assert a non-repository root degrades to `unreadable` with one warning.
//!
//! Rationale: git is the mandatory evidence source, so its reading must be
//! exact and its failure must stay a status row rather than a panic.

mod common;

use std::process::Command;

use chrono::{DateTime, Utc};
use podras::model::{SourceName, SourceState};
use podras::readers::ReadContext;
use podras::readers::git;

use common::TempRepo;

fn instant(raw: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(raw)
        .expect("fixture instants are valid")
        .with_timezone(&Utc)
}

fn commit_at(repo: &TempRepo, date: &str, message: &str) {
    repo.write("work.txt", message);
    repo.git(&["add", "-A"]);
    let output = Command::new("git")
        .args(["-c", "commit.gpgsign=false", "commit", "-m", message])
        .current_dir(repo.root())
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("run fixed-date commit");
    assert!(
        output.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn context(repo_root: &std::path::Path) -> ReadContext {
    ReadContext {
        repo_root: repo_root.to_path_buf(),
        as_of: None,
        branch: None,
    }
}

#[test]
fn reads_head_recent_commits_and_observed_at() {
    let repo = TempRepo::new();
    // Commit order deliberately disagrees with timestamp order so the reader's
    // own sort, not `git log`'s walk, decides the recent ordering.
    commit_at(&repo, "2026-11-03T12:00:00Z", "later change");
    commit_at(&repo, "2026-11-02T12:00:00Z", "middle change");

    let expected_head = repo.git(&["rev-parse", "HEAD"]);
    let read = git::read(&context(repo.root()));

    assert!(read.warnings.is_empty(), "warnings: {:?}", read.warnings);
    assert_eq!(read.status.name, SourceName::Git);
    assert_eq!(read.status.status, SourceState::Present);

    let summary = read.data.expect("fixture repository is readable");
    assert_eq!(summary.head_sha.as_deref(), Some(expected_head.as_str()));
    assert_eq!(summary.observed_at, Some(instant("2026-11-02T12:00:00Z")));
    assert_eq!(read.status.observed_at, summary.observed_at);

    let subjects: Vec<&str> = summary
        .recent
        .iter()
        .map(|event| event.subject.as_str())
        .collect();
    assert_eq!(
        subjects,
        vec!["later change", "middle change", "initial commit"]
    );
    assert_eq!(
        summary.recent[0].committed_at,
        instant("2026-11-03T12:00:00Z")
    );
    assert_eq!(
        summary.recent[1].committed_at,
        instant("2026-11-02T12:00:00Z")
    );
    assert_eq!(summary.recent[0].sha.len(), 40);
    assert!(summary.recent.iter().all(|event| event.outcome.is_none()));
}

#[test]
fn non_repository_is_unreadable_without_panic() {
    let dir = tempfile::tempdir().expect("create non-repository tempdir");
    let read = git::read(&context(dir.path()));

    assert!(read.data.is_none());
    assert_eq!(read.status.name, SourceName::Git);
    assert_eq!(read.status.status, SourceState::Unreadable);
    assert_eq!(read.warnings.len(), 1);

    let warning = &read.warnings[0];
    assert_eq!(warning.source, SourceName::Git);
    assert_eq!(warning.code, "git_unreadable");
    assert!(
        warning.detail.to_lowercase().contains("fatal"),
        "detail should carry git stderr: {}",
        warning.detail
    );
}
