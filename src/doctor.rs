//! Purpose: `podras doctor` — repository health diagnostics over podras's
//! adopted surfaces.
//!
//! Responsibilities:
//! - implement genesis `DoctorCheck`s for the beads export, the openspec
//!   directory, `.podras/curation.toml`, `.podras/roadmap.toml`, and the
//!   kaya ledger;
//! - run them through `genesis::doctor::DoctorRunner` behind [`run`] and
//!   share one runner with the status bridge via [`runner`].
//!
//! Rationale: the doctor observes, it never repairs: no check offers an
//! auto-fix, because every surface is scaffolded by a dedicated verb
//! (`podras init`) or an external tool, and a diagnostic that mutates
//! human-owned state could never be safe by default. A missing artifact is
//! reported with its remediation hint; a present-but-broken one fails with
//! the same message the real parser produces.

use std::path::Path;

use genesis::doctor::{DoctorCheck, DoctorReport, DoctorRunner};
use genesis::suite_linter::{LintResult, Severity};

use crate::error::{Error, Result};
use crate::roadmap::file::ROADMAP_REL_PATH;

/// Repository-relative path of the beads export the readers consume.
const BEADS_EXPORT: &str = ".beads/issues.jsonl";
/// Repository-relative path of the openspec change directory.
const OPENSPEC_DIR: &str = "openspec";
/// Repository-relative path of the kaya ledger.
const KAYA_LEDGER: &str = ".kaya/ledger.jsonl";
/// Remediation for a missing roadmap file, verbatim from the roadmap verb.
const ROADMAP_FIX: &str = "create .podras/roadmap.toml with the [identity], [[bets]] and [[no_gos]] tables described in README.md#roadmap-state";

/// The runner behind `podras doctor`, shared with the status bridge.
pub fn runner() -> DoctorRunner {
    DoctorRunner::new(vec![
        Box::new(BeadsExportCheck),
        Box::new(OpenspecDirCheck),
        Box::new(CurationCheck),
        Box::new(RoadmapCheck),
        Box::new(KayaLedgerCheck),
    ])
    .with_tool_name("podras")
}

/// Run the podras doctor checks against the repository.
pub fn run(repo: &Path, fix: bool) -> Result<DoctorReport> {
    runner().run(repo, fix).map_err(|error| Error::Domain {
        code: "doctor_failed",
        message: error.to_string(),
        suggestion: None,
    })
}

/// Check that the beads export the readers consume is present.
struct BeadsExportCheck;

impl DoctorCheck for BeadsExportCheck {
    fn name(&self) -> &'static str {
        "beads-export"
    }

    fn description(&self) -> &'static str {
        "Check that the .beads/issues.jsonl export is present"
    }

    fn run(&self, repo: &Path) -> std::result::Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if repo.join(BEADS_EXPORT).exists() {
            return Ok(vec![]);
        }
        Ok(vec![LintResult::with_fix(
            format!("no {BEADS_EXPORT} export"),
            Severity::Error,
            "bd export",
        )])
    }
}

/// Check that the openspec change directory is present.
struct OpenspecDirCheck;

impl DoctorCheck for OpenspecDirCheck {
    fn name(&self) -> &'static str {
        "openspec-dir"
    }

    fn description(&self) -> &'static str {
        "Check that the openspec/ directory is present"
    }

    fn run(&self, repo: &Path) -> std::result::Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if repo.join(OPENSPEC_DIR).is_dir() {
            return Ok(vec![]);
        }
        Ok(vec![LintResult::with_fix(
            "no openspec/ directory",
            Severity::Warning,
            "openspec init",
        )])
    }
}

/// Check that `.podras/curation.toml`, when present, parses.
///
/// A missing curation file fails with the `podras init` remediation: the
/// verb is idempotent and never destroys existing curation bytes.
struct CurationCheck;

impl DoctorCheck for CurationCheck {
    fn name(&self) -> &'static str {
        "curation"
    }

    fn description(&self) -> &'static str {
        "Check that .podras/curation.toml parses"
    }

    fn run(&self, repo: &Path) -> std::result::Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        match crate::curation::load(repo) {
            Ok(Some(_)) => Ok(vec![]),
            Ok(None) => Ok(vec![LintResult::with_fix(
                format!("no {}", crate::curation::CURATION_REL_PATH),
                Severity::Error,
                "podras init",
            )]),
            Err(error) => Ok(vec![LintResult::with_fix(
                format!("invalid `{}`: {error}", crate::curation::CURATION_REL_PATH),
                Severity::Error,
                "podras init",
            )]),
        }
    }
}

/// Check that `.podras/roadmap.toml`, when present, parses.
///
/// A missing roadmap fails: the roadmap verb hard-errors on the same
/// condition, so the doctor mirrors the verb rather than inventing a softer
/// verdict.
struct RoadmapCheck;

impl DoctorCheck for RoadmapCheck {
    fn name(&self) -> &'static str {
        "roadmap"
    }

    fn description(&self) -> &'static str {
        "Check that .podras/roadmap.toml parses"
    }

    fn run(&self, repo: &Path) -> std::result::Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        match crate::roadmap::file::load(repo) {
            Ok(Some(_)) => Ok(vec![]),
            Ok(None) => Ok(vec![LintResult::with_fix(
                format!("no {ROADMAP_REL_PATH} for this repository"),
                Severity::Error,
                ROADMAP_FIX,
            )]),
            Err(error) => Ok(vec![LintResult::with_fix(
                format!("invalid `{ROADMAP_REL_PATH}`: {error}"),
                Severity::Error,
                ROADMAP_FIX,
            )]),
        }
    }
}

/// Check that the kaya ledger is present.
struct KayaLedgerCheck;

impl DoctorCheck for KayaLedgerCheck {
    fn name(&self) -> &'static str {
        "kaya-ledger"
    }

    fn description(&self) -> &'static str {
        "Check that the .kaya/ledger.jsonl ledger is present"
    }

    fn run(&self, repo: &Path) -> std::result::Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if repo.join(KAYA_LEDGER).exists() {
            return Ok(vec![]);
        }
        Ok(vec![LintResult::with_fix(
            format!("no {KAYA_LEDGER} ledger"),
            Severity::Warning,
            "capture work with the kaya CLI",
        )])
    }
}
