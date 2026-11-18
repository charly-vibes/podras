//! Purpose: run the three validation experiments that gate a "done" claim for brief v1.
//!
//! Responsibilities:
//! - amnesia (`experiments/amnesia.rs`): `BRIEF.md` alone answers the cold-return
//!   questions, and the committed and on-demand semantic payloads are identical;
//! - agent cold-start (`experiments/cold_start.rs`): `init` plus a schema-valid
//!   `brief --json` from a fresh process that never leaks whisper content;
//! - staleness faults (`experiments/fault_injection.rs`): closed focus, deleted
//!   tracker, edited spec, aged export;
//! - fixtures and render assertions shared by all three (`experiments/fixtures.rs`).
//!
//! Rationale: these experiments are the acceptance gate in `docs/design.md`, so
//! they exercise the real binary and real fixtures rather than library internals.

mod common;

#[path = "experiments/fixtures.rs"]
mod fixtures;

#[path = "experiments/amnesia.rs"]
mod amnesia;

#[path = "experiments/cold_start.rs"]
mod cold_start;

#[path = "experiments/fault_injection.rs"]
mod fault_injection;
