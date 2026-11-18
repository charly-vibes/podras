//! Purpose: prove the read-only roadmap lens against its declared contract.
//!
//! Responsibilities:
//! - keep the lens separate from the brief's focus/parked queues;
//! - assert the state file's accepted and rejected shapes, and the two
//!   presence cases (absent file omits the object, empty file is present);
//! - assert closed lifecycle enums, identity staleness rules, and no-go
//!   whole-token matching including the negative cases;
//! - assert warnings stay advisories in `roadmap.warnings`, never health;
//! - assert the additive, bounded, deterministically ordered envelope;
//! - assert every read verb is byte-for-byte side-effect free.
//!
//! Rationale: the roadmap is a human decision record rendered for a cold
//! reader. Each scenario pins one contract line and its near-miss, so a
//! regression fails with the boundary named rather than with a diff.

mod common;

#[path = "roadmap/fixtures.rs"]
mod fixtures;

use fixtures::{
    AS_OF, BET_ID, BeadRow, BetSpec, CLOSED_EPIC_ID, CURATION, EPIC_ID, FOCUS_ID, IDLE_EPIC_ID,
    NO_GO_ID, PARKED_ID, TempRepo, array_ids, assert_section, brief_data, empty_roadmap,
    error_code, fixture_with_identity, fixture_with_roadmap, full_fixture, headings, identity,
    load_text, no_go, object_keys, review_no_go, roadmap_data, roadmap_envelope, roadmap_file,
    roadmap_narrative, section, seed_epic_states, seed_full, seven_bets_roadmap, stdout, view,
    warning_codes, warning_text, warnings_roadmap,
};
use podras::model::{
    BetStatus, Enforcement, Horizon, RoadmapWarningKind, SCHEMA_VERSION, Staleness,
};
use podras::roadmap::file::load;
use serde_json::{Value, json};

/// The exact headings the lens renders, in order.
const HEADINGS: [&str; 10] = [
    "Identity",
    "Now",
    "Next",
    "Later",
    "Deferred",
    "Shipped",
    "Killed",
    "Not now (parked)",
    "Boundaries (no-gos)",
    "Warnings",
];

/// The nine advisory codes, in declaration order.
const WARNING_CODES: [&str; 9] = [
    "bet_age_exceeds_appetite",
    "circuit_breaker_due",
    "bet_epic_missing",
    "bet_epic_closed_not_retired",
    "bet_no_transition",
    "horizon_epic_mismatch",
    "no_go_violation_candidate",
    "identity_review_overdue",
    "roadmap_review_overdue",
];

/// Find one warning by code and subject.
fn warning<'a>(data: &'a Value, code: &str, subject: &str) -> &'a Value {
    data["roadmap"]["warnings"]
        .as_array()
        .expect("roadmap payload carries warnings")
        .iter()
        .find(|warning| warning["code"] == code && warning["subject"] == subject)
        .unwrap_or_else(|| panic!("warning `{code}` for `{subject}` is present: {data}"))
}

