//! Purpose: harvest's tracker vocabulary and stable candidate ordering.
//!
//! Responsibilities:
//! - load the tracker vocabulary from the full beads export and open work;
//! - answer the tracker-linking and open-path predicates every family uses;
//! - own the stable total order and duplicate removal.
//!
//! Rationale: the vocabulary is the false-positive gate for the whole module,
//! so it lives in one place instead of being re-derived per detector. The
//! predicates themselves and the fourteen-day window are shared with health
//! and live in `crate::health::links`; only `load` touches the export.

use std::collections::BTreeSet;
use std::path::Path;

use chrono::NaiveDate;

use super::Candidate;
use crate::brief::BriefInputs;
use crate::health::links::{
    SIGNAL_HORIZON_DAYS, names_tracker, open_work_paths, tokenize, touches_path, tracker_prefix,
};

/// Repository-relative path of the beads export.
const BEADS_REL_PATH: &str = ".beads/issues.jsonl";

/// Stable total order and duplicate removal.
pub(super) fn sort_and_dedupe(candidates: &mut Vec<Candidate>) {
    candidates.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| first_ref(left).cmp(first_ref(right)))
            .then_with(|| left.summary.cmp(&right.summary))
    });
    candidates.dedup_by(|left, right| {
        left.kind == right.kind && left.summary == right.summary && left.evidence == right.evidence
    });
}

fn first_ref(candidate: &Candidate) -> &str {
    candidate
        .evidence
        .first()
        .map(|evidence| evidence.reference.as_str())
        .unwrap_or_default()
}

/// Tracker vocabulary observed in the full beads export plus open work paths.
pub(super) struct Vocabulary {
    pub(super) ids: BTreeSet<String>,
    prefixes: BTreeSet<String>,
    open_paths: BTreeSet<String>,
}

impl Vocabulary {
    pub(super) fn load(repo: &Path, inputs: &BriefInputs) -> Self {
        let ids = export_ids(repo);
        let prefixes = ids
            .iter()
            .filter_map(|id| tracker_prefix(id).map(str::to_string))
            .collect();
        let open_paths = inputs
            .beads
            .as_ref()
            .map(open_work_paths)
            .unwrap_or_default();
        Self {
            ids,
            prefixes,
            open_paths,
        }
    }

    /// True when `text` names a known tracker id or a `<prefix>-<id>` token.
    pub(super) fn names_tracker(&self, text: &str) -> bool {
        names_tracker(text, &self.ids, &self.prefixes)
    }

    /// True when `text` contains one of the open work-item path tokens.
    pub(super) fn touches_path(&self, text: &str) -> bool {
        touches_path(text, &self.open_paths)
    }

    /// Tracker-shaped tokens absent from the full export, lowercased and sorted.
    pub(super) fn unresolved_tracker_tokens(&self, text: &str) -> BTreeSet<String> {
        tokenize(text)
            .map(str::to_ascii_lowercase)
            .filter(|token| {
                tracker_prefix(token).is_some_and(|prefix| self.prefixes.contains(prefix))
                    && !self.ids.contains(token)
            })
            .collect()
    }
}

/// The run's single evaluation date.
pub(super) fn evaluation_date(inputs: &BriefInputs) -> NaiveDate {
    inputs.as_of.unwrap_or(inputs.now)
}

/// True when `date` falls inside the candidate horizon.
pub(super) fn within_window(date: NaiveDate, evaluation: NaiveDate) -> bool {
    (0..=SIGNAL_HORIZON_DAYS).contains(&(evaluation - date).num_days())
}

/// Every id in the full beads export, lowercased.
fn export_ids(repo: &Path) -> BTreeSet<String> {
    let Ok(text) = std::fs::read_to_string(repo.join(BEADS_REL_PATH)) else {
        return BTreeSet::new();
    };
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|row| row.get("id")?.as_str().map(str::to_ascii_lowercase))
        .collect()
}
