//! Purpose: the agent cold-start experiment — a fresh process reaches a valid brief.
//!
//! Responsibilities:
//! - `init` writes the PODRAS block that points a fresh agent at `brief --json`;
//! - `brief --json` from that process conforms to the committed schema;
//! - private whisper next-action content never reaches machine output.
//!
//! Rationale: cold start is the experiment that decides whether an agent with no
//! conversation history can orient itself from the repository alone.

use serde_json::Value;

use crate::fixtures::{AS_OF, FOCUS_ID, WHISPER_SECRET, podras_in, rich_fixture, stdout};

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
