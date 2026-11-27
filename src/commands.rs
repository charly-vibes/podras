//! Purpose: implement the CLI commands shared by the binary and tests.
//!
//! Responsibilities:
//! - own one function per subcommand, operating on a resolved repository root;
//! - wire the brief read path: context, collect, assemble, render, optional write;
//! - wire the roadmap lens read path and its separate render/write pair;
//! - emit the JSON Schema for the brief model.
//!
//! Rationale: keeping commands in the library lets integration tests and the
//! binary share exactly one dispatch layer; the binary only maps errors to
//! miette and exit codes.

use std::path::Path;
use std::process::Command;

use chrono::NaiveDate;
use genesis::guide::{OutputFormat, Verbosity};

use crate::error::{Error, Result};
use crate::model::Brief;
use crate::readers::{self, ReadContext};
use crate::roadmap::file::ROADMAP_REL_PATH;

/// Render the brief for the repository.
///
/// Degraded sources are values, not failures: every source (present, missing,
/// or unreadable) still produces a row and the command exits 0. The roadmap
/// lens degrades the same way: a malformed or unreadable `.podras/roadmap.toml`
/// omits the `roadmap` object, appends one curation warning naming the load
/// failure, and still exits 0. Only a write failure or an unusable render is a
/// hard error.
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
    let brief = assemble_with_roadmap(repo, &inputs);
    let reentry = inputs
        .whisper
        .as_ref()
        .and_then(|whisper| whisper.next_action.as_deref());
    emit_brief(repo, &brief, reentry, json, write, out_dir)
}

/// Render the roadmap lens for the repository.
///
/// The lens is read-only and additive: `.podras/roadmap.toml` is never
/// scaffolded or written here, and a malformed file is a hard domain error
/// rather than a degraded source — `podras brief` is the command that degrades
/// and warns instead. Candidates and review dates are rendered, never
/// inferred; no verb admits, defers, or kills a bet.
pub fn roadmap(
    repo: &Path,
    json: bool,
    write: bool,
    as_of: Option<NaiveDate>,
    out_dir: Option<&Path>,
) -> Result<()> {
    // The file is read before any source collection: a missing or malformed
    // roadmap is reported without paying for a full read pass.
    let Some(file) = crate::roadmap::file::load(repo)? else {
        return Err(Error::Domain {
            code: "roadmap_missing",
            message: format!("no {ROADMAP_REL_PATH} for this repository"),
            suggestion: Some(
                "create .podras/roadmap.toml with the [identity], [[bets]] and [[no_gos]] tables described in README.md#roadmap-state"
                    .into(),
            ),
        });
    };
    let inputs = readers::collect(&read_context(repo, as_of))?;
    let evaluation = inputs.as_of.unwrap_or(inputs.now);
    let records = crate::readers::beads::read_records(repo);
    let view = crate::roadmap::assemble(&file, &records, evaluation);
    let brief = crate::roadmap::attach(crate::brief::assemble(&inputs), Some(view));

    let target = out_dir.unwrap_or(repo);
    if write {
        crate::write::write_roadmap_renders_to(target, &brief)?;
    }
    if json {
        return print_envelope(&brief);
    }
    if write {
        return print_pair_written(
            out_dir.is_some(),
            target,
            crate::write::ROADMAP_MARKDOWN_FILE,
            crate::write::ROADMAP_JSON_FILE,
        );
    }
    println!("{}", crate::render::roadmap::render(&brief)?.trim_end());
    Ok(())
}

/// Assemble the brief, attaching the roadmap lens when its file loads.
///
/// An absent file means the brief carries no `roadmap` object at all; a
/// present file, even one with zero bets, is a present lens. A file that fails
/// to load never fails the brief: the lens is omitted and exactly one curation
/// warning records why, so the command keeps its "degraded sources are values"
/// contract. `podras roadmap` is the verb where a broken file stays a hard
/// error.
fn assemble_with_roadmap(repo: &Path, inputs: &crate::brief::BriefInputs) -> Brief {
    let file = match crate::roadmap::file::load(repo) {
        Ok(Some(file)) => file,
        Ok(None) => return crate::brief::assemble(inputs),
        Err(error) => {
            let mut brief = crate::brief::assemble(inputs);
            brief.warnings.push(roadmap_load_warning(&error));
            crate::brief::sort_warnings_in_place(&mut brief.warnings);
            return crate::roadmap::attach(brief, None);
        }
    };
    let evaluation = inputs.as_of.unwrap_or(inputs.now);
    let records = crate::readers::beads::read_records(repo);
    let view = crate::roadmap::assemble(&file, &records, evaluation);
    crate::roadmap::attach(crate::brief::assemble(inputs), Some(view))
}

