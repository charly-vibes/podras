//! Purpose: integration tests for the stage ⑦ bet lifecycle verbs.
//!
//! Responsibilities:
//! - `bet_add_appends_a_shaping_bet_that_round_trips`: add appends one valid
//!   `shaping` bet and the file round-trips;
//! - `bet_admit_moves_only_shaping_to_committed`: the admission guard;
//! - `bet_defer_requires_a_revive_trigger_and_stamps_the_review`: deferral
//!   semantics and its guards;
//! - `bet_kill_is_legal_from_every_status`: killing, with `killed_reason`
//!   recorded exactly when `--reason` is given;
//! - `bet_verbs_reject_unknown_ids`: every verb names the unknown bet;
//! - `bet_verbs_refuse_to_write_a_malformed_file`: the D3 error table blocks
//!   every write;
//! - `bet_add_is_deterministic_byte_for_byte`: identical runs write identical
//!   bytes;
//! - `bet_as_of_controls_the_stamped_dates`: the decision date is the stamp;
//! - `bet_verbs_require_a_scaffolded_roadmap_file`: no verb scaffolds state;
//! - `bet_verbs_exit_zero_on_success_and_one_on_domain_error`: the exit code
//!   contract.
//!
//! Rationale: the roadmap file is a human decision record that verbs append
//! to and transition but never corrupt. Each test pins one guard, one stamp,
//! or one refusal so a regression names the contract line it broke.

mod common;

#[path = "roadmap/fixtures.rs"]
mod fixtures;

use fixtures::{BetSpec, EPIC_ID, TempRepo, full_roadmap, identity, podras_in, roadmap_file};
use podras::model::{BetStatus, Horizon};
use podras::roadmap::file::{RoadmapFile, load};

/// The [identity]-only document every verb-rejection fixture starts from.
fn identity_only() -> String {
    roadmap_file(&identity(Some("2027-05-18")), &[], &[])
}

/// A valid roadmap document holding one bet per lifecycle status.
fn every_status_roadmap() -> String {
    let bets = [
        BetSpec::new("bet-shaping", "Shape the verbs", "shaping", "later"),
        BetSpec::new("bet-committed", "Commit the verbs", "committed", "next")
            .with_epic(EPIC_ID)
            .with_review_or_kill("2026-12-30"),
        BetSpec::new("bet-building", "Build the verbs", "building", "now")
            .with_epic(EPIC_ID)
            .with_review_or_kill("2026-12-30"),
        BetSpec::new("bet-deferred", "Defer the verbs", "deferred", "next")
            .with_revive_when("2027-01-15"),
        BetSpec::new("bet-shipped", "Ship the verbs", "shipped", "later"),
        BetSpec::new("bet-killed", "Kill the graph view", "killed", "later")
            .with_killed_reason("no value-prop evidence"),
    ];
    let rendered: Vec<String> = bets.iter().map(|bet| bet.render()).collect();
    roadmap_file(&identity(Some("2027-05-18")), &rendered, &[])
}

/// The validated roadmap file of a fixture repository.
fn loaded(repo: &TempRepo) -> RoadmapFile {
    load(repo.root())
        .expect("verb wrote a valid roadmap file")
        .expect("verb wrote a roadmap file")
}

/// The exact file bytes of a fixture repository's roadmap.
fn bytes(repo: &TempRepo) -> String {
    std::fs::read_to_string(repo.root().join(".podras/roadmap.toml"))
        .expect("roadmap file is readable")
}

/// Run one bet verb pinned to the fixture date; the caller asserts the result.
fn bet(repo: &TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut full = vec!["bet"];
    full.extend_from_slice(args);
    full.extend_from_slice(&["--as-of", "2026-11-18"]);
    podras_in(repo, &full)
}

/// The add arguments the happy-path tests share.
const ADD_ARGS: &[&str] = &[
    "add",
    "--id",
    "verb-lens",
    "--title",
    "Try the bet verbs",
    "--outcome",
    "A human curates bets from the CLI alone",
    "--horizon",
    "later",
    "--appetite-weeks",
    "3",
    "--owner",
    "sasha",
];

