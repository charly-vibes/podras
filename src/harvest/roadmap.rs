//! Purpose: derive roadmap-sourced harvest candidates.
//!
//! Responsibilities:
//! - emit `identity_review` when `identity.next_review` is strictly earlier
//!   than the evaluation date;
//! - emit one `retire_or_reshape` candidate per non-terminal bet that trips
//!   the circuit breaker, the idle-epic horizon, or a closed epic;
//! - degrade a malformed or unreadable roadmap into one warning and zero
//!   candidates instead of failing the run.
//!
//! Rationale: candidates are proposals, never decisions (decision 23), so this
//! module only names the bet and the triggering reason and never rewrites any
//! file. The predicates are imported from `crate::roadmap::warnings` so the
//! lens and harvest can never disagree. Unlike the lens, the five-bet envelope
//! bound does not apply here: harvest computes over every bet in the file,
//! because a dropped candidate is a lost prompt and the candidate list is
//! already unbounded. Ordering is the lens status rank, then `admitted_at`,
//! then id, with the single identity candidate last.

use std::path::Path;

use chrono::NaiveDate;

use super::{Candidate, CandidateKind, Evidence, EvidenceKind};
use crate::curation::parse_human_date;
use crate::error::Error;
use crate::health::links::SIGNAL_HORIZON_DAYS;
use crate::model::{BetStatus, SourceName, Warning};
use crate::readers::beads::{IssueRecord, read_records};
use crate::roadmap::file::{BetEntry, RoadmapFile, load};
use crate::roadmap::{circuit_breaker_overrun, epic_idle_days, epic_record};

/// Evidence ref citing the human-owned roadmap state file.
const ROADMAP_REF: &str = ".podras/roadmap.toml";

/// Lens status rank: the lifecycle's declaration order, the lens sort order.
fn status_rank(status: BetStatus) -> u8 {
    match status {
        BetStatus::Building => 0,
        BetStatus::Committed => 1,
        BetStatus::Deferred => 2,
        BetStatus::Shaping => 3,
        BetStatus::Shipped => 4,
        BetStatus::Killed => 5,
    }
}

/// True for the statuses a retire-or-reshape prompt still applies to.
fn non_terminal(status: BetStatus) -> bool {
    !matches!(status, BetStatus::Shipped | BetStatus::Killed)
}

/// One roadmap candidate keyed by the deterministic sort key.
struct Derived {
    rank: u8,
    sort_key: NaiveDate,
    id: String,
    candidate: Candidate,
}

/// Derive every roadmap candidate plus at most one degradation warning.
///
/// Absent file: no candidates, no warning — no roadmap means nothing to
/// propose. Malformed or unreadable file: zero roadmap candidates and one
/// `SourceName::Curation` warning mirroring the brief's degrade philosophy
/// (design decision D1), so a broken human-owned state file never blocks a
/// read-only run. Nothing here writes anything.
pub(super) fn candidates(
    repo: &Path,
    evaluation_date: NaiveDate,
) -> (Vec<Candidate>, Option<Warning>) {
    let file = match load(repo) {
        Ok(Some(file)) => file,
        Ok(None) => return (Vec::new(), None),
        Err(error) => {
            let (code, message) = match &error {
                Error::Domain { code, message, .. } => (*code, message.clone()),
                other => ("roadmap_unreadable", other.to_string()),
            };
            let warning = Warning {
                source: SourceName::Curation,
                code: code.to_string(),
                detail: format!("roadmap: {message}"),
            };
            return (Vec::new(), Some(warning));
        }
    };

    let records = read_records(repo);
    let mut derived: Vec<Derived> = file
        .bets
        .iter()
        .filter(|bet| non_terminal(bet.status))
        .filter_map(|bet| retire_or_reshape(bet, &records, evaluation_date))
        .collect();
    derived.sort_by(|left, right| {
        left.rank
            .cmp(&right.rank)
            .then_with(|| left.sort_key.cmp(&right.sort_key))
            .then_with(|| left.id.cmp(&right.id))
    });
    let mut candidates: Vec<Candidate> = derived
        .into_iter()
        .map(|derived| derived.candidate)
        .collect();

    if let Some(candidate) = identity_review(&file, evaluation_date) {
        candidates.push(candidate);
    }
    (candidates, None)
}

