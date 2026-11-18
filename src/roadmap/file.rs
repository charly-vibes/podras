//! Purpose: model `.podras/roadmap.toml` and validate its hand-edited entries.
//!
//! Responsibilities:
//! - declare the three tables (`[identity]`, `[[bets]]`, `[[no_gos]]`) as
//!   closed serde structs;
//! - load the file, treating absence as legal and malformed content as a
//!   domain error;
//! - aggregate every semantic violation into one message instead of failing on
//!   the first, so a human fixes the file in one pass.
//!
//! Rationale: the roadmap is a human decision record, so its shape belongs in
//! one place, exactly like curation. Dates stay strings because humans
//! hand-edit the file and `crate::curation::parse_human_date` is the single
//! parser; ids, enums, and counts are validated here because every later stage
//! (lens, verbs, harvest) assumes a validated file.

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::curation::parse_human_date;
use crate::error::{Error, Result};
use crate::model::{BetStatus, Enforcement, Horizon};

/// Repository-relative roadmap state path.
pub const ROADMAP_REL_PATH: &str = ".podras/roadmap.toml";

/// Maximum length, in characters, of a bet title.
const TITLE_MAX_CHARS: usize = 60;

/// Parsed `.podras/roadmap.toml`.
///
/// `[identity]` is required: whenever the lens is present it renders an
/// identity row (decision 24), so a file without it is not a roadmap. Bets and
/// no-gos default to empty, which is what makes a present file with zero bets a
/// present — not absent — lens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoadmapFile {
    /// What the project ought to be, and when it was last reviewed.
    pub identity: IdentityEntry,
    /// Committed bets.
    #[serde(default)]
    pub bets: Vec<BetEntry>,
    /// Binding boundaries.
    #[serde(default)]
    pub no_gos: Vec<NoGoEntry>,
}

/// The `[identity]` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityEntry {
    /// Link to the prose stating the project's purpose.
    pub path: String,
    /// `YYYY-MM-DD` or RFC 3339 review date.
    pub last_reviewed: String,
    /// Date the next review is due; absent means unscheduled.
    #[serde(default)]
    pub next_review: Option<String>,
}

/// One `[[bets]]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BetEntry {
    /// Slug identifier, unique among bets.
    pub id: String,
    /// Imperative title, at most 60 characters.
    pub title: String,
    /// What changes if the bet lands.
    pub outcome: String,
    /// Declared horizon.
    pub horizon: Horizon,
    /// Appetite in weeks, at least 1.
    pub appetite_weeks: u32,
    /// Human owner.
    pub owner: String,
    /// The single tracker epic backing the bet.
    #[serde(default)]
    pub epic: Option<String>,
    /// Lifecycle state.
    pub status: BetStatus,
    /// `YYYY-MM-DD` or RFC 3339 admission date.
    pub admitted_at: String,
    /// `YYYY-MM-DD` or RFC 3339 review date.
    pub last_reviewed: String,
    /// Dated review-or-kill decision; required for `committed`/`building`.
    #[serde(default)]
    pub review_or_kill: Option<String>,
    /// Trigger that revives a deferred bet; required for `deferred`, absent
    /// otherwise.
    #[serde(default)]
    pub revive_when: Option<String>,
    /// Why the bet was killed, when it was.
    #[serde(default)]
    pub killed_reason: Option<String>,
}

/// One `[[no_gos]]` entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoGoEntry {
    /// Slug identifier, unique among no-gos.
    pub id: String,
    /// Why the boundary exists.
    pub reason: String,
    /// What the boundary covers.
    pub scope: String,
    /// Declared enforcement tier.
    pub enforcement: Enforcement,
    /// What reversal requires.
    pub revoke_by: String,
    /// Whole-token terms matched against open bead titles and labels.
    #[serde(default)]
    pub match_terms: Vec<String>,
}

