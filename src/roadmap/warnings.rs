//! Purpose: decide the roadmap lens's warnings, staleness, and review dates.
//!
//! Responsibilities:
//! - derive identity staleness from `next_review` alone;
//! - fire the nine named `RoadmapWarningKind` predicates;
//! - match no-go terms as whole tokens against bead titles and labels;
//! - pick the earliest outstanding review per source and order the warnings.
//!
//! Rationale: roadmap health is named advisories, never a verdict: nothing here
//! touches `Health` or an exit code. Every predicate is a pure function of the
//! validated file, the tracker records, and the pinned evaluation date, so two
//! runs over identical inputs emit identical warnings. The one tunable is the
//! circuit breaker factor, kept here as the in-code default for the
//! `roadmap.circuit_breaker_factor` config key (decision 23).

use chrono::NaiveDate;

use crate::curation::parse_human_date;
use crate::health::links::SIGNAL_HORIZON_DAYS;
use crate::model::{
    BetStatus, BetView, Horizon, IdentityView, NoGoView, ReviewDue, RoadmapWarning,
    RoadmapWarningKind, Staleness,
};
use crate::readers::beads::IssueRecord;

use super::file::RoadmapFile;

/// Appetite multiplier after which `circuit_breaker_due` fires.
///
/// Documented as the config key `roadmap.circuit_breaker_factor`; the corpus
/// proposes 1.25x, 1.5x and 2x and this is the shipped default. The constant is
/// not configurable yet because no config layer exists for the roadmap lens.
pub const CIRCUIT_BREAKER_FACTOR: f64 = 1.25;

/// Subject carried by warnings that are about the project, not an entry.
const IDENTITY_SUBJECT: &str = "identity";

/// Build the identity row; staleness is decided by `next_review` alone.
///
/// An unparseable `last_reviewed` can only reach this function through an
/// unvalidated file, so it falls back to the evaluation date: the lens stays
/// total and deterministic instead of panicking or omitting the row.
pub(super) fn identity(file: &RoadmapFile, evaluation_date: NaiveDate) -> IdentityView {
    let entry = &file.identity;
    let last_reviewed = parse_human_date(&entry.last_reviewed).unwrap_or(evaluation_date);
    let next_review = entry.next_review.as_deref().and_then(parse_human_date);
    let staleness = match next_review {
        None => Staleness::Unknown,
        Some(date) if date < evaluation_date => Staleness::Stale,
        Some(_) => Staleness::Fresh,
    };
    IdentityView {
        path: entry.path.clone(),
        last_reviewed,
        next_review,
        staleness,
    }
}

/// True for the statuses that count as an active bet.
pub(super) fn active(status: BetStatus) -> bool {
    matches!(status, BetStatus::Committed | BetStatus::Building)
}

/// Earliest outstanding review per source.
///
/// `roadmap` is the minimum `review_or_kill` among active bets; a bet that is
/// shaping, deferred, shipped, or killed commits to no review, so it cannot
/// pull the date earlier. Neither field is a deadline: both are dates humans
/// already declared.
pub(super) fn review_due(identity: &IdentityView, bets: &[BetView]) -> ReviewDue {
    let roadmap = bets
        .iter()
        .filter(|bet| active(bet.status))
        .filter_map(|bet| bet.review_or_kill)
        .min();
    ReviewDue {
        identity: identity.next_review,
        roadmap,
    }
}

/// Collect every roadmap warning for the evaluated file.
///
/// Warning lists are never bounded; ordering happens here so the lens never
/// depends on hash-map or file iteration order.
pub(super) fn collect(
    identity: &IdentityView,
    bets: &[BetView],
    no_gos: &[NoGoView],
    records: &[IssueRecord],
    evaluation_date: NaiveDate,
) -> Vec<RoadmapWarning> {
    let mut warnings = Vec::new();

    if identity.next_review.is_none() {
        warnings.push(RoadmapWarning {
            code: RoadmapWarningKind::IdentityReviewOverdue,
            subject: IDENTITY_SUBJECT.to_string(),
            detail: "no next_review declared; identity staleness is unknown".to_string(),
        });
    }

    for bet in bets {
        bet_warnings(bet, records, evaluation_date, &mut warnings);
    }
    no_go_warnings(no_gos, records, &mut warnings);
    order(&mut warnings);
    warnings
}

