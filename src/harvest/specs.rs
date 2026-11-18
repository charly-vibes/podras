//! Purpose: derive unlinked OpenSpec change candidates.
//!
//! Responsibilities:
//! - select active changes inside the candidate window;
//! - drop changes whose id or title names a tracker id;
//! - attach the newest commit touching the change's directory as evidence.
//!
//! Rationale: a young spec change with no tracker link is work in flight that
//! nothing can report on. A change whose directory has no commit is dropped
//! rather than emitted without resolvable provenance.

use std::path::Path;

use super::detectors::Vocabulary;
use super::git::commit_touching;
use super::{Candidate, CandidateKind, Evidence, EvidenceKind, HORIZON_DAYS};
use crate::brief::BriefInputs;
use crate::model::WorkItem;

/// Repository-relative prefix for active spec changes.
const SPECS_CHANGES_DIR: &str = "openspec/changes";

/// Window spec changes whose id or title links no tracker id.
pub(super) fn unlinked_spec_candidates(
    repo: &Path,
    inputs: &BriefInputs,
    vocabulary: &Vocabulary,
) -> Vec<Candidate> {
    let Some(specs) = &inputs.specs else {
        return Vec::new();
    };
    specs
        .changes
        .iter()
        .filter(|item| is_active_change(item))
        .filter(|item| !links_tracker(item, vocabulary))
        .filter_map(|item| unlinked_candidate(repo, item))
        .collect()
}

/// A declared change whose newest edit is inside the candidate window.
fn is_active_change(item: &WorkItem) -> bool {
    item.status.as_deref() == Some("change")
        && item
            .age_days
            .is_some_and(|days| days <= HORIZON_DAYS as u32)
}

/// True when the change id or title names a known tracker id.
fn links_tracker(item: &WorkItem, vocabulary: &Vocabulary) -> bool {
    vocabulary.names_tracker(&item.id)
        || item
            .title
            .as_deref()
            .is_some_and(|title| vocabulary.names_tracker(title))
}

/// One candidate proven by the newest commit touching the change directory.
fn unlinked_candidate(repo: &Path, item: &WorkItem) -> Option<Candidate> {
    let path = format!("{SPECS_CHANGES_DIR}/{}", item.id);
    let sha = commit_touching(repo, &path)?;
    Some(Candidate {
        kind: CandidateKind::UnlinkedSpecChange,
        summary: format!("openspec change `{}` links no tracker id", item.id),
        evidence: vec![Evidence {
            kind: EvidenceKind::Git,
            reference: sha,
        }],
    })
}
