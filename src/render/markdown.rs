//! Purpose: render the brief as the fixed-section markdown narrative.
//!
//! Responsibilities:
//! - render the nine fixed sections in cold-return order;
//! - stamp per-source freshness on the final section.
//!
//! Rationale: the narrative render is the human contract; keeping it a pure
//! function of the brief guarantees regenerated renders overwrite hand edits.

use crate::error::{Error, Result};
use crate::model::Brief;

/// Render a brief as markdown.
pub fn render(_brief: &Brief) -> Result<String> {
    Err(Error::NotYetImplemented("render::markdown::render"))
}
