//! Purpose: assemble the roadmap lens — identity, bets, boundaries — read-only.
//!
//! Responsibilities:
//! - turn a validated `.podras/roadmap.toml` plus tracker records into the
//!   typed, bounded, deterministically ordered `RoadmapView`;
//! - attach that view to a brief and re-stamp the source hash;
//! - expose the file loader and the roadmap state path to the CLI boundary.
//!
//! Rationale: the roadmap is a separate lens (decision 16), never mixed into
//! focus/parked. `assemble` is a pure function of the validated file, the bead
//! records, and the pinned evaluation date — no clock, no filesystem, no
//! environment — because identical inputs under `--as-of` must produce
//! identical output. Bounds apply after sorting; `review_due` and the warnings
//! are computed over the retained lists, so a warning never names a bet the
//! lens does not show. `roadmap.truncation` covers bets and no-gos only: parked
//! omissions stay in the brief's own `truncation`.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use chrono::NaiveDate;

use crate::curation::parse_human_date;
use crate::model::{
    BetStatus, BetView, Brief, NoGoView, ROADMAP_BETS_BOUND, ROADMAP_NO_GOS_BOUND, RoadmapView,
    Truncation,
};
use crate::readers::beads::IssueRecord;

pub mod file;
mod warnings;

use file::{BetEntry, NoGoEntry};

pub use file::{ROADMAP_REL_PATH, RoadmapFile, load};

/// Assemble the roadmap lens for one evaluation date.
///
/// `file` is expected to come from [`load`], which validates slugs, enums,
/// dates, and lifecycle rules. When called with an unvalidated file, an
/// unparseable required date falls back to `evaluation_date` so the lens stays
/// total and deterministic rather than panicking or dropping a row.
///
/// Identity staleness is decided by the file alone (decision 21), so no brief
/// input participates: the lens is a pure function of the file, the tracker
/// records, and the date. Sorting happens before the bounds, and `review_due`
/// plus the warnings are computed after them, over the retained lists only.
pub fn assemble(
    file: &RoadmapFile,
    records: &[IssueRecord],
    evaluation_date: NaiveDate,
) -> RoadmapView {
    let identity = warnings::identity(file, evaluation_date);
    let mut bets: Vec<BetView> = file
        .bets
        .iter()
        .map(|entry| bet_view(entry, evaluation_date))
        .collect();
    let mut no_gos: Vec<NoGoView> = file.no_gos.iter().map(no_go_view).collect();

    bets.sort_by(compare_bets);
    no_gos.sort_by(|left, right| left.id.cmp(&right.id));

    let truncation = truncate(&mut bets, &mut no_gos);
    let review_due = warnings::review_due(&identity, &bets);
    let warnings = warnings::collect(&identity, &bets, &no_gos, records, evaluation_date);

    RoadmapView {
        identity,
        bets,
        no_gos,
        review_due,
        warnings,
        truncation,
    }
}

/// Attach the roadmap lens to a brief and re-stamp its source hash.
///
/// The hash covers the semantic payload, so adding the lens changes it; the
/// caller gets a brief whose hash still describes its own content.
pub fn attach(mut brief: Brief, view: Option<RoadmapView>) -> Brief {
    brief.roadmap = view;
    brief.source_hash = Some(crate::model::source_hash(&brief, brief.head_sha.as_deref()));
    brief
}

/// Build the rendered view of one validated bet entry.
fn bet_view(entry: &BetEntry, evaluation_date: NaiveDate) -> BetView {
    BetView {
        id: entry.id.clone(),
        title: entry.title.clone(),
        outcome: entry.outcome.clone(),
        horizon: entry.horizon,
        appetite_weeks: entry.appetite_weeks,
        owner: entry.owner.clone(),
        epic: entry.epic.clone(),
        status: entry.status,
        admitted_at: parse_human_date(&entry.admitted_at).unwrap_or(evaluation_date),
        last_reviewed: parse_human_date(&entry.last_reviewed).unwrap_or(evaluation_date),
        review_or_kill: entry.review_or_kill.as_deref().and_then(parse_human_date),
        revive_when: entry.revive_when.clone(),
        killed_reason: entry.killed_reason.clone(),
    }
}