/// Fire the bet-scoped predicates for one bet.
fn bet_warnings(
    bet: &BetView,
    records: &[IssueRecord],
    evaluation_date: NaiveDate,
    warnings: &mut Vec<RoadmapWarning>,
) {
    let active = active(bet.status);

    if active {
        let appetite_days = i64::from(bet.appetite_weeks) * 7;
        let age_days = (evaluation_date - bet.admitted_at).num_days();
        if age_days as f64 > appetite_days as f64 * CIRCUIT_BREAKER_FACTOR {
            warnings.push(RoadmapWarning {
                code: RoadmapWarningKind::CircuitBreakerDue,
                subject: bet.id.clone(),
                detail: format!(
                    "active {age_days}d, appetite {appetite_days}d, circuit breaker at {}d \
                     ({CIRCUIT_BREAKER_FACTOR}x)",
                    appetite_days as f64 * CIRCUIT_BREAKER_FACTOR
                ),
            });
        } else if age_days > appetite_days {
            warnings.push(RoadmapWarning {
                code: RoadmapWarningKind::BetAgeExceedsAppetite,
                subject: bet.id.clone(),
                detail: format!("active {age_days}d, appetite {appetite_days}d"),
            });
        }

        if bet.status == BetStatus::Committed && bet.epic.is_none() {
            warnings.push(RoadmapWarning {
                code: RoadmapWarningKind::BetEpicMissing,
                subject: bet.id.clone(),
                detail: "committed bet declares no epic".to_string(),
            });
        }

        if let Some(review_or_kill) = bet.review_or_kill
            && review_or_kill < evaluation_date
        {
            warnings.push(RoadmapWarning {
                code: RoadmapWarningKind::RoadmapReviewOverdue,
                subject: bet.id.clone(),
                detail: format!("review_or_kill passed on {review_or_kill}"),
            });
        }
    }

    let epic = bet
        .epic
        .as_deref()
        .and_then(|epic| records.iter().find(|record| record.id == epic));

    if let Some(record) = epic {
        if active && !record.open {
            warnings.push(RoadmapWarning {
                code: RoadmapWarningKind::BetEpicClosedNotRetired,
                subject: bet.id.clone(),
                detail: format!("epic `{}` is closed while the bet is active", record.id),
            });
        }
        if active && let Some(updated_at) = record.updated_at {
            let idle_days = (evaluation_date - updated_at.date_naive()).num_days();
            if idle_days > SIGNAL_HORIZON_DAYS {
                warnings.push(RoadmapWarning {
                    code: RoadmapWarningKind::BetNoTransition,
                    subject: bet.id.clone(),
                    detail: format!("epic `{}` has not changed in {idle_days}d", record.id),
                });
            }
        }
    }

    match bet.horizon {
        Horizon::Now if epic.is_none_or(|record| !record.open) => {
            let detail = match bet.epic.as_deref() {
                Some(id) => format!("horizon is now but epic `{id}` is missing or closed"),
                None => "horizon is now but no epic is linked".to_string(),
            };
            warnings.push(RoadmapWarning {
                code: RoadmapWarningKind::HorizonEpicMismatch,
                subject: bet.id.clone(),
                detail,
            });
        }
        Horizon::Later if epic.is_some_and(|record| !record.open) => {
            let epic_id = bet.epic.as_deref().unwrap_or_default();
            warnings.push(RoadmapWarning {
                code: RoadmapWarningKind::HorizonEpicMismatch,
                subject: bet.id.clone(),
                detail: format!("horizon is later but epic `{epic_id}` is closed"),
            });
        }
        _ => {}
    }
}

/// Fire `no_go_violation_candidate` for each open bead a no-go matches.
///
/// A no-go with no terms never fires: an empty list means "human review only",
/// not "matches everything".
fn no_go_warnings(
    no_gos: &[NoGoView],
    records: &[IssueRecord],
    warnings: &mut Vec<RoadmapWarning>,
) {
    for no_go in no_gos {
        if no_go.match_terms.is_empty() {
            continue;
        }
        for record in records.iter().filter(|record| record.open) {
            let matched = no_go
                .match_terms
                .iter()
                .find(|term| bead_matches(record, term.as_str()));
            if let Some(term) = matched {
                warnings.push(RoadmapWarning {
                    code: RoadmapWarningKind::NoGoViolationCandidate,
                    subject: no_go.id.clone(),
                    detail: format!("{} matched `{term}`", record.id),
                });
            }
        }
    }
}

