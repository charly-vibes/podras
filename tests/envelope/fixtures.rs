//! Purpose: schema-conformance fixtures for the envelope tests.
//!
//! Responsibilities:
//! - declare one source row per declared source state;
//! - declare a curation file with a human health verdict;
//! - declare the single warning carried into the schema fixture.
//!
//! Rationale: the schema test asserts the wire shape, so its inputs must cover
//! every declared enum and stay visibly separate from the assertions.

use podras::brief::CurationState;
use podras::curation::{CurationFile, CurationHealth, FocusEntry};
use podras::model::{SourceName, SourceState, SourceStatus, Staleness, Warning};

use super::{AS_OF, instant, source_status};

pub(super) fn schema_sources() -> Vec<SourceStatus> {
    vec![
        source_status(
            SourceName::Beads,
            SourceState::Present,
            Some(instant("2026-11-17T00:00:00Z")),
            Staleness::Fresh,
            None,
        ),
        source_status(
            SourceName::Git,
            SourceState::Present,
            Some(instant("2026-11-17T00:00:00Z")),
            Staleness::Fresh,
            None,
        ),
        source_status(
            SourceName::Openspec,
            SourceState::Missing,
            None,
            Staleness::Unknown,
            Some("openspec/ is absent"),
        ),
        source_status(
            SourceName::Whisper,
            SourceState::Unreadable,
            None,
            Staleness::Unknown,
            Some("notes could not be parsed"),
        ),
        source_status(
            SourceName::Curation,
            SourceState::Present,
            Some(instant("2026-11-01T00:00:00Z")),
            Staleness::Stale,
            None,
        ),
    ]
}

pub(super) fn schema_curation() -> CurationState {
    CurationState {
        curation: CurationFile {
            purpose_lite: Some("schema fixture".to_string()),
            max_focus: Some(1),
            publish_whisper: Some(false),
            health: Some(CurationHealth {
                status: "amber".to_string(),
                owner: Some("@sasha".to_string()),
                confirmed_at: Some(AS_OF.to_string()),
                reason: Some("fixture verdict".to_string()),
            }),
            focus: vec![FocusEntry {
                id: "fixture-focus".to_string(),
                reason: Some("fixture focus".to_string()),
                focused_since: Some("2026-11-01".to_string()),
                confirmed_at: Some("2026-11-01".to_string()),
            }],
            parked: Vec::new(),
        },
        updated_at: Some(instant("2026-11-01T00:00:00Z")),
    }
}

pub(super) fn schema_warnings() -> Vec<Warning> {
    vec![Warning {
        source: SourceName::Beads,
        code: "beads_parse_error".to_string(),
        detail: "one malformed line skipped".to_string(),
    }]
}
