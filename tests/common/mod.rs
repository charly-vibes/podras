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
        self.git_at(FIXED_DATE, args)
    }

    /// Run git in the fixture with an explicit author/committer date.
    pub fn git_at(&self, instant: &str, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.root())
            .env("GIT_AUTHOR_DATE", instant)
            .env("GIT_COMMITTER_DATE", instant)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .expect("run git in fixture repository");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "git {args:?} failed: {stderr}");
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Stage everything and commit with the given message.
    pub fn commit(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-m", message]);
    }

    /// Stage everything and commit at an explicit committer date.
    pub fn commit_at(&self, instant: &str, message: &str) {
        self.git_at(instant, &["add", "-A"]);
        self.git_at(instant, &["commit", "-m", message]);
    }

    /// Write `.beads/issues.jsonl` from `(id, title, status)` rows.
    pub fn beads_jsonl(&self, rows: &[(&str, &str, &str)]) {
        self.beads_jsonl_at(rows, &[], FIXED_DATE);
    }

    /// Write `.beads/issues.jsonl` with blocker edges and a chosen update instant.
    ///
    /// Each `(issue, blocker)` pair becomes a `blocks` dependency; a blocked
    /// issue is only blocked while `blocker` itself remains open.
    pub fn beads_jsonl_at(
        &self,
        rows: &[(&str, &str, &str)],
        blockers: &[(&str, &str)],
        updated_at: &str,
    ) {
        let mut body = String::new();
        for (id, title, status) in rows {
            let dependencies: Vec<serde_json::Value> = blockers
                .iter()
                .filter(|(issue, _)| issue == id)
                .map(|(_, blocker)| {
                    serde_json::json!({
                        "issue_id": id,
                        "depends_on_id": blocker,
                        "type": "blocks",
                        "created_at": updated_at,
                        "created_by": "Podras Tests",
                    })
                })
                .collect();
            let issue = serde_json::json!({
                "_type": "issue",
                "id": id,
                "title": title,
                "description": "",
                "status": status,
                "priority": 1,
                "issue_type": "task",
                "owner": "podras@tests.invalid",
                "created_at": updated_at,
                "created_by": "Podras Tests",
                "updated_at": updated_at,
                "metadata": {},
                "labels": [],
                "dependencies": dependencies,
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

    /// Write a whisper branch note with a `## Next action` block.
    pub fn whisper_branch_action(&self, topic: &str, text: &str, next_action: &str) {
        let content = format!(
            "- {FIXED_DATE} [id:fixture] (#{topic}) Format: 1\n  {text}\n\n## Next action\n{next_action}\n"
        );
        self.write(&format!(".whisper/branches/{topic}/notes.md"), &content);
    }

    /// Set one file's modified time to a deterministic RFC 3339 instant.
    pub fn set_mtime(&self, rel: &str, instant: &str) {
        use std::time::{Duration, UNIX_EPOCH};

        let parsed =
            chrono::DateTime::parse_from_rfc3339(instant).expect("fixture mtime is RFC 3339");
        let seconds = u64::try_from(parsed.timestamp()).expect("fixture mtime is after the epoch");
        let stamp = UNIX_EPOCH + Duration::from_secs(seconds);
        let file = fs::OpenOptions::new()
            .write(true)
            .open(self.root().join(rel))
            .expect("open fixture file to stamp mtime");
        file.set_times(fs::FileTimes::new().set_modified(stamp))
            .expect("set fixture file mtime");
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
