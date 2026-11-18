//! Purpose: index beads items by identifier for curated-entry enrichment.
//!
//! Responsibilities:
//! - build a lookup over open and blocked beads items;
//! - return the first item seen for an identifier.
//!
//! Rationale: curated focus and parked entries may refer to tracker items;
//! one borrowed index keeps enrichment O(1) without cloning the tracker.

use std::collections::HashMap;

use crate::model::WorkItem;

use super::BeadsSummary;

/// Tracker items addressed by identifier, for enriching curated entries.
pub(super) struct Tracker<'a> {
    items: HashMap<&'a str, &'a WorkItem>,
}

impl<'a> Tracker<'a> {
    pub(super) fn new(beads: Option<&'a BeadsSummary>) -> Self {
        let mut items = HashMap::new();
        if let Some(beads) = beads {
            for item in beads.open.iter().chain(beads.blocked.iter()) {
                items.entry(item.id.as_str()).or_insert(item);
            }
        }
        Self { items }
    }

    pub(super) fn get(&self, id: &str) -> Option<&'a WorkItem> {
        self.items.get(id).copied()
    }
}