/// Describe a roadmap load failure as one curation warning.
///
/// A domain error contributes its own stable code; anything else (I/O and the
/// errors `load` does not classify) is `roadmap_unreadable`. The detail keeps
/// the loader's message verbatim behind a `roadmap:` prefix so the warning
/// names the failing file and rule.
fn roadmap_load_warning(error: &Error) -> crate::model::Warning {
    let (code, message) = match error {
        Error::Domain { code, message, .. } => (*code, message.clone()),
        other => ("roadmap_unreadable", other.to_string()),
    };
    crate::model::Warning {
        source: crate::model::SourceName::Curation,
        code: code.to_string(),
        detail: format!("roadmap: {message}"),
    }
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
    print_pair_written(
        named,
        target,
        crate::write::MARKDOWN_FILE,
        crate::write::JSON_FILE,
    )
}

/// Print two written artifact names, prefixing the directory when named.
fn print_pair_written(named: bool, target: &Path, markdown: &str, json: &str) -> Result<()> {
    if named {
        println!(
            "wrote {}, {}",
            target.join(markdown).display(),
            target.join(json).display()
        );
    } else {
        println!("wrote {markdown}, {json}");
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

/// Append one shaping bet to the roadmap file (stage 7).
pub fn bet_add(repo: &Path, bet: crate::verbs::bet::AddBet, as_of: NaiveDate) -> Result<()> {
    crate::verbs::bet::add(repo, bet, as_of)
}

/// Admit a shaping bet: the dated shaping-to-committed decision (stage 7).
pub fn bet_admit(
    repo: &Path,
    id: &str,
    epic: &str,
    review_or_kill: &str,
    as_of: NaiveDate,
) -> Result<()> {
    crate::verbs::bet::admit(repo, id, epic, review_or_kill, as_of)
}

/// Defer a shaping, committed, or building bet until its revive trigger.
pub fn bet_defer(repo: &Path, id: &str, revive_when: &str, as_of: NaiveDate) -> Result<()> {
    crate::verbs::bet::defer(repo, id, revive_when, as_of)
}

/// Kill a bet from any status, optionally recording why.
pub fn bet_kill(repo: &Path, id: &str, reason: Option<&str>, as_of: NaiveDate) -> Result<()> {
    crate::verbs::bet::kill(repo, id, reason, as_of)
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

/// Print a genesis envelope as pretty JSON, trimming the trailing newline.
fn print_genesis_envelope<T: serde::Serialize>(
    envelope: genesis::envelope::Envelope<T>,
) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&envelope)?.trim_end());
    Ok(())
}

/// The verbosity level above which normal human output is shown.
const NORMAL_VERBOSITY: u8 = genesis::guide::Verbosity::Normal as u8;

/// Run `podras doctor`: check podras's adopted surfaces and print the report.
///
/// The doctor is read-only: no check offers an auto-fix, so `--fix` can
/// never mutate state. A degraded surface is data, not a failure — the
/// command exits 0 whenever the checks could run, exactly like `brief`.
/// Output is a `doctor` JSON envelope on non-TTY (or `--json`) and
/// `[status]` check lines with `--human`.
pub fn doctor(repo: &Path, fix: bool, format: OutputFormat, verbosity: Verbosity) -> Result<()> {
    let report = crate::doctor::run(repo, fix)?;
    match format {
        OutputFormat::Json => print_genesis_envelope(report.to_envelope(env!("CARGO_PKG_VERSION"))),
        OutputFormat::Human => {
            if verbosity.should_show(NORMAL_VERBOSITY) {
                println!(
                    "podras doctor: {} pass, {} warn, {} fail",
                    report.summary.pass, report.summary.warn, report.summary.fail
                );
                for check in &report.checks {
                    let status = if check.status.is_pass() {
                        "pass"
                    } else if check.status.is_fail() {
                        "fail"
                    } else {
                        "warn"
                    };
                    println!("  [{status}] {}: {}", check.name, check.message);
                }
            }
            Ok(())
        }
    }
}

