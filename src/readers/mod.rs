//! Purpose: expose the declared brief source readers.
//!
//! Responsibilities:
//! - declare one module per source: beads, git, openspec specs, whisper notes;
//! - define the shared read context and per-source read result;
//! - own `collect`, which reads every source in isolation and records status.
//!
//! Rationale: sources degrade independently. A failed read must become a
//! per-source status row and warning, never an aborted brief, so the
//! orchestration lives above the individual readers and every reader receives
//! the same immutable context.

use std::path::{Path, PathBuf};

use chrono::{NaiveDate, Utc};

use crate::brief::{
    BeadsSummary, BriefInputs, CurationState, GitSummary, SpecSummary, WhisperReading,
};
use crate::curation::{self, CURATION_REL_PATH, CurationFile};
use crate::error::{Error, Result};
use crate::freshness;
use crate::model::{SourceName, SourceState, SourceStatus, Warning};

pub mod beads;
pub mod git;
pub mod specs;
pub mod whisper;

/// Immutable inputs shared by every declared source reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadContext {
    /// Repository root the readers observe.
    pub repo_root: PathBuf,
    /// Pinned evaluation date, when `--as-of` was supplied.
    pub as_of: Option<NaiveDate>,
    /// Current branch, used to locate branch-scoped notes.
    pub branch: Option<String>,
}

/// One source read: typed payload plus its declared status and warnings.
///
/// `data` is `None` when the source was missing or unreadable; the status row
/// and warnings are always present so the brief can render the degradation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRead<T> {
    /// Reader-specific payload consumed by `brief::assemble`.
    pub data: Option<T>,
    /// Declared source row for the envelope.
    pub status: SourceStatus,
    /// Stable warnings accumulated while reading this source.
    pub warnings: Vec<Warning>,
}

/// Read every declared source, tolerating missing or unreadable sources.
///
/// Rows are accumulated in the fixed declaration order (beads, git, openspec,
/// whisper, curation) and each row's staleness is judged from its own
/// observation instant. No source failure is propagated: readers are already
/// infallible, and curation load failures become an `unreadable` row with
/// in-memory defaults so the brief still renders.
pub fn collect(context: &ReadContext) -> Result<BriefInputs> {
    let generated_at = Utc::now();
    let mut sources = Vec::with_capacity(SourceName::DECLARED.len());
    let mut warnings = Vec::new();
    let loaded = load_sources(context, &mut sources, &mut warnings);
    let curation = record_curation(
        &context.repo_root,
        context.as_of,
        &mut sources,
        &mut warnings,
    );
    Ok(BriefInputs {
        beads: loaded.beads,
        git: loaded.git,
        specs: loaded.specs,
        whisper: loaded.whisper,
        curation,
        sources,
        warnings,
        as_of: context.as_of,
        now: generated_at.date_naive(),
        generated_at,
    })
}

/// Data read from the four content sources, before curation is folded in.
struct Loaded {
    beads: Option<BeadsSummary>,
    git: Option<GitSummary>,
    specs: Option<SpecSummary>,
    whisper: Option<WhisperReading>,
}

/// Read the four content sources in declaration order, recording their rows.
fn load_sources(
    context: &ReadContext,
    sources: &mut Vec<SourceStatus>,
    warnings: &mut Vec<Warning>,
) -> Loaded {
    Loaded {
        beads: record(beads::read(context), context.as_of, sources, warnings),
        git: record(git::read(context), context.as_of, sources, warnings),
        specs: record(specs::read(context), context.as_of, sources, warnings),
        whisper: record(whisper::read(context), context.as_of, sources, warnings),
    }
}

/// Push one reader outcome into the source rows and warnings, returning its data.
fn record<T>(
    read: SourceRead<T>,
    as_of: Option<NaiveDate>,
    sources: &mut Vec<SourceStatus>,
    warnings: &mut Vec<Warning>,
) -> Option<T> {
    let SourceRead {
        data,
        mut status,
        warnings: read_warnings,
    } = read;
    status.staleness = freshness::evaluate(status.name, status.observed_at, as_of);
    sources.push(status);
    warnings.extend(read_warnings);
    data
}

/// Load curation and record the fifth declared source row.
///
/// Absent and malformed files both fall back to in-memory defaults so focus,
/// parked, and health state stay well-defined; only the row status and warning
/// tell the reader that the human state was unavailable.
fn record_curation(
    repo_root: &Path,
    as_of: Option<NaiveDate>,
    sources: &mut Vec<SourceStatus>,
    warnings: &mut Vec<Warning>,
) -> Option<CurationState> {
    let (state, mut status, read_warnings) = curation_outcome(repo_root);
    status.staleness = freshness::evaluate(status.name, status.observed_at, as_of);
    sources.push(status);
    warnings.extend(read_warnings);
    state
}

/// Load curation, mapping every outcome to data, a source row, and warnings.
fn curation_outcome(repo_root: &Path) -> (Option<CurationState>, SourceStatus, Vec<Warning>) {
    match curation::load(repo_root) {
        Ok(Some(curation)) => present_curation(repo_root, curation),
        Ok(None) => curation_fallback(
            SourceState::Missing,
            "curation_missing",
            format!("{CURATION_REL_PATH} does not exist"),
        ),
        Err(Error::Domain { code, message, .. }) if code == "curation_parse_error" => {
            curation_fallback(SourceState::Unreadable, code, message)
        }
        Err(error) => curation_fallback(
            SourceState::Unreadable,
            "curation_unreadable",
            error.to_string(),
        ),
    }
}

/// The present curation row, stamped with the file mtime.
fn present_curation(
    repo_root: &Path,
    curation: CurationFile,
) -> (Option<CurationState>, SourceStatus, Vec<Warning>) {
    let updated_at = curation::updated_at(repo_root);
    (
        Some(CurationState {
            curation,
            updated_at,
        }),
        curation_status(SourceState::Present, updated_at, None),
        Vec::new(),
    )
}

/// Defaults plus one row and warning for an absent or unreadable file.
fn curation_fallback(
    state: SourceState,
    code: &str,
    detail: String,
) -> (Option<CurationState>, SourceStatus, Vec<Warning>) {
    (
        Some(default_curation()),
        curation_status(state, None, Some(detail.clone())),
        vec![Warning {
            source: SourceName::Curation,
            code: code.to_string(),
            detail,
        }],
    )
}

/// In-memory curation defaults used when the file cannot be read.
fn default_curation() -> CurationState {
    CurationState {
        curation: CurationFile::default(),
        updated_at: None,
    }
}

/// Build the curation source row from its load outcome.
fn curation_status(
    state: SourceState,
    updated_at: Option<chrono::DateTime<Utc>>,
    detail: Option<String>,
) -> SourceStatus {
    SourceStatus {
        name: SourceName::Curation,
        status: state,
        observed_at: updated_at,
        staleness: crate::model::Staleness::Unknown,
        detail,
    }
}
