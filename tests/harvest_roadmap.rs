//! Purpose: prove harvest proposes roadmap candidates without deciding anything.
//!
//! Responsibilities:
//! - `identity_review` fires only when `identity.next_review` is strictly
//!   earlier than the evaluation date; absent and fresh schedules emit nothing;
//! - `retire_or_reshape` fires once per non-terminal bet that trips the
//!   circuit breaker, an idle epic, or a closed epic, naming the bet id and
//!   every triggering reason; shipped and killed bets are exempt;
//! - a run writes nothing and two runs at one `--as-of` emit the same ordered
//!   candidate set;
//! - a malformed roadmap degrades harvest to zero roadmap candidates at exit 0.
//!
//! Rationale: harvest is derive-only, so every scenario here is a negative
//! claim about the roadmap plus one about determinism. The candidate kinds are
//! advisory prompts for a human, never decisions, which is why the terminal
//! exemptions and the write-nothing property are tested next to each trigger.

mod common;

use common::{BeadRow, TempRepo};

use std::collections::BTreeMap;

use serde_json::Value;

/// Pinned evaluation date for every scenario in this file.
const AS_OF: &str = "2026-11-18";

/// Open epic, recently updated: triggers nothing on its own.
const EPIC_ID: &str = "podras-epic";
/// Epic closed while its bet is still non-terminal.
const CLOSED_EPIC_ID: &str = "podras-epic-closed";
/// Epic untouched for longer than the fourteen-day horizon.
const IDLE_EPIC_ID: &str = "podras-epic-idle";

/// The `data.candidates` array of a `harvest --json` envelope.
fn candidates_of(envelope: &Value) -> Vec<Value> {
    envelope["data"]["candidates"]
        .as_array()
        .expect("harvest envelope carries a candidates array")
        .clone()
}

/// Every candidate of one kind, in payload order.
fn kind_candidates<'a>(candidates: &'a [Value], kind: &str) -> Vec<&'a Value> {
    candidates
        .iter()
        .filter(|candidate| candidate["kind"].as_str() == Some(kind))
        .collect()
}

/// Assert one candidate carries a non-empty summary and roadmap evidence
/// whose ref resolves to the roadmap state file.
fn assert_roadmap_candidate(candidate: &Value) {
    assert!(
        candidate["summary"]
            .as_str()
            .is_some_and(|summary| !summary.is_empty()),
        "candidate carries a deterministic summary: {candidate}"
    );
    let evidence = candidate["evidence"]
        .as_array()
        .expect("candidate carries an evidence array");
    assert!(!evidence.is_empty(), "evidence-free candidates are dropped");
    for item in evidence {
        assert_eq!(item["kind"].as_str(), Some("roadmap"), "{item}");
        assert_eq!(item["ref"].as_str(), Some(".podras/roadmap.toml"), "{item}");
    }
}

/// Assert the run decided nothing: state files keep their exact bytes.
fn assert_no_bytes_changed(before: &BTreeMap<String, Vec<u8>>, repo: &TempRepo) {
    let after = repo.tree_snapshot();
    assert_eq!(
        before, &after,
        "a harvest run wrote nothing; a byte change is a decision"
    );
}

fn run_harvest(repo: &TempRepo) -> Value {
    let output = assert_cmd::Command::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(["harvest", "--json", "--as-of", AS_OF])
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8(output).expect("harvest envelope is UTF-8");
    serde_json::from_str(stdout.trim()).expect("harvest --json emits a JSON envelope")
}

/// Fixture with one open, one closed, and one idle epic on the tracker.
fn repo_with_epics() -> TempRepo {
    let repo = TempRepo::new();
    repo.beads_jsonl_rows(&[
        BeadRow {
            id: EPIC_ID,
            title: "Open epic",
            status: "open",
            labels: &[],
            updated_at: "2026-11-15T00:00:00Z",
        },
        BeadRow {
            id: CLOSED_EPIC_ID,
            title: "Closed epic",
            status: "closed",
            labels: &[],
            updated_at: "2026-11-15T00:00:00Z",
        },
        BeadRow {
            id: IDLE_EPIC_ID,
            title: "Idle epic",
            status: "open",
            labels: &[],
            updated_at: "2026-10-01T00:00:00Z",
        },
    ]);
    repo.curation("purpose_lite = \"harvest roadmap fixture\"\npublish_whisper = false\n");
    repo
}

/// One `[[bets]]` table with the common fixture shape.
fn bet(id: &str, status: &str, horizon: &str, epic: Option<&str>, admitted_at: &str) -> String {
    let mut out = format!(
        "[[bets]]\nid = \"{id}\"\ntitle = \"Do the thing\"\noutcome = \"The thing lands\"\n\
         horizon = \"{horizon}\"\nappetite_weeks = 6\nowner = \"charly\"\n"
    );
    if let Some(epic) = epic {
        out.push_str(&format!("epic = \"{epic}\"\n"));
    }
    out.push_str(&format!(
        "status = \"{status}\"\nadmitted_at = \"{admitted_at}\"\nlast_reviewed = \"{admitted_at}\"\n"
    ));
    if matches!(status, "committed" | "building") {
        out.push_str("review_or_kill = \"2026-12-30\"\n");
    }
    if status == "deferred" {
        out.push_str("revive_when = \"when demand returns\"\n");
    }
    if status == "killed" {
        out.push_str("killed_reason = \"no value-prop evidence\"\n");
    }
    out.push('\n');
    out
}

