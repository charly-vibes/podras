//! Purpose: the staleness-fault experiments — evidence that contradicts the human word.
//!
//! Responsibilities:
//! - `closed_focus_is_flagged_and_stays_listed`: a closed focus fires
//!   `focus_closed`, and curation still beats inference;
//! - `missing_beads_degrades_without_fabricating_focus`: a deleted tracker
//!   degrades honestly and fabricates neither title nor status;
//! - `edited_spec_advances_observed_freshness`: a re-observed spec moves from
//!   stale to fresh and fires `spec_change_without_bead`;
//! - `aged_beads_export_is_stale_and_downgrades_green`: an aged export downgrades
//!   a green human word to amber.
//!
//! Rationale: faults are the case where a tool is most tempted to smooth over
//! missing evidence, so each one is injected into a real repository and read back
//! through the binary.

use crate::fixtures::{
    AS_OF, FOCUS_ID, GREEN_CURATION, NEXT_ID, RICH_CURATION, TempRepo, assert_exact_staleness,
    assert_health_not_green, brief_data, brief_envelope, section, source, stdout,
};

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

    let markdown = stdout(&repo, &["brief", "--as-of", AS_OF]);
    let sources = section(&markdown, "Sources & freshness");
    assert!(
        sources.contains("| beads | missing |"),
        "the narrative names the missing tracker row: {sources}"
    );
    let verdict = assert_health_not_green(&markdown);
    assert!(
        verdict.starts_with("Health: **unknown**"),
        "a missing tracker leaves the unknown verdict intact: {verdict}"
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

#[test]
fn aged_beads_export_is_stale_and_downgrades_green() {
    let repo = TempRepo::new();
    repo.curation(GREEN_CURATION);
    repo.set_mtime(".podras/curation.toml", "2026-11-16T00:00:00Z");
    repo.beads_jsonl_at(
        &[(FOCUS_ID, "Brief read path", "open")],
        &[],
        "2026-11-01T00:00:00Z",
    );
    repo.commit_at("2026-11-17T00:00:00Z", "podras-a: brief read path");

    let data = brief_data(&repo);
    let beads = source(&data, "beads");
    assert_eq!(
        beads["status"], "present",
        "an aged export is still readable: {beads}"
    );
    assert_eq!(
        beads["staleness"], "stale",
        "17 days past the 7-day horizon is stale: {beads}"
    );
    assert!(
        beads["observed_at"]
            .as_str()
            .unwrap_or_default()
            .starts_with("2026-11-01"),
        "staleness is judged from the export's own updated_at: {beads}"
    );

    let health = &data["health"];
    assert_eq!(
        health["status"], "amber",
        "stale mandatory evidence downgrades green: {health}"
    );
    assert_eq!(
        health["basis"], "human",
        "the human word still supplies the basis: {health}"
    );
    let reason = health["reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("beads") && reason.contains("stale"),
        "the downgrade names the stale mandatory source: {health}"
    );

    let markdown = stdout(&repo, &["brief", "--as-of", AS_OF]);
    let verdict = assert_health_not_green(&markdown);
    assert!(
        verdict.contains("**amber**"),
        "the narrative renders amber, never green: {verdict}"
    );
    assert_exact_staleness(&markdown, &[("beads", "stale"), ("git", "fresh")]);
}
