//! Purpose: implement the CLI commands shared by the binary and tests.
//!
//! Responsibilities:
//! - own one function per subcommand, operating on a resolved repository root;
//! - wire the brief read path: context, collect, assemble, render, optional write;
//! - emit the JSON Schema for the brief model.
//!
//! Rationale: keeping commands in the library lets integration tests and the
//! binary share exactly one dispatch layer; the binary only maps errors to
//! miette and exit codes.

use std::path::Path;
use std::process::Command;

use chrono::NaiveDate;

use crate::error::Result;
use crate::readers::{self, ReadContext};

/// Render the brief for the repository.
///
/// Degraded sources are values, not failures: every source (present, missing,
/// or unreadable) still produces a row and the command exits 0. Only a write
/// failure or an unusable render is a hard error.
pub fn brief(repo: &Path, json: bool, write: bool, as_of: Option<NaiveDate>) -> Result<()> {
    let context = ReadContext {
        repo_root: repo.to_path_buf(),
        as_of,
        branch: current_branch(repo),
    };
    let inputs = readers::collect(&context)?;
    let brief = crate::brief::assemble(&inputs);

    if json {
        let envelope = crate::render::envelope::to_json(&brief)?;
        println!("{}", envelope.trim_end());
    } else {
        let reentry = inputs
            .whisper
            .as_ref()
            .and_then(|whisper| whisper.next_action.as_deref());
        let narrative = crate::render::markdown::render(&brief, reentry)?;
        println!("{}", narrative.trim_end());
    }

    if write {
        let whisper_next_action = inputs
            .whisper
            .as_ref()
            .and_then(|whisper| whisper.next_action.as_deref());
        crate::write::write_renders(repo, &brief, whisper_next_action)?;
    }

    Ok(())
}

/// Resolve the current branch name without failing the brief.
///
/// A detached HEAD (`HEAD`) and an unreadable ref both yield `None`; git
/// degradation is reported by the git reader, not by this helper.
fn current_branch(repo: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if branch.is_empty() || branch == "HEAD" {
        None
    } else {
        Some(branch)
    }
}

/// Select the curated focus item for the repository.
pub fn focus(repo: &Path, id: &str) -> Result<()> {
    crate::verbs::focus::run(repo, id)
}

/// Park an item until its resurface trigger.
pub fn park(repo: &Path, id: &str, reason: &str, resurface: &str) -> Result<()> {
    crate::verbs::park::run(repo, id, reason, resurface)
}

/// Initialize podras state for the repository.
pub fn init(repo: &Path) -> Result<()> {
    crate::verbs::init::run(repo)
}

/// Emit the brief JSON Schema (2020-12) to stdout.
pub fn schema() -> Result<()> {
    print!("{}", crate::render::envelope::schema()?);
    Ok(())
}
