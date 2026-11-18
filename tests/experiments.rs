//! Purpose: run the three validation experiments that gate a "done" claim for brief v1.
//!
//! Responsibilities:
//! - amnesia: `BRIEF.md` alone answers the cold-return questions;
//! - agent cold-start: `init` plus a schema-valid `brief --json` from a fresh process;
//! - staleness faults: closed focus, deleted tracker, edited spec;
//! - keep the committed and on-demand semantic payloads identical.
//!
//! Rationale: these experiments are the acceptance gate in `docs/design.md`, so
//! they exercise the real binary and real fixtures rather than library internals.

mod common;

use assert_cmd::Command;
use serde_json::Value;

use common::TempRepo;

/// Pinned evaluation date for every experiment.
const AS_OF: &str = "2026-11-18";
/// Human-curated focus.
const FOCUS_ID: &str = "podras-a";
/// Open issue blocked by the focus.
const BLOCKED_ID: &str = "podras-b";
/// Ready issue that belongs in next.
const NEXT_ID: &str = "podras-c";
/// Closed issue that must never appear as ready.
const CLOSED_ID: &str = "podras-d";
/// Curated parked item whose trigger is in the future.
const PARKED_ID: &str = "podras-f";
/// Parked item's resurface date.
const PARKED_TRIGGER: &str = "2027-01-01";
/// Private whisper content that must not reach machine output.
const WHISPER_SECRET: &str = "private-next-action-launch-code";
/// Openspec proposal used by the staleness experiment.
const SPEC_PATH: &str = "openspec/changes/brief-read-path/proposal.md";

/// Run the real binary inside a fixture repository.
fn podras_in(repo: &TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(args)
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .assert()
}

/// Run the binary and return stdout as text.
fn stdout(repo: &TempRepo, args: &[&str]) -> String {
    let output = podras_in(repo, args).success().get_output().stdout.clone();
    String::from_utf8(output).expect("podras output is UTF-8")
}

/// Run `brief --json` and return the genesis envelope.
fn brief_envelope(repo: &TempRepo) -> Value {
    serde_json::from_str(stdout(repo, &["brief", "--json", "--as-of", AS_OF]).trim())
        .expect("brief --json emits a JSON envelope")
}

/// Run `brief --json` and return its typed payload.
fn brief_data(repo: &TempRepo) -> Value {
    brief_envelope(repo)["data"].clone()
}

/// Find one declared source row in a brief payload.
fn source<'a>(data: &'a Value, name: &str) -> &'a Value {
    data["sources"]
        .as_array()
        .expect("sources is an array")
        .iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("source `{name}` is declared: {data}"))
}

/// Return one `## ` section body from the narrative render.
fn section<'a>(markdown: &'a str, heading: &str) -> &'a str {
    let marker = format!("## {heading}\n");
    let start = markdown
        .find(&marker)
        .unwrap_or_else(|| panic!("render has section `{heading}`: {markdown}"))
        + marker.len();
    let rest = &markdown[start..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    &rest[..end]
}

/// Curation that selects a focus, parks an item, and leaves health unknown.
const RICH_CURATION: &str = r#"purpose_lite = "podras: project status for a cold return"
max_focus = 1
publish_whisper = false

[health]
status = "unknown"
owner = "@sasha"
confirmed_at = "2026-11-15"

[[focus]]
id = "podras-a"
reason = "finish the brief read path"
focused_since = "2026-11-10"
confirmed_at = "2026-11-15"

[[parked]]
id = "podras-f"
reason = "harvest moves to v1.1"
resurface = "2027-01-01"
parked_at = "2026-11-10"
confirmed_at = "2026-11-10"
"#;

/// A realistic repository: blocked/ready/closed issues, curation, specs, notes, commits.
fn rich_fixture() -> TempRepo {
    let repo = TempRepo::new();
    repo.beads_jsonl_at(
        &[
            (FOCUS_ID, "Brief read path", "open"),
            (BLOCKED_ID, "Envelope render", "open"),
            (NEXT_ID, "Agent cold start", "open"),
            (CLOSED_ID, "Old scaffold", "closed"),
            ("podras-e", "Housekeeping", "open"),
            (PARKED_ID, "Harvest v1.1", "open"),
        ],
        &[(BLOCKED_ID, FOCUS_ID)],
        "2026-11-17T00:00:00Z",
    );
    repo.curation(RICH_CURATION);
    repo.set_mtime(".podras/curation.toml", "2026-11-16T00:00:00Z");
    repo.spec_change("brief-read-path", "# Read path for podras-a\n");
    repo.set_mtime(SPEC_PATH, "2026-11-16T00:00:00Z");
    repo.whisper_branch_action("main", "private session narrative", WHISPER_SECRET);
    repo.commit_at("2026-11-17T00:00:00Z", "podras-a: brief read path");
    repo
}