/// Build the rendered view of one validated no-go entry.
fn no_go_view(entry: &NoGoEntry) -> NoGoView {
    NoGoView {
        id: entry.id.clone(),
        reason: entry.reason.clone(),
        scope: entry.scope.clone(),
        enforcement: entry.enforcement,
        revoke_by: entry.revoke_by.clone(),
        match_terms: entry.match_terms.clone(),
    }
}

/// Render order: status rank, then admission date, then id.
fn compare_bets(left: &BetView, right: &BetView) -> Ordering {
    status_rank(left.status)
        .cmp(&status_rank(right.status))
        .then_with(|| left.admitted_at.cmp(&right.admitted_at))
        .then_with(|| left.id.cmp(&right.id))
}

/// Status rank used by the lens: active work first, graveyard last.
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

/// Apply both render bounds after sorting and record what was dropped.
fn truncate(bets: &mut Vec<BetView>, no_gos: &mut Vec<NoGoView>) -> Truncation {
    let mut omitted_counts = BTreeMap::new();
    let omitted_bets = bound(bets, ROADMAP_BETS_BOUND);
    if omitted_bets > 0 {
        omitted_counts.insert("bets".to_string(), omitted_bets);
    }
    let omitted_no_gos = bound(no_gos, ROADMAP_NO_GOS_BOUND);
    if omitted_no_gos > 0 {
        omitted_counts.insert("no_gos".to_string(), omitted_no_gos);
    }
    Truncation {
        applied: !omitted_counts.is_empty(),
        omitted_counts,
    }
}

fn bound<T>(items: &mut Vec<T>, limit: usize) -> u32 {
    if items.len() <= limit {
        return 0;
    }
    let omitted = items.len() - limit;
    items.truncate(limit);
    omitted as u32
}

#[cfg(test)]
pub(crate) mod test_support {
    use chrono::{DateTime, NaiveDate, Utc};

    use crate::model::{
        BetStatus, BetView, Enforcement, Horizon, IdentityView, NoGoView, Staleness,
    };
    use crate::readers::beads::IssueRecord;
    use crate::roadmap::file::{BetEntry, IdentityEntry, NoGoEntry, RoadmapFile};