/// Load the roadmap file, returning `None` when the project has none.
///
/// Absence is legal and is not this layer's error. A malformed file is
/// `roadmap_parse_error`; a well-formed file that violates a semantic rule is
/// `roadmap_invalid` with every violation joined by `"; "`. A file that exists
/// but cannot be read (permissions, a directory in its place, invalid UTF-8)
/// is `roadmap_unreadable`, a domain error like the other two: the verb that
/// needs the lens fails hard, while the brief degrades it to a warning.
pub fn load(repo: &Path) -> Result<Option<RoadmapFile>> {
    let path = repo.join(ROADMAP_REL_PATH);
    let raw = match std::fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(Error::Domain {
                code: "roadmap_unreadable",
                message: format!("{ROADMAP_REL_PATH}: {error}"),
                suggestion: Some(
                    "make .podras/roadmap.toml a readable file, or delete it when the project has \
                     no roadmap"
                        .to_string(),
                ),
            });
        }
    };
    let file: RoadmapFile = toml::from_str(&raw).map_err(|error| Error::Domain {
        code: "roadmap_parse_error",
        message: format!("{ROADMAP_REL_PATH}: {error}"),
        suggestion: Some(
            "fix the TOML, or delete .podras/roadmap.toml when the project has no roadmap; the \
             schema is docs/design.md decision 17"
                .to_string(),
        ),
    })?;
    validate(&file).map_err(|message| Error::Domain {
        code: "roadmap_invalid",
        message: format!("{ROADMAP_REL_PATH}: {message}"),
        suggestion: Some(
            "fix the listed entries; the roadmap schema is docs/design.md decision 17".to_string(),
        ),
    })?;
    Ok(Some(file))
}

/// Check every semantic rule, returning one message per violation.
///
/// The messages are aggregated by [`validate`] so a human sees every problem at
/// once; the order is identity, bets in file order, then no-gos in file order.
pub fn violations(file: &RoadmapFile) -> Vec<String> {
    let mut violations = Vec::new();
    identity_violations(&file.identity, &mut violations);
    let mut bet_ids = BTreeSet::new();
    for (index, bet) in file.bets.iter().enumerate() {
        bet_violations(bet, index, &mut bet_ids, &mut violations);
    }
    let mut no_go_ids = BTreeSet::new();
    for (index, no_go) in file.no_gos.iter().enumerate() {
        no_go_violations(no_go, index, &mut no_go_ids, &mut violations);
    }
    violations
}

/// Validate a roadmap file, joining every violation into one message.
pub fn validate(file: &RoadmapFile) -> std::result::Result<(), String> {
    let violations = violations(file);
    if violations.is_empty() {
        return Ok(());
    }
    Err(violations.join("; "))
}

/// True when `id` matches `^[a-z0-9][a-z0-9-]*$`.
fn is_slug(id: &str) -> bool {
    let mut characters = id.chars();
    let starts_well = characters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit());
    starts_well
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

/// Serde name of a status, used verbatim in validation messages.
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

fn identity_violations(identity: &IdentityEntry, violations: &mut Vec<String>) {
    if identity.path.trim().is_empty() {
        violations.push("identity: path is empty".to_string());
    }
    if parse_human_date(&identity.last_reviewed).is_none() {
        violations.push(format!(
            "identity: last_reviewed `{}` is not a date (YYYY-MM-DD or RFC 3339)",
            identity.last_reviewed
        ));
    }
    if let Some(next_review) = identity.next_review.as_deref()
        && parse_human_date(next_review).is_none()
    {
        violations.push(format!(
            "identity: next_review `{next_review}` is not a date (YYYY-MM-DD or RFC 3339)"
        ));
    }
}

