//! Purpose: pinned fixtures and render assertions shared by the brief v1 experiments.
//!
//! Responsibilities:
//! - pinned ids, dates, and curation documents every experiment seeds from;
//! - `rich_fixture`: blocked/ready/closed issues, curation, specs, notes, commits;
//! - binary drivers (`podras_in`, `stdout`, `brief_envelope`, `brief_data`);
//! - narrative assertions (section order, sources table, exact staleness, health).
//!
//! Rationale: every experiment must read the same realistic repository through the
//! same accessors, so the fixtures live apart from the scenarios themselves.

use assert_cmd::Command;
use serde_json::Value;

pub(crate) use crate::common::TempRepo;

/// Pinned evaluation date for every experiment.
pub(crate) const AS_OF: &str = "2026-11-18";
/// Human-curated focus.
pub(crate) const FOCUS_ID: &str = "podras-a";
/// Open issue blocked by the focus.
pub(crate) const BLOCKED_ID: &str = "podras-b";
/// Ready issue that belongs in next.
pub(crate) const NEXT_ID: &str = "podras-c";
/// Closed issue that must never appear as ready.
pub(crate) const CLOSED_ID: &str = "podras-d";
/// Curated parked item whose trigger is in the future.
pub(crate) const PARKED_ID: &str = "podras-f";
/// Parked item's resurface date.
pub(crate) const PARKED_TRIGGER: &str = "2027-01-01";
/// Private whisper content that must not reach machine output.
pub(crate) const WHISPER_SECRET: &str = "private-next-action-launch-code";
/// Openspec proposal used by the staleness experiment.
pub(crate) const SPEC_PATH: &str = "openspec/changes/brief-read-path/proposal.md";

/// Run the real binary inside a fixture repository.
pub(crate) fn podras_in(repo: &TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(args)
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .assert()
}

/// Run the binary and return stdout as text.
pub(crate) fn stdout(repo: &TempRepo, args: &[&str]) -> String {
    let output = podras_in(repo, args).success().get_output().stdout.clone();
    String::from_utf8(output).expect("podras output is UTF-8")
}

/// Run `brief --json` and return the genesis envelope.
pub(crate) fn brief_envelope(repo: &TempRepo) -> Value {
    serde_json::from_str(stdout(repo, &["brief", "--json", "--as-of", AS_OF]).trim())
        .expect("brief --json emits a JSON envelope")
}

/// Run `brief --json` and return its typed payload.
pub(crate) fn brief_data(repo: &TempRepo) -> Value {
    brief_envelope(repo)["data"].clone()
}

/// Find one declared source row in a brief payload.
pub(crate) fn source<'a>(data: &'a Value, name: &str) -> &'a Value {
    data["sources"]
        .as_array()
        .expect("sources is an array")
        .iter()
        .find(|row| row["name"] == name)
        .unwrap_or_else(|| panic!("source `{name}` is declared: {data}"))
}

/// Return one `## ` section body from the narrative render.
pub(crate) fn section<'a>(markdown: &'a str, heading: &str) -> &'a str {
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
pub(crate) const RICH_CURATION: &str = r#"purpose_lite = "podras: project status for a cold return"
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

/// Curation whose fresh human word is green until evidence contradicts it.
pub(crate) const GREEN_CURATION: &str = r#"purpose_lite = "podras: project status for a cold return"
max_focus = 1

[health]
status = "green"
owner = "@sasha"
reason = "brief read path shipped"
confirmed_at = "2026-11-15"

[[focus]]
id = "podras-a"
reason = "finish the brief read path"
focused_since = "2026-11-10"
confirmed_at = "2026-11-15"
"#;

/// A realistic repository: blocked/ready/closed issues, curation, specs, notes, commits.
pub(crate) fn rich_fixture() -> TempRepo {
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
pub(crate) fn assert_section_order(markdown: &str) {
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
pub(crate) fn assert_sources_table(markdown: &str) {
    let sources = section(markdown, "Sources & freshness");
    for name in ["beads", "git", "openspec", "whisper", "curation"] {
        let row = source_row(sources, name);
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

/// Assert the exact per-source staleness the pinned evaluation date implies.
pub(crate) fn assert_exact_staleness(markdown: &str, expected: &[(&str, &str)]) {
    let sources = section(markdown, "Sources & freshness");
    for (name, staleness) in expected {
        let row = source_row(sources, name);
        let cells: Vec<&str> = row.split('|').map(str::trim).collect();
        assert_eq!(
            cells[4], *staleness,
            "`{name}` staleness at {AS_OF} is exactly `{staleness}`: {row}"
        );
    }
}

/// One row of the sources table, identified by declared source name.
fn source_row<'a>(sources: &'a str, name: &str) -> &'a str {
    sources
        .lines()
        .find(|line| line.starts_with(&format!("| {name} |")))
        .unwrap_or_else(|| panic!("sources table has a `{name}` row: {sources}"))
}

/// Assert the narrative's health verdict line exists and is never green.
pub(crate) fn assert_health_not_green(markdown: &str) -> String {
    let health = section(markdown, "Health + confidence");
    let line = health
        .lines()
        .find(|line| line.starts_with("Health: "))
        .unwrap_or_else(|| panic!("health section carries a verdict line: {health}"));
    assert!(
        !line.contains("**green**"),
        "the narrative never claims green: {line}"
    );
    line.to_string()
}
