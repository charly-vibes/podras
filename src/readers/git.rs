//! Purpose: read git history and HEAD state.
//!
//! Responsibilities:
//! - report HEAD sha and recent commits with timestamps;
//! - report the observation time of the reading;
//! - degrade to `unreadable` with one warning when git cannot answer.
//!
//! Rationale: git is the mandatory evidence source for what actually changed;
//! the reader stays read-only and leaves interpretation to harvest/health.
//! No shell is involved: arguments are passed directly so repository paths and
//! commit subjects never become shell syntax. A repository without commits has
//! no resolvable HEAD and is therefore unreadable, exactly like a missing one.

use std::process::{Command, Output};

use chrono::{DateTime, Utc};

use crate::brief::GitSummary;
use crate::model::{RecentEvent, SourceName, SourceState, SourceStatus, Staleness, Warning};
use crate::readers::{ReadContext, SourceRead};

/// Maximum commits returned in one reading.
const MAX_COMMITS: usize = 200;

/// Warning code emitted when git cannot answer.
const GIT_UNREADABLE: &str = "git_unreadable";

/// Read git HEAD and recent commits for the repository in `context`.
pub fn read(context: &ReadContext) -> SourceRead<GitSummary> {
    let head_sha = match resolve_head(context) {
        Ok(sha) => sha,
        Err(reading) => return *reading,
    };
    let observed_at = match head_observed_at(context) {
        Ok(instant) => instant,
        Err(reading) => return *reading,
    };
    let recent = match recent_events(context) {
        Ok(recent) => recent,
        Err(reading) => return *reading,
    };

    present(head_sha, recent, observed_at)
}

/// Resolve HEAD's sha; resolving the branch proves HEAD is fully readable.
fn resolve_head(context: &ReadContext) -> std::result::Result<String, Box<SourceRead<GitSummary>>> {
    let head_sha = match run(context, &["rev-parse", "HEAD"]) {
        Ok(stdout) if !stdout.is_empty() => stdout,
        Ok(_) => {
            return Err(Box::new(unreadable(
                "git rev-parse HEAD produced no output".to_string(),
            )));
        }
        Err(detail) => return Err(Box::new(unreadable(detail))),
    };

    // The branch value itself is supplied by the caller in `context.branch` and
    // is not part of the summary; this call only proves the ref resolves.
    run(context, &["rev-parse", "--abbrev-ref", "HEAD"])
        .map_err(|detail| Box::new(unreadable(detail)))?;
    Ok(head_sha)
}

/// HEAD commit time, which is the observation time of the reading.
fn head_observed_at(
    context: &ReadContext,
) -> std::result::Result<DateTime<Utc>, Box<SourceRead<GitSummary>>> {
    match run(context, &["log", "-1", "--format=%cI"]) {
        Ok(stdout) => match parse_instant(&stdout) {
            Some(instant) => Ok(instant),
            None => Err(Box::new(unreadable(format!(
                "git log -1 --format=%cI returned an unparseable time: {stdout}"
            )))),
        },
        Err(detail) => Err(Box::new(unreadable(detail))),
    }
}

/// Recent commits in one reading.
fn recent_events(
    context: &ReadContext,
) -> std::result::Result<Vec<RecentEvent>, Box<SourceRead<GitSummary>>> {
    let max_count = format!("--max-count={MAX_COMMITS}");
    match run(context, &["log", &max_count, "--format=%H%x1f%cI%x1f%s"]) {
        Ok(stdout) => Ok(parse_recent(&stdout)),
        Err(detail) => Err(Box::new(unreadable(detail))),
    }
}

/// The readable declaration for a successful reading.
fn present(
    head_sha: String,
    recent: Vec<RecentEvent>,
    observed_at: DateTime<Utc>,
) -> SourceRead<GitSummary> {
    SourceRead {
        data: Some(GitSummary {
            head_sha: Some(head_sha),
            recent,
            observed_at: Some(observed_at),
        }),
        status: SourceStatus {
            name: SourceName::Git,
            status: SourceState::Present,
            observed_at: Some(observed_at),
            staleness: Staleness::Unknown,
            detail: None,
        },
        warnings: Vec::new(),
    }
}

/// Run git in the repository root without a shell, returning trimmed stdout.
fn run(context: &ReadContext, args: &[&str]) -> std::result::Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(&context.repo_root)
        .output()
        .map_err(|error| format!("git {} could not run: {error}", args.join(" ")))?;
    if !output.status.success() {
        return Err(failure_detail(args, &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Build the warning detail from a failed git invocation, carrying its stderr.
fn failure_detail(args: &[&str], output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        match output.status.code() {
            Some(code) => format!("git {} exited {code} with no stderr", args.join(" ")),
            None => format!("git {} was terminated by a signal", args.join(" ")),
        }
    } else {
        format!("git {} failed: {stderr}", args.join(" "))
    }
}

/// Parse `%H%x1f%cI%x1f%s` output into events sorted newest first.
///
/// Git already walks history newest first, but committer dates can disagree
/// with walk order; sorting defensively keeps `recent` ordered by (committed_at
/// descending, sha ascending) as the brief contract requires. Lines that do
/// not parse are skipped rather than aborting the reading.
fn parse_recent(stdout: &str) -> Vec<RecentEvent> {
    let mut events: Vec<RecentEvent> = stdout.lines().filter_map(parse_event).collect();
    events.sort_by(|left, right| {
        right
            .committed_at
            .cmp(&left.committed_at)
            .then_with(|| left.sha.cmp(&right.sha))
    });
    events
}

/// Parse one log line, tolerating separators inside the subject.
fn parse_event(line: &str) -> Option<RecentEvent> {
    let mut fields = line.splitn(3, '\x1f');
    let sha = fields.next()?;
    let committed_at = fields.next()?;
    let subject = fields.next()?;
    if sha.is_empty() {
        return None;
    }
    Some(RecentEvent {
        sha: sha.to_string(),
        committed_at: parse_instant(committed_at)?,
        subject: subject.to_string(),
        outcome: None,
    })
}

/// Parse an RFC 3339 git timestamp into UTC.
fn parse_instant(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|instant| instant.with_timezone(&Utc))
}

/// One unreadable reading: no data, an unreadable row, and its warning.
fn unreadable(detail: String) -> SourceRead<GitSummary> {
    SourceRead {
        data: None,
        status: SourceStatus {
            name: SourceName::Git,
            status: SourceState::Unreadable,
            observed_at: None,
            staleness: Staleness::Unknown,
            detail: Some(detail.clone()),
        },
        warnings: vec![Warning {
            source: SourceName::Git,
            code: GIT_UNREADABLE.to_string(),
            detail,
        }],
    }
}