fn bet_violations<'a>(
    bet: &'a BetEntry,
    index: usize,
    seen: &mut BTreeSet<&'a str>,
    violations: &mut Vec<String>,
) {
    let at = |what: String| format!("bets[{index}] (`{}`): {what}", bet.id);

    if !is_slug(&bet.id) {
        violations.push(format!(
            "bets[{index}]: id `{}` is not a slug matching ^[a-z0-9][a-z0-9-]*$",
            bet.id
        ));
    }
    if !seen.insert(bet.id.as_str()) {
        violations.push(format!("bets[{index}]: duplicate id `{}`", bet.id));
    }

    let title_chars = bet.title.chars().count();
    if !(1..=TITLE_MAX_CHARS).contains(&title_chars) {
        violations.push(at(format!(
            "title is {title_chars} characters, expected 1..={TITLE_MAX_CHARS}"
        )));
    }
    if bet.outcome.trim().is_empty() {
        violations.push(at("outcome is empty".to_string()));
    }
    if bet.owner.trim().is_empty() {
        violations.push(at("owner is empty".to_string()));
    }
    if bet.appetite_weeks < 1 {
        violations.push(at("appetite_weeks must be at least 1".to_string()));
    }
    if let Some(epic) = bet.epic.as_deref()
        && epic.trim().is_empty()
    {
        violations.push(at("epic is empty".to_string()));
    }

    let active = matches!(bet.status, BetStatus::Committed | BetStatus::Building);
    let has_review = bet
        .review_or_kill
        .as_deref()
        .is_some_and(|raw| !raw.trim().is_empty());
    if active && !has_review {
        violations.push(at(format!(
            "review_or_kill is required for status `{}`",
            status_name(bet.status)
        )));
    }
    if let Some(raw) = bet
        .review_or_kill
        .as_deref()
        .filter(|raw| !raw.trim().is_empty())
        && parse_human_date(raw).is_none()
    {
        violations.push(at(format!(
            "review_or_kill `{raw}` is not a date (YYYY-MM-DD or RFC 3339)"
        )));
    }

    let deferred = bet.status == BetStatus::Deferred;
    let has_revive = bet
        .revive_when
        .as_deref()
        .is_some_and(|raw| !raw.trim().is_empty());
    if deferred && !has_revive {
        violations.push(at(
            "revive_when is required for status `deferred`".to_string()
        ));
    }
    if !deferred && has_revive {
        violations.push(at(format!(
            "revive_when is only allowed for status `deferred`, not `{}`",
            status_name(bet.status)
        )));
    }

    if parse_human_date(&bet.admitted_at).is_none() {
        violations.push(at(format!(
            "admitted_at `{}` is not a date (YYYY-MM-DD or RFC 3339)",
            bet.admitted_at
        )));
    }
    if parse_human_date(&bet.last_reviewed).is_none() {
        violations.push(at(format!(
            "last_reviewed `{}` is not a date (YYYY-MM-DD or RFC 3339)",
            bet.last_reviewed
        )));
    }
}