#[test]
fn lens_separate_from_now_lens() {
    let repo = full_fixture();

    // Library: attaching the lens changes nothing the now-lens renders.
    let with_lens = fixtures::attached(&repo, true);
    let without_lens = fixtures::attached(&repo, false);
    assert!(without_lens.roadmap.is_none(), "absent lens stays absent");
    assert!(with_lens.roadmap.is_some(), "present lens attaches");
    assert_eq!(
        with_lens.focus, without_lens.focus,
        "focus is curation-only"
    );
    assert_eq!(with_lens.next, without_lens.next, "next is computed-only");
    assert_eq!(
        with_lens.parked, without_lens.parked,
        "parked is curation-only"
    );
    assert_eq!(
        with_lens.health, without_lens.health,
        "the lens never edits health"
    );

    // Binary: curation owns focus/parked, the roadmap file owns bets, and the
    // two id sets never overlap. A bet's epic is an ordinary tracker bead, so
    // it may legitimately appear in a computed now-lens queue.
    let data = brief_data(&repo);
    let focus_ids = array_ids(&data["focus"]);
    let parked_ids = array_ids(&data["parked"]);
    assert_eq!(focus_ids, vec![FOCUS_ID.to_string()], "focus is curated");
    assert_eq!(parked_ids, vec![PARKED_ID.to_string()], "parked is curated");
    let bet_ids = array_ids(&data["roadmap"]["bets"]);
    assert_eq!(bet_ids, vec![BET_ID.to_string()]);
    assert!(
        !focus_ids
            .iter()
            .chain(parked_ids.iter())
            .any(|id| bet_ids.contains(id)),
        "lens entries and curation entries are disjoint: focus {focus_ids:?}, parked {parked_ids:?}, bets {bet_ids:?}"
    );
    assert!(
        data.get("roadmap").is_some(),
        "the lens is present while focus/parked remain curation state: {data}"
    );

    // Narrative: two separate render paths, no shared headings and no shared
    // prose. The focus id and reason exist only in curation and never render
    // in the lens.
    let lens = roadmap_narrative(&repo);
    assert_eq!(headings(&lens), HEADINGS.to_vec());
    let brief = stdout(&repo, &["brief", "--as-of", AS_OF]);
    assert!(
        brief.contains("## Focus"),
        "the brief keeps its own focus section: {brief}"
    );
    for marker in [
        "# Roadmap lens",
        "## Identity",
        "## Boundaries (no-gos)",
        "## Now",
        "## Later",
        "## Deferred",
        "## Shipped",
        "## Killed",
        "## Warnings",
    ] {
        assert!(
            !brief.contains(&format!("{marker}\n")),
            "the brief never renders the roadmap section `{marker}`: {brief}"
        );
    }
    for lens_prose in [
        "A cold reader predicts the next bet from the lens alone",
        "podras never mutates the tracker",
    ] {
        assert!(
            !brief.contains(lens_prose),
            "lens prose never leaks into BRIEF.md: `{lens_prose}` in {brief}"
        );
    }
    for heading in [
        "Focus",
        "In flight",
        "Blocked / waiting",
        "Next / ready",
        "Recent",
        "Health + confidence",
    ] {
        assert!(
            !lens.contains(&format!("## {heading}\n")),
            "the lens never renders the brief section `{heading}`: {lens}"
        );
    }
    for focus_only in [FOCUS_ID, "finish the brief read path"] {
        assert!(
            !lens.contains(focus_only),
            "the lens never renders curation focus `{focus_only}`: {lens}"
        );
    }

    // The canonical lens renders the identity row, one bet, boundaries, and no
    // warnings, exactly.
    assert_section(
        &lens,
        "Identity",
        &[
            "- path: README.md#why",
            "- last reviewed: 2026-11-01",
            "- review due: 2027-05-18",
            "- staleness: fresh",
        ],
    );
    let bet_lines = fixtures::canonical_bet_lines();
    assert_section(&lens, "Now", &[bet_lines[0], bet_lines[1], bet_lines[2]]);
    for group in ["Next", "Later", "Deferred", "Shipped", "Killed", "Warnings"] {
        assert_section(&lens, group, &["none"]);
    }
    assert_section(
        &lens,
        "Not now (parked)",
        &[
            "- podras-f — harvest moves to v1.1 (parked: 2026-11-10; confirmed: 2026-11-10; resurface: 2027-01-01)",
        ],
    );
    let boundary = format!(
        "- {NO_GO_ID} — podras never mutates the tracker (scope: all verbs; enforcement: automated; revoke by: a new design decision)"
    );
    assert_section(&lens, "Boundaries (no-gos)", &[boundary.as_str()]);
}

