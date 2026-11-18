//! Purpose: light-read whisper session notes, latest branch entry only.
//!
//! Responsibilities:
//! - locate repo-local `.whisper/branches/<branch>/notes.md`, falling back to
//!   `main`, then the global `~/.whisper/repos/<repo>/branches/<slug>/` store;
//! - extract the latest entry timestamp and its first next-action line;
//! - hash the observed note bytes so freshness is comparable without content.
//!
//! Rationale: whisper content is private by default and expensive to mine;
//! v1 reads exactly one entry so the brief can stamp freshness without
//! importing session narrative into the committed render. Missing or
//! unreadable notes are values, never errors, so the brief always renders.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::brief::WhisperReading;
use crate::model::{SourceName, SourceState, SourceStatus, Staleness, Warning};
use crate::readers::{ReadContext, SourceRead};

/// Read the latest whisper branch entry for the repository.
pub fn read(context: &ReadContext) -> SourceRead<WhisperReading> {
    read_with_home(context, None)
}

/// Read the latest whisper branch entry, resolving the global store under
/// `home` when supplied; `None` means `$HOME`.
pub fn read_with_home(context: &ReadContext, home: Option<&Path>) -> SourceRead<WhisperReading> {
    let branch = context
        .branch
        .clone()
        .or_else(|| current_branch(&context.repo_root))
        .unwrap_or_else(|| "main".to_string());

    let mut candidates = Vec::new();
    candidates.push(
        context
            .repo_root
            .join(".whisper/branches")
            .join(&branch)
            .join("notes.md"),
    );
    if branch != "main" {
        candidates.push(context.repo_root.join(".whisper/branches/main/notes.md"));
    }

    if let Some(home) = home.map(Path::to_path_buf).or_else(default_home) {
        let repo_key = origin_repo_key(&context.repo_root)
            .unwrap_or_else(|| local_repo_key(&context.repo_root));
        let branch_slug = branch.replace('/', "--");
        let root = home.join(".whisper/repos").join(repo_key).join("branches");
        candidates.push(root.join(&branch_slug).join("notes.md"));
        if branch_slug != "main" {
            candidates.push(root.join("main").join("notes.md"));
        }
    }

    for path in candidates {
        match fs::read_to_string(&path) {
            Ok(content) => return present(&content),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return unreadable(&path, &error),
        }
    }

    missing(&branch)
}

fn present(content: &str) -> SourceRead<WhisperReading> {
    let (observed_at, next_action) = parse_latest_entry(content);
    SourceRead {
        data: Some(WhisperReading {
            observed_at,
            next_action,
            content_hash: Some(sha256_hex(content.as_bytes())),
        }),
        status: SourceStatus {
            name: SourceName::Whisper,
            status: SourceState::Present,
            observed_at,
            staleness: Staleness::Unknown,
            detail: None,
        },
        warnings: Vec::new(),
    }
}

fn unreadable(path: &Path, error: &std::io::Error) -> SourceRead<WhisperReading> {
    SourceRead {
        data: None,
        status: SourceStatus {
            name: SourceName::Whisper,
            status: SourceState::Unreadable,
            observed_at: None,
            staleness: Staleness::Unknown,
            detail: Some(format!("could not read {}", path.display())),
        },
        warnings: vec![Warning {
            source: SourceName::Whisper,
            code: "whisper_unreadable".to_string(),
            detail: error.to_string(),
        }],
    }
}

fn missing(branch: &str) -> SourceRead<WhisperReading> {
    SourceRead {
        data: None,
        status: SourceStatus {
            name: SourceName::Whisper,
            status: SourceState::Missing,
            observed_at: None,
            staleness: Staleness::Unknown,
            detail: None,
        },
        warnings: vec![Warning {
            source: SourceName::Whisper,
            code: "whisper_missing".to_string(),
            detail: format!("no whisper notes found for branch `{branch}`"),
        }],
    }
}

/// Parse the newest entry: its header timestamp and first next-action line.
///
/// Entries without a `- <RFC3339>` header still yield a next action when the
/// file carries a `## Next action` heading; the observation instant is `None`.
fn parse_latest_entry(content: &str) -> (Option<DateTime<Utc>>, Option<String>) {
    let lines: Vec<&str> = content.lines().collect();
    let headers: Vec<(usize, DateTime<Utc>)> = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| entry_timestamp(line).map(|stamp| (index, stamp)))
        .collect();

    if let Some(&(start, observed_at)) = headers.last() {
        let end = headers
            .iter()
            .find(|(index, _)| *index > start)
            .map(|(index, _)| *index)
            .unwrap_or(lines.len());
        return (Some(observed_at), next_action(&lines[start + 1..end]));
    }

    (None, next_action(&lines))
}

/// Parse a `- <RFC3339 timestamp> ...` entry header.
fn entry_timestamp(line: &str) -> Option<DateTime<Utc>> {
    let rest = line.trim_start().strip_prefix("- ")?;
    let token = rest.split_whitespace().next()?;
    DateTime::parse_from_rfc3339(token)
        .ok()
        .map(|stamp| stamp.with_timezone(&Utc))
}

/// First non-empty line after the first `## Next action` heading.
///
/// The action ends at the next heading: a `#`-prefixed line is a new section,
/// so a `## Next action` with no body before `## Validation` yields `None`.
fn next_action(lines: &[&str]) -> Option<String> {
    let heading = lines
        .iter()
        .position(|line| line.trim_start().starts_with("## Next action"))?;
    for line in &lines[heading + 1..] {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('#') {
            return None;
        }
        return Some(line.to_string());
    }
    None
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn current_branch(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if branch.is_empty() || branch == "HEAD" {
        None
    } else {
        Some(branch)
    }
}

fn origin_repo_key(repo_root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let url = String::from_utf8(output.stdout).ok()?;
    canonical_repo_key(url.trim())
}

/// Canonical `host/org/repo` key for ssh, scp-like, and URL origins.
fn canonical_repo_key(origin: &str) -> Option<String> {
    let origin = origin.trim().trim_end_matches('/');
    if origin.is_empty() {
        return None;
    }

    if let Some((_, remainder)) = origin.split_once("://") {
        let (authority, path) = remainder.split_once('/')?;
        let host = authority.rsplit('@').next().unwrap_or(authority);
        return join_repo_key(host, path);
    }

    let (authority, path) = origin.rsplit_once(':')?;
    let host = authority.rsplit('@').next().unwrap_or(authority);
    join_repo_key(host, path)
}

fn join_repo_key(host: &str, path: &str) -> Option<String> {
    let host = host.trim_matches('/');
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    if host.is_empty() || path.is_empty() {
        return None;
    }
    Some(format!("{host}/{path}"))
}

fn local_repo_key(repo_root: &Path) -> String {
    let name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo");
    format!("local/{name}")
}

fn default_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
