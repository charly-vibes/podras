//! Purpose: run the three roadmap-lens validation experiments from `docs/design.md`.
//!
//! Responsibilities:
//! - determinism: two runs over the same file under `--as-of` are semantically
//!   identical, including which five bets survived the bound;
//! - cold return: a stranger reading only the lens can name the current bet and
//!   the next one, and sees the binding boundaries;
//! - no-go discipline: a candidate matching a no-go's terms is flagged with
//!   provenance, near-tokens stay silent, and no file or tracker byte changes.
//!
//! Rationale: these three gates are the design's acceptance criteria for stage
//! ⑥. They exercise the real binary over a committed fixture, because the
//! property under test is what a reader and a re-run actually observe.

use std::fs;

use podras::model::{Brief, semantic_payload};
use serde_json::Value;

use crate::common::TempRepo;
use crate::fixtures::{section, stdout};
use crate::roadmap_fixtures::{
    AS_OF, BeadRow, BetSpec, CURATION, EPIC_ID, array_ids, identity, no_go, roadmap_file,
    seed_epic_states, seven_bets_roadmap,
};

/// Run `roadmap --json` and return its data payload.
fn payload(repo: &TempRepo) -> Value {
    let raw = stdout(repo, &["roadmap", "--json", "--as-of", AS_OF]);
    serde_json::from_str::<Value>(&raw).expect("roadmap --json emits an envelope")["data"].clone()
}

/// Drop the volatile generation stamps so two runs are byte-comparable.
fn stabilized(mut data: Value) -> Value {
    if let Some(object) = data.as_object_mut() {
        object.remove("generated_at");
        object.remove("source_hash");
    }
    data
}

/// The id of a bet bullet, taken from its `- <id> — <title>` prefix.
fn bet_id(line: &str) -> Option<&str> {
    line.strip_prefix("- ")?.split(" — ").next()
}

/// A `## ` section body with its blank edges removed.
fn body<'a>(markdown: &'a str, heading: &str) -> &'a str {
    section(markdown, heading).trim_matches('\n')
}

/// A seven-bet fixture whose rank/admission ordering the bound must preserve.
fn determinism_repo() -> TempRepo {
    let repo = TempRepo::new();
    seed_epic_states(&repo);
    repo.curation(CURATION);
    repo.roadmap(&seven_bets_roadmap());
    repo.commit("roadmap determinism fixture");
    repo
}

/// A lens with one bet in each horizon and one binding no-go.
fn cold_return_repo() -> TempRepo {
    let repo = TempRepo::new();
    repo.beads_jsonl_rows(&[
        BeadRow {
            id: EPIC_ID,
            title: "Roadmap lens epic",
            status: "open",
            labels: &[],
            updated_at: "2026-11-17T00:00:00Z",
        },
        BeadRow {
            id: "podras-epic-verbs",
            title: "Curation verbs epic",
            status: "open",
            labels: &[],
            updated_at: "2026-11-17T00:00:00Z",
        },
    ]);
    repo.curation(CURATION);
    let bets = [
        BetSpec::new("brief-v2", "Ship the roadmap lens", "building", "now")
            .with_outcome("A cold reader predicts the next bet from the lens alone")
            .with_epic(EPIC_ID)
            .with_review_or_kill("2026-12-30"),
        BetSpec::new(
            "curation-verbs",
            "Add the curation verbs",
            "committed",
            "next",
        )
        .with_outcome("Admission, deferral and kill are dated verbs")
        .with_epic("podras-epic-verbs")
        .with_review_or_kill("2026-12-30")
        .admitted("2026-11-05", "2026-11-05"),
        BetSpec::new(
            "harvest-extension",
            "Extend harvest to the lens",
            "shaping",
            "later",
        )
        .admitted("2026-11-05", "2026-11-05"),
    ];
    let rendered: Vec<String> = bets.iter().map(|bet| bet.render()).collect();
    repo.roadmap(&roadmap_file(
        &identity(Some("2027-05-18")),
        &rendered,
        &[no_go(
            "no-tracker-writes",
            "podras never mutates the tracker",
            "all verbs",
            "automated",
            "a new design decision",
            &["auto-close"],
        )],
    ));
    repo.commit("cold return fixture");
    repo
}