/// Run `podras config`: discover and validate podras's human-owned config
/// files (read-only).
///
/// The config adoption is read and validate side only; missing files are
/// reported as absent, never created. Output is a JSON envelope on non-TTY
/// (or `--json`) and one line per config file or validation with `--human`.
pub fn config(repo: &Path, format: OutputFormat, verbosity: Verbosity) -> Result<()> {
    let report = crate::config::inspect(repo)?;
    match format {
        OutputFormat::Json => {
            let warnings = report
                .validations
                .iter()
                .filter(|validation| validation.severity == "error")
                .map(|validation| genesis::envelope::Warning {
                    rule_name: validation.field.clone(),
                    entity_id: None,
                    message: validation.message.clone(),
                    suggested_remediation: None,
                })
                .collect();
            print_genesis_envelope(genesis::envelope::Envelope::success(
                env!("CARGO_PKG_VERSION"),
                genesis::envelope::EnvelopeKind::Ok,
                &report,
                warnings,
                Vec::new(),
            ))
        }
        OutputFormat::Human => {
            if verbosity.should_show(NORMAL_VERBOSITY) {
                for file in &report.files {
                    let state = if file.found { "found" } else { "missing" };
                    println!("{state}: {} ({})", file.tool, file.marker);
                }
                for validation in &report.validations {
                    println!(
                        "{}: {}: {}",
                        validation.severity, validation.field, validation.message
                    );
                }
            }
            Ok(())
        }
    }
}

/// Run `podras feedback`: report an issue against podras's repository.
///
/// The request delegates to `genesis::feedback::handle_feedback` with
/// podras's repo context; a dry run previews the body on stderr and the
/// outcome lands in the envelope payload on stdout (JSON) or one message
/// line (`--human`).
pub fn feedback(
    repo: &Path,
    kind: &str,
    title: Option<&str>,
    dry_run: bool,
    from_last_error: bool,
    format: OutputFormat,
    verbosity: Verbosity,
) -> Result<()> {
    let data = crate::feedback::run(repo, kind, title, dry_run, from_last_error)?;
    match format {
        OutputFormat::Json => print_genesis_envelope(genesis::envelope::Envelope::success(
            env!("CARGO_PKG_VERSION"),
            genesis::envelope::EnvelopeKind::Ok,
            &data,
            Vec::new(),
            Vec::new(),
        )),
        OutputFormat::Human => {
            if verbosity.should_show(NORMAL_VERBOSITY) {
                println!("{}", data.message);
            }
            Ok(())
        }
    }
}

/// Run `podras status`: aggregate podras's health surfaces into one report.
///
/// Status is read-only and never fails on a degraded repository: sections
/// carry the observed health and the command exits 0. Output is a JSON
/// envelope on non-TTY (or `--json`) and section lines with `--human`.
pub fn status(repo: &Path, format: OutputFormat, verbosity: Verbosity) -> Result<()> {
    let report = crate::status::build(repo)?;
    let report = crate::status::StatusReport::from_multi(report);
    match format {
        OutputFormat::Json => print_genesis_envelope(report.to_envelope(env!("CARGO_PKG_VERSION"))),
        OutputFormat::Human => {
            if verbosity.should_show(NORMAL_VERBOSITY) {
                let summary = report.summary;
                println!(
                    "podras status: {} tools, {} healthy, {} warning, {} error",
                    summary.total, summary.healthy, summary.warnings, summary.errors
                );
                for section in &report.sections {
                    let level = if section.level.is_error() {
                        "error"
                    } else if section.level.is_issue() {
                        "warning"
                    } else {
                        "healthy"
                    };
                    println!("  {}: {} ({})", section.tool, section.summary, level);
                    for item in &section.items {
                        println!("    {}: {}", item.label, item.value);
                    }
                    for suggestion in &section.suggestions {
                        println!("    -> {suggestion}");
                    }
                }
            }
            Ok(())
        }
    }
}
