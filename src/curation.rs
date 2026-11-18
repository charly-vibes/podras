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

use chrono::{DateTime, NaiveDate, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::model::{Resurface, ResurfaceParseError};

/// Repository-relative curation path.
pub const CURATION_REL_PATH: &str = ".podras/curation.toml";

/// One focused item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
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
#[serde(deny_unknown_fields)]
pub struct ParkEntry {
    /// Parked item identifier.
    pub id: String,
    /// Why it is parked; the park verb requires it, and a missing or empty value
    /// invalidates the entry at read time.
    #[serde(default)]
    pub reason: String,
    /// `YYYY-MM-DD` date or `event:<name>` trigger; the park verb requires it,
    /// and a missing or malformed value invalidates the entry at read time.
    #[serde(default)]
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
#[serde(deny_unknown_fields)]
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
#[serde(default, deny_unknown_fields)]
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

/// Parse a human date field, accepting `YYYY-MM-DD` or an RFC 3339 timestamp.
///
/// The model keeps these fields as strings because humans hand-edit the file;
/// this is the one place that decides what counts as a usable date.
pub fn parse_human_date(raw: &str) -> Option<NaiveDate> {
    let raw = raw.trim();
    if let Ok(date) = NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        return Some(date);
    }
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|timestamp| timestamp.date_naive())
}

impl FocusEntry {
    /// `focused_since` as a date, when present and parseable.
    pub fn focused_since(&self) -> Option<NaiveDate> {
        self.focused_since.as_deref().and_then(parse_human_date)
    }

    /// `confirmed_at` as a date, when present and parseable.
    pub fn confirmed_at(&self) -> Option<NaiveDate> {
        self.confirmed_at.as_deref().and_then(parse_human_date)
    }
}

impl ParkEntry {
    /// `parked_at` as a date, when present and parseable.
    pub fn parked_at(&self) -> Option<NaiveDate> {
        self.parked_at.as_deref().and_then(parse_human_date)
    }

    /// `confirmed_at` as a date, when present and parseable.
    pub fn confirmed_at(&self) -> Option<NaiveDate> {
        self.confirmed_at.as_deref().and_then(parse_human_date)
    }

    /// Parse the mandatory resurface trigger (`YYYY-MM-DD` or `event:<name>`).
    pub fn resurface(&self) -> Result<Resurface, ResurfaceParseError> {
        self.resurface.parse()
    }
}

impl CurationHealth {
    /// `confirmed_at` as a date, when present and parseable.
    pub fn confirmed_at(&self) -> Option<NaiveDate> {
        self.confirmed_at.as_deref().and_then(parse_human_date)
    }
}

/// Load curation from the repository, returning `None` when absent.
///
/// A malformed file is a domain error carrying `curation_parse_error`; the
/// curation reader turns that into an `unreadable` source warning, so callers
/// degrade instead of aborting the brief. Unknown keys are rejected at the top
/// level and inside `[health]`, `[[focus]]`, and `[[parked]]` entries.
pub fn load(repo: &Path) -> Result<Option<CurationFile>> {
    let path = repo.join(CURATION_REL_PATH);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(Error::Io(error)),
    };
    toml::from_str(&raw)
        .map(Some)
        .map_err(|error| Error::Domain {
            code: "curation_parse_error",
            message: format!("{CURATION_REL_PATH}: {error}"),
            suggestion: Some(
                "fix the TOML, or delete .podras/curation.toml and run podras init to start \
                 from the template"
                    .to_string(),
            ),
        })
}

/// Persist curation to the repository, creating `.podras/` as needed.
///
/// The bytes go to a temporary file in the same directory and are renamed over
/// `curation.toml`, so a reader never observes a half-written file and a failed
/// write leaves the previous file intact.
pub fn save(repo: &Path, curation: &CurationFile) -> Result<()> {
    std::fs::create_dir_all(repo.join(".podras"))?;
    let path = repo.join(CURATION_REL_PATH);
    let body = toml::to_string_pretty(curation).map_err(|error| Error::Domain {
        code: "curation_serialize_error",
        message: format!("cannot serialize {CURATION_REL_PATH}: {error}"),
        suggestion: None,
    })?;
    let temp = repo
        .join(".podras")
        .join(format!(".curation.toml.{}.tmp", std::process::id()));
    std::fs::write(&temp, body)?;
    if let Err(error) = std::fs::rename(&temp, &path) {
        let _ = std::fs::remove_file(&temp);
        return Err(Error::Io(error));
    }
    Ok(())
}

/// File mtime of the last curation write, in UTC.
pub fn updated_at(repo: &Path) -> Option<DateTime<Utc>> {
    let modified = std::fs::metadata(repo.join(CURATION_REL_PATH))
        .ok()?
        .modified()
        .ok()?;
    Some(DateTime::<Utc>::from(modified))
}

/// Newest human `confirmed_at` across health and focus/parked entries.
pub fn confirmed_at(file: &CurationFile) -> Option<NaiveDate> {
    let health = file.health.as_ref().and_then(CurationHealth::confirmed_at);
    let focus = file.focus.iter().filter_map(FocusEntry::confirmed_at);
    let parked = file.parked.iter().filter_map(ParkEntry::confirmed_at);
    health.into_iter().chain(focus).chain(parked).max()
}