#[test]
fn roadmap_file_shape() {
    // No file: absence is legal and the payload must not grow the key.
    let absent = TempRepo::new();
    seed_full(&absent);
    absent.curation(CURATION);
    absent.commit("brief only");

    assert!(
        load(absent.root()).expect("absence is legal").is_none(),
        "a repository without .podras/roadmap.toml loads no roadmap"
    );
    let brief = brief_data(&absent);
    assert!(
        brief.get("roadmap").is_none(),
        "absent file omits the roadmap object entirely, never `null`: {brief}"
    );

    let runs: Vec<Vec<&str>> = vec![
        vec!["roadmap", "--as-of", AS_OF],
        vec!["roadmap", "--json", "--as-of", AS_OF],
        vec!["roadmap", "--write", "--as-of", AS_OF],
    ];
    for args in &runs {
        let failed = fixtures::podras_in(&absent, args).failure();
        let stderr =
            String::from_utf8(failed.get_output().stderr.clone()).expect("roadmap stderr is UTF-8");
        assert!(
            stderr.contains("no .podras/roadmap.toml"),
            "`{args:?}` names the missing file: {stderr}"
        );
    }

    // Present file with zero bets: a present lens, never the absent case.
    let empty = fixture_with_roadmap(&empty_roadmap());
    assert!(
        load(empty.root()).expect("empty lens loads").is_some(),
        "a present file is a present lens even with zero bets"
    );
    let data = roadmap_data(&empty);
    for key in ["bets", "no_gos"] {
        assert!(
            data["roadmap"][key]
                .as_array()
                .expect("list field is an array")
                .is_empty(),
            "`{key}` is an explicit empty list: {data}"
        );
    }
    assert_eq!(data["roadmap"]["identity"]["path"], "README.md#why");
    assert_eq!(data["roadmap"]["truncation"]["applied"], false);
    assert_eq!(
        object_keys(&data["roadmap"]["truncation"]["omitted_counts"]),
        Vec::<String>::new(),
        "nothing was truncated"
    );
    assert_eq!(
        brief_data(&empty)["roadmap"],
        data["roadmap"],
        "both verbs render the same lens"
    );

    let narrative = roadmap_narrative(&empty);
    assert_eq!(headings(&narrative), HEADINGS.to_vec());
    assert_section(
        &narrative,
        "Identity",
        &[
            "- path: README.md#why",
            "- last reviewed: 2026-11-01",
            "- review due: 2027-05-18",
            "- staleness: fresh",
        ],
    );
    for group in ["Now", "Next", "Later", "Deferred", "Shipped", "Killed"] {
        assert_section(&narrative, group, &["none"]);
    }
    assert_section(&narrative, "Boundaries (no-gos)", &["none"]);
    assert_section(&narrative, "Warnings", &["none"]);

    let view = view(&empty);
    assert!(view.bets.is_empty() && view.no_gos.is_empty());
    assert_eq!(view.identity.path, "README.md#why");
}

