//! Purpose: hold the per-source readings that compose a brief.
//!
//! Responsibilities:
//! - declare the summary types each reader produces;
//! - bundle reader output, source rows, warnings, and the `--as-of` instant;
//! - own `assemble`, the pure function turning inputs into a deterministic `Brief`.
//!
//! Rationale: readers must not know about rendering or bounds. Keeping the
//! inputs in one struct lets `assemble` stay a pure function of observed data:
//! no clock, no filesystem, no cache. Curation beats inference here — focus and
//! parked state come only from `.podras/curation.toml`, and a parked item never
//! re-enters a computed queue. The assembly steps live in submodules so each
//! one reads as the single rule it enforces.

use std::collections::BTreeSet;

use chrono::{DateTime, NaiveDate, Utc};

use crate::curation::CurationFile;
use crate::model::{Brief, RecentEvent, SourceStatus, WorkItem};

mod curation_view;
mod health;
mod ordering;
mod parking;
mod sections;
mod tracker;

use tracker::Tracker;

/// Beads tracker reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadsSummary {
    /// Open issues.
    pub open: Vec<WorkItem>,
    /// Blocked issues with their blockers.
    pub blocked: Vec<WorkItem>,
    /// Ready issues (no open blockers).
    pub ready: Vec<WorkItem>,
    /// When the export was observed.
    pub observed_at: Option<DateTime<Utc>>,
}

/// Git history reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSummary {
    /// Repository HEAD sha.
    pub head_sha: Option<String>,
    /// Recent commits, newest first.
    pub recent: Vec<RecentEvent>,
    /// When the history was observed.
    pub observed_at: Option<DateTime<Utc>>,
}

/// OpenSpec change-store reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecSummary {
    /// Declared spec changes.
    pub changes: Vec<WorkItem>,
    /// Number of changes that appear stale.
    pub stale_count: usize,
    /// When the change store was observed.
    pub observed_at: Option<DateTime<Utc>>,
}

/// The latest timestamped whisper entry, kept as harvest evidence.
///
/// The body is internal: the brief never renders it, so session narrative
/// cannot reach a committed artifact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhisperEntry {
    /// Notes file the entry was read from.
    pub path: std::path::PathBuf,
    /// Entry header timestamp, verbatim as written (RFC 3339).
    pub timestamp: String,
    /// Entry body after the header; never serialized or rendered.
    pub body: String,
}

/// Light whisper-notes reading: latest branch entry only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhisperReading {
    /// When the notes were observed.
    pub observed_at: Option<DateTime<Utc>>,
    /// Latest recorded next action, if any.
    pub next_action: Option<String>,
    /// Hash of the observed note content.
    pub content_hash: Option<String>,
    /// The newest timestamped entry, when one exists.
    pub latest_entry: Option<WhisperEntry>,
}

/// Curation state as read from `.podras/curation.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurationState {
    /// Parsed curation file.
    pub curation: CurationFile,
    /// File mtime of the last curation write.
    pub updated_at: Option<DateTime<Utc>>,
}

/// Everything `assemble` needs, with absent (not failed) sources as `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefInputs {
    /// Beads reading, when readable.
    pub beads: Option<BeadsSummary>,
    /// Git reading, when readable.
    pub git: Option<GitSummary>,
    /// Spec-store reading, when readable.
    pub specs: Option<SpecSummary>,
    /// Whisper reading, when readable.
    pub whisper: Option<WhisperReading>,
    /// Curation state, when readable.
    pub curation: Option<CurationState>,
    /// One status row per declared source.
    pub sources: Vec<SourceStatus>,
    /// Stable warnings accumulated while reading.
    pub warnings: Vec<crate::model::Warning>,
    /// Requested observation instant, when pinned.
    pub as_of: Option<NaiveDate>,
    /// Caller-supplied wall-clock date, used only when `as_of` is absent.
    pub now: NaiveDate,
    /// Caller-supplied generation instant; the only volatile field.
    pub generated_at: DateTime<Utc>,
}

/// Compose a brief from observed inputs.
pub fn assemble(inputs: &BriefInputs) -> Brief {
    let evaluation_date = inputs.as_of.unwrap_or(inputs.now);
    let mut warnings = inputs.warnings.clone();
    let curation = inputs.curation.as_ref().map(|state| &state.curation);
    let tracker = Tracker::new(inputs.beads.as_ref());

    let (focus_ids, focus): (BTreeSet<String>, Vec<WorkItem>) =
        curation_view::lead_focus(curation, &tracker);
    let parking = parking::resolve_parking(curation, &tracker, evaluation_date, &mut warnings);
    let sections = sections::computed(focus, inputs, parking, &focus_ids);
    let (sections, truncation) = ordering::apply_bounds(sections);
    let signals = crate::health::signals(inputs);
    let impaired_mandatory = crate::freshness::impaired_mandatory(&inputs.sources);
    let health = health::resolve_health(
        curation.and_then(|curation| curation.health.as_ref()),
        evaluation_date,
        signals,
        &impaired_mandatory,
        &inputs.sources,
        &mut warnings,
    );
    ordering::sort_warnings(&mut warnings);

    let mut brief = sections::envelope(
        inputs,
        evaluation_date,
        sections,
        truncation,
        health,
        warnings,
    );
    brief.source_hash = Some(crate::model::source_hash(&brief, brief.head_sha.as_deref()));
    brief
}
