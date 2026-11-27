//! Purpose: genesis config adoption for podras's human-owned config files.
//!
//! Responsibilities:
//! - implement `genesis::config::ConfigFile` for `.podras/curation.toml`
//!   and `.podras/roadmap.toml`;
//! - build the [`registry`] that exposes both files to genesis discovery
//!   and validation;
//! - assemble the read-only [`ConfigReport`] behind `podras config`.
//!
//! Rationale: config files stay human-owned: the adoption is read and
//! validate side only, and no adopted path ever writes a config file.
//! Implementing `ConfigFile` for the existing parsed types means the
//! registry can never disagree with the parsers the brief and lens
//! actually read.

use std::path::{Path, PathBuf};

use genesis::config::{ConfigFile, ConfigRegistry, ConfigStore, ConfigValidation};
use serde::Serialize;

use crate::curation::{CURATION_REL_PATH, CurationFile};
use crate::error::Result;
use crate::roadmap::file::{ROADMAP_REL_PATH, RoadmapFile};

/// Tool name under which the curation file registers with `ConfigRegistry`.
pub const CURATION_TOOL: &str = "podras";
/// Tool name under which the roadmap file registers with `ConfigRegistry`.
pub const ROADMAP_TOOL: &str = "podras-roadmap";

/// Build the podras config registry: both human-owned files, read-only.
pub fn registry() -> ConfigRegistry {
    let mut registry = ConfigRegistry::new();
    registry.register::<CurationFile>(CURATION_TOOL, CURATION_REL_PATH);
    registry.register::<RoadmapFile>(ROADMAP_TOOL, ROADMAP_REL_PATH);
    registry
}

impl ConfigFile for CurationFile {
    fn path(repo_root: &Path) -> PathBuf {
        repo_root.join(CURATION_REL_PATH)
    }
}

impl ConfigFile for RoadmapFile {
    fn path(repo_root: &Path) -> PathBuf {
        repo_root.join(ROADMAP_REL_PATH)
    }
}

/// Serializable report of one discovered config file.
#[derive(Debug, Serialize)]
pub struct ConfigFileReport {
    /// Tool name the file is registered under.
    pub tool: String,
    /// Marker path relative to the repository root.
    pub marker: String,
    /// Whether the file exists.
    pub found: bool,
}

/// Serializable report of one config validation result.
///
/// `field` carries the tool name for read failures (the registry reports
/// those under the registered tool) and the offending field otherwise.
#[derive(Debug, Serialize)]
pub struct ConfigValidationReport {
    /// Field that failed validation, or the tool name for read failures.
    pub field: String,
    /// Human-readable description of the issue.
    pub message: String,
    /// Severity: `warning` or `error`.
    pub severity: String,
}

/// Serializable payload of `podras config`.
#[derive(Debug, Serialize)]
pub struct ConfigReport {
    /// Discovered config files, one per registered tool.
    pub files: Vec<ConfigFileReport>,
    /// Validation results across the discovered files.
    pub validations: Vec<ConfigValidationReport>,
}

/// Discover and validate podras's config files in the repository (read-only).
pub fn inspect(repo: &Path) -> Result<ConfigReport> {
    let registry = registry();
    let store = ConfigStore::new(registry);

    let files = ConfigStore::discover(repo, store.registry())
        .into_iter()
        .map(|discovered| ConfigFileReport {
            tool: discovered.tool_name,
            marker: discovered.marker,
            found: discovered.found,
        })
        .collect();

    let validations = store
        .validate_all(repo)
        .into_iter()
        .map(|validation: ConfigValidation| {
            let severity = match validation.severity {
                genesis::config::ValidationSeverity::Warning => "warning",
                genesis::config::ValidationSeverity::Error => "error",
            };
            ConfigValidationReport {
                field: validation.field,
                message: validation.message,
                severity: severity.to_string(),
            }
        })
        .collect();

    Ok(ConfigReport { files, validations })
}