#[test]
fn bet_lifecycle_enums() {
    // Every lifecycle status loads, and the typed enum round-trips its wire
    // name; requirement companions are supplied per status.
    for status in [
        "shaping",
        "committed",
        "building",
        "deferred",
        "shipped",
        "killed",
    ] {
        let mut spec = BetSpec::new("bet-enum", "Exercise the lifecycle", status, "next");
        if matches!(status, "committed" | "building") {
            spec = spec.with_epic(EPIC_ID).with_review_or_kill("2026-12-30");
        }
        if status == "deferred" {
            spec = spec.with_revive_when("when the lens ships");
        }
        let document = roadmap_file(&identity(Some("2027-05-18")), &[spec.render()], &[]);
        let file = load_text(&document)
            .unwrap_or_else(|error| panic!("status `{status}` loads: {error}"))
            .expect("file is present");
        assert_eq!(
            serde_json::to_value(file.bets[0].status).expect("status serializes"),
            json!(status)
        );
        let parsed: BetStatus =
            serde_json::from_value(json!(status)).expect("status wire name is closed and typed");
        assert_eq!(
            serde_json::to_value(parsed).expect("status serializes"),
            json!(status)
        );
    }

    for horizon in ["now", "next", "later"] {
        let spec = BetSpec::new("bet-enum", "Exercise the horizon", "shaping", horizon);
        let document = roadmap_file(&identity(Some("2027-05-18")), &[spec.render()], &[]);
        let file = load_text(&document)
            .expect("valid horizon")
            .expect("present");
        assert_eq!(
            serde_json::to_value(file.bets[0].horizon).expect("horizon serializes"),
            json!(horizon)
        );
        let parsed: Horizon =
            serde_json::from_value(json!(horizon)).expect("horizon wire name is typed");
        assert_eq!(
            serde_json::to_value(parsed).expect("horizon serializes"),
            json!(horizon)
        );
    }

    for tier in ["automated", "human_review"] {
        let document = roadmap_file(
            &identity(Some("2027-05-18")),
            &[],
            &[no_go(
                "no-tier",
                "never mutate",
                "all verbs",
                tier,
                "a new decision",
                &[],
            )],
        );
        let file = load_text(&document)
            .expect("valid enforcement")
            .expect("present");
        assert_eq!(
            serde_json::to_value(file.no_gos[0].enforcement).expect("enforcement serializes"),
            json!(tier)
        );
        let parsed: Enforcement =
            serde_json::from_value(json!(tier)).expect("enforcement wire name is typed");
        assert_eq!(
            serde_json::to_value(parsed).expect("enforcement serializes"),
            json!(tier)
        );
    }

    // The enums are closed: no other member deserializes.
    assert!(
        serde_json::from_value::<BetStatus>(json!("paused")).is_err(),
        "`paused` is not a bet status"
    );
    assert!(
        serde_json::from_value::<Horizon>(json!("someday")).is_err(),
        "`someday` is not a horizon"
    );
    assert!(
        serde_json::from_value::<Enforcement>(json!("ci")).is_err(),
        "`ci` is not an enforcement tier"
    );

    // Structural failures are parse errors; semantic violations are invalid.
    let valid_identity = identity(Some("2027-05-18"));
    let cases: [(&str, String); 9] = [
        (
            "roadmap_parse_error",
            "[identity\npath = \"README.md#why\"\n".to_string(),
        ),
        (
            "roadmap_parse_error",
            roadmap_file(
                &valid_identity,
                &[BetSpec::new("bet-enum", "Title", "paused", "now").render()],
                &[],
            ),
        ),
        (
            "roadmap_parse_error",
            roadmap_file(
                &valid_identity,
                &[BetSpec::new("bet-enum", "Title", "shaping", "someday").render()],
                &[],
            ),
        ),
        (
            "roadmap_parse_error",
            roadmap_file(
                &valid_identity,
                &[],
                &[no_go(
                    "no-tier",
                    "reason",
                    "scope",
                    "ci",
                    "a new decision",
                    &[],
                )],
            ),
        ),
        (
            "roadmap_invalid",
            roadmap_file(
                &valid_identity,
                &[BetSpec::new("Bad Slug", "Title", "shaping", "now").render()],
                &[],
            ),
        ),
        (
            "roadmap_invalid",
            roadmap_file(
                &valid_identity,
                &[BetSpec::new("missing-review", "Title", "committed", "next")
                    .with_epic(EPIC_ID)
                    .render()],
                &[],
            ),
        ),
        (
            "roadmap_invalid",
            roadmap_file(
                &valid_identity,
                &[BetSpec::new("missing-trigger", "Title", "deferred", "next").render()],
                &[],
            ),
        ),
        (
            "roadmap_invalid",
            roadmap_file(
                &valid_identity,
                &[BetSpec::new("stray-trigger", "Title", "shaping", "next")
                    .with_revive_when("never")
                    .render()],
                &[],
            ),
        ),
        (
            "roadmap_invalid",
            roadmap_file(
                &valid_identity,
                &[],
                &[review_no_go("empty-term", "human review only", &[""])],
            ),
        ),
    ];
    for (expected, document) in cases {
        let error = load_text(&document).expect_err("invalid roadmap is rejected");
        assert_eq!(error_code(&error), Some(expected), "{expected}: {error}");
    }

    // The binary surfaces the same codes and refuses to guess.
    let invalid = TempRepo::new();
    invalid.roadmap(&roadmap_file(
        &valid_identity,
        &[BetSpec::new("Bad Slug", "Title", "shaping", "now").render()],
        &[],
    ));
    let failed = fixtures::podras_in(&invalid, &["roadmap", "--json", "--as-of", AS_OF]).failure();
    let stderr =
        String::from_utf8(failed.get_output().stderr.clone()).expect("roadmap stderr is UTF-8");
    assert!(
        stderr.contains("roadmap_invalid"),
        "a semantic violation reports its code: {stderr}"
    );

    invalid.roadmap("[[bets]\nid = \"x\"\n");
    let failed = fixtures::podras_in(&invalid, &["roadmap", "--json", "--as-of", AS_OF]).failure();
    let stderr =
        String::from_utf8(failed.get_output().stderr.clone()).expect("roadmap stderr is UTF-8");
    assert!(
        stderr.contains("roadmap_parse_error"),
        "malformed TOML reports the parse code: {stderr}"
    );
}

