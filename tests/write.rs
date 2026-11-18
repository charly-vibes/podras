//! Purpose: prove `brief --write` overwrites hand edits, reports its output, and
//! keeps whisper private.
//!
//! Responsibilities:
//! - `hand_edit_render_overwritten`: regenerated artifacts replace manual edits;
//! - `whisper_privacy`: whisper next actions never enter committed artifacts
//!   unless `publish_whisper` is enabled, and never enter `BRIEF.json`;
//! - `write_reports_written_paths`: `--write` prints the paths it wrote instead
//!   of the narrative, while `--json --write` prints the envelope;
//! - `failed_pair_write_keeps_previous_render`: an unwritable artifact leaves the
//!   previous render pair untouched and exits non-zero.
//!
//! Rationale: committed renders are generated, not authored, so overwrite
//! semantics, whisper privacy, and all-or-nothing replacement are the
//! properties users must be able to trust without reading the writer.

mod common;

use assert_cmd::Command;
use serde_json::Value;

/// The private next-action text that must not reach committed artifacts.
const SECRET: &str = "privacy-secret-launch-code";
/// The whisper entry timestamp rendered as the source observation.
const OBSERVED: &str = "2026-11-01T00:00:00Z";

fn podras_in(repo: &common::TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(args)
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .assert()
}

fn stdout(repo: &common::TempRepo, args: &[&str]) -> String {
    let output = podras_in(repo, args).success().get_output().stdout.clone();
    String::from_utf8(output).expect("podras output is UTF-8")
}