#[test]
fn bet_add_appends_a_shaping_bet_that_round_trips() {
    let repo = fixtures::fixture_with_roadmap(&full_roadmap());

    bet(&repo, ADD_ARGS).success();

    let file = loaded(&repo);
    assert_eq!(file.bets.len(), 2, "the appended bet is last");
    let added = file.bets.last().expect("the appended bet");
    assert_eq!(added.id, "verb-lens");
    assert_eq!(added.title, "Try the bet verbs");
    assert_eq!(added.outcome, "A human curates bets from the CLI alone");
    assert_eq!(added.horizon, Horizon::Later);
    assert_eq!(added.appetite_weeks, 3);
    assert_eq!(added.owner, "sasha");
    assert_eq!(added.epic, None, "add without --epic links none");
    assert_eq!(added.status, BetStatus::Shaping);
    assert_eq!(
        added.admitted_at, None,
        "admission stamps admitted_at; add must not"
    );
    assert_eq!(
        added.last_reviewed, "2026-11-18",
        "add stamps last_reviewed with the decision date"
    );
    assert_eq!(added.review_or_kill, None);
    assert_eq!(added.revive_when, None);
    assert_eq!(added.killed_reason, None);

    // The untouched bet, no-go, and identity survive the canonical rewrite.
    let first = &file.bets[0];
    assert_eq!(first.id, "brief-v2");
    assert_eq!(first.status, BetStatus::Building);
    assert_eq!(
        first.review_or_kill.as_deref(),
        Some("2026-12-30"),
        "existing fields survive the rewrite"
    );
    assert_eq!(
        file.no_gos.first().map(|no_go| no_go.id.as_str()),
        Some("no-tracker-writes"),
        "[[no_gos]] survives the rewrite"
    );
    assert_eq!(
        file.identity.path, "README.md#why",
        "[identity] survives the rewrite"
    );
}

#[test]
fn bet_admit_moves_only_shaping_to_committed() {
    let repo = fixtures::fixture_with_roadmap(&identity_only());
    bet(&repo, ADD_ARGS).success();

    bet(
        &repo,
        &[
            "admit",
            "verb-lens",
            "--epic",
            EPIC_ID,
            "--review-or-kill",
            "2026-12-30",
        ],
    )
    .success();

    let admitted = &loaded(&repo).bets[0];
    assert_eq!(admitted.status, BetStatus::Committed);
    assert_eq!(admitted.admitted_at.as_deref(), Some("2026-11-18"));
    assert_eq!(admitted.last_reviewed, "2026-11-18");
    assert_eq!(admitted.review_or_kill.as_deref(), Some("2026-12-30"));
    assert_eq!(admitted.epic.as_deref(), Some(EPIC_ID));

    // Guard: admitting a committed bet is a domain error naming bet and rule.
    let before = bytes(&repo);
    bet(
        &repo,
        &[
            "admit",
            "verb-lens",
            "--epic",
            EPIC_ID,
            "--review-or-kill",
            "2027-01-15",
        ],
    )
    .failure()
    .stderr(predicates::str::contains("bet_invalid_transition"));
    assert_eq!(
        bytes(&repo),
        before,
        "a rejected admit leaves the file untouched"
    );
}

#[test]
fn bet_defer_requires_a_revive_trigger_and_stamps_the_review() {
    let repo = fixtures::fixture_with_roadmap(&full_roadmap());

    bet(&repo, &["defer", "brief-v2", "--revive-when", "2027-01-15"]).success();

    let deferred = &loaded(&repo).bets[0];
    assert_eq!(deferred.status, BetStatus::Deferred);
    assert_eq!(deferred.revive_when.as_deref(), Some("2027-01-15"));
    assert_eq!(
        deferred.last_reviewed, "2026-11-18",
        "deferral is a dated decision stamped in last_reviewed"
    );
    assert_eq!(
        deferred.review_or_kill, None,
        "review_or_kill is forbidden on deferred"
    );

    // Guard: deferring a deferred bet is rejected.
    bet(&repo, &["defer", "brief-v2", "--revive-when", "2027-02-01"])
        .failure()
        .stderr(predicates::str::contains("bet_invalid_transition"));

    // Guard: deferring a shipped bet is rejected; killing is the only exit.
    let repo = fixtures::fixture_with_roadmap(&every_status_roadmap());
    bet(
        &repo,
        &["defer", "bet-shipped", "--revive-when", "2027-02-01"],
    )
    .failure()
    .stderr(predicates::str::contains("bet_invalid_transition"));
}

