//! Purpose: expose the curation verbs as library entry points.
//!
//! Responsibilities:
//! - declare the focus, park, and init verb modules;
//! - keep verb implementations out of the CLI boundary.
//!
//! Rationale: verbs mutate human-owned state, while the read path stays pure.
//! Separating them lets tests target mutation without invoking the binary.

pub mod focus;
pub mod init;
pub mod park;
