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
use crate::model::Brief;
use crate::readers::{self, ReadContext};

/// Render the brief for the repository.
///
/// Degraded sources are values, not failures: every source (present, missing,
/// or unreadable) still produces a row and the command exits 0. Only a write
/// failure or an unusable render is a hard error.
///
/// Output: the narrative by default, the envelope with `--json`; `--write`
/// commits both artifacts into `out_dir` (the repository root unless supplied)
/// and prints their names instead of the narrative, or the envelope when
/// `--json` is also set.
pub fn brief(
    repo: &Path,
    json: bool,
    write: bool,
    as_of: Option<NaiveDate>,
    out_dir: Option<&Path>,
) -> Result<()> {
    let inputs = readers::collect(&read_context(repo, as_of))?;
    let brief = crate::brief::assemble(&inputs);
    let reentry = inputs
        .whisper
        .as_ref()
        .and_then(|whisper| whisper.next_action.as_deref());
    emit_brief(repo, &brief, reentry, json, write, out_dir)
}

/// Emit the brief: envelope, write report, or narrative, in that precedence.
fn emit_brief(
    repo: &Path,
    brief: &Brief,
    reentry: Option<&str>,
    json: bool,
    write: bool,
    out_dir: Option<&Path>,
) -> Result<()> {
    let target = out_dir.unwrap_or(repo);
    if write {
        crate::write::write_renders_to(repo, target, brief, reentry)?;
    }
    if json {
        return print_envelope(brief);
    }
    if write {
        return print_written(out_dir.is_some(), target);
    }
    print_narrative(brief, reentry)
}

/// Print the brief envelope, trimming the trailing newline.
fn print_envelope(brief: &Brief) -> Result<()> {
    println!("{}", crate::render::envelope::to_json(brief)?.trim_end());
    Ok(())
}

/// Print the names of the two written artifacts.
fn print_written(named: bool, target: &Path) -> Result<()> {
    if named {
        println!(
            "wrote {}, {}",
            target.join(crate::write::MARKDOWN_FILE).display(),
            target.join(crate::write::JSON_FILE).display()
        );
    } else {
        println!(
            "wrote {}, {}",
            crate::write::MARKDOWN_FILE,
            crate::write::JSON_FILE
        );
    }
    Ok(())
}

/// Print the human narrative.
fn print_narrative(brief: &Brief, reentry: Option<&str>) -> Result<()> {
    let narrative = crate::render::markdown::render(brief, reentry)?;
    println!("{}", narrative.trim_end());
    Ok(())
}

/// Harvest mismatch candidates without writing curation or tracker state.
///
/// Degraded sources are values, not failures: harvest exits 0 with the same
/// source rows and warnings as the brief. There is deliberately no `--write`,
/// no output directory, and no curation flag: candidates are proposed, never
/// promoted, filed, or closed.
pub fn harvest(repo: &Path, json: bool, as_of: Option<NaiveDate>) -> Result<()> {
    let inputs = readers::collect(&read_context(repo, as_of))?;
    let harvest = crate::harvest::assemble(repo, &inputs);

    if json {
        println!("{}", crate::harvest::to_json(&harvest)?.trim_end());
    } else {
        println!("{}", crate::harvest::render(&harvest).trim_end());
    }

    Ok(())
}

/// Build the read context for one run.
fn read_context(repo: &Path, as_of: Option<NaiveDate>) -> ReadContext {
    ReadContext {
        repo_root: repo.to_path_buf(),
        as_of,
        branch: current_branch(repo),
    }
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
