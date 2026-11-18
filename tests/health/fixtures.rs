//! Purpose: typed brief fixtures shared by the health verdict tests.
//!
//! Responsibilities:
//! - build dated work items, git history, focus, park, and spec entries;
//! - assemble brief inputs from the parts each scenario varies;
//! - locate the single firing signal of a requested kind.
//!
//! Rationale: every health verdict is only as trustworthy as the fixture that
//! drives it, so the shapes live in one place, pinned to one evaluation date,
//! and each test states only the parts its verdict depends on.

use chrono::{DateTime, NaiveDate, Utc};
use podras::brief::{
    BeadsSummary, BriefInputs, CurationState, GitSummary, SpecSummary, WhisperReading,
};
use podras::curation::{CurationFile, CurationHealth, FocusEntry, ParkEntry};
use podras::health::signals;
use podras::model::{
    RecentEvent, Signal, SignalKind, SourceName, SourceState, SourceStatus, Staleness, WorkItem,
};

/// Pinned evaluation date shared by every fixture.
const AS_OF: &str = "2026-11-18";

fn day(raw: &str) -> NaiveDate {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").expect("fixture dates are valid")
}

fn instant(date: &str, hour: u32) -> DateTime<Utc> {
    day(date)
        .and_hms_opt(hour, 0, 0)
        .expect("fixture hours are valid")
        .and_utc()
}

fn observed_at() -> Option<DateTime<Utc>> {
    Some(instant(AS_OF, 9))
}

pub fn open_item(id: &str) -> WorkItem {
    WorkItem {
        id: id.to_string(),
        title: Some(format!("title {id}")),
        status: Some("open".to_string()),
        priority: Some(2),
        blocked_by: Vec::new(),
        age_days: Some(3),
        reason: None,
        confirmed_at: None,
    }
}

pub fn titled_open_item(id: &str, title: &str) -> WorkItem {
    WorkItem {
        title: Some(title.to_string()),
        ..open_item(id)
    }
}

pub fn spec_item(id: &str, title: &str, status: &str, age_days: Option<u32>) -> WorkItem {
    WorkItem {
        id: id.to_string(),
        title: Some(title.to_string()),
        status: Some(status.to_string()),
        priority: None,
        blocked_by: Vec::new(),
        age_days,
        reason: None,
        confirmed_at: None,
    }
}

fn beads(open: Vec<WorkItem>) -> Option<BeadsSummary> {
    Some(BeadsSummary {
        open,
        blocked: Vec::new(),
        ready: Vec::new(),
        observed_at: observed_at(),
    })
}

pub fn event(sha: &str, subject: &str, date: &str) -> RecentEvent {
    RecentEvent {
        sha: sha.to_string(),
        committed_at: instant(date, 10),
        subject: subject.to_string(),
        outcome: None,
    }
}

fn git(recent: Vec<RecentEvent>) -> Option<GitSummary> {
    Some(GitSummary {
        head_sha: Some("head123".to_string()),
        recent,
        observed_at: observed_at(),
    })
}

/// Git history whose newest commit is inside the fourteen-day window.
fn fresh_events() -> Vec<RecentEvent> {
    vec![event("head123", "initial commit", "2026-11-17")]
}

/// Git history whose newest commit is outside the fourteen-day window.
fn quiet_events() -> Vec<RecentEvent> {
    vec![event("old123", "initial commit", "2026-10-01")]
}

fn specs(changes: Vec<WorkItem>) -> Option<SpecSummary> {
    Some(SpecSummary {
        changes,
        stale_count: 0,
        observed_at: observed_at(),
    })
}

pub fn focus_entry(id: &str, focused_since: &str, confirmed_at: &str) -> FocusEntry {
    FocusEntry {
        id: id.to_string(),
        reason: Some(format!("reason for {id}")),
        focused_since: Some(focused_since.to_string()),
        confirmed_at: Some(confirmed_at.to_string()),
    }
}

