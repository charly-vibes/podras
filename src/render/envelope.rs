//! Purpose: wrap the brief in the genesis agent envelope.
//!
//! Responsibilities:
//! - emit typed JSON with the brief as the envelope payload;
//! - keep the envelope serialization identical for on-demand and committed runs;
//! - emit the brief JSON Schema (2020-12) from the model.
//!
//! Rationale: machine consumers need the genesis envelope contract, and one
//! wrapper prevents ad-hoc JSON assembly at the CLI boundary. Per-source
//! warnings are data: they stay inside `Brief.warnings`, so the genesis
//! wrapper's own warning list is always empty.

use genesis::envelope::{Envelope, EnvelopeKind};

use crate::error::Result;
use crate::model::Brief;

/// JSON Schema dialect emitted for the brief payload.
const JSON_SCHEMA_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

/// Build the agent envelope for a brief.
///
/// The payload is the brief itself; the wrapper carries no warnings or hints,
/// because every warning is a typed field inside the brief.
pub fn envelope(brief: &Brief) -> Result<Envelope<Brief>> {
    Ok(Envelope::success(
        env!("CARGO_PKG_VERSION"),
        EnvelopeKind::Ok,
        brief.clone(),
        Vec::new(),
        Vec::new(),
    ))
}

/// Render the agent envelope as pretty JSON with a trailing newline.
pub fn to_json(brief: &Brief) -> Result<String> {
    let envelope = envelope(brief)?;
    let mut rendered = serde_json::to_string_pretty(&envelope)?;
    rendered.push('\n');
    Ok(rendered)
}

/// Render the brief JSON Schema in the 2020-12 dialect, file-ready.
///
/// The committed `schema/brief-envelope.schema.json` is exactly this output,
/// byte for byte; `tests/envelope.rs` regenerates and compares it.
pub fn schema() -> Result<String> {
    let mut schema = schemars::schema_for!(Brief);
    schema.ensure_object().insert(
        "$schema".to_string(),
        serde_json::Value::String(JSON_SCHEMA_2020_12.to_string()),
    );
    let mut rendered = serde_json::to_string_pretty(&schema)?;
    rendered.push('\n');
    Ok(rendered)
}
