//! Purpose: prove the OpenSpec reader observes changes and specs best-effort.
//!
//! Responsibilities:
//! - assert change and spec directories become work items with ids, H1 titles,
//!   and declared status;
//! - assert stale counting and observation time follow the newest mtimes;
//! - assert an absent `openspec/` tree degrades to a missing source row plus a
//!   stable warning instead of an error.
//!
//! Rationale: the spec store is the weakest source in the brief, so its
//! degradation path is the contract worth pinning: no reader failure, one
//! warning, and data `None`.

mod common;

use std::process::Command;

use chrono::{NaiveDate, TimeZone, Utc};
use common::TempRepo;
use podras::model::{SourceName, SourceState, Staleness};
use podras::readers::{ReadContext, specs};

/// Evaluation date used for every assertion (matches the fixture fixed date).
fn as_of() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 1).expect("valid evaluation date")
}

/// Build a read context pinned to [`as_of`].
fn context(repo: &TempRepo) -> ReadContext {
    ReadContext {
        repo_root: repo.root().to_path_buf(),
        as_of: Some(as_of()),
        branch: Some("main".to_string()),
    }
}

/// Force a fixed mtime on every file below a repository-relative directory.
fn set_mtime(repo: &TempRepo, rel: &str, stamp: &str) {
    // Apply the stamp to every file below the directory so the newest mtime the
    // reader observes is fixed regardless of how the fixture wrote the files.
    let output = Command::new("find")
        .arg(repo.root().join(rel))
        .args(["-type", "f", "-exec", "touch", "-d", stamp, "{}", "+"])
        .output()
        .expect("run find on the fixture tree");
    assert!(
        output.status.success(),
        "touch -d {stamp} under {rel} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Repository holding two changes (one stale), an archived change, a spec, and
/// fixed mtimes.
fn seeded_repo() -> TempRepo {
    let repo = TempRepo::new();
    repo.write(
        "openspec/changes/fresh-change/proposal.md",
        "# Fresh change\n\nWhat it does.\n",
    );
    repo.write(
        "openspec/changes/stale-change/proposal.md",
        "# Stale change\n\nWhat it does.\n",
    );
    repo.write(
        "openspec/changes/archive/old-change/proposal.md",
        "# Archived change\n",
    );
    repo.write(
        "openspec/specs/brief-v1/spec.md",
        "# Brief v1 spec\n\n## Requirements\n",
    );
    repo.write("openspec/changes/stale-change/tasks.md", "## 1. Work\n");
    set_mtime(
        &repo,
        "openspec/changes/fresh-change",
        "2026-10-30T12:00:00Z",
    );
    set_mtime(
        &repo,
        "openspec/changes/stale-change",
        "2026-10-01T00:00:00Z",
    );
    set_mtime(
        &repo,
        "openspec/changes/archive/old-change",
        "2026-10-15T00:00:00Z",
    );
    set_mtime(&repo, "openspec/specs/brief-v1", "2026-10-20T00:00:00Z");
    repo
}

#[test]
fn specs_reader_reads_changes_and_specs() {
    let repo = seeded_repo();

    let read = specs::read(&context(&repo), as_of());

    assert_eq!(read.status.name, SourceName::Openspec);
    assert_eq!(read.status.status, SourceState::Present);
    assert!(
        read.warnings.is_empty(),
        "present store must not warn: {:?}",
        read.warnings
    );

    let summary = read.data.expect("present store yields a summary");
    assert_eq!(summary.stale_count, 1, "only stale-change is past 14 days");

    let observed = summary.observed_at.expect("newest mtime is observed");
    assert_eq!(
        observed,
        Utc.with_ymd_and_hms(2026, 10, 30, 12, 0, 0)
            .single()
            .expect("valid instant")
    );
    assert_eq!(read.status.observed_at, summary.observed_at);
    assert_eq!(read.status.staleness, Staleness::Unknown);

    let ids: Vec<&str> = summary
        .changes
        .iter()
        .map(|item| item.id.as_str())
        .collect();
    assert_eq!(ids, vec!["fresh-change", "stale-change", "brief-v1"]);

    let fresh = &summary.changes[0];
    assert_eq!(fresh.title.as_deref(), Some("Fresh change"));
    assert_eq!(fresh.status.as_deref(), Some("change"));
    assert_eq!(fresh.priority, None);
    assert!(fresh.blocked_by.is_empty());
    assert_eq!(fresh.age_days, Some(2));
    assert_eq!(fresh.reason, None);
    assert_eq!(fresh.confirmed_at, None);

    let stale = &summary.changes[1];
    assert_eq!(stale.title.as_deref(), Some("Stale change"));
    assert_eq!(stale.status.as_deref(), Some("change"));
    assert_eq!(stale.age_days, Some(31));

    let spec = &summary.changes[2];
    assert_eq!(spec.title.as_deref(), Some("Brief v1 spec"));
    assert_eq!(spec.status.as_deref(), Some("spec"));
    assert_eq!(spec.age_days, Some(12));
}

#[test]
fn specs_only_layout_stays_present() {
    let repo = TempRepo::new();
    repo.write("openspec/specs/only-spec/spec.md", "# Only spec\n");
    set_mtime(&repo, "openspec/specs/only-spec", "2026-10-29T00:00:00Z");

    let read = specs::read(&context(&repo), as_of());

    assert_eq!(read.status.name, SourceName::Openspec);
    assert_eq!(read.status.status, SourceState::Present);
    assert!(
        read.warnings.is_empty(),
        "an absent changes/ subtree must not discard the specs: {:?}",
        read.warnings
    );

    let summary = read.data.expect("specs-only store yields a summary");
    assert_eq!(summary.changes.len(), 1);
    assert_eq!(summary.stale_count, 0, "specs never count as stale");
    assert_eq!(
        summary.observed_at,
        Some(
            Utc.with_ymd_and_hms(2026, 10, 29, 0, 0, 0)
                .single()
                .expect("valid instant")
        )
    );

    let spec = &summary.changes[0];
    assert_eq!(spec.id, "only-spec");
    assert_eq!(spec.title.as_deref(), Some("Only spec"));
    assert_eq!(spec.status.as_deref(), Some("spec"));
    assert_eq!(spec.age_days, Some(3));
}

#[test]
fn empty_openspec_root_stays_present() {
    let repo = TempRepo::new();
    std::fs::create_dir_all(repo.root().join("openspec")).expect("create empty openspec root");

    let read = specs::read(&context(&repo), as_of());

    assert_eq!(read.status.status, SourceState::Present);
    assert!(read.warnings.is_empty(), "warnings: {:?}", read.warnings);
    let summary = read.data.expect("an empty root still yields a summary");
    assert!(summary.changes.is_empty());
    assert_eq!(summary.stale_count, 0);
    assert_eq!(summary.observed_at, None);
}

#[test]
fn titles_fall_back_to_the_other_document() {
    let repo = TempRepo::new();
    repo.write(
        "openspec/changes/change-with-spec/spec.md",
        "# Change from spec\n",
    );
    repo.write(
        "openspec/specs/spec-with-proposal/proposal.md",
        "# Spec from proposal\n",
    );

    let read = specs::read(&context(&repo), as_of());
    let summary = read.data.expect("present store yields a summary");

    let change = summary
        .changes
        .iter()
        .find(|item| item.id == "change-with-spec")
        .expect("change is observed");
    assert_eq!(change.title.as_deref(), Some("Change from spec"));

    let spec = summary
        .changes
        .iter()
        .find(|item| item.id == "spec-with-proposal")
        .expect("spec is observed");
    assert_eq!(spec.title.as_deref(), Some("Spec from proposal"));
}

#[test]
fn missing_openspec_tree_warns_without_failing() {
    let repo = TempRepo::new();

    let read = specs::read(&context(&repo), as_of());

    assert!(read.data.is_none(), "absent store carries no data");
    assert_eq!(read.status.name, SourceName::Openspec);
    assert_eq!(read.status.status, SourceState::Missing);
    assert_eq!(read.status.observed_at, None);
    assert_eq!(read.status.staleness, Staleness::Unknown);
    assert_eq!(read.warnings.len(), 1);
    assert_eq!(read.warnings[0].source, SourceName::Openspec);
    assert_eq!(read.warnings[0].code, "openspec_missing");
}