#[test]
fn identity_staleness_never_fresh() {
    // Absent next_review: unknown plus the advisory, never green by omission.
    let unscheduled = fixture_with_identity(None);
    let data = roadmap_data(&unscheduled);
    assert_eq!(data["roadmap"]["identity"]["staleness"], "unknown");
    assert!(
        data["roadmap"]["identity"]["next_review"].is_null(),
        "no next_review is reported as null: {data}"
    );
    assert!(data["roadmap"]["review_due"]["identity"].is_null());
    assert_eq!(warning_codes(&data), vec!["identity_review_overdue"]);
    assert_eq!(view(&unscheduled).identity.staleness, Staleness::Unknown);

    let narrative = roadmap_narrative(&unscheduled);
    assert_section(
        &narrative,
        "Identity",
        &[
            "- path: README.md#why",
            "- last reviewed: 2026-11-01",
            "- review due: none",
            "- staleness: unknown",
        ],
    );
    let warnings = section(&narrative, "Warnings");
    let first = warnings
        .lines()
        .next()
        .expect("the advisory renders a line");
    assert!(
        first.starts_with("- identity_review_overdue — identity — "),
        "the advisory names the code and the identity subject: {warnings}"
    );
    assert!(
        first.contains("next_review"),
        "the advisory explains the missing schedule: {warnings}"
    );

    // Passed next_review: stale, and the date stays the declared review date.
    let passed = fixture_with_identity(Some("2026-10-01"));
    let data = roadmap_data(&passed);
    assert_eq!(data["roadmap"]["identity"]["staleness"], "stale");
    assert_eq!(data["roadmap"]["review_due"]["identity"], "2026-10-01");
    assert!(
        warning_codes(&data).is_empty(),
        "an overdue identity is stale, not an advisory replay: {data}"
    );
    assert_eq!(view(&passed).identity.staleness, Staleness::Stale);
    assert!(
        section(&roadmap_narrative(&passed), "Identity").contains("- staleness: stale"),
        "the narrative states the stale verdict"
    );

    // Equal or future next_review: fresh, decided by the pinned date alone.
    for next_review in ["2026-11-18", "2027-05-18"] {
        let fresh = fixture_with_identity(Some(next_review));
        let data = roadmap_data(&fresh);
        assert_eq!(
            data["roadmap"]["identity"]["staleness"], "fresh",
            "next_review {next_review} at --as-of {AS_OF}"
        );
        assert_eq!(data["roadmap"]["review_due"]["identity"], next_review);
        assert!(warning_codes(&data).is_empty());
        assert_eq!(
            view(&fresh).identity.staleness,
            Staleness::Fresh,
            "next_review {next_review} is not before --as-of {AS_OF}"
        );
    }
}

#[test]
fn no_go_violation_flags_only() {
    let repo = TempRepo::new();
    repo.beads_jsonl_rows(&[
        BeadRow {
            id: "podras-close",
            title: "Ship auto-close on merge",
            status: "open",
            labels: &[],
            updated_at: "2026-11-10T00:00:00Z",
        },
        BeadRow {
            id: "podras-label",
            title: "Housekeeping",
            status: "open",
            labels: &["auto-promote"],
            updated_at: "2026-11-10T00:00:00Z",
        },
        BeadRow {
            id: "podras-autoclose",
            title: "Support autoclose in the CLI",
            status: "open",
            labels: &[],
            updated_at: "2026-11-10T00:00:00Z",
        },
        BeadRow {
            id: "podras-suffix",
            title: "autoclosed issues stay open",
            status: "open",
            labels: &[],
            updated_at: "2026-11-10T00:00:00Z",
        },
        BeadRow {
            id: "podras-scope",
            title: "all verbs",
            status: "open",
            labels: &[],
            updated_at: "2026-11-10T00:00:00Z",
        },
        BeadRow {
            id: FOCUS_ID,
            title: "Brief read path",
            status: "open",
            labels: &[],
            updated_at: "2026-11-10T00:00:00Z",
        },
        BeadRow {
            id: PARKED_ID,
            title: "Harvest v1.1",
            status: "open",
            labels: &[],
            updated_at: "2026-11-10T00:00:00Z",
        },
    ]);
    repo.curation(CURATION);
    repo.roadmap(&roadmap_file(
        &identity(Some("2027-05-18")),
        &[],
        &[
            review_no_go("no-auto-close", "never auto-close issues", &["auto-close"]),
            review_no_go("empty-terms", "human review only", &[]),
            review_no_go("no-auto-promote", "never auto-promote", &["auto-promote"]),
        ],
    ));
    repo.commit("no-go fixture");

    let data = roadmap_data(&repo);
    let warnings = data["roadmap"]["warnings"]
        .as_array()
        .expect("warnings array");
    assert_eq!(
        warnings.len(),
        2,
        "exactly two candidates fire; empty terms and near-tokens stay silent: {data}"
    );
    assert_eq!(
        warning_codes(&data),
        vec!["no_go_violation_candidate".to_string(); 2]
    );

    // Provenance: the no-go id and the offending bead both appear.
    let title_case = warning(&data, "no_go_violation_candidate", "no-auto-close");
    assert!(
        warning_text(title_case).contains("podras-close"),
        "the title match names its bead: {title_case}"
    );
    assert!(
        warning_text(title_case).contains("`auto-close`"),
        "the match names the term: {title_case}"
    );
    let label_case = warning(&data, "no_go_violation_candidate", "no-auto-promote");
    assert!(
        warning_text(label_case).contains("podras-label"),
        "the label match names its bead: {label_case}"
    );

    let joined = warnings
        .iter()
        .map(warning_text)
        .collect::<Vec<_>>()
        .join(" | ");
    for never in [
        "podras-autoclose",
        "podras-suffix",
        "podras-scope",
        "empty-terms",
    ] {
        assert!(
            !joined.contains(never),
            "`{never}` must never be flagged: {joined}"
        );
    }

    // The type is closed too: only the violation kind fired.
    let view = view(&repo);
    assert_eq!(view.warnings.len(), 2);
    for warning in &view.warnings {
        assert!(matches!(
            warning.code,
            RoadmapWarningKind::NoGoViolationCandidate
        ));
    }

    // An open bead with an empty-term no-go and a closed matching bead both
    // stay unreported, including under the narrative render.
    repo.roadmap(&roadmap_file(
        &identity(Some("2027-05-18")),
        &[],
        &[review_no_go("empty-terms", "human review only", &[])],
    ));
    assert!(
        roadmap_data(&repo)["roadmap"]["warnings"]
            .as_array()
            .expect("warnings array")
            .is_empty(),
        "empty match_terms never fires, even against scope-shaped titles"
    );
}