/// An identity table with the given optional `next_review`.
fn identity(next_review: Option<&str>) -> String {
    let mut out =
        String::from("[identity]\npath = \"README.md#why\"\nlast_reviewed = \"2026-11-01\"\n");
    if let Some(next_review) = next_review {
        out.push_str(&format!("next_review = \"{next_review}\"\n"));
    }
    out.push('\n');
    out
}

/// Write a roadmap document into the fixture repository.
fn with_roadmap(repo: &TempRepo, identity: &str, bets: &[String]) {
    let document = format!("{identity}{}", bets.join(""));
    repo.roadmap(&document);
    repo.commit("roadmap fixture");
}

#[test]
fn harvest_runs_on_fixture_repo_with_passed_next_review() {
    let repo = repo_with_epics();
    with_roadmap(&repo, &identity(Some("2026-11-01")), &[]);
    let envelope = run_harvest(&repo);
    let candidates = candidates_of(&envelope);
    let identity_candidates = kind_candidates(&candidates, "identity_review");
    assert_eq!(
        identity_candidates.len(),
        1,
        "a passed next_review emits exactly one identity_review candidate: {envelope}"
    );
    assert_roadmap_candidate(identity_candidates[0]);
    assert!(
        identity_candidates[0]["summary"]
            .as_str()
            .unwrap()
            .contains("2026-11-01"),
        "the summary names the passed date"
    );
    assert_eq!(
        candidates
            .last()
            .and_then(|candidate| candidate["kind"].as_str()),
        Some("identity_review"),
        "the identity candidate is deterministic and ordered last: {candidates:?}"
    );
}

#[test]
fn harvest_runs_on_fixtures_with_absent_and_future_next_review() {
    for next_review in [None, Some("2027-05-18")] {
        let repo = repo_with_epics();
        with_roadmap(&repo, &identity(next_review), &[]);
        let candidates = candidates_of(&run_harvest(&repo));
        assert!(
            kind_candidates(&candidates, "identity_review").is_empty(),
            "next_review {next_review:?} emits no identity_review candidate: {candidates:?}"
        );
    }
}

#[test]
fn harvest_runs_on_fixture_repo_with_each_retire_or_reshape_trigger() {
    let repo = repo_with_epics();
    let bets = [
        // Circuit breaker: admitted 2026-09-01 (78 days) against a 2-week
        // appetite (breaker at 17.5 days).
        bet(
            "bet-breaker",
            "committed",
            "next",
            Some(EPIC_ID),
            "2026-09-01",
        )
        .replace("appetite_weeks = 6", "appetite_weeks = 2"),
        // Idle epic: last transition 2026-10-01, well past fourteen days.
        bet("bet-idle", "building", "now", Some(IDLE_EPIC_ID), "2026-11-10"),
        // Closed epic under a non-terminal bet.
        bet(
            "bet-closed",
            "shaping",
            "later",
            Some(CLOSED_EPIC_ID),
            "2026-11-10",
        ),
    ];
    with_roadmap(&repo, &identity(Some("2027-05-18")), &bets);
    let candidates = candidates_of(&run_harvest(&repo));
    let retire = kind_candidates(&candidates, "retire_or_reshape");
    assert_eq!(retire.len(), 3, "each trigger fires once: {candidates:?}");

    let summaries: Vec<&str> = retire
        .iter()
        .map(|candidate| candidate["summary"].as_str().expect("summary"))
        .collect();
    assert_roadmap_candidate(retire[0]);
    assert!(
        summaries[0].contains("bet-idle") && summaries[0].contains("bet_no_transition"),
        "the summary names the bet id and the reason"
    );
    assert!(
        summaries[1].contains("bet-breaker") && summaries[1].contains("circuit_breaker_due"),
        "an idle epic fires bet_no_transition",
    );
    assert!(
        summaries[2].contains("bet-closed") && summaries[2].contains("bet_epic_closed_not_retired"),
        "a closed epic under a non-terminal bet fires",
    );

    // Deterministic order: lens status rank (building, committed, deferred,
    // shaping), then admitted_at, then id — the file order above defeats it.
    let ids: Vec<&str> = summaries
        .iter()
        .map(|summary| {
            summary
                .split('`')
                .nth(1)
                .expect("summary names the bet id")
        })
        .collect();
    assert_eq!(
        ids,
        vec!["bet-idle", "bet-breaker", "bet-closed"],
        "candidates order by status rank, admitted_at, then id: {summaries:?}"
    );

    for candidate in &retire {
        assert_roadmap_candidate(candidate);
    }
}