#[test]
fn bet_kill_is_legal_from_every_status() {
    let cases = [
        ("bet-shaping", false),
        ("bet-committed", true),
        ("bet-building", true),
        ("bet-deferred", false),
        ("bet-shipped", true),
        ("bet-killed", true),
    ];
    for (id, with_reason) in cases {
        let repo = fixtures::fixture_with_roadmap(&every_status_roadmap());
        let mut args = vec!["kill", id];
        if with_reason {
            args.extend_from_slice(&["--reason", "no evidence of value"]);
        }
        bet(&repo, &args).success();

        let file = loaded(&repo);
        let killed = file
            .bets
            .iter()
            .find(|bet| bet.id == id)
            .expect("the killed bet");
        assert_eq!(killed.status, BetStatus::Killed, "kill from {id}");
        assert_eq!(
            killed.last_reviewed, "2026-11-18",
            "kill stamps last_reviewed"
        );
        assert_eq!(
            killed.review_or_kill, None,
            "review_or_kill is forbidden on killed"
        );
        assert_eq!(
            killed.revive_when, None,
            "revive_when is forbidden on killed"
        );
        assert_eq!(
            killed.killed_reason.as_deref(),
            if with_reason {
                Some("no evidence of value")
            } else {
                None
            },
            "killed_reason is recorded exactly when --reason is given"
        );
    }
}

#[test]
fn bet_verbs_reject_unknown_ids() {
    let repo = fixtures::fixture_with_roadmap(&identity_only());
    let before = bytes(&repo);

    bet(
        &repo,
        &[
            "admit",
            "ghost",
            "--epic",
            EPIC_ID,
            "--review-or-kill",
            "2026-12-30",
        ],
    )
    .failure()
    .stderr(predicates::str::contains("bet_unknown"));
    bet(&repo, &["defer", "ghost", "--revive-when", "2027-01-15"])
        .failure()
        .stderr(predicates::str::contains("bet_unknown"));
    bet(&repo, &["kill", "ghost"])
        .failure()
        .stderr(predicates::str::contains("bet_unknown"));

    assert_eq!(bytes(&repo), before, "unknown ids never touch the file");
}

#[test]
fn bet_verbs_refuse_to_write_a_malformed_file() {
    let unknown_key = format!(
        r#"{identity}
[[bets]]
id = "brief-v2"
title = "Ship the roadmap lens"
outcome = "A cold reader predicts the next bet"
horizon = "now"
appetite_weeks = 6
owner = "charly"
status = "building"
admitted_at = "2026-11-01"
last_reviewed = "2026-11-01"
review_or_kill = "2026-12-30"
surprise = true
"#,
        identity = identity(Some("2027-05-18"))
    );
    let semantic = format!(
        r#"{identity}
[[bets]]
id = "Bad_Id"
title = ""
outcome = "n"
horizon = "now"
appetite_weeks = 0
owner = ""
status = "building"
admitted_at = "2026-11-01"
last_reviewed = "2026-11-01"
"#,
        identity = identity(Some("2027-05-18"))
    );

    // Parse error blocks the write.
    let repo = fixtures::fixture_with_roadmap(&unknown_key);
    let before = bytes(&repo);
    bet(&repo, ADD_ARGS)
        .failure()
        .stderr(predicates::str::contains("roadmap_parse_error"));
    assert_eq!(
        bytes(&repo),
        before,
        "a malformed file is never overwritten"
    );

    // Semantic violations block the write as one aggregate error.
    let repo = fixtures::fixture_with_roadmap(&semantic);
    let before = bytes(&repo);
    bet(&repo, ADD_ARGS)
        .failure()
        .stderr(predicates::str::contains("roadmap_invalid"));
    assert_eq!(bytes(&repo), before);

    // An unreadable file blocks the write.
    let repo = fixtures::fixture_with_roadmap(&identity_only());
    std::fs::write(repo.root().join(".podras/roadmap.toml"), [0xFF, 0xFE, 0x00])
        .expect("write unreadable roadmap");
    let before = std::fs::read(repo.root().join(".podras/roadmap.toml")).expect("read bytes");
    bet(&repo, ADD_ARGS)
        .failure()
        .stderr(predicates::str::contains("roadmap_unreadable"));
    assert_eq!(
        std::fs::read(repo.root().join(".podras/roadmap.toml")).expect("read bytes"),
        before
    );
}