/// Assert the narrative carries the nine cold-return sections in order.
fn assert_section_order(markdown: &str) {
    let headings: Vec<&str> = markdown
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .collect();
    assert_eq!(
        headings,
        vec![
            "Purpose / current state",
            "Focus",
            "In flight",
            "Blocked / waiting",
            "Next / ready",
            "Parked",
            "Recent",
            "Health + confidence",
            "Sources & freshness",
        ],
        "BRIEF.md carries the nine cold-return sections in order: {markdown}"
    );
}

/// Assert every declared source has an observed_at, staleness, and freshness stamp.
fn assert_sources_table(markdown: &str) {
    let sources = section(markdown, "Sources & freshness");
    for name in ["beads", "git", "openspec", "whisper", "curation"] {
        let row = sources
            .lines()
            .find(|line| line.starts_with(&format!("| {name} |")))
            .unwrap_or_else(|| panic!("sources table has a `{name}` row: {sources}"));
        let cells: Vec<&str> = row.split('|').map(str::trim).collect();
        assert_ne!(cells[3], "none", "`{name}` has an observed_at: {row}");
        assert!(
            matches!(cells[4], "fresh" | "stale" | "unknown"),
            "`{name}` has a per-source staleness: {row}"
        );
    }
    assert!(
        markdown.contains("Freshness: generated_at ")
            && markdown.contains("per-source observed_at and staleness are listed above."),
        "the committed render carries a per-source freshness stamp: {markdown}"
    );
}

#[test]
fn brief_md_answers_cold_return_questions() {
    let repo = rich_fixture();
    podras_in(&repo, &["brief", "--write", "--as-of", AS_OF]).success();

    let markdown = repo.read("BRIEF.md");
    assert_section_order(&markdown);

    let purpose = section(&markdown, "Purpose / current state");
    assert!(
        purpose.contains("podras: project status for a cold return"),
        "purpose answers what this repository is: {purpose}"
    );

    let focus = section(&markdown, "Focus");
    assert!(focus.contains(FOCUS_ID), "focus id is named: {focus}");

    let blocked = section(&markdown, "Blocked / waiting");
    assert!(
        blocked.contains(BLOCKED_ID),
        "blocked id is named: {blocked}"
    );
    assert!(
        blocked.contains(&format!("blocked by: {FOCUS_ID}")),
        "blocked item names its blocker: {blocked}"
    );

    let next = section(&markdown, "Next / ready");
    assert!(next.contains(NEXT_ID), "next id is named: {next}");
    assert!(
        !next.contains(FOCUS_ID) && !next.contains(PARKED_ID) && !next.contains(CLOSED_ID),
        "next excludes focus, parked, and closed items: {next}"
    );

    let parked = section(&markdown, "Parked");
    assert!(parked.contains(PARKED_ID), "parked id is named: {parked}");
    assert!(
        parked.contains(&format!("resurface: {PARKED_TRIGGER}")),
        "parked item names its resurface trigger: {parked}"
    );

    let health = section(&markdown, "Health + confidence");
    assert!(
        health.contains("Health: **unknown**"),
        "unknown curation health is never rendered green: {health}"
    );
    assert!(
        health.contains("basis: none") && health.contains("reason:"),
        "health verdict carries basis and reason: {health}"
    );

    let sources = section(&markdown, "Sources & freshness");
    assert!(sources.contains("| source | status | observed_at | staleness | detail |"));
    assert_sources_table(&markdown);
}
#[test]
fn committed_json_matches_on_demand_payload() {
    let repo = rich_fixture();
    podras_in(&repo, &["brief", "--write", "--as-of", AS_OF]).success();

    let committed: Value =
        serde_json::from_str(repo.read("BRIEF.json").trim()).expect("BRIEF.json is JSON");
    let on_demand = brief_data(&repo);

    let mut committed_data = committed["data"].clone();
    let mut on_demand_data = on_demand;
    for payload in [&mut committed_data, &mut on_demand_data] {
        payload
            .as_object_mut()
            .expect("brief payload is an object")
            .remove("generated_at");
    }

    assert_eq!(
        committed_data, on_demand_data,
        "committed and on-demand payloads differ only in generated_at"
    );
}

