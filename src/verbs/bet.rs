//! Purpose: implement `podras bet add|admit|defer|kill` (stage 7).
//!
//! Responsibilities:
//! - append one `shaping` bet (`add`), stamping `last_reviewed` only;
//! - guard and record the shaping-to-committed admission (`admit`);
//! - guard and record deferral from an active state (`defer`);
//! - kill a bet from any status, optionally recording a reason (`kill`);
//! - always load, mutate, validate, then atomically save; never write over a
//!   malformed file.
//!
//! Rationale: the roadmap file is a human decision record (decision 17), so
//! these verbs are its only writers (decision 26) and every one of them first
//! loads and validates the existing file: a file that fails load or validate
//! blocks the write with the lens's error codes instead of being overwritten.
//! The field set is closed, so a transition stamps no new field: the decision
//! date lands in `last_reviewed`, and admission additionally stamps the bet's
//! `admitted_at` (decision 22). Deferral from shaping, committed, or building
//! is one dated decision — stopping mid-build is a deferral, not a fifth
//! state — recorded as `status = "deferred"` plus `revive_when`. Kill is legal
//! from every status; `killed_reason` is recorded exactly when `--reason` is
//! given, and `review_or_kill` / `revive_when` are cleared because they are
//! forbidden on a killed bet. `bet start` and `bet ship` stay human hand-edits
//! (decision 26) and are deliberately absent.

use std::path::Path;

use chrono::NaiveDate;

use crate::error::{Error, Result};
use crate::model::{BetStatus, Horizon};
use crate::roadmap::file::{self, BetEntry, RoadmapFile};

/// Append one `shaping` bet, stamping `last_reviewed` with the decision date.
///
/// The arguments of one `bet add` invocation, minus the repo path and the
/// decision date. Groups the verb's seven descriptive parameters so the
/// signature stays within clippy's arity gate.
#[derive(Debug, Clone)]
pub struct AddBet {
    pub id: String,
    pub title: String,
    pub outcome: String,
    pub horizon: String,
    pub appetite_weeks: u32,
    pub owner: String,
    pub epic: Option<String>,
}

/// Append one `shaping` bet, stamping `last_reviewed` with the decision date.
///
/// The file must already exist and be valid: it is human-scaffolded, and no
/// verb creates it. `admitted_at` stays absent; admission stamps it.
pub fn add(repo: &Path, bet: AddBet, as_of: NaiveDate) -> Result<()> {
    let AddBet {
        id,
        title,
        outcome,
        horizon,
        appetite_weeks,
        owner,
        epic,
    } = &bet;
    let horizon = parse_horizon(horizon)?;
    let mut state = load_state(repo)?;
    if state.bets.iter().any(|bet| bet.id == *id) {
        return Err(Error::Domain {
            code: "bet_invalid",
            message: format!("bet `{id}` already exists; bet ids are unique"),
            suggestion: Some("choose another --id, or review the existing bet".to_string()),
        });
    }
    state.bets.push(BetEntry {
        id: id.to_string(),
        title: title.to_string(),
        outcome: outcome.to_string(),
        horizon,
        appetite_weeks: *appetite_weeks,
        owner: owner.to_string(),
        epic: epic.as_deref().map(str::to_string),
        status: BetStatus::Shaping,
        admitted_at: None,
        last_reviewed: stamp(as_of),
        review_or_kill: None,
        revive_when: None,
        killed_reason: None,
    });
    write_state(repo, state)?;
    println!("added bet `{id}` (shaping)");
    Ok(())
}

/// Admit a shaping bet: the dated shaping-to-committed decision.
///
/// Stamps `admitted_at` and `last_reviewed` with the decision date and links
/// the bet's single tracker epic. Any other current status is rejected.
pub fn admit(
    repo: &Path,
    id: &str,
    epic: &str,
    review_or_kill: &str,
    as_of: NaiveDate,
) -> Result<()> {
    let mut replaced = false;
    transition(repo, id, as_of, |bet| {
        if bet.status != BetStatus::Shaping {
            return Err(transition_error(
                id,
                bet.status,
                "admission is shaping to committed only",
            ));
        }
        replaced = bet.epic.is_some();
        bet.status = BetStatus::Committed;
        bet.admitted_at = Some(stamp(as_of));
        bet.epic = Some(epic.to_string());
        bet.review_or_kill = Some(review_or_kill.to_string());
        Ok(())
    })?;
    if replaced {
        println!("note: replaced the epic the shaping bet already carried");
    }
    println!("admitted bet `{id}` (committed, review or kill by {review_or_kill})");
    Ok(())
}

/// Defer an active bet: shaping, committed, or building, into deferred.
///
/// Stopping mid-build is a deferral, not a fifth state. Stamps `last_reviewed`
/// with the decision date (the field set is closed, so deferral adds no new
/// field), sets `revive_when`, and clears `review_or_kill`, which is forbidden
/// on a deferred bet.
pub fn defer(repo: &Path, id: &str, revive_when: &str, as_of: NaiveDate) -> Result<()> {
    transition(repo, id, as_of, |bet| {
        if !matches!(
            bet.status,
            BetStatus::Shaping | BetStatus::Committed | BetStatus::Building
        ) {
            return Err(transition_error(
                id,
                bet.status,
                "deferral is from shaping, committed, or building only; killing is the only exit \
                 from shipped or killed",
            ));
        }
        bet.status = BetStatus::Deferred;
        bet.revive_when = Some(revive_when.to_string());
        bet.review_or_kill = None;
        Ok(())
    })?;
    println!("deferred bet `{id}` until {revive_when}");
    Ok(())
}

