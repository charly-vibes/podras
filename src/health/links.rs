//! Purpose: own the shared tracker-linking and detection vocabulary.
//!
//! Responsibilities:
//! - collect every tracker id and prefix in a beads reading;
//! - recognize tracker references inside free text;
//! - extract path-like tokens from work-item titles and open item ids;
//! - declare the one candidate/signal window used by health and harvest.
//!
//! Rationale: git readings carry commit subjects, not changed paths, and both
//! health signals and harvest candidates ask the same questions of those
//! subjects. The orphan rule therefore treats an open work item's id as its
//! canonical path token and any path-like token in its title (for example
//! `src/health.rs`) as a touched path; a commit links to work when its subject
//! names either. Keeping the vocabulary and the fourteen-day window here is
//! what stops harvest from re-deriving rules that health already owns.

use std::collections::BTreeSet;

use crate::brief::BeadsSummary;

/// Candidate and signal window shared by health and harvest: fourteen days.
pub(crate) const SIGNAL_HORIZON_DAYS: i64 = 14;

/// Every tracker identifier in the reading, lowercased.
pub(crate) fn tracker_ids(beads: &BeadsSummary) -> BTreeSet<String> {
    beads
        .open
        .iter()
        .chain(beads.blocked.iter())
        .chain(beads.ready.iter())
        .map(|item| item.id.to_ascii_lowercase())
        .collect()
}

/// Prefixes before the final `-` of tracker ids, e.g. `podras` from `podras-d2e`.
pub(crate) fn tracker_prefixes(beads: &BeadsSummary) -> BTreeSet<String> {
    beads
        .open
        .iter()
        .chain(beads.blocked.iter())
        .chain(beads.ready.iter())
        .filter_map(|item| tracker_prefix(&item.id).map(str::to_ascii_lowercase))
        .collect()
}

/// Recognizable path tokens for every open work item: title tokens and the id.
pub(crate) fn open_work_paths(beads: &BeadsSummary) -> BTreeSet<String> {
    beads
        .open
        .iter()
        .flat_map(|item| {
            let mut tokens = path_tokens(item.title.as_deref());
            tokens.insert(item.id.to_ascii_lowercase());
            tokens
        })
        .collect()
}

/// True when `text` names a known tracker id or a `<prefix>-<id>` token.
pub(crate) fn names_tracker(
    text: &str,
    known: &BTreeSet<String>,
    prefixes: &BTreeSet<String>,
) -> bool {
    tokenize(text).any(|token| {
        let token = token.to_ascii_lowercase();
        known.contains(&token)
            || tracker_prefix(&token).is_some_and(|prefix| prefixes.contains(prefix))
    })
}

/// True when `text` contains one of the path-like tokens verbatim.
pub(crate) fn touches_path(text: &str, paths: &BTreeSet<String>) -> bool {
    let text = text.to_ascii_lowercase();
    paths.iter().any(|path| text.contains(path.as_str()))
}

/// Split prose into id-shaped tokens on anything but alphanumerics, `-`, `_`.
pub(crate) fn tokenize(text: &str) -> impl Iterator<Item = &str> {
    text.split(|character: char| {
        !(character.is_ascii_alphanumeric() || character == '-' || character == '_')
    })
    .filter(|token| !token.is_empty())
}

/// Path-like tokens in an item title, e.g. `src/health.rs`.
pub(crate) fn path_tokens(title: Option<&str>) -> BTreeSet<String> {
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

/// Prefix before the final `-`, when both halves are id-shaped.
pub(crate) fn tracker_prefix(token: &str) -> Option<&str> {
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
