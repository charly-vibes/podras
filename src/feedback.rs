//! Purpose: `podras feedback` — agent issue reporting against podras's own
//! repository, through the genesis feedback handler.
//!
//! Responsibilities:
//! - mirror the genesis feedback arguments for the podras CLI;
//! - delegate to `genesis::feedback::handle_feedback` with podras's repo
//!   context (`charly-vibes/podras`);
//! - map the `gh` fallback ladder outcome into a serializable envelope
//!   payload.
//!
//! Rationale: feedback is advisory — it reports an issue and never touches
//! podras state — so this module only translates between the genesis
//! feedback API and podras's envelope conventions.

use std::path::Path;

use genesis::feedback::gh::GhResult;
use serde::Serialize;

use crate::error::{Error, Result};

/// GitHub repository that receives podras feedback.
const FEEDBACK_REPO: &str = "charly-vibes/podras";

/// Serializable payload of the `podras feedback` success envelope: the
/// actionable fields of the gh fallback ladder outcome.
#[derive(Debug, Serialize)]
pub struct FeedbackData {
    /// True when the report was handled (submitted, prepared, or dry-run).
    pub ok: bool,
    /// How the report landed: `submitted`, `fallback_url`, or `local_file`
    /// (genesis maps a dry-run onto `fallback_url` without filing).
    pub outcome: String,
    /// Issue URL when created, or the prefilled web URL for manual filing.
    pub url: Option<String>,
    /// Issue number when created.
    pub number: Option<u64>,
    /// Local file path when the network was unavailable.
    pub file: Option<String>,
    /// Human-readable summary of what happened.
    pub message: String,
}

/// Handle one feedback request against podras's repository.
pub fn run(
    repo: &Path,
    kind: &str,
    title: Option<&str>,
    dry_run: bool,
    from_last_error: bool,
) -> Result<FeedbackData> {
    let mut args = genesis::feedback::FeedbackArgs::new(kind, dry_run, from_last_error);
    if let Some(title) = title {
        args = args.with_title(title);
    }

    let result = genesis::feedback::handle_feedback(
        &args,
        "podras",
        env!("CARGO_PKG_VERSION"),
        FEEDBACK_REPO,
        repo,
    )
    .map_err(|message| Error::Domain {
        code: "feedback_failed",
        message,
        suggestion: Some("valid kinds: bug, feature, question, chore".into()),
    })?;

    Ok(match result {
        GhResult::Created { url, number } => FeedbackData {
            ok: true,
            outcome: "submitted".into(),
            url: Some(url),
            number: Some(number),
            file: None,
            message: format!("issue #{number} created"),
        },
        GhResult::FallbackUrl(url) => FeedbackData {
            ok: true,
            outcome: "fallback_url".into(),
            url: Some(url),
            number: None,
            file: None,
            message: "gh unavailable: file the issue via the prefilled URL".into(),
        },
        GhResult::LocalFile(path) => FeedbackData {
            ok: true,
            outcome: "local_file".into(),
            url: None,
            number: None,
            file: Some(path.display().to_string()),
            message: "network unavailable: the issue body was written to a local file".into(),
        },
    })
}