/// Kill a bet from any status, optionally recording why.
///
/// Stamps `last_reviewed` with the decision date, records `killed_reason`
/// exactly when `--reason` is given, and clears `review_or_kill` and
/// `revive_when`, which are forbidden on a killed bet.
pub fn kill(repo: &Path, id: &str, reason: Option<&str>, as_of: NaiveDate) -> Result<()> {
    let reason = reason.map(str::trim).filter(|reason| !reason.is_empty());
    transition(repo, id, as_of, |bet| {
        bet.status = BetStatus::Killed;
        bet.killed_reason = reason.map(str::to_string);
        bet.review_or_kill = None;
        bet.revive_when = None;
        Ok(())
    })?;
    match reason {
        Some(reason) => println!("killed bet `{id}`: {reason}"),
        None => println!("killed bet `{id}`"),
    }
    Ok(())
}

/// Load, find, mutate, re-validate, and atomically save the roadmap file.
fn transition(
    repo: &Path,
    id: &str,
    as_of: NaiveDate,
    apply: impl FnOnce(&mut BetEntry) -> Result<()>,
) -> Result<()> {
    let mut state = load_state(repo)?;
    let bet = state
        .bets
        .iter_mut()
        .find(|bet| bet.id == id)
        .ok_or_else(|| unknown_bet(id))?;
    apply(bet)?;
    bet.last_reviewed = stamp(as_of);
    write_state(repo, state)
}

/// Load the validated roadmap file, refusing an absent one.
///
/// The file is human-scaffolded; no verb creates it, so an absent file is the
/// same `roadmap_missing` domain error the lens reports.
fn load_state(repo: &Path) -> Result<RoadmapFile> {
    file::load(repo)?.ok_or_else(|| Error::Domain {
        code: "roadmap_missing",
        message: format!(
            "{}: no roadmap file; scaffold it by hand before running bet verbs",
            file::ROADMAP_REL_PATH
        ),
        suggestion: Some(
            "write an [identity] table (docs/design.md decision 17); verbs never create \
             roadmap state"
                .to_string(),
        ),
    })
}

/// Re-validate and atomically save the mutated file.
///
/// Re-validation covers the fields the verb just set (slug ids, dates, the
/// review-or-kill and revive-when presence rules), so an invalid write is a
/// domain error and the file keeps its previous bytes.
fn write_state(repo: &Path, state: RoadmapFile) -> Result<()> {
    file::validate(&state).map_err(|message| Error::Domain {
        code: "roadmap_invalid",
        message: format!("{}: {message}", file::ROADMAP_REL_PATH),
        suggestion: Some(
            "fix the flagged fields; the roadmap schema is docs/design.md decision 17".to_string(),
        ),
    })?;
    file::save(repo, &state)
}

/// The closed `horizon` enum member for `bet add`.
fn parse_horizon(raw: &str) -> Result<Horizon> {
    match raw {
        "now" => Ok(Horizon::Now),
        "next" => Ok(Horizon::Next),
        "later" => Ok(Horizon::Later),
        other => Err(Error::Domain {
            code: "bet_invalid_horizon",
            message: format!("`{other}` is not a horizon; use now, next, or later"),
            suggestion: Some("pass --horizon now|next|later".to_string()),
        }),
    }
}

/// The unknown-id domain error, naming the bet.
fn unknown_bet(id: &str) -> Error {
    Error::Domain {
        code: "bet_unknown",
        message: format!(
            "no bet `{id}` in {}; list the bets with `podras roadmap`",
            file::ROADMAP_REL_PATH
        ),
        suggestion: Some("check the bet id against `podras roadmap`".to_string()),
    }
}

/// The transition-guard domain error, naming the bet and the violated rule.
fn transition_error(id: &str, status: BetStatus, rule: &str) -> Error {
    Error::Domain {
        code: "bet_invalid_transition",
        message: format!(
            "cannot transition bet `{id}` from `{}`: {rule}",
            status_name(status)
        ),
        suggestion: Some("check the bet's status with `podras roadmap`".to_string()),
    }
}

/// Serde name of a status, used verbatim in error messages.
fn status_name(status: BetStatus) -> &'static str {
    match status {
        BetStatus::Shaping => "shaping",
        BetStatus::Committed => "committed",
        BetStatus::Building => "building",
        BetStatus::Deferred => "deferred",
        BetStatus::Shipped => "shipped",
        BetStatus::Killed => "killed",
    }
}

/// The decision date in the `YYYY-MM-DD` form the state file stores.
fn stamp(as_of: NaiveDate) -> String {
    as_of.format("%Y-%m-%d").to_string()
}
