//! Purpose: library root for podras, the per-repository project-status CLI.
//!
//! Responsibilities:
//! - declare the model, reader, render, health, and verb modules;
//! - re-export the brief type and status enums for consumers and tests.
//!
//! Rationale: the binary stays a thin CLI boundary; every behavior worth
//! testing lives in the library.

pub mod brief;
pub mod commands;
pub mod curation;
pub mod error;
pub mod freshness;
pub mod harvest;
pub mod hash;
pub mod health;
pub mod model;
pub mod readers;
pub mod render;
pub mod verbs;
pub mod write;

pub use error::{Error, Result};
pub use model::{
    Brief, CurationStamp, Health, HealthBasis, HealthStatus, ParkedItem, RecentEvent,
    SCHEMA_VERSION, Signal, SignalKind, SourceName, SourceState, SourceStatus, Staleness,
    Truncation, Warning, WorkItem,
};
