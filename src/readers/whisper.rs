//! Purpose: light-read whisper session notes, latest branch entry only.
//!
//! Responsibilities:
//! - locate `.whisper/branches/<topic>/notes.md`;
//! - extract the latest next-action line and a content hash.
//!
//! Rationale: whisper content is private by default and expensive to mine;
//! v1 reads exactly one entry so the brief can stamp freshness without
//! importing session narrative into the committed render.

use std::path::Path;

use crate::brief::WhisperReading;
use crate::error::{Error, Result};

/// Read the latest whisper branch entry for the repository.
pub fn read(_repo: &Path) -> Result<WhisperReading> {
    Err(Error::NotYetImplemented("readers::whisper::read"))
}
