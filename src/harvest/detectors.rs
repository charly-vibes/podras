//! Purpose: shared candidate detection vocabulary and ordering.
//!
//! Responsibilities:
//! - load the tracker vocabulary from the full beads export and open work;
//! - answer the tracker-linking and open-path predicates every family uses;
//! - own the stable total order and duplicate removal.
//!
//! Rationale: the vocabulary is the false-positive gate for the whole module,
//! so it lives in one place instead of being re-derived per detector. Every
//! predicate is pure over already-read values; only `load` touches the export.

use std::collections::BTreeSet;
use std::path::Path;

use chrono::NaiveDate;

use super::{Candidate, HORIZON_DAYS};
use crate::brief::BriefInputs;

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
            .filter_map(|id| prefix(id).map(str::to_string))
            .collect();
        let mut open_paths = BTreeSet::new();
        if let Some(beads) = &inputs.beads {
            for item in &beads.open {
                open_paths.insert(item.id.to_ascii_lowercase());
                open_paths.extend(path_tokens(item.title.as_deref()));
            }
        }
        Self {
            ids,
            prefixes,
            open_paths,
        }
    }

    /// True when `text` names a known tracker id or a `<prefix>-<id>` token.
    pub(super) fn names_tracker(&self, text: &str) -> bool {
        tokenize(text).any(|token| {
            let token = token.to_ascii_lowercase();
            self.ids.contains(&token)
                || prefix(&token).is_some_and(|prefix| self.prefixes.contains(prefix))
        })
    }

    /// True when `text` contains one of the open work-item path tokens.
    pub(super) fn touches_path(&self, text: &str) -> bool {
        let text = text.to_ascii_lowercase();
        self.open_paths
            .iter()
            .any(|path| text.contains(path.as_str()))
    }

    /// Tracker-shaped tokens absent from the full export, lowercased and sorted.
    pub(super) fn unresolved_tracker_tokens(&self, text: &str) -> BTreeSet<String> {
        tokenize(text)
            .map(str::to_ascii_lowercase)
            .filter(|token| {
                prefix(token).is_some_and(|prefix| self.prefixes.contains(prefix))
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
    (0..=HORIZON_DAYS).contains(&(evaluation - date).num_days())
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

/// Prefix before the final `-`, when both halves are id-shaped.
fn prefix(token: &str) -> Option<&str> {
    let (prefix, suffix) = token.rsplit_once('-')?;
    if prefix.is_empty()
        || suffix.is_empty()
        || !prefix
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
        || !suffix
            .chars()
            .any(|character| character.is_ascii_alphanumeric())
    {
        return None;
    }
    Some(prefix)
}

/// Split prose into id-shaped tokens on anything but alphanumerics, `-`, `_`.
pub(super) fn tokenize(text: &str) -> impl Iterator<Item = &str> {
    text.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '-' || character == '_')
    })
    .filter(|token| !token.is_empty())
}

/// Path-like tokens in an item title, e.g. `src/health.rs`.
fn path_tokens(title: Option<&str>) -> BTreeSet<String> {
    title
        .into_iter()
        .flat_map(|title| title.split_whitespace())
        .map(|word| {
            word.trim_matches(|character: char| {
                !(character.is_ascii_alphanumeric()
                    || character == '/'
                    || character == '.'
                    || character == '-'
                    || character == '_')
            })
        })
        .filter(|word| word.contains('/') && word.len() > 1)
        .map(str::to_ascii_lowercase)
        .collect()
}
