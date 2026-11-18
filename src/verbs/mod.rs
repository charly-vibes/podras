//! Purpose: expose the curation verbs as library entry points.
//!
//! Responsibilities:
//! - declare the focus, park, and init verb modules;
//! - resolve verb targets against the readable beads export;
//! - keep verb implementations out of the CLI boundary.
//!
//! Rationale: verbs mutate human-owned state, while the read path stays pure.
//! Separating them lets tests target mutation without invoking the binary.

use std::path::Path;

use crate::error::{Error, Result};
use crate::readers::{self, ReadContext};

pub mod focus;
pub mod init;
pub mod park;

/// Reject ids the readable beads export does not list as open.
///
/// An absent or unreadable export cannot answer the question, so the verb
/// accepts the id instead of inventing a tracker condition the repo never made.
/// `code` is the verb-specific failure code, so each verb keeps its own
/// diagnostic while sharing one resolution rule.
pub(crate) fn resolve_open_item(repo: &Path, id: &str, code: &'static str) -> Result<()> {
    let context = ReadContext {
        repo_root: repo.to_path_buf(),
        as_of: None,
        branch: None,
    };
    let evaluation_date = context
        .as_of
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let Some(beads) = readers::beads::read(&context, evaluation_date).data else {
        return Ok(());
    };
    if beads.open.iter().any(|item| item.id == id) {
        return Ok(());
    }
    Err(Error::Domain {
        code,
        message: format!("`{id}` is not an open issue in the beads export"),
        suggestion: Some("run bd list".to_string()),
    })
}