/// True when the term matches the bead title or one of its labels.
fn bead_matches(record: &IssueRecord, term: &str) -> bool {
    token_matches(&record.title, term)
        || record.labels.iter().any(|label| token_matches(label, term))
}

/// True when `term` occurs in `text` delimited as a whole token.
///
/// Both sides are lowercased. A token character is `[a-z0-9_-]`, so
/// `auto-close` matches `block auto-close, always` but never `autoclose` or
/// `x_auto-close`, where the surrounding characters are still token
/// characters. An empty term matches nothing: it is a validation error, and
/// treating it as a match would make corrupt input fire everywhere.
pub(crate) fn token_matches(text: &str, term: &str) -> bool {
    let term = term.to_lowercase();
    if term.is_empty() {
        return false;
    }
    let text = text.to_lowercase();
    let mut offset = 0;
    while let Some(found) = text[offset..].find(&term) {
        let start = offset + found;
        let end = start + term.len();
        let left_boundary = text[..start]
            .chars()
            .next_back()
            .is_none_or(|character| !is_token_char(character));
        let right_boundary = text[end..]
            .chars()
            .next()
            .is_none_or(|character| !is_token_char(character));
        if left_boundary && right_boundary {
            return true;
        }
        // Advance one character past the false start, staying on a char
        // boundary; the term may begin with a multi-byte character.
        offset = start + text[start..].chars().next().map_or(1, char::len_utf8);
    }
    false
}

fn is_token_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '-' || character == '_'
}