#[test]
fn agent_cold_start_validates_schema_and_hides_whisper() {
    let repo = rich_fixture();
    podras_in(&repo, &["init"]).success();

    let agents = repo.read("AGENTS.md");
    assert!(
        agents.contains("<!-- PODRAS:START -->") && agents.contains("## podras"),
        "init writes the PODRAS block: {agents}"
    );
    assert!(
        agents.contains("podras brief --json"),
        "the block tells a fresh agent to start from the JSON brief: {agents}"
    );

    let raw = stdout(&repo, &["brief", "--json", "--as-of", AS_OF]);
    let envelope: Value = serde_json::from_str(raw.trim()).expect("fresh process emits JSON");
    assert_eq!(envelope["ok"].as_bool(), Some(true));
    assert_eq!(envelope["envelope_kind"], "ok");

    let data = &envelope["data"];
    let schema_text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schema/brief-envelope.schema.json"
    ))
    .expect("committed brief schema exists");
    let schema: Value = serde_json::from_str(&schema_text).expect("schema is JSON");
    jsonschema::validator_for(&schema)
        .expect("schema compiles")
        .validate(data)
        .expect("brief payload conforms to schema/brief-envelope.schema.json");

    assert_eq!(data["schema_version"], "brief.v1");
    for key in ["focus", "next", "blocked", "health", "sources"] {
        assert!(data.get(key).is_some(), "payload carries `{key}`: {data}");
    }
    let focus_ids: Vec<&str> = data["focus"]
        .as_array()
        .expect("focus is an array")
        .iter()
        .map(|item| item["id"].as_str().expect("focus item has an id"))
        .collect();
    assert_eq!(focus_ids, vec![FOCUS_ID], "focus follows curation");
    assert!(
        !data["next"]
            .as_array()
            .expect("next is an array")
            .is_empty()
    );
    assert!(
        !data["blocked"]
            .as_array()
            .expect("blocked is an array")
            .is_empty()
    );
    assert_eq!(data["health"]["status"], "unknown");
    assert_eq!(
        data["sources"]
            .as_array()
            .expect("sources is an array")
            .len(),
        5,
        "all declared sources are present: {}",
        data["sources"]
    );
    assert!(
        !raw.contains(WHISPER_SECRET),
        "whisper next-action content never reaches the JSON: {raw}"
    );
}

#[test]
fn closed_focus_is_flagged_and_stays_listed() {
    let repo = TempRepo::new();
    repo.curation(RICH_CURATION);
    repo.set_mtime(".podras/curation.toml", "2026-11-16T00:00:00Z");
    repo.beads_jsonl_at(
        &[
            (FOCUS_ID, "Brief read path", "closed"),
            (NEXT_ID, "Agent cold start", "open"),
        ],
        &[],
        "2026-11-17T00:00:00Z",
    );
    repo.commit_at("2026-11-17T00:00:00Z", "podras-c: agent cold start");

    let data = brief_data(&repo);
    let focus = data["focus"].as_array().expect("focus is an array");
    assert_eq!(focus.len(), 1, "curation keeps exactly one focus");
    assert_eq!(focus[0]["id"], FOCUS_ID, "curation beats inference");
    assert!(
        focus[0]["title"].is_null() && focus[0]["status"].is_null(),
        "a closed tracker item supplies no fabricated metadata: {}",
        focus[0]
    );

    let health = &data["health"];
    assert!(
        health["signals"]
            .as_array()
            .expect("signals is an array")
            .iter()
            .any(|signal| signal["kind"] == "focus_closed"),
        "closing the focused issue fires focus_closed: {health}"
    );
    assert!(
        health["reason"]
            .as_str()
            .unwrap_or_default()
            .contains(FOCUS_ID),
        "the verdict reason names the closed focus: {health}"
    );
    assert_ne!(health["status"], "green", "signals are never green");

    let markdown = stdout(&repo, &["brief", "--as-of", AS_OF]);
    let rendered = section(&markdown, "Health + confidence");
    assert!(
        rendered.contains("focus closed"),
        "the narrative surfaces the focus_closed signal: {rendered}"
    );
    assert!(
        !rendered.contains("**green**"),
        "the narrative never claims green: {rendered}"
    );
}

