//! Purpose: binary entry point for the podras CLI.
//!
//! Responsibilities:
//! - intercept `--version --json` before clap parses arguments;
//! - parse the v1 command surface and resolve the repository root;
//! - map domain errors to miette diagnostics and a failing exit code.
//!
//! Rationale: the binary is a thin boundary. All behavior lives in the library
//! so integration tests can exercise commands directly.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use miette::Report;

#[derive(Debug, Parser)]
#[command(
    name = "podras",
    version,
    about = "Per-repository project status brief for humans and AI agents"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    /// Verbosity plumbing from genesis (`-v`/`-vv`/`-vvv`, `-q`); honored by
    /// the doctor, config, feedback, and status subcommands.
    #[command(flatten)]
    verbose: genesis::guide::CliVerbosity,
    /// Output-format plumbing from genesis (`--json`/`--human`); on non-TTY
    /// output the new subcommands emit JSON envelopes by default.
    #[command(flatten)]
    format: genesis::guide::CliFormat,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Render the project status brief.
    Brief(BriefArgs),
    /// Render the roadmap lens: identity, bets, and boundaries.
    Roadmap(RoadmapArgs),
    /// Surface mismatch candidates without changing any state.
    Harvest(HarvestArgs),
    /// Select the curated focus item.
    Focus {
        /// Issue id to focus.
        id: String,
    },
    /// Park an item until a resurface trigger.
    Park(ParkArgs),
    /// Curate the bet lifecycle of the roadmap lens (stage 7).
    Bet(BetArgs),
    /// Scaffold curation state and the AGENTS.md managed block.
    Init,
    /// Emit the brief JSON Schema.
    Schema,
    /// Check podras's adopted surfaces and report their health.
    Doctor(DoctorArgs),
    /// Discover and validate podras's human-owned config files (read-only).
    Config,
    /// Report an issue against podras's repository.
    Feedback(FeedbackCliArgs),
    /// Aggregate podras's health surfaces into one report.
    Status,
}

/// Arguments of `podras doctor`.
#[derive(Debug, Args)]
struct DoctorArgs {
    /// Apply available auto-fixes (doctor currently offers none).
    #[arg(long)]
    fix: bool,
}

/// Arguments of `podras feedback`.
#[derive(Debug, Args)]
struct FeedbackCliArgs {
    /// Kind of feedback: bug, feature, question, or chore.
    #[arg(default_value = "bug")]
    kind: String,
    /// User-supplied issue title overriding the derived one.
    #[arg(long)]
    title: Option<String>,
    /// Print the issue body and gh command without submitting.
    #[arg(long)]
    dry_run: bool,
    /// Read the last error from scratch to auto-populate the body.
    #[arg(long)]
    from_last_error: bool,
}

