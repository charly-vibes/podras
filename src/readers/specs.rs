//! Purpose: observe the OpenSpec change store.
//!
//! Responsibilities:
//! - enumerate active changes and declared specs as work items, listing each
//!   subtree independently so one absent subtree never hides the other;
//! - count changes whose newest edit is past the stale horizon;
//! - report the newest observed mtime and a best-effort status row.
//!
//! Rationale: the spec store is a best-effort source. Its absence must only
//! produce a missing-source warning, so this reader returns a `SourceRead` and
//! never an error: unreadable files are skipped and only the H1 title is
//! parsed, keeping the observation cheap and dependency-free.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::{DateTime, NaiveDate, Utc};

use crate::brief::SpecSummary;
use crate::model::{SourceName, SourceState, SourceStatus, Staleness, Warning, WorkItem};
use crate::readers::{ReadContext, SourceRead};

/// Age in days past which an unedited change counts as stale.
const STALE_AFTER_DAYS: i64 = 14;

/// Archived changes are history, not active work.
const ARCHIVE_DIR: &str = "archive";

/// Change titles come from `proposal.md`, falling back to `spec.md`.
const CHANGE_TITLES: (&str, &str) = ("proposal.md", "spec.md");

/// Spec titles come from `spec.md`, falling back to `proposal.md`.
const SPEC_TITLES: (&str, &str) = ("spec.md", "proposal.md");

/// Read the OpenSpec change store for the repository.
pub fn read(context: &ReadContext) -> SourceRead<SpecSummary> {
    let openspec = context.repo_root.join("openspec");
    let as_of = context.as_of.unwrap_or_else(|| Utc::now().date_naive());
    let Some(store) = open_store(&openspec) else {
        return missing();
    };

    let changes = observe(store.changes, "change", CHANGE_TITLES, true, as_of);
    let specs = observe(store.specs, "spec", SPEC_TITLES, false, as_of);

    let stale_count = changes.stale_count + specs.stale_count;
    let observed_at = newest(changes.observed_at, specs.observed_at);
    let items = changes.items.into_iter().chain(specs.items).collect();

    present(items, stale_count, observed_at)
}

/// The two store subtrees, split by whether their entries count as stale.
struct Store {
    changes: Vec<PathBuf>,
    specs: Vec<PathBuf>,
}

/// Locate both subtrees, each listed independently.
///
/// `None` only when the `openspec` root itself is absent or unlistable: an
/// absent, unlistable, or empty subtree contributes no entries, so a
/// specs-only or changes-only store still reads as present.
fn open_store(openspec: &Path) -> Option<Store> {
    fs::read_dir(openspec).ok()?;
    Some(Store {
        changes: child_directories(&openspec.join("changes"), true).unwrap_or_default(),
        specs: child_directories(&openspec.join("specs"), false).unwrap_or_default(),
    })
}

/// One subtree's contribution to the summary.
#[derive(Default)]
struct Observed {
    items: Vec<WorkItem>,
    stale_count: usize,
    observed_at: Option<DateTime<Utc>>,
}

/// Observe every directory in one subtree, keeping the newest mtime.
fn observe(
    directories: Vec<PathBuf>,
    status: &str,
    titles: (&str, &str),
    stale_eligible: bool,
    as_of: NaiveDate,
) -> Observed {
    let mut observed = Observed::default();
    for directory in directories {
        let newest = newest_mtime(&directory);
        if let Some(mtime) = newest {
            observed.observed_at = Some(
                observed
                    .observed_at
                    .map_or(mtime, |current| current.max(mtime)),
            );
        }
        let age_days = newest.map(|mtime| age_days(as_of, mtime));
        if stale_eligible && age_days.is_some_and(|days| days > STALE_AFTER_DAYS as u32) {
            observed.stale_count += 1;
        }
        observed
            .items
            .push(entry_item(&directory, status, titles, age_days));
    }
    observed
}

/// The work item for one change or spec directory.
fn entry_item(
    directory: &Path,
    status: &str,
    titles: (&str, &str),
    age_days: Option<u32>,
) -> WorkItem {
    let (primary, fallback) = titles;
    WorkItem {
        id: directory_name(directory),
        title: first_h1(&directory.join(primary)).or_else(|| first_h1(&directory.join(fallback))),
        status: Some(status.to_string()),
        priority: None,
        blocked_by: Vec::new(),
        age_days,
        reason: None,
        confirmed_at: None,
    }
}

/// Newest of two optionally observed mtimes.
fn newest(left: Option<DateTime<Utc>>, right: Option<DateTime<Utc>>) -> Option<DateTime<Utc>> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (left, right) => left.or(right),
    }
}

/// The readable declaration for an observed store.
fn present(
    changes: Vec<WorkItem>,
    stale_count: usize,
    observed_at: Option<DateTime<Utc>>,
) -> SourceRead<SpecSummary> {
    SourceRead {
        data: Some(SpecSummary {
            changes,
            stale_count,
            observed_at,
        }),
        status: SourceStatus {
            name: SourceName::Openspec,
            status: SourceState::Present,
            observed_at,
            staleness: Staleness::Unknown,
            detail: None,
        },
        warnings: Vec::new(),
    }
}

/// The declaration used when the store is absent or unreadable.
fn missing() -> SourceRead<SpecSummary> {
    let detail = "openspec/ directory is missing or unreadable".to_string();
    SourceRead {
        data: None,
        status: SourceStatus {
            name: SourceName::Openspec,
            status: SourceState::Missing,
            observed_at: None,
            staleness: Staleness::Unknown,
            detail: Some(detail.clone()),
        },
        warnings: vec![Warning {
            source: SourceName::Openspec,
            code: "openspec_missing".to_string(),
            detail,
        }],
    }
}

/// Directories directly below `parent`, sorted by name.
///
/// `None` means the parent itself could not be listed; `skip_archive` drops the
/// `changes/archive` subtree because archived changes are not active work.
fn child_directories(parent: &Path, skip_archive: bool) -> Option<Vec<PathBuf>> {
    let entries = fs::read_dir(parent).ok()?;
    let mut directories: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| {
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");
            !name.starts_with('.') && !(skip_archive && name == ARCHIVE_DIR)
        })
        .collect();
    directories.sort();
    Some(directories)
}

/// Directory name used as the work-item id.
fn directory_name(directory: &Path) -> String {
    directory
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string()
}

/// Newest file mtime anywhere below `directory`, ignoring unreadable entries.
fn newest_mtime(directory: &Path) -> Option<DateTime<Utc>> {
    let mut newest: Option<SystemTime> = None;
    let mut stack = vec![directory.to_path_buf()];
    while let Some(path) = stack.pop() {
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.is_dir() {
            let Ok(entries) = fs::read_dir(&path) else {
                continue;
            };
            for entry in entries.flatten() {
                stack.push(entry.path());
            }
            continue;
        }
        let Ok(modified) = metadata.modified() else {
            continue;
        };
        newest = Some(newest.map_or(modified, |current| current.max(modified)));
    }
    newest.map(DateTime::<Utc>::from)
}

/// Whole days between `as_of` and the observed mtime date, never negative.
fn age_days(as_of: NaiveDate, observed: DateTime<Utc>) -> u32 {
    let days = as_of
        .signed_duration_since(observed.date_naive())
        .num_days();
    u32::try_from(days.max(0)).unwrap_or(u32::MAX)
}

/// First markdown H1 in the file, when the file is readable.
fn first_h1(path: &Path) -> Option<String> {
    let text = fs::read_to_string(path).ok()?;
    for line in text.lines() {
        if let Some(title) = line.strip_prefix("# ") {
            let title = title.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}