#[test]
fn missing_beads_degrades_without_fabricating_focus() {
    let repo = TempRepo::new();
    repo.curation(RICH_CURATION);
    repo.set_mtime(".podras/curation.toml", "2026-11-16T00:00:00Z");
    repo.beads_jsonl(&[(FOCUS_ID, "Brief read path", "open")]);
    repo.commit_at("2026-11-17T00:00:00Z", "podras-a: brief read path");
    std::fs::remove_dir_all(repo.root().join(".beads")).expect("delete the tracker fixture");

    let envelope = brief_envelope(&repo);
    assert_eq!(
        envelope["ok"].as_bool(),
        Some(true),
        "a missing source exits 0"
    );
    let data = &envelope["data"];

    let beads = source(data, "beads");
    assert_eq!(
        beads["status"], "missing",
        "the tracker is declared missing: {beads}"
    );
    assert!(
        data["warnings"]
            .as_array()
            .expect("warnings is an array")
            .iter()
            .any(|warning| warning["source"] == "beads" && warning["code"] == "beads_missing"),
        "a missing tracker emits a beads warning: {}",
        data["warnings"]
    );
    assert_ne!(
        data["health"]["status"], "green",
        "a missing mandatory source is never green: {}",
        data["health"]
    );

    let focus = data["focus"].as_array().expect("focus is an array");
    assert_eq!(focus.len(), 1, "only curated focus exists");
    assert_eq!(focus[0]["id"], FOCUS_ID);
    assert_eq!(source(data, "curation")["status"], "present");
    assert_eq!(
        focus[0]["reason"], "finish the brief read path",
        "focus provenance is curation, not a fabricated tracker reading: {}",
        focus[0]
    );
    assert!(
        focus[0]["title"].is_null() && focus[0]["status"].is_null(),
        "no tracker title or status is fabricated: {}",
        focus[0]
    );
    assert!(
        data["next"]
            .as_array()
            .expect("next is an array")
            .is_empty()
    );
    assert!(
        data["blocked"]
            .as_array()
            .expect("blocked is an array")
            .is_empty()
    );
}

#[test]
fn edited_spec_advances_observed_freshness() {
    let repo = TempRepo::new();
    repo.beads_jsonl_at(
        &[(FOCUS_ID, "Brief read path", "open")],
        &[],
        "2026-11-17T00:00:00Z",
    );
    repo.spec_change("read-path", "# Read path\n");
    repo.set_mtime(
        "openspec/changes/read-path/proposal.md",
        "2026-10-01T00:00:00Z",
    );
    repo.commit_at("2026-11-17T00:00:00Z", "podras-a: read path");

    let first = brief_data(&repo);
    let first_spec = source(&first, "openspec");
    assert_eq!(first_spec["staleness"], "stale");
    assert!(
        first_spec["observed_at"]
            .as_str()
            .unwrap_or_default()
            .starts_with("2026-10-01"),
        "first observation reads the proposal mtime: {first_spec}"
    );

    repo.write(
        "openspec/changes/read-path/proposal.md",
        "# Read path\n\nEdited after the first observation.\n",
    );
    repo.set_mtime(
        "openspec/changes/read-path/proposal.md",
        "2026-11-17T00:00:00Z",
    );

    let second = brief_data(&repo);
    let second_spec = source(&second, "openspec");
    assert_eq!(second_spec["staleness"], "fresh");
    assert!(
        second_spec["observed_at"]
            .as_str()
            .unwrap_or_default()
            .starts_with("2026-11-17"),
        "second observation advances to the edited mtime: {second_spec}"
    );
    assert_ne!(
        first_spec["observed_at"], second_spec["observed_at"],
        "the two runs must not claim identical freshness evidence"
    );
    assert_ne!(
        first_spec["staleness"], second_spec["staleness"],
        "the edited spec changes its freshness judgement"
    );
    assert!(
        second["health"]["signals"]
            .as_array()
            .expect("signals is an array")
            .iter()
            .any(|signal| signal["kind"] == "spec_change_without_bead"),
        "the newly changed unlinked spec fires spec_change_without_bead: {}",
        second["health"]
    );
}