#[test]
fn roadmap_determinism() {
    let repo = determinism_repo();

    let first = payload(&repo);
    let second = payload(&repo);

    assert_eq!(first["as_of"], AS_OF, "the pinned date is reported");
    assert_eq!(
        first["roadmap"]["identity"]["staleness"], "fresh",
        "the pinned date decides staleness, not the wall clock"
    );

    let parse = |value: &Value| {
        serde_json::from_value::<Brief>(value.clone()).expect("the payload is a typed brief")
    };
    assert_eq!(
        semantic_payload(&parse(&first)),
        semantic_payload(&parse(&second)),
        "two runs over identical inputs are semantically identical"
    );
    assert_eq!(
        stabilized(first.clone()),
        stabilized(second.clone()),
        "the only difference between two runs is generated_at"
    );

    // The bound keeps the same five bets in the same order on both runs.
    let survivors = array_ids(&first["roadmap"]["bets"]);
    assert_eq!(
        survivors,
        vec!["bet-build", "bet-commit", "bet-defer", "shape-a", "shape-b"],
        "rank, then admission, then id decides which five survive"
    );
    assert_eq!(survivors, array_ids(&second["roadmap"]["bets"]));
    assert_eq!(
        first["roadmap"]["truncation"]["omitted_counts"]["bets"], 2,
        "the two dropped bets are recorded, not silently lost"
    );

    // A written pair is reproducible too: markdown is byte-identical and the
    // JSON payload is identical once the generation stamp is dropped.
    let first_dir = tempfile::tempdir().expect("first render directory");
    let second_dir = tempfile::tempdir().expect("second render directory");
    for dir in [&first_dir, &second_dir] {
        stdout(
            &repo,
            &[
                "roadmap",
                "--write",
                "--out-dir",
                dir.path().to_str().expect("UTF-8 render path"),
                "--as-of",
                AS_OF,
            ],
        );
    }
    let markdown_first = fs::read(first_dir.path().join("ROADMAP.md")).expect("first ROADMAP.md");
    let markdown_second =
        fs::read(second_dir.path().join("ROADMAP.md")).expect("second ROADMAP.md");
    assert_eq!(
        markdown_first, markdown_second,
        "ROADMAP.md carries no volatile stamp and must be byte-identical"
    );
    let json_first: Value = serde_json::from_slice(
        &fs::read(first_dir.path().join("ROADMAP.json")).expect("first ROADMAP.json"),
    )
    .expect("ROADMAP.json is an envelope");
    let json_second: Value = serde_json::from_slice(
        &fs::read(second_dir.path().join("ROADMAP.json")).expect("second ROADMAP.json"),
    )
    .expect("ROADMAP.json is an envelope");
    assert_eq!(
        stabilized(json_first["data"].clone()),
        stabilized(json_second["data"].clone()),
        "the written envelope is reproducible under the pinned date"
    );
}

#[test]
fn roadmap_cold_return() {
    let repo = cold_return_repo();
    let lens = stdout(&repo, &["roadmap", "--as-of", AS_OF]);

    // A stranger has only this render: no brief, no tracker, no repository.
    let now = body(&lens, "Now");
    let next = body(&lens, "Next");
    let later = body(&lens, "Later");

    assert_eq!(
        now.lines().next().and_then(bet_id),
        Some("brief-v2"),
        "the current bet is the first `## Now` bullet: {now}"
    );
    assert_eq!(
        next.lines().next().and_then(bet_id),
        Some("curation-verbs"),
        "the next bet is the first `## Next` bullet: {next}"
    );
    assert_eq!(
        later.lines().next().and_then(bet_id),
        Some("harvest-extension"),
        "the later bet is labelled later, never merged into now: {later}"
    );
    assert!(
        !now.contains("curation-verbs") && !now.contains("harvest-extension"),
        "a wrong horizon would put a future bet under Now: {lens}"
    );
    assert!(
        !next.contains("harvest-extension"),
        "a later bet must not read as the next bet: {next}"
    );

    // The prediction is checkable: status, horizon, owner, appetite, and the
    // review-or-kill date all render.
    assert!(now.contains(
        "(status: building; horizon: now; appetite: 6w; owner: charly; epic: podras-epic)"
    ));
    assert!(now.contains("review or kill: 2026-12-30"));
    assert!(next.contains("(status: committed; horizon: next;"));

    // What the project ought to be, and how stale that statement is.
    assert!(body(&lens, "Identity").contains("- path: README.md#why"));
    assert!(body(&lens, "Identity").contains("- staleness: fresh"));

    // A missing boundary is a fail: the no-go is visible without any inference.
    assert!(
        body(&lens, "Boundaries (no-gos)").contains("- no-tracker-writes — "),
        "the binding boundary renders in the lens: {lens}"
    );
    assert!(
        body(&lens, "Boundaries (no-gos)").contains("enforcement: automated"),
        "the declared enforcement tier is visible, not inferred"
    );
}