#[test]
fn roadmap_warnings_not_verdict() {
    let with_lens = TempRepo::new();
    seed_epic_states(&with_lens);
    with_lens.curation(CURATION);
    with_lens.roadmap(&warnings_roadmap());
    with_lens.commit("warnings lens");

    let without = TempRepo::new();
    seed_epic_states(&without);
    without.curation(CURATION);
    without.commit("no lens");

    let envelope = roadmap_envelope(&with_lens);
    assert_eq!(envelope["ok"], true, "advisories never fail the verb");
    let data = envelope["data"].clone();

    assert_eq!(
        warning_codes(&data),
        vec![
            "bet_age_exceeds_appetite",
            "circuit_breaker_due",
            "bet_epic_missing",
            "bet_epic_closed_not_retired",
            "bet_no_transition",
            "horizon_epic_mismatch",
            "identity_review_overdue",
            "roadmap_review_overdue",
        ],
        "all eight fired advisories, in declaration order: {data}"
    );

    for payload in data["roadmap"]["warnings"].as_array().expect("warnings") {
        assert!(
            !payload["subject"].as_str().unwrap_or_default().is_empty(),
            "every warning names a subject: {payload}"
        );
        assert!(
            !payload["detail"].as_str().unwrap_or_default().is_empty(),
            "every warning carries evidence: {payload}"
        );
    }

    assert!(warning_text(warning(&data, "bet_age_exceeds_appetite", "bet-aged")).contains("17d"));
    assert!(warning_text(warning(&data, "bet_age_exceeds_appetite", "bet-aged")).contains("14d"));
    assert!(warning_text(warning(&data, "circuit_breaker_due", "bet-breaker")).contains("78d"));
    assert!(
        warning_text(warning(&data, "bet_no_transition", "bet-stale-epic")).contains(IDLE_EPIC_ID),
        "staleness names the idle epic"
    );
    assert!(
        warning_text(warning(&data, "bet_epic_closed_not_retired", "bet-breaker"))
            .contains(CLOSED_EPIC_ID)
    );
    assert!(warning_text(warning(&data, "horizon_epic_mismatch", "bet-breaker")).contains("now"));
    assert!(
        warning_text(warning(&data, "roadmap_review_overdue", "bet-breaker"))
            .contains("2026-10-01"),
        "the overdue review names its declared date"
    );
    assert_eq!(
        data["roadmap"]["review_due"]["roadmap"], "2026-10-01",
        "the earliest active review-or-kill is the roadmap review date"
    );
    assert!(
        data["roadmap"]["review_due"]["identity"].is_null(),
        "identity declares no scheduled review"
    );

    // Never a verdict: identical health with and without the lens, no roadmap
    // code in the health section, and no roadmap source row.
    let with_data = brief_data(&with_lens);
    let without_data = brief_data(&without);
    assert_eq!(
        with_data["health"], without_data["health"],
        "adding advisories must not move the health verdict"
    );
    let mut expected_sources = vec!["beads", "git", "openspec", "whisper", "curation"];
    expected_sources.sort_unstable();
    let mut actual_sources: Vec<&str> = with_data["sources"]
        .as_array()
        .expect("sources array")
        .iter()
        .map(|source| source["name"].as_str().expect("source name"))
        .collect();
    actual_sources.sort_unstable();
    assert_eq!(actual_sources, expected_sources, "the lens is not a source");

    let signal_kinds = [
        "stale_focus",
        "wip_over_cap",
        "stuck_project",
        "focus_closed",
        "parked_expired",
        "orphan_commits",
        "spec_change_without_bead",
    ];
    for signal in with_data["health"]["signals"]
        .as_array()
        .expect("signals array")
    {
        let kind = signal["kind"].as_str().expect("signal kind");
        assert!(
            signal_kinds.contains(&kind),
            "`{kind}` is a health signal; roadmap codes are not SignalKind variants"
        );
    }
    let health_text = with_data["health"].to_string();
    for code in WARNING_CODES {
        assert!(
            !health_text.contains(code),
            "`{code}` must stay in roadmap.warnings, never in health: {health_text}"
        );
    }

    // Narrative: one bounded-nowhere list of warnings, no health verdict.
    let fired: Vec<&str> = WARNING_CODES
        .iter()
        .copied()
        .filter(|code| *code != "no_go_violation_candidate")
        .collect();
    let lens = roadmap_narrative(&with_lens);
    let lines: Vec<&str> = section(&lens, "Warnings").lines().collect();
    assert_eq!(lines.len(), fired.len());
    for code in fired {
        assert!(
            lines
                .iter()
                .any(|line| line.starts_with(&format!("- {code} — "))),
            "`{code}` renders as `- {code} — <subject> — <detail>`: {lens}"
        );
    }
    assert!(
        !lens.contains("Health"),
        "the lens never renders a health verdict: {lens}"
    );

    // Exit code is always zero, narrative and envelope.
    stdout(&with_lens, &["roadmap", "--as-of", AS_OF]);
}