/// The generated_at stamp is the only volatile line in BRIEF.md.
fn normalize_markdown(text: &str) -> String {
    text.lines()
        .map(|line| {
            if line.starts_with("Freshness: generated_at ") {
                "Freshness: generated_at <normalized>; per-source observed_at and staleness are listed above.".to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse BRIEF.json and drop every generation stamp, leaving stable data.
fn normalize_json(text: &str) -> Value {
    let mut value: Value =
        serde_json::from_str(text.trim()).expect("BRIEF.json is a JSON envelope");
    value
        .as_object_mut()
        .expect("envelope is an object")
        .remove("generated_at");
    if let Some(data) = value.get_mut("data").and_then(Value::as_object_mut) {
        data.remove("generated_at");
    }
    value
}

/// Seed a whisper note whose next action is `SECRET` and curation gating it.
fn whisper_fixture(repo: &common::TempRepo, publish: bool) {
    repo.curation(&format!(
        "purpose_lite = \"write fixture\"\npublish_whisper = {publish}\n"
    ));
    repo.write(
        ".whisper/branches/main/notes.md",
        &format!(
            "- {OBSERVED} [id:fixture] (#main) Format: 1\n  private session\n\n## Next action\n{SECRET}\n"
        ),
    );
}

#[test]
fn hand_edit_render_overwritten() {
    let repo = common::TempRepo::new();
    whisper_fixture(&repo, false);

    podras_in(&repo, &["brief", "--write"]).success();
    let first_markdown = repo.read("BRIEF.md");
    let first_json = repo.read("BRIEF.json");

    repo.write(
        "BRIEF.md",
        &format!("{first_markdown}\nHAND EDIT markdown\n"),
    );
    repo.write("BRIEF.json", "{\"hand\": \"edit\"}\n");

    podras_in(&repo, &["brief", "--write"]).success();
    let second_markdown = repo.read("BRIEF.md");
    let second_json = repo.read("BRIEF.json");

    assert!(
        !second_markdown.contains("HAND EDIT"),
        "regenerated BRIEF.md replaces hand edits: {second_markdown}"
    );
    assert!(
        !second_json.contains("HAND EDIT"),
        "regenerated BRIEF.json replaces hand edits: {second_json}"
    );
    assert_eq!(
        normalize_markdown(&first_markdown),
        normalize_markdown(&second_markdown),
        "a rerun regenerates exactly the generated markdown (modulo generated_at)"
    );
    assert_eq!(
        normalize_json(&first_json),
        normalize_json(&second_json),
        "a rerun regenerates exactly the generated JSON (modulo generated_at)"
    );
}

#[test]
fn whisper_privacy() {
    let repo = common::TempRepo::new();
    whisper_fixture(&repo, false);

    podras_in(&repo, &["brief", "--write"]).success();
    let committed_markdown = repo.read("BRIEF.md");
    let committed_json = repo.read("BRIEF.json");

    assert!(
        !committed_markdown.contains(SECRET),
        "the committed render must not publish a private next action: {committed_markdown}"
    );
    assert!(
        committed_markdown.contains(OBSERVED),
        "the committed render still stamps whisper observation freshness: {committed_markdown}"
    );
    assert!(
        !committed_json.contains(SECRET),
        "the committed JSON must never carry the private next action: {committed_json}"
    );
    let envelope: Value =
        serde_json::from_str(committed_json.trim()).expect("BRIEF.json is a JSON envelope");
    let whisper = envelope["data"]["sources"]
        .as_array()
        .expect("data.sources is an array")
        .iter()
        .find(|source| source["name"] == "whisper")
        .expect("the whisper source row is declared");
    assert_eq!(
        whisper["observed_at"].as_str(),
        Some(OBSERVED),
        "the whisper source row keeps its observation instant"
    );

    let on_demand = stdout(&repo, &["brief"]);
    assert!(
        on_demand.contains(&format!("Re-entry: {SECRET}")),
        "on-demand output shows the private next action: {on_demand}"
    );

    whisper_fixture(&repo, true);
    podras_in(&repo, &["brief", "--write"]).success();
    assert!(
        repo.read("BRIEF.md").contains(SECRET),
        "publish_whisper = true includes the next action in the committed render"
    );
    assert!(
        !repo.read("BRIEF.json").contains(SECRET),
        "the private next action never enters BRIEF.json, even when published"
    );
}

/// Temp files the writer stages beside the committed artifacts.
fn leftover_temps(repo: &common::TempRepo) -> Vec<String> {
    std::fs::read_dir(repo.root())
        .expect("read repository root")
        .map(|entry| entry.expect("read directory entry").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| name.starts_with(".BRIEF") && name.ends_with(".tmp"))
        .collect()
}

#[test]
fn write_reports_written_paths() {
    let repo = common::TempRepo::new();
    whisper_fixture(&repo, false);

    let printed = stdout(&repo, &["brief", "--write"]);

    assert_eq!(
        printed.trim_end(),
        "wrote BRIEF.md, BRIEF.json",
        "`--write` reports the artifacts it committed"
    );
    assert!(
        !printed.contains("Re-entry:"),
        "`--write` prints no narrative: {printed}"
    );
    assert!(
        !printed.contains(OBSERVED),
        "`--write` prints no freshness stamp from the committed render: {printed}"
    );
    assert!(
        repo.root().join("BRIEF.md").is_file() && repo.root().join("BRIEF.json").is_file(),
        "`--write` commits both artifacts"
    );
}

#[test]
fn json_write_prints_envelope_and_writes_both() {
    let repo = common::TempRepo::new();
    whisper_fixture(&repo, false);

    let printed = stdout(&repo, &["brief", "--json", "--write"]);
    let envelope: Value =
        serde_json::from_str(printed.trim()).expect("`--json --write` prints the envelope");

    assert_eq!(
        envelope["data"]["schema_version"].as_str(),
        Some("brief.v1"),
        "`--json --write` still prints the typed payload: {envelope}"
    );
    assert!(
        repo.root().join("BRIEF.md").is_file() && repo.root().join("BRIEF.json").is_file(),
        "`--json --write` commits both artifacts"
    );
    assert_eq!(
        normalize_json(&printed),
        normalize_json(&repo.read("BRIEF.json")),
        "the printed envelope and the committed JSON agree modulo generated_at"
    );
}

#[test]
fn failed_pair_write_keeps_previous_render() {
    let repo = common::TempRepo::new();
    whisper_fixture(&repo, false);

    podras_in(&repo, &["brief", "--write"]).success();
    // A sentinel distinguishes the previous render from a fresh one regardless
    // of how fast the two runs follow each other.
    let previous_markdown = format!("{}\nPAIR-WRITE-SENTINEL\n", repo.read("BRIEF.md"));
    repo.write("BRIEF.md", &previous_markdown);

    // A directory where BRIEF.json must land cannot be replaced by a rename, so
    // the pair write must fail before the markdown target is touched.
    std::fs::remove_file(repo.root().join("BRIEF.json")).expect("remove the committed JSON");
    std::fs::create_dir(repo.root().join("BRIEF.json")).expect("block the JSON target");

    let failure = podras_in(&repo, &["brief", "--write"])
        .failure()
        .get_output()
        .stderr
        .clone();
    let stderr = String::from_utf8(failure).expect("podras diagnostics are UTF-8");

    assert!(
        stderr.contains("io error"),
        "an unwritable artifact fails with an io diagnostic: {stderr}"
    );
    assert_eq!(
        repo.read("BRIEF.md"),
        previous_markdown,
        "a failed pair write leaves the previous markdown render byte-identical"
    );
    assert!(
        repo.root().join("BRIEF.json").is_dir(),
        "the unwritable target is left as it was found"
    );
    assert!(
        leftover_temps(&repo).is_empty(),
        "a failed pair write removes every staged temp file: {:?}",
        leftover_temps(&repo)
    );
}
