//! Purpose: assemble harvest candidates from observed sources.
//!
//! Responsibilities:
//! - own the public candidate contract (payload, kinds, evidence);
//! - gate every candidate family on a readable tracker vocabulary;
//! - declare the detector, notes, git, spec, and render submodules;
//! - re-export the render entry points so the module API stays flat.
//!
//! Rationale: discovery is not commitment. A candidate is a human triage prompt,
//! so the assembler is a pure function of the already-collected inputs plus the
//! repository path used for bounded, read-only git queries. Ordering is total
//! (`kind` declaration order, first evidence ref, summary) and a candidate whose
//! evidence cannot be resolved is dropped rather than emitted bare. False
//! negatives are acceptable; false positives are not.

mod detectors;
mod git;
mod notes;
mod render;
mod specs;

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::brief::BriefInputs;
use crate::model::{SourceStatus, Warning};

pub use render::{render, to_json};

/// Schema identifier carried by every harvest payload.
pub const SCHEMA_VERSION: &str = "harvest.v1";

/// The complete typed harvest payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Harvest {
    /// Payload contract version.
    pub schema_version: String,
    /// Volatile generation instant; never part of the candidate set.
    pub generated_at: DateTime<Utc>,
    /// The same source rows the brief reports.
    pub sources: Vec<SourceStatus>,
    /// Candidate list in stable total order.
    pub candidates: Vec<Candidate>,
    /// Per-source degradation, never truncated.
    pub warnings: Vec<Warning>,
}

/// Candidate families. Declaration order is sort order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateKind {
    /// The latest note entry names an id or commit that does not resolve.
    NoteReferenceUnresolved,
    /// A window commit names no tracker id and touches no open work path.
    OrphanCommit,
    /// A window spec change links no tracker id in its id or title.
    UnlinkedSpecChange,
}

impl CandidateKind {
    /// Every kind, in declaration order.
    pub const ALL: [CandidateKind; 3] = [
        CandidateKind::NoteReferenceUnresolved,
        CandidateKind::OrphanCommit,
        CandidateKind::UnlinkedSpecChange,
    ];

    /// Serialized name, matching `serde`.
    pub fn as_str(self) -> &'static str {
        match self {
            CandidateKind::NoteReferenceUnresolved => "note_reference_unresolved",
            CandidateKind::OrphanCommit => "orphan_commit",
            CandidateKind::UnlinkedSpecChange => "unlinked_spec_change",
        }
    }
}

/// Where a candidate's evidence lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// A full git commit sha.
    Git,
    /// A notes path plus the entry timestamp.
    Notes,
}

impl EvidenceKind {
    /// Serialized name, matching `serde`.
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceKind::Git => "git",
            EvidenceKind::Notes => "notes",
        }
    }
}

/// One resolvable provenance entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Evidence {
    /// Evidence family.
    pub kind: EvidenceKind,
    /// Git sha or `<notes path>#<entry timestamp>`, serialized as `ref`.
    #[serde(rename = "ref")]
    pub reference: String,
}

/// One proposed mismatch with provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    /// Which rule produced the candidate.
    pub kind: CandidateKind,
    /// One deterministic line for human triage.
    pub summary: String,
    /// Non-empty by construction.
    pub evidence: Vec<Evidence>,
}

/// Assemble harvest candidates from already-collected inputs.
///
/// `repo` is only used for bounded read-only queries (`git log`, `git
/// rev-parse`, and the full beads export). The whisper note itself is already
/// located and parsed by the reader in `inputs`. Nothing is written.
pub fn assemble(repo: &Path, inputs: &BriefInputs) -> Harvest {
    let vocabulary = detectors::Vocabulary::load(repo, inputs);
    let mut candidates = Vec::new();

    // Tracker vocabulary is the gate for every family: without a readable
    // export every reference would look unresolved, which is a false positive.
    if !vocabulary.ids.is_empty() {
        candidates.extend(notes::note_candidates(repo, inputs, &vocabulary));
        candidates.extend(git::orphan_commit_candidates(inputs, &vocabulary));
        candidates.extend(specs::unlinked_spec_candidates(repo, inputs, &vocabulary));
    }

    detectors::sort_and_dedupe(&mut candidates);

    Harvest {
        schema_version: SCHEMA_VERSION.to_string(),
        generated_at: inputs.generated_at,
        sources: inputs.sources.clone(),
        candidates,
        warnings: inputs.warnings.clone(),
    }
}