#[test]
fn roadmap_envelope_additive_bounded() {
    let with_lens = TempRepo::new();
    seed_full(&with_lens);
    with_lens.curation(CURATION);
    with_lens.roadmap(&seven_bets_roadmap());
    with_lens.commit("seven bets");

    let without = TempRepo::new();
    seed_full(&without);
    without.curation(CURATION);
    without.commit("no lens");

    let with_data = roadmap_data(&with_lens);
    let without_data = brief_data(&without);

    // Additive: identical top-level shape plus exactly one new key.
    let mut expected = object_keys(&without_data);
    expected.push("roadmap".to_string());
    expected.sort();
    assert_eq!(
        object_keys(&with_data),
        expected,
        "the lens adds `roadmap` and nothing else"
    );
    assert_eq!(
        with_data["schema_version"], SCHEMA_VERSION,
        "additive objects keep the brief.v1 major"
    );
    assert_eq!(with_data["schema_version"], "brief.v1");

    // Bounded bets: rank, then admitted asc, then id; two dropped.
    assert_eq!(
        array_ids(&with_data["roadmap"]["bets"]),
        vec!["bet-build", "bet-commit", "bet-defer", "shape-a", "shape-b"]
    );
    assert_eq!(with_data["roadmap"]["truncation"]["applied"], true);
    assert_eq!(
        with_data["roadmap"]["truncation"]["omitted_counts"]["bets"],
        2
    );
    assert_eq!(
        with_data["roadmap"]["truncation"]["omitted_counts"]["no_gos"],
        2
    );
    assert_eq!(
        object_keys(&with_data["roadmap"]["truncation"]["omitted_counts"]),
        vec!["bets".to_string(), "no_gos".to_string()],
        "the lens truncation keys are exactly bets and no_gos"
    );
    assert_eq!(
        array_ids(&with_data["roadmap"]["no_gos"]),
        vec!["no-go-01", "no-go-02", "no-go-03", "no-go-04", "no-go-05"]
    );

    // The brief's own truncation never gains lens keys.
    let brief_truncation = with_data["truncation"]["omitted_counts"].clone();
    assert!(
        brief_truncation.get("bets").is_none() && brief_truncation.get("no_gos").is_none(),
        "roadmap bounds live in roadmap.truncation: {brief_truncation}"
    );

    // The library view agrees with the wire shape, typed field for field.
    let view = view(&with_lens);
    let ids: Vec<&str> = view.bets.iter().map(|bet| bet.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["bet-build", "bet-commit", "bet-defer", "shape-a", "shape-b"]
    );
    assert_eq!(view.bets.len(), 5);
    assert_eq!(view.no_gos.len(), 5);
    assert!(view.truncation.applied);
    assert_eq!(view.truncation.omitted_counts.get("bets").copied(), Some(2));
    assert_eq!(
        view.truncation.omitted_counts.get("no_gos").copied(),
        Some(2)
    );

    // The dropped bets are the lowest-ranked ones and are not rendered.
    let lens = roadmap_narrative(&with_lens);
    assert_eq!(headings(&lens), HEADINGS.to_vec());
    assert!(section(&lens, "Now").contains("- bet-build — "));
    assert!(section(&lens, "Next").contains("- bet-commit — "));
    assert!(section(&lens, "Next").contains("- shape-a — "));
    assert!(section(&lens, "Deferred").contains("- bet-defer — "));
    assert!(section(&lens, "Later").contains("- shape-b — "));
    assert_section(&lens, "Shipped", &["none"]);
    let killed = section(&lens, "Killed");
    assert_eq!(
        killed.lines().next(),
        Some("none"),
        "every bet in the Killed group was dropped by the bound: {killed}"
    );
    assert_eq!(
        killed
            .lines()
            .filter(|line| fixtures::is_omission(line))
            .collect::<Vec<_>>(),
        vec!["(2 more omitted)"],
        "the lens-wide bets bound is stated once, after the final group: {killed}"
    );
    assert_eq!(
        lens.matches("(2 more omitted)").count(),
        1,
        "the bet bound is lens-wide, not repeated per group: {lens}"
    );

    // Same-rank bets keep admission order, then id order.
    let next = section(&lens, "Next");
    assert!(
        next.find("- bet-commit — ").expect("commit renders")
            < next.find("- shape-a — ").expect("shape-a renders"),
        "rank order precedes admission order: {next}"
    );
}

