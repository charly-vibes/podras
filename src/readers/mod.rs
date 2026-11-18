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

use std::path::PathBuf;

use chrono::NaiveDate;

use crate::brief::BriefInputs;
use crate::error::{Error, Result};
use crate::model::{SourceStatus, Warning};

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
pub fn collect(_context: &ReadContext) -> Result<BriefInputs> {
    Err(Error::NotYetImplemented("readers::collect"))
}
