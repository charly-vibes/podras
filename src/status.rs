//! Purpose: `podras status` — the aggregated health dashboard over podras's
//! adopted surfaces.
//!
//! Responsibilities:
//! - implement `StatusContributor` for the config surface (registry
//!   discovery and validation results);
//! - surface the doctor checks through the genesis `DoctorStatusBridge`;
//! - assemble the `MultiToolStatus` report behind [`build`].
//!
//! Rationale: status is a read-only dashboard: a degraded surface is data,
//! not a failure, so the report always aggregates whatever the contributors
//! can observe and never blocks on an unhealthy repository.

use std::path::Path;

use genesis::status::{
    MultiToolStatus, StatusBuilder, StatusContributor, StatusItem, StatusSection,
};

use crate::error::Result;

/// Status contributor over podras's config files.
struct ConfigStatus;

impl StatusContributor for ConfigStatus {
    fn name(&self) -> &'static str {
        "podras-config"
    }

    fn status(&self, repo: &Path) -> std::result::Result<StatusSection, String> {
        let report = crate::config::inspect(repo).map_err(|error| error.to_string())?;

        let mut items: Vec<StatusItem> = report
            .files
            .iter()
            .map(|file| {
                if file.found {
                    StatusItem::healthy(&file.tool, &file.marker)
                } else {
                    StatusItem::warning(&file.tool, format!("{} missing", file.marker))
                }
            })
            .collect();

        // Broken configs escalate the section level: one item per
        // validation, severity-matched.
        for validation in &report.validations {
            let item = if validation.severity == "error" {
                StatusItem::error(&validation.field, &validation.message)
            } else {
                StatusItem::warning(&validation.field, &validation.message)
            };
            items.push(item);
        }

        let errors = report
            .validations
            .iter()
            .filter(|validation| validation.severity == "error")
            .count();
        if errors > 0 {
            return Ok(StatusSection::with_items(
                self.name(),
                format!("{errors} config error(s)"),
                items,
            ));
        }

        let found = report.files.iter().filter(|file| file.found).count();
        if found == report.files.len() {
            return Ok(StatusSection::with_items(
                self.name(),
                "all config files present and valid",
                items,
            ));
        }

        Ok(
            StatusSection::with_items(self.name(), "some config files missing", items)
                .with_suggestion("podras init"),
        )
    }
}

/// Build the aggregated status report for the repository.
pub fn build(repo: &Path) -> Result<MultiToolStatus> {
    let mut builder = StatusBuilder::new();
    builder.register(Box::new(genesis::status::DoctorStatusBridge::new(
        "podras",
        crate::doctor::runner(),
    )));
    builder.register(Box::new(ConfigStatus));

    builder
        .build(repo)
        .map_err(|error| crate::error::Error::Domain {
            code: "status_failed",
            message: error,
            suggestion: None,
        })
}

/// podras's status envelope payload: genesis's aggregated report plus its
/// computed summary (genesis serializes only `sections`; `summary()` is a
/// method, so podras lifts it into the data it emits).
#[derive(Debug, serde::Serialize)]
pub struct StatusReport {
    /// Sections, one per registered contributor.
    pub sections: Vec<StatusSection>,
    /// Aggregate health counts.
    pub summary: genesis::status::StatusSummary,
}

impl StatusReport {
    /// Lift genesis's aggregate into podras's serializable payload.
    pub fn from_multi(multi: MultiToolStatus) -> Self {
        let summary = multi.summary();
        Self {
            sections: multi.sections,
            summary,
        }
    }

    /// Wrap the report in a genesis success envelope, deriving warnings from
    /// warning-level sections (mirrors `MultiToolStatus::to_envelope`).
    pub fn to_envelope(&self, cli_version: &str) -> genesis::envelope::Envelope<&Self> {
        let warnings: Vec<genesis::envelope::Warning> = self
            .sections
            .iter()
            .filter(|s| s.level == genesis::status::StatusLevel::Warning)
            .map(|s| genesis::envelope::Warning {
                rule_name: s.tool.clone(),
                entity_id: None,
                message: s.summary.clone(),
                suggested_remediation: s.suggestions.first().cloned(),
            })
            .collect();
        genesis::envelope::Envelope::success(
            cli_version,
            genesis::envelope::EnvelopeKind::Ok,
            self,
            warnings,
            vec![],
        )
    }
}