#[test]
fn roadmap_no_go_discipline() {
    let repo = TempRepo::new();
    repo.beads_jsonl_rows(&[
        BeadRow {
            id: "podras-close",
            title: "Ship auto-close on merge",
            status: "open",
            labels: &[],
            updated_at: "2026-11-17T00:00:00Z",
        },
        BeadRow {
            id: "podras-autoclose",
            title: "Support autoclose in the CLI",
            status: "open",
            labels: &[],
            updated_at: "2026-11-17T00:00:00Z",
        },
    ]);
    repo.roadmap(&roadmap_file(
        &identity(Some("2027-05-18")),
        &[],
        &[no_go(
            "no-tracker-writes",
            "podras never mutates the tracker",
            "all verbs",
            "automated",
            "a new design decision",
            &["auto-close"],
        )],
    ));
    repo.commit("no-go discipline fixture");

    let before = repo.tree_snapshot();

    let envelope: Value =
        serde_json::from_str(&stdout(&repo, &["roadmap", "--json", "--as-of", AS_OF]))
            .expect("roadmap --json emits an envelope");
    assert_eq!(envelope["ok"], true, "an advisory never fails the verb");

    let warnings = envelope["data"]["roadmap"]["warnings"]
        .as_array()
        .expect("roadmap payload carries warnings");
    assert_eq!(
        warnings.len(),
        1,
        "exactly the matching open bead is a candidate: {}",
        envelope["data"]
    );
    let candidate = &warnings[0];
    assert_eq!(candidate["code"], "no_go_violation_candidate");

    // Provenance: both the boundary and the offending bead are named, and the
    // matched term is quoted so a human can audit the hit.
    let provenance = format!(
        "{} {}",
        candidate["subject"].as_str().unwrap_or_default(),
        candidate["detail"].as_str().unwrap_or_default()
    );
    for expected in ["no-tracker-writes", "podras-close", "auto-close"] {
        assert!(
            provenance.contains(expected),
            "the candidate names `{expected}`: {provenance}"
        );
    }
    assert!(
        !provenance.contains("podras-autoclose"),
        "`autoclose` is a different token from `auto-close`: {provenance}"
    );

    // The narrative marks the same candidate and nothing else changed.
    let lens = stdout(&repo, &["roadmap", "--as-of", AS_OF]);
    let rendered = body(&lens, "Warnings");
    let line = rendered
        .lines()
        .next()
        .expect("the candidate renders one line");
    assert!(
        line.starts_with("- no_go_violation_candidate — no-tracker-writes — "),
        "the narrative names the code and the boundary: {rendered}"
    );
    assert!(
        line.contains("podras-close"),
        "the narrative names the bead: {rendered}"
    );
    assert_eq!(
        rendered.lines().count(),
        1,
        "the near-token bead produces no second flag: {rendered}"
    );

    assert_eq!(
        repo.tree_snapshot(),
        before,
        "flagging a candidate writes nothing: no state file, no render, no tracker row"
    );
    assert_eq!(
        repo.read(".podras/roadmap.toml"),
        roadmap_file(
            &identity(Some("2027-05-18")),
            &[],
            &[no_go(
                "no-tracker-writes",
                "podras never mutates the tracker",
                "all verbs",
                "automated",
                "a new design decision",
                &["auto-close"],
            )],
        ),
        "the human-owned state file is never rewritten by a read"
    );
    assert!(
        repo.read(".beads/issues.jsonl")
            .contains("\"Ship auto-close on merge\""),
        "the tracker export is untouched, including its labels and status"
    );
}