fn no_go_violations<'a>(
    no_go: &'a NoGoEntry,
    index: usize,
    seen: &mut BTreeSet<&'a str>,
    violations: &mut Vec<String>,
) {
    let at = |what: String| format!("no_gos[{index}] (`{}`): {what}", no_go.id);

    if !is_slug(&no_go.id) {
        violations.push(format!(
            "no_gos[{index}]: id `{}` is not a slug matching ^[a-z0-9][a-z0-9-]*$",
            no_go.id
        ));
    }
    if !seen.insert(no_go.id.as_str()) {
        violations.push(format!("no_gos[{index}]: duplicate id `{}`", no_go.id));
    }
    if no_go.reason.trim().is_empty() {
        violations.push(at("reason is empty".to_string()));
    }
    if no_go.scope.trim().is_empty() {
        violations.push(at("scope is empty".to_string()));
    }
    if no_go.revoke_by.trim().is_empty() {
        violations.push(at("revoke_by is empty".to_string()));
    }
    for (term_index, term) in no_go.match_terms.iter().enumerate() {
        if term.trim().is_empty() {
            violations.push(at(format!("match_terms[{term_index}] is empty")));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The example from docs/design.md decision 17, verbatim.
    const DOCUMENTED: &str = r#"
[identity]
path = "README.md#why"
last_reviewed = "2026-11-18"
next_review = "2027-05-18"

[[bets]]
id = "brief-v2"
title = "Ship the roadmap lens"
outcome = "A cold reader predicts the next bet from the lens alone"
horizon = "now"
appetite_weeks = 6
owner = "charly"
epic = "podras-abc"
status = "building"
admitted_at = "2026-11-18"
last_reviewed = "2026-11-18"
review_or_kill = "2026-12-30"

[[no_gos]]
id = "no-tracker-writes"
reason = "podras never mutates the tracker; harvest proposes only"
scope = "all verbs"
enforcement = "automated"
revoke_by = "never — requires a new design decision"
match_terms = ["auto-close", "auto-promote"]
"#;

    fn documented() -> RoadmapFile {
        toml::from_str(DOCUMENTED).expect("documented example parses")
    }

    fn single_bet(extra: &str) -> String {
        format!(
            r#"
[identity]
path = "README.md#why"
last_reviewed = "2026-11-18"

[[bets]]
id = "bet-one"
title = "Do the thing"
outcome = "The thing is done"
horizon = "now"
appetite_weeks = 6
owner = "charly"
status = "shaping"
admitted_at = "2026-11-18"
last_reviewed = "2026-11-18"
{extra}
"#
        )
    }

    #[test]
    fn documented_example_round_trips() {
        let file = documented();
        assert_eq!(file.bets.len(), 1);
        assert_eq!(file.no_gos.len(), 1);
        assert_eq!(file.identity.path, "README.md#why");
        assert_eq!(file.bets[0].horizon, Horizon::Now);
        assert_eq!(file.bets[0].status, BetStatus::Building);
        assert_eq!(file.bets[0].epic.as_deref(), Some("podras-abc"));
        assert_eq!(file.no_gos[0].enforcement, Enforcement::Automated);
        assert_eq!(
            file.no_gos[0].match_terms,
            vec!["auto-close".to_string(), "auto-promote".to_string()]
        );
        assert!(validate(&file).is_ok(), "{:?}", validate(&file));

        let serialized = toml::to_string_pretty(&file).expect("roadmap file serializes");
        let reparsed: RoadmapFile = toml::from_str(&serialized).expect("serialized file re-parses");
        assert_eq!(file, reparsed);
    }

    #[test]
    fn present_file_with_zero_bets_is_legal() {
        let file: RoadmapFile = toml::from_str(
            r#"
[identity]
path = "README.md#why"
last_reviewed = "2026-11-18"
"#,
        )
        .expect("identity-only file parses");
        assert!(file.bets.is_empty());
        assert!(file.no_gos.is_empty());
        assert!(validate(&file).is_ok());
    }

    #[test]
    fn identity_table_is_required() {
        let parsed = toml::from_str::<RoadmapFile>(
            r#"
[[bets]]
id = "bet-one"
title = "Do the thing"
outcome = "Done"
horizon = "now"
appetite_weeks = 1
owner = "charly"
status = "shaping"
admitted_at = "2026-11-18"
last_reviewed = "2026-11-18"
"#,
        );
        assert!(
            parsed.is_err(),
            "a roadmap without [identity] is not a roadmap"
        );
    }

    #[test]
    fn duplicate_bet_id_is_rejected() {
        let raw = r#"
[identity]
path = "README.md#why"
last_reviewed = "2026-11-18"

[[bets]]
id = "bet-one"
title = "First landing"
outcome = "First outcome"
horizon = "now"
appetite_weeks = 6
owner = "charly"
status = "shaping"
admitted_at = "2026-11-18"
last_reviewed = "2026-11-18"

[[bets]]
id = "bet-one"
title = "Second landing"
outcome = "Second outcome"
horizon = "next"
appetite_weeks = 4
owner = "charly"
status = "committed"
admitted_at = "2026-11-18"
last_reviewed = "2026-11-18"
review_or_kill = "2026-12-30"
"#;
        let file: RoadmapFile = toml::from_str(raw).expect("duplicate ids still parse as TOML");
        let message = validate(&file).expect_err("duplicate id is invalid");
        assert!(message.contains("duplicate id `bet-one`"), "{message}");

        let repo = tempfile::tempdir().expect("temp repo");
        std::fs::create_dir_all(repo.path().join(".podras")).expect("create .podras");
        std::fs::write(repo.path().join(ROADMAP_REL_PATH), raw).expect("write roadmap");
        let error = load(repo.path()).expect_err("duplicate id is a domain error");
        assert!(
            matches!(
                error,
                Error::Domain {
                    code: "roadmap_invalid",
                    ..
                }
            ),
            "{error:?}"
        );
    }

    #[test]
    fn bad_slug_is_rejected() {
        for id in ["Bet-One", "1_bet", "-bet", "bet one", ""] {
            let file: RoadmapFile =
                toml::from_str(&single_bet("").replace("bet-one", id)).expect("TOML parses");
            let message = validate(&file).expect_err("non-slug id is invalid");
            assert!(message.contains("is not a slug"), "{id}: {message}");
        }
        for id in ["bet-one", "b", "1", "9-lives", "a--b"] {
            let file: RoadmapFile =
                toml::from_str(&single_bet("").replace("bet-one", id)).expect("TOML parses");
            assert!(validate(&file).is_ok(), "{id} should be a slug");
        }
    }

    #[test]
    fn title_of_sixty_one_characters_is_rejected() {
        let long = "x".repeat(61);
        let file: RoadmapFile =
            toml::from_str(&single_bet("").replace("Do the thing", &long)).expect("TOML parses");
        let message = validate(&file).expect_err("long title is invalid");
        assert!(message.contains("expected 1..=60"), "{message}");

        let edge = "y".repeat(60);
        let file: RoadmapFile =
            toml::from_str(&single_bet("").replace("Do the thing", &edge)).expect("TOML parses");
        assert!(validate(&file).is_ok());
    }

    #[test]
    fn zero_appetite_is_rejected() {
        let file: RoadmapFile =
            toml::from_str(&single_bet("").replace("appetite_weeks = 6", "appetite_weeks = 0"))
                .expect("TOML parses");
        let message = validate(&file).expect_err("zero appetite is invalid");
        assert!(
            message.contains("appetite_weeks must be at least 1"),
            "{message}"
        );
    }

    #[test]
    fn committed_without_review_or_kill_is_rejected() {
        let file: RoadmapFile = toml::from_str(
            &single_bet("").replace(r#"status = "shaping""#, r#"status = "committed""#),
        )
        .expect("TOML parses");
        let message = validate(&file).expect_err("committed needs review_or_kill");
        assert!(message.contains("review_or_kill is required"), "{message}");

        let file: RoadmapFile = toml::from_str(&single_bet("").replace(
            r#"status = "shaping""#,
            "status = \"committed\"\nreview_or_kill = \"2026-12-30\"",
        ))
        .expect("TOML parses");
        assert!(validate(&file).is_ok());
    }

    #[test]
    fn deferred_requires_revive_when() {
        let file: RoadmapFile = toml::from_str(
            &single_bet("").replace(r#"status = "shaping""#, r#"status = "deferred""#),
        )
        .expect("TOML parses");
        let message = validate(&file).expect_err("deferred needs revive_when");
        assert!(message.contains("revive_when is required"), "{message}");
    }

    #[test]
    fn non_deferred_rejects_revive_when() {
        let file: RoadmapFile = toml::from_str(&single_bet("revive_when = \"when the API lands\""))
            .expect("TOML parses");
        let message = validate(&file).expect_err("only deferred may carry revive_when");
        assert!(message.contains("revive_when is only allowed"), "{message}");
    }

    #[test]
    fn empty_match_term_is_rejected() {
        let file: RoadmapFile = toml::from_str(
            r#"
[identity]
path = "README.md#why"
last_reviewed = "2026-11-18"

[[no_gos]]
id = "no-writes"
reason = "never mutate the tracker"
scope = "all verbs"
enforcement = "human_review"
revoke_by = "a new decision record"
match_terms = ["auto-close", "  "]
"#,
        )
        .expect("TOML parses");
        let message = validate(&file).expect_err("empty match term is invalid");
        assert!(message.contains("match_terms[1] is empty"), "{message}");
    }

    #[test]
    fn duplicate_no_go_id_is_rejected() {
        let file: RoadmapFile = toml::from_str(
            r#"
[identity]
path = "README.md#why"
last_reviewed = "2026-11-18"

[[no_gos]]
id = "no-writes"
reason = "never mutate the tracker"
scope = "all verbs"
enforcement = "human_review"
revoke_by = "a new decision record"

[[no_gos]]
id = "no-writes"
reason = "duplicate"
scope = "all verbs"
enforcement = "human_review"
revoke_by = "a new decision record"
"#,
        )
        .expect("TOML parses");
        let message = validate(&file).expect_err("duplicate no-go id is invalid");
        assert!(
            message.contains("no_gos[1]: duplicate id `no-writes`"),
            "{message}"
        );
    }

    #[test]
    fn dates_accept_iso_and_rfc3339() {
        let file: RoadmapFile = toml::from_str(&single_bet("").replace(
            "admitted_at = \"2026-11-18\"",
            "admitted_at = \"2026-11-18T10:30:00Z\"",
        ))
        .expect("TOML parses");
        assert!(validate(&file).is_ok());

        let file: RoadmapFile = toml::from_str(&single_bet("").replace(
            "last_reviewed = \"2026-11-18\"",
            "last_reviewed = \"18/11/2026\"",
        ))
        .expect("TOML parses");
        let message = validate(&file).expect_err("non-ISO date is invalid");
        assert!(message.contains("last_reviewed"), "{message}");
        assert!(message.contains("is not a date"), "{message}");
    }

    #[test]
    fn violations_aggregate_into_one_message() {
        let file: RoadmapFile = toml::from_str(
            r#"
[identity]
path = ""
last_reviewed = "not-a-date"

[[bets]]
id = "Bad Id"
title = ""
outcome = ""
horizon = "now"
appetite_weeks = 0
owner = ""
status = "committed"
admitted_at = "not-a-date"
last_reviewed = "2026-11-18"
"#,
        )
        .expect("TOML parses");
        let message = validate(&file).expect_err("everything is wrong");
        assert!(message.contains("path is empty"), "{message}");
        assert!(message.contains("last_reviewed `not-a-date`"), "{message}");
        assert!(message.contains("is not a slug"), "{message}");
        assert!(message.contains("title is 0 characters"), "{message}");
        assert!(message.contains("outcome is empty"), "{message}");
        assert!(message.contains("owner is empty"), "{message}");
        assert!(
            message.contains("appetite_weeks must be at least 1"),
            "{message}"
        );
        assert!(message.contains("review_or_kill is required"), "{message}");
        assert!(message.contains("admitted_at `not-a-date`"), "{message}");
        assert!(message.split("; ").count() >= 9, "{message}");
    }

    #[test]
    fn unknown_key_is_a_parse_error() {
        let parsed = toml::from_str::<RoadmapFile>(
            r#"
[identity]
path = "README.md#why"
last_reviewed = "2026-11-18"
bogus = "nope"
"#,
        );
        assert!(parsed.is_err(), "unknown keys must be rejected by serde");
    }

    #[test]
    fn unreadable_roadmap_is_a_domain_error() {
        let repo = tempfile::tempdir().expect("temp repo");
        std::fs::create_dir_all(repo.path().join(".podras")).expect("create .podras");
        // Bytes that are not UTF-8: the file exists but cannot be read as text.
        std::fs::write(repo.path().join(ROADMAP_REL_PATH), [0xFF, 0xFE, 0x00])
            .expect("write unreadable roadmap");

        let error = load(repo.path()).expect_err("unreadable content is not a roadmap");
        match error {
            Error::Domain {
                code,
                message,
                suggestion,
            } => {
                assert_eq!(code, "roadmap_unreadable");
                assert!(message.contains(ROADMAP_REL_PATH), "{message}");
                assert!(
                    suggestion
                        .as_deref()
                        .is_some_and(|fix| fix.contains("readable")),
                    "the error names the fix: {suggestion:?}"
                );
            }
            other => panic!("expected a domain error, got {other:?}"),
        }
    }

    #[test]
    fn load_reports_domain_codes() {
        let repo = tempfile::tempdir().expect("temp repo");
        assert!(load(repo.path()).expect("missing file is legal").is_none());

        std::fs::create_dir_all(repo.path().join(".podras")).expect("create .podras");
        std::fs::write(repo.path().join(ROADMAP_REL_PATH), DOCUMENTED).expect("write roadmap");
        let file = load(repo.path())
            .expect("documented example loads")
            .expect("file is present");
        assert_eq!(file.bets.len(), 1);

        std::fs::write(
            repo.path().join(ROADMAP_REL_PATH),
            "[identity]\npath = \"x\"\nbogus = 1\n",
        )
        .expect("write broken roadmap");
        let error = load(repo.path()).expect_err("unknown key fails");
        assert!(
            matches!(
                error,
                Error::Domain {
                    code: "roadmap_parse_error",
                    ..
                }
            ),
            "{error:?}"
        );

        std::fs::write(
            repo.path().join(ROADMAP_REL_PATH),
            "[identity]\npath = \"x\"\nlast_reviewed = \"2026-11-18\"\n\n[[bets]]\nid = \"Bad\"\n\
             title = \"T\"\noutcome = \"O\"\nhorizon = \"now\"\nappetite_weeks = 1\nowner = \"O\"\n\
             status = \"shaping\"\nadmitted_at = \"2026-11-18\"\nlast_reviewed = \"2026-11-18\"\n",
        )
        .expect("write invalid roadmap");
        let error = load(repo.path()).expect_err("semantic violation fails");
        assert!(
            matches!(
                error,
                Error::Domain {
                    code: "roadmap_invalid",
                    ..
                }
            ),
            "{error:?}"
        );
    }
}