#[derive(Debug, Args)]
struct BriefArgs {
    /// Emit the typed JSON envelope.
    #[arg(long)]
    json: bool,
    /// Write BRIEF.md and BRIEF.json.
    #[arg(long)]
    write: bool,
    /// Pin the observation instant (YYYY-MM-DD).
    #[arg(long, value_parser = clap::value_parser!(chrono::NaiveDate))]
    as_of: Option<chrono::NaiveDate>,
    /// Directory for the render pair; requires `--write`.
    #[arg(long, requires = "write")]
    out_dir: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct RoadmapArgs {
    /// Emit the typed JSON envelope.
    #[arg(long)]
    json: bool,
    /// Write ROADMAP.md and ROADMAP.json.
    #[arg(long)]
    write: bool,
    /// Pin the evaluation date (YYYY-MM-DD).
    #[arg(long, value_parser = clap::value_parser!(chrono::NaiveDate))]
    as_of: Option<chrono::NaiveDate>,
    /// Directory for the render pair; requires `--write`.
    #[arg(long, requires = "write")]
    out_dir: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct HarvestArgs {
    /// Emit the typed JSON envelope.
    #[arg(long)]
    json: bool,
    /// Pin the evaluation date (YYYY-MM-DD).
    #[arg(long, value_parser = clap::value_parser!(chrono::NaiveDate))]
    as_of: Option<chrono::NaiveDate>,
}

#[derive(Debug, Args)]
struct ParkArgs {
    /// Issue id to park.
    id: String,
    /// Why the item is parked.
    #[arg(long)]
    reason: String,
    /// `YYYY-MM-DD` date or `event:<name>` trigger.
    #[arg(long)]
    resurface: String,
}

/// `podras bet <subcommand>` — the roadmap bet lifecycle verbs.
#[derive(Debug, Args)]
struct BetArgs {
    #[command(subcommand)]
    action: BetAction,
}

#[derive(Debug, Subcommand)]
enum BetAction {
    /// Append one shaping bet to the roadmap file.
    Add(BetAddArgs),
    /// Admit a shaping bet: the dated shaping-to-committed decision.
    Admit(BetAdmitArgs),
    /// Defer a shaping, committed, or building bet until a revive trigger.
    Defer(BetDeferArgs),
    /// Kill a bet from any status, optionally recording why.
    Kill(BetKillArgs),
}

#[derive(Debug, Args)]
struct BetAddArgs {
    /// Slug id of the new bet, unique among bets.
    #[arg(long)]
    id: String,
    /// Imperative title, at most 60 characters.
    #[arg(long)]
    title: String,
    /// What changes if the bet lands.
    #[arg(long)]
    outcome: String,
    /// Declared horizon: now, next, or later.
    #[arg(long)]
    horizon: String,
    /// Appetite in weeks, at least 1.
    #[arg(long)]
    appetite_weeks: u32,
    /// Human owner of the bet.
    #[arg(long)]
    owner: String,
    /// The single tracker epic backing the bet.
    #[arg(long)]
    epic: Option<String>,
    /// Decision date (YYYY-MM-DD); defaults to today.
    #[arg(long, value_parser = clap::value_parser!(chrono::NaiveDate))]
    as_of: Option<chrono::NaiveDate>,
}

#[derive(Debug, Args)]
struct BetAdmitArgs {
    /// Bet id to admit.
    id: String,
    /// The single tracker epic backing the bet.
    #[arg(long)]
    epic: String,
    /// Dated review-or-kill decision (YYYY-MM-DD).
    #[arg(long)]
    review_or_kill: String,
    /// Decision date (YYYY-MM-DD); defaults to today.
    #[arg(long, value_parser = clap::value_parser!(chrono::NaiveDate))]
    as_of: Option<chrono::NaiveDate>,
}

#[derive(Debug, Args)]
struct BetDeferArgs {
    /// Bet id to defer.
    id: String,
    /// Date or trigger that revives the bet.
    #[arg(long)]
    revive_when: String,
    /// Decision date (YYYY-MM-DD); defaults to today.
    #[arg(long, value_parser = clap::value_parser!(chrono::NaiveDate))]
    as_of: Option<chrono::NaiveDate>,
}

#[derive(Debug, Args)]
struct BetKillArgs {
    /// Bet id to kill.
    id: String,
    /// Why the bet was killed.
    #[arg(long)]
    reason: Option<String>,
    /// Decision date (YYYY-MM-DD); defaults to today.
    #[arg(long, value_parser = clap::value_parser!(chrono::NaiveDate))]
    as_of: Option<chrono::NaiveDate>,
}

fn main() -> ExitCode {
    if genesis::cli::maybe_print_version_json("podras", env!("CARGO_PKG_VERSION")) {
        return ExitCode::SUCCESS;
    }

    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(report) => {
            eprintln!("{report:?}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> miette::Result<()> {
    let Cli {
        command,
        verbose,
        format,
    } = cli;
    let format = format.format();
    let verbosity = verbose.verbosity();
    match command {
        Command::Brief(args) => {
            let repo = repository()?;
            podras::commands::brief(
                &repo,
                args.json,
                args.write,
                args.as_of,
                args.out_dir.as_deref(),
            )
            .map_err(into_report)
        }
        Command::Roadmap(args) => {
            let repo = repository()?;
            podras::commands::roadmap(
                &repo,
                args.json,
                args.write,
                args.as_of,
                args.out_dir.as_deref(),
            )
            .map_err(into_report)
        }
        Command::Harvest(args) => {
            let repo = repository()?;
            podras::commands::harvest(&repo, args.json, args.as_of).map_err(into_report)
        }
        Command::Focus { id } => {
            let repo = repository()?;
            podras::commands::focus(&repo, &id).map_err(into_report)
        }
        Command::Park(args) => {
            let repo = repository()?;
            podras::commands::park(&repo, &args.id, &args.reason, &args.resurface)
                .map_err(into_report)
        }
        Command::Bet(args) => {
            let repo = repository()?;
            bet(&repo, args.action).map_err(into_report)
        }
        Command::Init => {
            let repo = repository()?;
            podras::commands::init(&repo).map_err(into_report)
        }
        Command::Schema => podras::commands::schema().map_err(into_report),
        Command::Doctor(args) => {
            let repo = repository()?;
            podras::commands::doctor(&repo, args.fix, format, verbosity).map_err(into_report)
        }
        Command::Config => {
            let repo = repository()?;
            podras::commands::config(&repo, format, verbosity).map_err(into_report)
        }
        Command::Feedback(args) => {
            let repo = repository()?;
            podras::commands::feedback(
                &repo,
                &args.kind,
                args.title.as_deref(),
                args.dry_run,
                args.from_last_error,
                format,
                verbosity,
            )
            .map_err(into_report)
        }
        Command::Status => {
            let repo = repository()?;
            podras::commands::status(&repo, format, verbosity).map_err(into_report)
        }
    }
}

/// Dispatch one bet verb, defaulting the decision date to today.
fn bet(repo: &std::path::Path, action: BetAction) -> podras::Result<()> {
    match action {
        BetAction::Add(args) => podras::commands::bet_add(
            repo,
            podras::verbs::bet::AddBet {
                id: args.id,
                title: args.title,
                outcome: args.outcome,
                horizon: args.horizon,
                appetite_weeks: args.appetite_weeks,
                owner: args.owner,
                epic: args.epic,
            },
            as_of_arg(&args.as_of),
        ),
        BetAction::Admit(args) => podras::commands::bet_admit(
            repo,
            &args.id,
            &args.epic,
            &args.review_or_kill,
            as_of_arg(&args.as_of),
        ),
        BetAction::Defer(args) => {
            podras::commands::bet_defer(repo, &args.id, &args.revive_when, as_of_arg(&args.as_of))
        }
        BetAction::Kill(args) => podras::commands::bet_kill(
            repo,
            &args.id,
            args.reason.as_deref(),
            as_of_arg(&args.as_of),
        ),
    }
}

/// The `--as-of` decision date, defaulting to today.
fn as_of_arg(as_of: &Option<chrono::NaiveDate>) -> chrono::NaiveDate {
    (*as_of).unwrap_or_else(|| chrono::Utc::now().date_naive())
}

fn repository() -> miette::Result<PathBuf> {
    genesis::git::repo_root()
        .map_err(|error| miette::miette!("cannot locate repository root: {error}"))
}

fn into_report(error: podras::Error) -> Report {
    Report::new(error)
}
