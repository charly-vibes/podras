//! Purpose: dispatch brief rendering to the markdown and envelope renderers.
//!
//! Responsibilities:
//! - name the supported output formats;
//! - route one brief to either renderer.
//!
//! Rationale: both renders must derive from the same brief value so the
//! narrative and agent envelope never diverge in content.

use crate::error::Result;
use crate::model::Brief;

pub mod envelope;
pub mod markdown;

/// Supported render targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderFormat {
    /// Human/committed narrative render.
    Markdown,
    /// Typed agent envelope.
    Json,
}

/// Render a brief in the requested format.
pub fn render(brief: &Brief, format: RenderFormat) -> Result<String> {
    match format {
        RenderFormat::Markdown => markdown::render(brief, None),
        RenderFormat::Json => envelope::to_json(brief),
    }
}