/// Sort warnings by code declaration order, then subject, then detail.
pub(super) fn order(warnings: &mut [RoadmapWarning]) {
    warnings.sort_by(|left, right| {
        (left.code as usize)
            .cmp(&(right.code as usize))
            .then_with(|| left.subject.cmp(&right.subject))
            .then_with(|| left.detail.cmp(&right.detail))
    });
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone, Utc};

    use super::*;
    use crate::model::Enforcement;
    use crate::roadmap::file::IdentityEntry;
    use crate::roadmap::test_support::{
        bet, identity_view, no_go, record, roadmap_file, today, today_instant,
    };

    fn codes(warnings: &[RoadmapWarning]) -> Vec<RoadmapWarningKind> {
        warnings.iter().map(|warning| warning.code).collect()
    }

    #[test]
    fn identity_staleness_uses_next_review_alone() {
        let evaluation = today();
        let file = roadmap_file(
            IdentityEntry {
                path: "README.md#why".to_string(),
                last_reviewed: "2026-01-01".to_string(),
                next_review: None,
            },
            Vec::new(),
            Vec::new(),
        );
        let view = identity(&file, evaluation);
        assert_eq!(view.staleness, Staleness::Unknown);
        assert_eq!(view.next_review, None);
        assert_eq!(
            view.last_reviewed,
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        );

        let file = roadmap_file(
            IdentityEntry {
                path: "README.md#why".to_string(),
                last_reviewed: "2026-01-01".to_string(),
                next_review: Some("2026-11-17".to_string()),
            },
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(identity(&file, evaluation).staleness, Staleness::Stale);

        let file = roadmap_file(
            IdentityEntry {
                path: "README.md#why".to_string(),
                last_reviewed: "2026-01-01".to_string(),
                next_review: Some("2026-11-18".to_string()),
            },
            Vec::new(),
            Vec::new(),
        );
        // Equal to the evaluation date is fresh, not stale.
        assert_eq!(identity(&file, evaluation).staleness, Staleness::Fresh);
    }

    #[test]
    fn identity_review_overdue_fires_only_without_next_review() {
        let evaluation = today();
        let no_schedule = identity_view(None);
        let warnings = collect(&no_schedule, &[], &[], &[], evaluation);
        assert_eq!(
            codes(&warnings),
            vec![RoadmapWarningKind::IdentityReviewOverdue]
        );
        assert_eq!(warnings[0].subject, "identity");

        let scheduled = identity_view(Some("2027-01-01"));
        assert!(collect(&scheduled, &[], &[], &[], evaluation).is_empty());
    }

    #[test]
    fn age_warnings_escalate_to_the_circuit_breaker() {
        let evaluation = today();
        let mut over = bet("over", BetStatus::Building);
        over.appetite_weeks = 6;
        over.admitted_at = today() - chrono::Duration::days(43);
        let warnings = collect(
            &identity_view(Some("2027-01-01")),
            &[over],
            &[],
            &[],
            evaluation,
        );
        assert_eq!(
            codes(&warnings),
            vec![RoadmapWarningKind::BetAgeExceedsAppetite]
        );

        let mut breaker = bet("breaker", BetStatus::Building);
        breaker.appetite_weeks = 6;
        breaker.admitted_at = today() - chrono::Duration::days(53);
        let warnings = collect(
            &identity_view(Some("2027-01-01")),
            &[breaker],
            &[],
            &[],
            evaluation,
        );
        assert_eq!(
            codes(&warnings),
            vec![RoadmapWarningKind::CircuitBreakerDue]
        );

        // Exactly at appetite is not over it.
        let mut exactly = bet("exactly", BetStatus::Building);
        exactly.appetite_weeks = 6;
        exactly.admitted_at = today() - chrono::Duration::days(42);
        assert!(
            collect(
                &identity_view(Some("2027-01-01")),
                &[exactly],
                &[],
                &[],
                evaluation
            )
            .is_empty()
        );
    }

    #[test]
    fn inactive_bets_never_age_out() {
        let evaluation = today();
        for status in [
            BetStatus::Shaping,
            BetStatus::Deferred,
            BetStatus::Shipped,
            BetStatus::Killed,
        ] {
            let mut old = bet("old", status);
            old.admitted_at = today() - chrono::Duration::days(365);
            let warnings = collect(
                &identity_view(Some("2027-01-01")),
                &[old],
                &[],
                &[],
                evaluation,
            );
            assert!(
                !codes(&warnings).contains(&RoadmapWarningKind::BetAgeExceedsAppetite),
                "{status:?} should not age out"
            );
        }
    }

    #[test]
    fn committed_without_epic_fires() {
        let evaluation = today();
        let mut committed = bet("no-epic", BetStatus::Committed);
        committed.epic = None;
        let warnings = collect(
            &identity_view(Some("2027-01-01")),
            &[committed],
            &[],
            &[],
            evaluation,
        );
        assert!(
            codes(&warnings).contains(&RoadmapWarningKind::BetEpicMissing),
            "{warnings:?}"
        );
    }

    #[test]
    fn closed_epic_while_active_fires() {
        let evaluation = today();
        let mut active_bet = bet("active", BetStatus::Building);
        active_bet.epic = Some("podras-abc".to_string());
        let closed = record("podras-abc", "podras-abc", false, Some(today_instant()));
        let warnings = collect(
            &identity_view(Some("2027-01-01")),
            &[active_bet],
            &[],
            &[closed],
            evaluation,
        );
        assert!(codes(&warnings).contains(&RoadmapWarningKind::BetEpicClosedNotRetired));
    }

    #[test]
    fn stale_epic_update_fires() {
        let evaluation = today();
        let mut active_bet = bet("active", BetStatus::Building);
        active_bet.epic = Some("podras-abc".to_string());
        let old = Utc.with_ymd_and_hms(2026, 11, 1, 0, 0, 0).unwrap();
        let stale = record("podras-abc", "podras-abc", true, Some(old));
        let warnings = collect(
            &identity_view(Some("2027-01-01")),
            &[active_bet],
            &[],
            &[stale],
            evaluation,
        );
        assert!(codes(&warnings).contains(&RoadmapWarningKind::BetNoTransition));

        let fresh = record("podras-abc", "podras-abc", true, Some(today_instant()));
        let mut fresh_bet = bet("active", BetStatus::Building);
        fresh_bet.epic = Some("podras-abc".to_string());
        let warnings = collect(
            &identity_view(Some("2027-01-01")),
            &[fresh_bet],
            &[],
            &[fresh],
            evaluation,
        );
        assert!(!codes(&warnings).contains(&RoadmapWarningKind::BetNoTransition));
    }

    #[test]
    fn horizon_epic_mismatch_cases() {
        let evaluation = today();
        let identity = identity_view(Some("2027-01-01"));

        let mut now_no_epic = bet("now-no-epic", BetStatus::Building);
        now_no_epic.horizon = Horizon::Now;
        now_no_epic.epic = None;
        let warnings = collect(&identity, &[now_no_epic], &[], &[], evaluation);
        assert!(codes(&warnings).contains(&RoadmapWarningKind::HorizonEpicMismatch));

        let mut now_closed = bet("now-closed", BetStatus::Building);
        now_closed.horizon = Horizon::Now;
        now_closed.epic = Some("podras-abc".to_string());
        let closed = record("podras-abc", "podras-abc", false, Some(today_instant()));
        let warnings = collect(&identity, &[now_closed], &[], &[closed], evaluation);
        assert!(codes(&warnings).contains(&RoadmapWarningKind::HorizonEpicMismatch));

        let mut later_open = bet("later-open", BetStatus::Shaping);
        later_open.horizon = Horizon::Later;
        later_open.epic = Some("podras-abc".to_string());
        let open = record("podras-abc", "podras-abc", true, Some(today_instant()));
        let warnings = collect(&identity, &[later_open], &[], &[open], evaluation);
        assert!(!codes(&warnings).contains(&RoadmapWarningKind::HorizonEpicMismatch));

        let mut later_closed = bet("later-closed", BetStatus::Shaping);
        later_closed.horizon = Horizon::Later;
        later_closed.epic = Some("podras-abc".to_string());
        let closed = record("podras-abc", "podras-abc", false, None);
        let warnings = collect(&identity, &[later_closed], &[], &[closed], evaluation);
        assert!(codes(&warnings).contains(&RoadmapWarningKind::HorizonEpicMismatch));
    }

    #[test]
    fn review_or_kill_passed_fires() {
        let evaluation = today();
        let mut active_bet = bet("active", BetStatus::Committed);
        active_bet.epic = Some("podras-abc".to_string());
        active_bet.review_or_kill = Some(today() - chrono::Duration::days(1));
        let warnings = collect(
            &identity_view(Some("2027-01-01")),
            &[active_bet],
            &[],
            &[],
            evaluation,
        );
        assert!(codes(&warnings).contains(&RoadmapWarningKind::RoadmapReviewOverdue));

        let mut on_the_day = bet("on-the-day", BetStatus::Committed);
        on_the_day.epic = Some("podras-abc".to_string());
        on_the_day.review_or_kill = Some(today());
        let warnings = collect(
            &identity_view(Some("2027-01-01")),
            &[on_the_day],
            &[],
            &[],
            evaluation,
        );
        assert!(!codes(&warnings).contains(&RoadmapWarningKind::RoadmapReviewOverdue));
    }

    #[test]
    fn review_due_takes_the_earliest_active_review() {
        let identity = identity_view(Some("2027-05-18"));
        let mut first = bet("first", BetStatus::Building);
        first.review_or_kill = Some(NaiveDate::from_ymd_opt(2026, 12, 30).unwrap());
        let mut second = bet("second", BetStatus::Committed);
        second.review_or_kill = Some(NaiveDate::from_ymd_opt(2026, 12, 1).unwrap());
        let mut deferred = bet("deferred", BetStatus::Deferred);
        deferred.review_or_kill = Some(NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        let due = review_due(&identity, &[first, second, deferred]);
        assert_eq!(due.identity, NaiveDate::from_ymd_opt(2027, 5, 18));
        assert_eq!(due.roadmap, NaiveDate::from_ymd_opt(2026, 12, 1));
    }

    #[test]
    fn token_matching_respects_boundaries() {
        assert!(token_matches("block auto-close, always", "auto-close"));
        assert!(token_matches("auto-close", "auto-close"));
        assert!(token_matches("Auto-Close", "auto-close"));
        assert!(!token_matches("add autoclose support", "auto-close"));
        assert!(!token_matches("x_auto-close", "auto-close"));
        assert!(!token_matches("auto-closex", "auto-close"));
        assert!(
            !token_matches("auto-close_extra", "auto-close"),
            "underscore is a token char"
        );
        assert!(!token_matches("auto-close_extra", "auto-closex"));
        assert!(!token_matches("anything", ""));
        assert!(token_matches("(auto-promote)", "auto-promote"));
        assert!(token_matches("auto-promote.", "auto-promote"));
        assert!(!token_matches("auto-promotion", "auto-promote"));
    }

    #[test]
    fn no_go_matches_open_beads_including_labels() {
        let mut no_go_entry = no_go("no-tracker-writes", vec!["auto-close", "auto-promote"]);
        no_go_entry.enforcement = Enforcement::Automated;
        let open = record(
            "podras-abc",
            "Add auto-close support",
            true,
            Some(today_instant()),
        );
        let closed = record(
            "podras-def",
            "Add auto-promote support",
            false,
            Some(today_instant()),
        );
        let labelled = IssueRecord {
            title: "Harvest proposal".to_string(),
            labels: vec!["auto-promote".to_string()],
            ..record(
                "podras-ghi",
                "Harvest proposal",
                true,
                Some(today_instant()),
            )
        };
        let warnings = collect(
            &identity_view(Some("2027-01-01")),
            &[],
            &[no_go_entry],
            &[open, closed, labelled],
            today(),
        );
        let mismatches: Vec<_> = warnings
            .iter()
            .filter(|warning| warning.code == RoadmapWarningKind::NoGoViolationCandidate)
            .collect();
        assert_eq!(mismatches.len(), 2, "{warnings:?}");
        assert!(
            mismatches
                .iter()
                .any(|warning| warning.detail == "podras-abc matched `auto-close`")
        );
        assert!(
            mismatches
                .iter()
                .any(|warning| warning.detail == "podras-ghi matched `auto-promote`")
        );
        assert!(
            mismatches
                .iter()
                .all(|warning| warning.subject == "no-tracker-writes")
        );
        assert!(
            !mismatches
                .iter()
                .any(|warning| warning.detail.contains("podras-def")),
            "closed beads never fire"
        );
    }

    #[test]
    fn no_go_without_terms_never_fires() {
        let wildcard = no_go("no-writes", Vec::new());
        let open = record(
            "podras-abc",
            "auto-close everything",
            true,
            Some(today_instant()),
        );
        let warnings = collect(
            &identity_view(Some("2027-01-01")),
            &[],
            &[wildcard],
            &[open],
            today(),
        );
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn first_matching_term_wins() {
        let entry = no_go("no-writes", vec!["auto-close", "auto-promote"]);
        let open = record(
            "podras-abc",
            "auto-promote and auto-close",
            true,
            Some(today_instant()),
        );
        let warnings = collect(
            &identity_view(Some("2027-01-01")),
            &[],
            &[entry],
            &[open],
            today(),
        );
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert_eq!(warnings[0].detail, "podras-abc matched `auto-close`");
    }

    #[test]
    fn warning_order_is_code_then_subject_then_detail() {
        let mut warnings = vec![
            RoadmapWarning {
                code: RoadmapWarningKind::BetAgeExceedsAppetite,
                subject: "b".to_string(),
                detail: "x".to_string(),
            },
            RoadmapWarning {
                code: RoadmapWarningKind::NoGoViolationCandidate,
                subject: "a".to_string(),
                detail: "x".to_string(),
            },
            RoadmapWarning {
                code: RoadmapWarningKind::BetAgeExceedsAppetite,
                subject: "a".to_string(),
                detail: "y".to_string(),
            },
            RoadmapWarning {
                code: RoadmapWarningKind::BetAgeExceedsAppetite,
                subject: "a".to_string(),
                detail: "x".to_string(),
            },
        ];
        order(&mut warnings);
        assert_eq!(
            codes(&warnings),
            vec![
                RoadmapWarningKind::BetAgeExceedsAppetite,
                RoadmapWarningKind::BetAgeExceedsAppetite,
                RoadmapWarningKind::BetAgeExceedsAppetite,
                RoadmapWarningKind::NoGoViolationCandidate,
            ]
        );
        assert_eq!(warnings[0].subject, "a");
        assert_eq!(warnings[0].detail, "x");
        assert_eq!(warnings[1].detail, "y");
        assert_eq!(warnings[2].subject, "b");
    }

    #[test]
    fn records_without_update_time_do_not_fire_no_transition() {
        let evaluation = today();
        let mut active_bet = bet("active", BetStatus::Building);
        active_bet.epic = Some("podras-abc".to_string());
        let unknown = record("podras-abc", "podras-abc", true, None);
        let warnings = collect(
            &identity_view(Some("2027-01-01")),
            &[active_bet],
            &[],
            &[unknown],
            evaluation,
        );
        assert!(!codes(&warnings).contains(&RoadmapWarningKind::BetNoTransition));
    }
}
