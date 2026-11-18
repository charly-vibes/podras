//! Purpose: define and load/save the human-owned curation state.
//!
//! Responsibilities:
//! - model `.podras/curation.toml` (purpose, bounds, health verdict, focus, parked);
//! - serialize the file so focus/park verbs and humans share one format;
//! - own the only authority for focus and parked state.
//!
//! Rationale: curation is the single source of intent. Tracker priority and
//! commit activity may inform, but never decide, what is focused or parked, so
//! the shape lives here and nowhere else.

use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};

/// Repository-relative curation path.
pub const CURATION_REL_PATH: &str = ".podras/curation.toml";

/// One focused item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FocusEntry {
    /// Focused item identifier.
    pub id: String,
    /// Why it is focused.
    #[serde(default)]
    pub reason: Option<String>,
    /// When it became the focus.
    #[serde(default)]
    pub focused_since: Option<String>,
    /// When a human last confirmed the focus.
    #[serde(default)]
    pub confirmed_at: Option<String>,
}

/// One parked item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ParkEntry {
    /// Parked item identifier.
    pub id: String,
    /// Why it is parked; required by the park verb.
    pub reason: String,
    /// `YYYY-MM-DD` date or `event:<name>` trigger; required.
    pub resurface: String,
    /// When it was parked.
    #[serde(default)]
    pub parked_at: Option<String>,
    /// When a human last confirmed the park.
    #[serde(default)]
    pub confirmed_at: Option<String>,
}

/// Human health verdict stored in curation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CurationHealth {
    /// Verdict: `green`, `amber`, or `red`.
    pub status: String,
    /// Human owner of the verdict.
    #[serde(default)]
    pub owner: Option<String>,
    /// When the verdict was confirmed.
    #[serde(default)]
    pub confirmed_at: Option<String>,
    /// Optional explanation.
    #[serde(default)]
    pub reason: Option<String>,
}

/// Parsed `.podras/curation.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct CurationFile {
    /// One-line project purpose.
    pub purpose_lite: Option<String>,
    /// Focus cap; defaults to 1 when absent.
    pub max_focus: Option<usize>,
    /// Whether whisper content may enter the committed render.
    pub publish_whisper: Option<bool>,
    /// Human health verdict.
    pub health: Option<CurationHealth>,
    /// Focused items.
    pub focus: Vec<FocusEntry>,
    /// Parked items.
    pub parked: Vec<ParkEntry>,
}

/// Load curation from the repository, returning `None` when absent.
pub fn load(_repo: &Path) -> Result<Option<CurationFile>> {
    Err(Error::NotYetImplemented("curation::load"))
}

/// Persist curation to the repository, creating `.podras/` as needed.
pub fn save(_repo: &Path, _curation: &CurationFile) -> Result<()> {
    Err(Error::NotYetImplemented("curation::save"))
}