#[test]
fn harvest_runs_on_fixture_repo_with_terminal_bets_exempt() {
    let repo = repo_with_epics();
    let bets = [
        // Both terminal and both tripping every predicate shape: breaker-age
        // admission, closed epic, idle epic.
        bet(
            "bet-shipped",
            "shipped",
            "later",
            Some(CLOSED_EPIC_ID),
            "2026-09-01",
        )
        .replace("appetite_weeks = 6", "appetite_weeks = 2"),
        bet(
            "bet-killed",
            "killed",
            "later",
            Some(IDLE_EPIC_ID),
            "2026-09-01",
        )
        .replace("appetite_weeks = 6", "appetite_weeks = 2"),
    ];
    with_roadmap(&repo, &identity(Some("2027-05-18")), &bets);
    let candidates = candidates_of(&run_harvest(&repo));
    assert!(
        kind_candidates(&candidates, "retire_or_reshape").is_empty(),
        "shipped and killed bets never emit retire_or_reshape: {candidates:?}"
    );
}

#[test]
fn harvest_roadmap_run_writes_nothing() {
    let repo = repo_with_epics();
    let bets = [
        bet(
            "bet-breaker",
            "committed",
            "next",
            Some(EPIC_ID),
            "2026-09-01",
        )
        .replace("appetite_weeks = 6", "appetite_weeks = 2"),
        bet("bet-closed", "shaping", "later", Some(CLOSED_EPIC_ID), "2026-11-10"),
    ];
    with_roadmap(&repo, &identity(Some("2026-11-01")), &bets);
    let before = repo.tree_snapshot();
    run_harvest(&repo);
    assert_no_bytes_changed(&before, &repo);
    for path in [
        ".podras/roadmap.toml",
        ".podras/curation.toml",
        ".beads/issues.jsonl",
    ] {
        assert!(
            before.contains_key(path),
            "the fixture carries {path} so the comparison is meaningful"
        );
    }
}

#[test]
fn harvest_roadmap_run_twice_on_fixture_repo() {
    let repo = repo_with_epics();
    let bets = [
        bet(
            "bet-breaker",
            "committed",
            "next",
            Some(EPIC_ID),
            "2026-09-01",
        )
        .replace("appetite_weeks = 6", "appetite_weeks = 2"),
        bet("bet-idle", "deferred", "next", Some(IDLE_EPIC_ID), "2026-11-08"),
        bet("bet-closed", "shaping", "later", Some(CLOSED_EPIC_ID), "2026-11-10"),
    ];
    with_roadmap(&repo, &identity(Some("2026-11-01")), &bets);
    let first = run_harvest(&repo);
    let second = run_harvest(&repo);
    let strip = |envelope: &Value| {
        let mut payload = envelope.clone();
        payload["data"]["generated_at"] = serde_json::Value::Null;
        payload
    };
    assert_eq!(
        strip(&first), strip(&second),
        "two runs at one --as-of emit the same candidate set in the same order"
    );
    let candidates = candidates_of(&first);
    let kinds: Vec<&str> = candidates
        .iter()
        .map(|candidate| candidate["kind"].as_str().expect("kind"))
        .collect();
    assert!(
        kinds.contains(&"identity_review") && kinds.contains(&"retire_or_reshape"),
        "the fixture yields both roadmap kinds: {kinds:?}"
    );
    assert!(
        kinds.last().copied() == Some("identity_review"),
        "the identity candidate is ordered last: {kinds:?}"
    );
    let first_roadmap = kinds
        .iter()
        .position(|kind| matches!(*kind, "retire_or_reshape" | "identity_review"))
        .expect("roadmap kinds present");
    let last_retire = kinds
        .iter()
        .rposition(|kind| *kind == "retire_or_reshape")
        .expect("retire candidates present");
    assert!(
        kinds[first_roadmap..=last_retire]
            .iter()
            .all(|kind| *kind == "retire_or_reshape"),
        "retire_or_reshape candidates are contiguous before the identity candidate: {kinds:?}"
    );
    for candidate in &candidates {
        assert_roadmap_candidate(candidate);
    }
}

#[test]
fn harvest_runs_on_fixture_repo_with_malformed_roadmap() {
    let repo = repo_with_epics();
    repo.roadmap("[identity]\npath = 6\n");
    repo.commit("malformed roadmap fixture");
    let before = repo.tree_snapshot();
    let envelope = run_harvest(&repo);
    assert_eq!(envelope["ok"].as_bool(), Some(true), "{envelope}");
    let candidates = candidates_of(&envelope);
    assert!(
        kind_candidates(&candidates, "identity_review").is_empty()
            && kind_candidates(&candidates, "retire_or_reshape").is_empty(),
        "a malformed roadmap degrades to zero roadmap candidates: {candidates:?}"
    );
    assert_no_bytes_changed(&before, &repo);
}
