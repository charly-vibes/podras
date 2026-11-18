//! Purpose: define podras's domain error type and result alias.
//!
//! Responsibilities:
//! - wrap I/O, TOML, and JSON failures at the library boundary;
//! - carry structured domain errors with a stable code and optional remediation;
//! - mark entry points owned by later tickets while the read path is staged.
//!
//! Rationale: thiserror keeps failures typed inside the library, and the binary
//! converts them exactly once into miette diagnostics. Genesis envelopes remain
//! the machine-output contract, not an internal control-flow mechanism.

use thiserror::Error;

/// Crate-wide error type.
#[derive(Debug, Error)]
pub enum Error {
    /// Filesystem failures: missing repository, unreadable source, failed write.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Malformed TOML (curation file or repository config).
    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),

    /// Malformed or unencodable JSON.
    #[error("json error: {0}")]
    Serde(#[from] serde_json::Error),

    /// A domain violation with a stable machine-readable code.
    #[error("{code}: {message}")]
    Domain {
        /// Stable code, e.g. `curation.duplicate_focus`.
        code: &'static str,
        /// Human-readable explanation.
        message: String,
        /// Optional remediation hint.
        suggestion: Option<String>,
    },

    /// Deliberate red-stage marker: the owning ticket has not landed yet.
    #[error("not yet implemented: {0}")]
    NotYetImplemented(&'static str),
}

/// Result alias used throughout the crate.
pub type Result<T, E = Error> = std::result::Result<T, E>;

impl miette::Diagnostic for Error {}