    /// Pinned evaluation date used by every roadmap unit test.
    pub(crate) fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 18).expect("valid date")
    }

    /// Midnight UTC on the pinned evaluation date, for tracker update times.
    pub(crate) fn today_instant() -> DateTime<Utc> {
        today()
            .and_hms_opt(0, 0, 0)
            .expect("midnight is representable")
            .and_utc()
    }

    /// An identity row with a given `next_review`; staleness is irrelevant to
    /// warning predicates and is set to `Unknown` unless recomputed.
    pub(crate) fn identity_view(next_review: Option<&str>) -> IdentityView {
        IdentityView {
            path: "README.md#why".to_string(),
            last_reviewed: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
            next_review: next_review.and_then(crate::curation::parse_human_date),
            staleness: Staleness::Unknown,
        }
    }

    /// A bet shaped to fire no warning by default: horizon `next`, no epic.
    pub(crate) fn bet(id: &str, status: BetStatus) -> BetView {
        BetView {
            id: id.to_string(),
            title: "Do the thing".to_string(),
            outcome: "The thing is done".to_string(),
            horizon: Horizon::Next,
            appetite_weeks: 6,
            owner: "charly".to_string(),
            epic: None,
            status,
            admitted_at: today(),
            last_reviewed: today(),
            review_or_kill: None,
            revive_when: None,
            killed_reason: None,
        }
    }

    /// One tracker record.
    pub(crate) fn record(
        id: &str,
        title: &str,
        open: bool,
        updated_at: Option<DateTime<Utc>>,
    ) -> IssueRecord {
        IssueRecord {
            id: id.to_string(),
            title: title.to_string(),
            status: if open { "open" } else { "closed" }.to_string(),
            labels: Vec::new(),
            updated_at,
            open,
        }
    }

    /// One no-go with the given match terms.
    pub(crate) fn no_go(id: &str, match_terms: Vec<&str>) -> NoGoView {
        NoGoView {
            id: id.to_string(),
            reason: "never mutate the tracker".to_string(),
            scope: "all verbs".to_string(),
            enforcement: Enforcement::HumanReview,
            revoke_by: "a new decision record".to_string(),
            match_terms: match_terms.into_iter().map(str::to_string).collect(),
        }
    }

    /// A file with the given tables; used where only the shape matters.
    pub(crate) fn roadmap_file(
        identity: IdentityEntry,
        bets: Vec<BetEntry>,
        no_gos: Vec<NoGoEntry>,
    ) -> RoadmapFile {
        RoadmapFile {
            identity,
            bets,
            no_gos,
        }
    }

    /// The file entry equivalent of a rendered bet.
    pub(crate) fn entry(bet: &BetView) -> BetEntry {
        BetEntry {
            id: bet.id.clone(),
            title: bet.title.clone(),
            outcome: bet.outcome.clone(),
            horizon: bet.horizon,
            appetite_weeks: bet.appetite_weeks,
            owner: bet.owner.clone(),
            epic: bet.epic.clone(),
            status: bet.status,
            admitted_at: bet.admitted_at.to_string(),
            last_reviewed: bet.last_reviewed.to_string(),
            review_or_kill: bet.review_or_kill.map(|date| date.to_string()),
            revive_when: bet.revive_when.clone(),
            killed_reason: bet.killed_reason.clone(),
        }
    }

    /// A file entry equivalent of a rendered no-go.
    pub(crate) fn no_go_entry(no_go: &NoGoView) -> NoGoEntry {
        NoGoEntry {
            id: no_go.id.clone(),
            reason: no_go.reason.clone(),
            scope: no_go.scope.clone(),
            enforcement: no_go.enforcement,
            revoke_by: no_go.revoke_by.clone(),
            match_terms: no_go.match_terms.clone(),
        }
    }

    /// Identity entry shared by assembly tests.
    pub(crate) fn identity_entry() -> IdentityEntry {
        IdentityEntry {
            path: "README.md#why".to_string(),
            last_reviewed: "2026-11-18".to_string(),
            next_review: Some("2027-05-18".to_string()),
        }
    }

    /// A generation instant pinned by every test.
    pub(crate) fn generation_instant() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(0, 0).expect("epoch is representable")
    }

    /// Minimal inputs for a brief, with every source absent.
    pub(crate) fn brief_inputs() -> crate::brief::BriefInputs {
        crate::brief::BriefInputs {
            beads: None,
            git: None,
            specs: None,
            whisper: None,
            curation: None,
            sources: Vec::new(),
            warnings: Vec::new(),
            as_of: Some(today()),
            now: today(),
            generated_at: generation_instant(),
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::Duration;

    use super::test_support::{
        bet, brief_inputs, entry, identity_entry, no_go, no_go_entry, roadmap_file, today,
    };
    use super::*;
    use crate::model::Horizon;

    #[test]
    fn bets_sort_by_status_rank() {
        let mut bets = vec![
            bet("killed", BetStatus::Killed),
            bet("shipped", BetStatus::Shipped),
            bet("deferred", BetStatus::Deferred),
            bet("shaping", BetStatus::Shaping),
            bet("committed", BetStatus::Committed),
            bet("building", BetStatus::Building),
        ];
        // The comparator is the full six-rank order; the lens then bounds it.
        bets.sort_by(compare_bets);
        let statuses: Vec<BetStatus> = bets.iter().map(|bet| bet.status).collect();
        assert_eq!(
            statuses,
            vec![
                BetStatus::Building,
                BetStatus::Committed,
                BetStatus::Deferred,
                BetStatus::Shaping,
                BetStatus::Shipped,
                BetStatus::Killed,
            ]
        );

        let mut file = roadmap_file(identity_entry(), Vec::new(), Vec::new());
        for bet in &bets {
            file.bets.push(entry(bet));
        }
        let view = assemble(&file, &[], today());
        assert_eq!(view.bets.len(), 5);
        assert!(
            view.bets.iter().all(|bet| bet.status != BetStatus::Killed),
            "the bound drops the lowest-ranked bet"
        );
        assert_eq!(view.truncation.omitted_counts.get("bets"), Some(&1_u32));
    }

    #[test]
    fn ties_sort_by_admission_then_id() {
        let mut file = roadmap_file(identity_entry(), Vec::new(), Vec::new());
        let mut oldest = bet("z-oldest", BetStatus::Building);
        oldest.admitted_at = today() - Duration::days(10);
        let mut newest = bet("a-newest", BetStatus::Building);
        newest.admitted_at = today();
        let mut middle_b = bet("b-middle", BetStatus::Building);
        middle_b.admitted_at = today() - Duration::days(2);
        let mut middle_a = bet("a-middle", BetStatus::Building);
        middle_a.admitted_at = today() - Duration::days(2);
        for bet in [newest, middle_b, middle_a, oldest] {
            file.bets.push(entry(&bet));
        }

        let view = assemble(&file, &[], today());
        let ids: Vec<&str> = view.bets.iter().map(|bet| bet.id.as_str()).collect();
        assert_eq!(ids, vec!["z-oldest", "a-middle", "b-middle", "a-newest"]);
    }

    #[test]
    fn bounds_apply_after_sort_and_record_omissions() {
        let mut file = roadmap_file(identity_entry(), Vec::new(), Vec::new());
        for index in 0..7 {
            let mut building = bet(&format!("bet-{index}"), BetStatus::Building);
            building.admitted_at = today() - Duration::days(i64::from(6 - index));
            let mut killed = bet(&format!("dead-{index}"), BetStatus::Killed);
            killed.admitted_at = today() - Duration::days(i64::from(index));
            file.bets.push(entry(&building));
            file.bets.push(entry(&killed));
        }
        for index in 0..6 {
            file.no_gos.push(no_go_entry(&no_go(
                &format!("no-go-{index}"),
                vec!["auto-close"],
            )));
        }

        let view = assemble(&file, &[], today());
        assert_eq!(view.bets.len(), 5);
        assert_eq!(view.no_gos.len(), 5);
        assert!(
            view.bets
                .iter()
                .all(|bet| bet.status == BetStatus::Building)
        );
        assert_eq!(
            view.bets
                .iter()
                .map(|bet| bet.id.as_str())
                .collect::<Vec<_>>(),
            vec!["bet-0", "bet-1", "bet-2", "bet-3", "bet-4"]
        );
        assert_eq!(
            view.no_gos
                .iter()
                .map(|no_go| no_go.id.as_str())
                .collect::<Vec<_>>(),
            vec!["no-go-0", "no-go-1", "no-go-2", "no-go-3", "no-go-4"]
        );
        assert!(view.truncation.applied);
        assert_eq!(view.truncation.omitted_counts.get("bets"), Some(&9_u32));
        assert_eq!(view.truncation.omitted_counts.get("no_gos"), Some(&1_u32));
    }

    #[test]
    fn bounds_leave_short_lists_untouched() {
        let mut file = roadmap_file(identity_entry(), Vec::new(), Vec::new());
        file.bets.push(entry(&bet("only", BetStatus::Shaping)));
        let view = assemble(&file, &[], today());
        assert_eq!(view.bets.len(), 1);
        assert!(!view.truncation.applied);
        assert!(view.truncation.omitted_counts.is_empty());
    }

    #[test]
    fn empty_file_is_a_present_lens() {
        let file = roadmap_file(identity_entry(), Vec::new(), Vec::new());
        let view = assemble(&file, &[], today());
        assert!(view.bets.is_empty());
        assert!(view.no_gos.is_empty());
        assert!(view.warnings.is_empty());
        assert_eq!(view.identity.path, "README.md#why");
        assert_eq!(
            view.review_due.identity,
            NaiveDate::from_ymd_opt(2027, 5, 18)
        );
        assert_eq!(view.review_due.roadmap, None);
    }

    #[test]
    fn entries_render_typed_enums_and_dates() {
        let mut file = roadmap_file(identity_entry(), Vec::new(), Vec::new());
        let mut building = bet("brief-v2", BetStatus::Building);
        building.outcome = "A cold reader predicts the next bet".to_string();
        building.epic = Some("podras-abc".to_string());
        building.review_or_kill = Some(NaiveDate::from_ymd_opt(2026, 12, 30).unwrap());
        file.bets.push(entry(&building));
        file.no_gos
            .push(no_go_entry(&no_go("no-tracker-writes", vec!["auto-close"])));

        let view = assemble(&file, &[], today());
        let rendered = &view.bets[0];
        assert_eq!(rendered.id, "brief-v2");
        assert_eq!(rendered.horizon, Horizon::Next);
        assert_eq!(rendered.status, BetStatus::Building);
        assert_eq!(
            rendered.review_or_kill,
            NaiveDate::from_ymd_opt(2026, 12, 30)
        );
        assert_eq!(view.no_gos[0].match_terms, vec!["auto-close".to_string()]);
        assert_eq!(
            view.review_due.roadmap,
            NaiveDate::from_ymd_opt(2026, 12, 30)
        );
    }

    #[test]
    fn warnings_cover_retained_bets_only() {
        let mut file = roadmap_file(identity_entry(), Vec::new(), Vec::new());
        for index in 0..7 {
            let mut committed = bet(&format!("bet-{index}"), BetStatus::Committed);
            committed.admitted_at = today() - Duration::days(i64::from(index));
            // Committed without an epic: every retained one warns.
            file.bets.push(entry(&committed));
        }
        let view = assemble(&file, &[], today());
        let mut retained: Vec<&str> = view.bets.iter().map(|bet| bet.id.as_str()).collect();
        assert_eq!(retained.len(), 5, "the bound keeps five bets");
        retained.sort_unstable();

        // The retained set is derived from the sorted, bounded `bets`, so the
        // assertion cannot lock in a stale slice of ids.
        let warned: Vec<&str> = view
            .warnings
            .iter()
            .filter(|warning| warning.code == crate::model::RoadmapWarningKind::BetEpicMissing)
            .map(|warning| warning.subject.as_str())
            .collect();
        assert_eq!(warned, retained, "a truncated bet emits no warning");
        assert_eq!(
            view.warnings.len(),
            retained.len(),
            "the warning list names retained entries only: {:?}",
            view.warnings
        );
    }

    #[test]
    fn review_due_uses_retained_bets_only() {
        let mut file = roadmap_file(identity_entry(), Vec::new(), Vec::new());
        for index in 0..7 {
            let mut building = bet(&format!("bet-{index}"), BetStatus::Building);
            building.admitted_at = today() - Duration::days(i64::from(6 - index));
            // The truncated tail declares the earliest reviews.
            building.review_or_kill = Some(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap());
            if index < 5 {
                building.review_or_kill = Some(
                    NaiveDate::from_ymd_opt(2027, 3, 1 + u32::try_from(index).unwrap()).unwrap(),
                );
            }
            file.bets.push(entry(&building));
        }
        let view = assemble(&file, &[], today());
        assert_eq!(view.bets.len(), 5);
        assert_eq!(
            view.review_due.roadmap,
            NaiveDate::from_ymd_opt(2027, 3, 1),
            "the earliest review among retained bets, not among all bets"
        );
    }

    #[test]
    fn assemble_is_pure_over_its_arguments() {
        let mut file = roadmap_file(identity_entry(), Vec::new(), Vec::new());
        let mut building = bet("brief-v2", BetStatus::Building);
        building.admitted_at = today() - Duration::days(50);
        file.bets.push(entry(&building));

        let first = assemble(&file, &[], today());
        let second = assemble(&file, &[], today());
        assert_eq!(first, second);

        // The evaluation date is the only clock: a later date changes the
        // advisories without touching the file.
        let later = assemble(&file, &[], today() + Duration::days(10));
        assert_ne!(first.warnings, later.warnings);
        assert_eq!(first.bets, later.bets);
    }

    #[test]
    fn attach_sets_the_lens_and_restamps_the_hash() {
        let inputs = brief_inputs();
        let before = crate::brief::assemble(&inputs);
        let previous_hash = before.source_hash.clone();

        let file = roadmap_file(identity_entry(), Vec::new(), Vec::new());
        let view = assemble(&file, &[], today());
        let attached = attach(before, Some(view.clone()));

        assert_eq!(attached.schema_version, crate::model::SCHEMA_VERSION);
        assert_eq!(attached.roadmap.as_ref(), Some(&view));
        assert_ne!(attached.source_hash, previous_hash);
        assert_eq!(
            attached.source_hash,
            Some(crate::model::source_hash(
                &attached,
                attached.head_sha.as_deref()
            ))
        );

        // Attaching no lens leaves the semantic payload unchanged.
        let plain = crate::brief::assemble(&inputs);
        let untouched = attach(plain, None);
        assert!(untouched.roadmap.is_none());
        assert_eq!(untouched.source_hash, previous_hash);
    }

    #[test]
    fn factor_constant_is_documented_default() {
        assert_eq!(crate::roadmap::warnings::CIRCUIT_BREAKER_FACTOR, 1.25);
    }
}