pub fn park_date(id: &str, resurface: &str) -> ParkEntry {
    ParkEntry {
        id: id.to_string(),
        reason: format!("reason for {id}"),
        resurface: resurface.to_string(),
        parked_at: Some("2026-10-01".to_string()),
        confirmed_at: Some("2026-10-01".to_string()),
    }
}

pub fn park_event(id: &str, resurface: &str) -> ParkEntry {
    ParkEntry {
        id: id.to_string(),
        reason: format!("reason for {id}"),
        resurface: format!("event:{resurface}"),
        parked_at: Some("2026-10-01".to_string()),
        confirmed_at: Some("2026-10-01".to_string()),
    }
}

fn curation(
    health: Option<CurationHealth>,
    max_focus: Option<usize>,
    focus: Vec<FocusEntry>,
    parked: Vec<ParkEntry>,
) -> Option<CurationState> {
    Some(CurationState {
        curation: CurationFile {
            purpose_lite: Some("podras: status read path for a cold return".to_string()),
            max_focus,
            publish_whisper: Some(false),
            health,
            focus,
            parked,
        },
        updated_at: observed_at(),
    })
}

pub fn human(status: &str, confirmed_at: &str) -> CurationHealth {
    CurationHealth {
        status: status.to_string(),
        owner: Some("@sasha".to_string()),
        confirmed_at: Some(confirmed_at.to_string()),
        reason: Some("human note".to_string()),
    }
}

fn inputs(
    curation: Option<CurationState>,
    beads: Option<BeadsSummary>,
    git: Option<GitSummary>,
    specs: Option<SpecSummary>,
) -> BriefInputs {
    BriefInputs {
        beads,
        git,
        specs,
        whisper: Some(WhisperReading {
            observed_at: observed_at(),
            next_action: None,
            content_hash: None,
            latest_entry: None,
        }),
        curation,
        sources: Vec::new(),
        warnings: Vec::new(),
        as_of: Some(day(AS_OF)),
        now: day("2026-11-01"),
        generated_at: instant(AS_OF, 12),
    }
}

pub fn mandatory_row(name: SourceName, status: SourceState, staleness: Staleness) -> SourceStatus {
    SourceStatus {
        name,
        status,
        observed_at: observed_at(),
        staleness,
        detail: None,
    }
}

/// The parts a health scenario varies; every other fixture stays at the quiet
/// baseline.
#[derive(Default)]
pub struct Scenario {
    /// Focus entries under test.
    pub focus: Vec<FocusEntry>,
    /// Parked entries under test.
    pub parked: Vec<ParkEntry>,
    /// Human verdict, when the scenario declares one.
    pub verdict: Option<CurationHealth>,
    /// Open beads items.
    pub open: Vec<WorkItem>,
    /// Recent commits.
    pub recent: Vec<RecentEvent>,
    /// Spec changes.
    pub changes: Vec<WorkItem>,
    /// Observed source rows.
    pub sources: Vec<SourceStatus>,
}

impl Scenario {
    /// No human verdict and no firing signal: the newest commit is outside
    /// every window.
    pub fn quiet() -> Scenario {
        Scenario {
            recent: quiet_events(),
            ..Scenario::default()
        }
    }

    /// No human verdict, with fresh git history inside the fourteen-day window.
    pub fn fresh() -> Scenario {
        Scenario {
            recent: fresh_events(),
            ..Scenario::default()
        }
    }

    /// Assemble the brief inputs this scenario describes.
    pub fn inputs(self) -> BriefInputs {
        let mut inputs = inputs(
            curation(self.verdict, Some(1), self.focus, self.parked),
            beads(self.open),
            git(self.recent),
            specs(self.changes),
        );
        inputs.sources = self.sources;
        inputs
    }
}

pub fn kinds(signals: &[Signal]) -> Vec<SignalKind> {
    signals.iter().map(|signal| signal.kind).collect()
}

pub fn firing(inputs: &BriefInputs, kind: SignalKind) -> Signal {
    signals(inputs)
        .into_iter()
        .find(|signal| signal.kind == kind)
        .unwrap_or_else(|| panic!("expected {kind:?} to fire"))
}