#[test]
fn bet_add_is_deterministic_byte_for_byte() {
    let first = fixtures::fixture_with_roadmap(&identity_only());
    let second = fixtures::fixture_with_roadmap(&identity_only());
    bet(&first, ADD_ARGS).success();
    bet(&second, ADD_ARGS).success();
    assert_eq!(
        bytes(&first),
        bytes(&second),
        "two identical add runs write identical bytes"
    );

    // A whole verb sequence is deterministic too.
    let sequence: [&[&str]; 2] = [
        &[
            "admit",
            "verb-lens",
            "--epic",
            EPIC_ID,
            "--review-or-kill",
            "2026-12-30",
        ],
        &["defer", "verb-lens", "--revive-when", "2027-01-15"],
    ];
    for step in sequence {
        bet(&first, step).success();
        bet(&second, step).success();
    }
    assert_eq!(
        bytes(&first),
        bytes(&second),
        "a verb sequence writes identical bytes"
    );
}

#[test]
fn bet_as_of_controls_the_stamped_dates() {
    let repo = fixtures::fixture_with_roadmap(&identity_only());
    podras_in(
        &repo,
        &[
            "bet",
            "add",
            "--id",
            "verb-lens",
            "--title",
            "Try the bet verbs",
            "--outcome",
            "A human curates bets from the CLI alone",
            "--horizon",
            "later",
            "--appetite-weeks",
            "3",
            "--owner",
            "sasha",
            "--as-of",
            "2026-03-01",
        ],
    )
    .success();

    let added = &loaded(&repo).bets[0];
    assert_eq!(
        added.last_reviewed, "2026-03-01",
        "--as-of is the decision date add stamps"
    );

    podras_in(
        &repo,
        &[
            "bet",
            "admit",
            "verb-lens",
            "--epic",
            EPIC_ID,
            "--review-or-kill",
            "2026-12-30",
            "--as-of",
            "2026-04-15",
        ],
    )
    .success();
    let admitted = &loaded(&repo).bets[0];
    assert_eq!(admitted.admitted_at.as_deref(), Some("2026-04-15"));
    assert_eq!(admitted.last_reviewed, "2026-04-15");

    podras_in(
        &repo,
        &[
            "bet",
            "defer",
            "verb-lens",
            "--revive-when",
            "2027-01-15",
            "--as-of",
            "2026-05-20",
        ],
    )
    .success();
    let deferred = &loaded(&repo).bets[0];
    assert_eq!(deferred.last_reviewed, "2026-05-20");

    podras_in(
        &repo,
        &["bet", "kill", "verb-lens", "--as-of", "2026-06-30"],
    )
    .success();
    let killed = &loaded(&repo).bets[0];
    assert_eq!(killed.last_reviewed, "2026-06-30");
}

#[test]
fn bet_verbs_require_a_scaffolded_roadmap_file() {
    let repo = fixtures::fixture_with_roadmap(&identity_only());
    std::fs::remove_file(repo.root().join(".podras/roadmap.toml"))
        .expect("remove the scaffolded file");

    bet(&repo, ADD_ARGS)
        .failure()
        .stderr(predicates::str::contains("roadmap_missing"));
    bet(&repo, &["kill", "ghost"])
        .failure()
        .stderr(predicates::str::contains("roadmap_missing"));
    assert!(
        !repo.root().join(".podras/roadmap.toml").exists(),
        "no verb scaffolds the roadmap file"
    );
}

#[test]
fn bet_verbs_exit_zero_on_success_and_one_on_domain_error() {
    let repo = fixtures::fixture_with_roadmap(&identity_only());

    let ok = bet(&repo, ADD_ARGS).success().get_output().status.code();
    assert_eq!(ok, Some(0), "success exits 0");

    let admitted = bet(
        &repo,
        &[
            "admit",
            "verb-lens",
            "--epic",
            EPIC_ID,
            "--review-or-kill",
            "2026-12-30",
        ],
    )
    .success()
    .get_output()
    .status
    .code();
    assert_eq!(admitted, Some(0), "a legal transition exits 0");

    let rejected = {
        // committed -> deferred is legal; a second defer is the guard hit.
        let _ = bet(
            &repo,
            &["defer", "verb-lens", "--revive-when", "2027-01-15"],
        )
        .success()
        .get_output()
        .status
        .code();
        bet(
            &repo,
            &["defer", "verb-lens", "--revive-when", "2027-02-01"],
        )
        .failure()
        .get_output()
        .status
        .code()
    };
    assert_eq!(rejected, Some(1), "a domain error exits 1");

    // A missing required flag is a clap usage error, not a domain error.
    let usage = podras_in(
        &repo,
        &["bet", "add", "--id", "verb-lens", "--as-of", "2026-11-18"],
    )
    .failure()
    .get_output()
    .status
    .code();
    assert_eq!(usage, Some(2), "missing --title is a clap usage error");
}