#[test]
fn roadmap_read_only() {
    let repo = full_fixture();
    let before = repo.tree_snapshot();

    stdout(&repo, &["roadmap", "--as-of", AS_OF]);
    stdout(&repo, &["roadmap", "--json", "--as-of", AS_OF]);
    stdout(&repo, &["brief", "--json", "--as-of", AS_OF]);

    assert_eq!(
        repo.tree_snapshot(),
        before,
        "reads must not write one byte: no render, no roadmap edit, no tracker write"
    );

    // --write into an external directory: the repository stays untouched and
    // the target receives exactly the pair.
    let external = tempfile::tempdir().expect("create external render directory");
    let external_path = external.path().to_str().expect("UTF-8 temp path");
    let printed = stdout(
        &repo,
        &[
            "roadmap",
            "--write",
            "--out-dir",
            external_path,
            "--as-of",
            AS_OF,
        ],
    );
    assert!(
        printed.contains("ROADMAP.md") && printed.contains("ROADMAP.json"),
        "the write report names both artifacts: {printed}"
    );
    assert_eq!(
        repo.tree_snapshot(),
        before,
        "writing outside the repository never touches it"
    );
    let mut written: Vec<String> = std::fs::read_dir(external.path())
        .expect("external directory is readable")
        .map(|entry| {
            entry
                .expect("external directory entry")
                .file_name()
                .to_string_lossy()
                .to_string()
        })
        .collect();
    written.sort();
    assert_eq!(written, vec!["ROADMAP.json", "ROADMAP.md"]);

    let markdown =
        std::fs::read_to_string(external.path().join("ROADMAP.md")).expect("ROADMAP.md is written");
    assert!(
        markdown.starts_with("# Roadmap lens"),
        "the committed markdown is the lens narrative: {markdown}"
    );
    let json: Value = serde_json::from_str(
        &std::fs::read_to_string(external.path().join("ROADMAP.json"))
            .expect("ROADMAP.json is written"),
    )
    .expect("ROADMAP.json is an envelope");
    assert_eq!(
        json["data"]["roadmap"]["bets"]
            .as_array()
            .expect("bets array")
            .len(),
        1,
        "the written envelope carries the same lens"
    );

    // --write in the repository root adds exactly the pair and rewrites
    // nothing else.
    let printed = stdout(&repo, &["roadmap", "--write", "--as-of", AS_OF]);
    assert_eq!(printed, "wrote ROADMAP.md, ROADMAP.json");

    let after = repo.tree_snapshot();
    let mut added: Vec<String> = after
        .keys()
        .filter(|path| !before.contains_key(*path))
        .cloned()
        .collect();
    added.sort();
    assert_eq!(
        added,
        vec!["ROADMAP.json".to_string(), "ROADMAP.md".to_string()],
        "the lens write adds only the committed pair"
    );
    let removed: Vec<&String> = before
        .keys()
        .filter(|path| !after.contains_key(*path))
        .collect();
    assert!(removed.is_empty(), "no file is removed: {removed:?}");
    for (path, bytes) in &before {
        assert_eq!(
            after.get(path),
            Some(bytes),
            "`{path}` keeps its exact bytes across every lens verb"
        );
    }
}