/// The retire-or-reshape candidate for one bet, when a trigger fires.
///
/// Triggers reuse the lens's own predicates — shared with the lens warnings
/// (the 1.25x circuit breaker and the fourteen-day idle horizon) so the lens
/// and harvest can never disagree.
/// The circuit breaker ages admitted bets only — an unadmitted bet has no
/// appetite clock — while the epic predicates reach every non-terminal
/// status: a shaping or deferred bet that tripped one is exactly the row a
/// human needs to see. One candidate per bet even when several triggers
/// fire, with the reasons joined in trigger order.
fn retire_or_reshape(
    bet: &BetEntry,
    records: &[IssueRecord],
    evaluation_date: NaiveDate,
) -> Option<Derived> {
    // Admission is optional until a verb admits the bet, so the circuit
    // breaker — which ages the bet against its appetite — only applies to
    // admitted bets; an unadmitted bet has no appetite clock. The epic
    // predicates reach every non-terminal bet regardless.
    let admitted_at = bet.admitted_at.as_deref().and_then(parse_human_date);
    let mut reasons = Vec::new();

    if let Some(admitted_at) = admitted_at
        && circuit_breaker_overrun(admitted_at, bet.appetite_weeks, evaluation_date)
    {
        let appetite_days = i64::from(bet.appetite_weeks) * 7;
        let age_days = (evaluation_date - admitted_at).num_days();
        reasons.push(format!(
            "circuit_breaker_due: admitted {age_days}d ago against a {appetite_days}d appetite"
        ));
    }

    if let Some(record) = epic_record(bet.epic.as_deref(), records) {
        if !record.open {
            reasons.push(format!(
                "bet_epic_closed_not_retired: epic `{}` is closed",
                record.id
            ));
        } else if let Some(idle_days) = epic_idle_days(record, evaluation_date)
            && idle_days > SIGNAL_HORIZON_DAYS
        {
            reasons.push(format!(
                "bet_no_transition: epic `{}` has not changed in {idle_days}d",
                record.id
            ));
        }
    }

    if reasons.is_empty() {
        return None;
    }

    Some(Derived {
        rank: status_rank(bet.status),
        // Unadmitted bets sort by their last review, keeping the order total
        // and deterministic without inventing an admission date.
        sort_key: admitted_at
            .unwrap_or_else(|| parse_human_date(&bet.last_reviewed).unwrap_or(evaluation_date)),
        id: bet.id.clone(),
        candidate: Candidate {
            kind: CandidateKind::RetireOrReshape,
            summary: format!("bet `{}`: {}", bet.id, reasons.join("; ")),
            evidence: vec![Evidence {
                kind: EvidenceKind::Roadmap,
                reference: ROADMAP_REF.to_string(),
            }],
        },
    })
}

/// The single identity candidate, when the schedule has passed.
///
/// An absent `next_review` means staleness is unknown, not overdue (decision
/// 21), so it emits nothing; equal or later is fresh. Only the strictly
/// earlier case proposes a review.
fn identity_review(file: &RoadmapFile, evaluation_date: NaiveDate) -> Option<Candidate> {
    let next_review = file
        .identity
        .next_review
        .as_deref()
        .and_then(parse_human_date)?;
    if next_review >= evaluation_date {
        return None;
    }
    Some(Candidate {
        kind: CandidateKind::IdentityReview,
        summary: format!("identity_review: next_review passed on {next_review}"),
        evidence: vec![Evidence {
            kind: EvidenceKind::Roadmap,
            reference: ROADMAP_REF.to_string(),
        }],
    })
}
