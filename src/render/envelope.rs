//! Purpose: wrap the brief in the genesis agent envelope.
//!
//! Responsibilities:
//! - emit typed JSON with the brief as the envelope payload;
//! - keep the envelope serialization identical for on-demand and committed runs.
//!
//! Rationale: machine consumers need the genesis envelope contract, and one
//! wrapper prevents ad-hoc JSON assembly at the CLI boundary.

use genesis::envelope::Envelope;

use crate::error::{Error, Result};
use crate::model::Brief;

/// Build the agent envelope for a brief.
pub fn envelope(_brief: &Brief) -> Result<Envelope<Brief>> {
    Err(Error::NotYetImplemented("render::envelope::envelope"))
}
