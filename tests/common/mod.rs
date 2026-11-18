//! Purpose: deterministic fixture-repository helpers for integration tests.
//!
//! Responsibilities:
//! - create an initialized git repository with fixed commit identity and dates;
//! - write beads exports, curation, whisper notes, and spec changes;
//! - run git commands against the fixture.
//!
//! Rationale: brief output must be reproducible, so fixtures never read the
//! wall clock or the developer's identity.

#![allow(dead_code)]

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

/// Fixed instant used for every fixture timestamp.
const FIXED_DATE: &str = "2026-11-01T00:00:00Z";

/// A temporary git repository seeded for brief tests.
pub struct TempRepo {
    dir: TempDir,
}

impl TempRepo {
    /// Create a tempdir with `main` checked out and one initial commit.
    pub fn new() -> Self {
        let repo = Self {
            dir: tempfile::tempdir().expect("create temporary repository directory"),
        };
        repo.git(&["init", "--initial-branch=main"]);
        repo.git(&["config", "user.email", "podras@tests.invalid"]);
        repo.git(&["config", "user.name", "Podras Tests"]);
        repo.write(".gitkeep", "");
        repo.commit("initial commit");
        repo
    }

    /// Repository root path.
    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    /// Write a repository-relative file, creating parent directories.
    pub fn write(&self, rel: &str, content: &str) {
        let path = self.root().join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent directory");
        }
        fs::write(&path, content).expect("write fixture file");
    }

    /// Read a repository-relative file.
    pub fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.root().join(rel)).expect("read fixture file")
    }

    /// Run git in the fixture with fixed author/committer dates.
    pub fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.root())
            .env("GIT_AUTHOR_DATE", FIXED_DATE)
            .env("GIT_COMMITTER_DATE", FIXED_DATE)
            .output()
            .expect("run git in fixture repository");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            output.status.success(),
            "git {args:?} failed: {stderr}"
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Stage everything and commit with the given message.
    pub fn commit(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-m", message]);
    }

    /// Write `.beads/issues.jsonl` from `(id, title, status)` rows.
    pub fn beads_jsonl(&self, rows: &[(&str, &str, &str)]) {
        let mut body = String::new();
        for (id, title, status) in rows {
            let issue = serde_json::json!({
                "_type": "issue",
                "id": id,
                "title": title,
                "description": "",
                "status": status,
                "priority": 1,
                "issue_type": "task",
                "owner": "podras@tests.invalid",
                "created_at": FIXED_DATE,
                "created_by": "Podras Tests",
                "updated_at": FIXED_DATE,
                "metadata": {},
                "labels": [],
                "dependencies": [],
                "comments": []
            });
            body.push_str(&serde_json::to_string(&issue).expect("serialize issue fixture"));
            body.push('\n');
        }
        self.write(".beads/issues.jsonl", &body);
    }

    /// Write `.podras/curation.toml`.
    pub fn curation(&self, toml: &str) {
        self.write(".podras/curation.toml", toml);
    }

    /// Write a whisper branch note at `.whisper/branches/<topic>/notes.md`.
    pub fn whisper_branch(&self, topic: &str, text: &str) {
        let content = format!("- {FIXED_DATE} [id:fixture] (#{topic}) Format: 1\n  {text}\n");
        self.write(&format!(".whisper/branches/{topic}/notes.md"), &content);
    }

    /// Write an OpenSpec change at `openspec/changes/<name>/proposal.md`.
    pub fn spec_change(&self, name: &str, proposal: &str) {
        self.write(&format!("openspec/changes/{name}/proposal.md"), proposal);
    }
}

impl Default for TempRepo {
    fn default() -> Self {
        Self::new()
    }
}