//! Purpose: hold the per-source readings that compose a brief.
//!
//! Responsibilities:
//! - declare the summary types each reader produces;
//! - bundle reader output, source rows, warnings, and the `--as-of` instant;
//! - own `assemble`, the pure function turning inputs into a deterministic `Brief`.
//!
//! Rationale: readers must not know about rendering or bounds. Keeping the
//! inputs in one struct lets `assemble` stay a pure function of observed data:
//! no clock, no filesystem, no cache. Curation beats inference here — focus and
//! parked state come only from `.podras/curation.toml`, and a parked item never
//! re-enters a computed queue.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};

use crate::curation::{CurationFile, CurationHealth, FocusEntry};
use crate::model::{
    BLOCKED_BOUND, Brief, CurationStamp, FOCUS_BOUND, Health, HealthBasis, HealthStatus,
    NEXT_BOUND, PARKED_BOUND, ParkedItem, RECENT_BOUND, RecentEvent, Resurface, SCHEMA_VERSION,
    SourceName, SourceState, SourceStatus, Staleness, Truncation, Warning, WorkItem,
};

/// A human health verdict older than this many days is treated as absent.
const HUMAN_VERDICT_HORIZON_DAYS: i64 = 14;

/// Beads tracker reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeadsSummary {
    /// Open issues.
    pub open: Vec<WorkItem>,
    /// Blocked issues with their blockers.
    pub blocked: Vec<WorkItem>,
    /// Ready issues (no open blockers).
    pub ready: Vec<WorkItem>,
    /// When the export was observed.
    pub observed_at: Option<DateTime<Utc>>,
}

/// Git history reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSummary {
    /// Repository HEAD sha.
    pub head_sha: Option<String>,
    /// Recent commits, newest first.
    pub recent: Vec<RecentEvent>,
    /// When the history was observed.
    pub observed_at: Option<DateTime<Utc>>,
}

/// OpenSpec change-store reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecSummary {
    /// Declared spec changes.
    pub changes: Vec<WorkItem>,
    /// Number of changes that appear stale.
    pub stale_count: usize,
    /// When the change store was observed.
    pub observed_at: Option<DateTime<Utc>>,
}

/// Light whisper-notes reading: latest branch entry only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhisperReading {
    /// When the notes were observed.
    pub observed_at: Option<DateTime<Utc>>,
    /// Latest recorded next action, if any.
    pub next_action: Option<String>,
    /// Hash of the observed note content.
    pub content_hash: Option<String>,
}

/// Curation state as read from `.podras/curation.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurationState {
    /// Parsed curation file.
    pub curation: CurationFile,
    /// File mtime of the last curation write.
    pub updated_at: Option<DateTime<Utc>>,
}

/// Everything `assemble` needs, with absent (not failed) sources as `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefInputs {
    /// Beads reading, when readable.
    pub beads: Option<BeadsSummary>,
    /// Git reading, when readable.
    pub git: Option<GitSummary>,
    /// Spec-store reading, when readable.
    pub specs: Option<SpecSummary>,
    /// Whisper reading, when readable.
    pub whisper: Option<WhisperReading>,
    /// Curation state, when readable.
    pub curation: Option<CurationState>,
    /// One status row per declared source.
    pub sources: Vec<SourceStatus>,
    /// Stable warnings accumulated while reading.
    pub warnings: Vec<Warning>,
    /// Requested observation instant, when pinned.
    pub as_of: Option<NaiveDate>,
    /// Caller-supplied wall-clock date, used only when `as_of` is absent.
    pub now: NaiveDate,
    /// Caller-supplied generation instant; the only volatile field.
    pub generated_at: DateTime<Utc>,
}

/// Compose a brief from observed inputs.
pub fn assemble(inputs: &BriefInputs) -> Brief {
    let evaluation_date = inputs.as_of.unwrap_or(inputs.now);
    let mut warnings = inputs.warnings.clone();
    let curation = inputs.curation.as_ref().map(|state| &state.curation);
    let tracker = Tracker::new(inputs.beads.as_ref());

    let focus_entries = sorted_focus_entries(curation);
    let focus_ids: BTreeSet<String> = curation
        .map(|curation| {
            curation
                .focus
                .iter()
                .map(|entry| entry.id.clone())
                .collect()
        })
        .unwrap_or_default();
    let mut focus: Vec<WorkItem> = focus_entries
        .iter()
        .map(|entry| curated_work_item(entry, tracker.get(&entry.id)))
        .collect();

    let parking = resolve_parking(curation, &tracker, evaluation_date, &mut warnings);
    let parked_ids: BTreeSet<String> = parking
        .parked
        .iter()
        .map(|item| item.id.clone())
        .chain(parking.expired.iter().map(|item| item.id.clone()))
        .collect();

    let mut in_flight: Vec<WorkItem> = inputs
        .beads
        .as_ref()
        .map(|beads| {
            beads
                .open
                .iter()
                .filter(|item| !parked_ids.contains(&item.id))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let mut blocked: Vec<WorkItem> = inputs
        .beads
        .as_ref()
        .map(|beads| {
            beads
                .blocked
                .iter()
                .filter(|item| !parked_ids.contains(&item.id))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let mut next: Vec<WorkItem> = inputs
        .beads
        .as_ref()
        .map(|beads| {
            beads
                .ready
                .iter()
                .filter(|item| !parked_ids.contains(&item.id) && !focus_ids.contains(&item.id))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    let mut needs_retriage = parking.expired;
    let mut parked = parking.parked;
    let mut recent: Vec<RecentEvent> = inputs
        .git
        .as_ref()
        .map(|git| git.recent.clone())
        .unwrap_or_default();

    sort_work_items(&mut in_flight);
    sort_work_items(&mut blocked);
    sort_work_items(&mut next);
    sort_work_items(&mut needs_retriage);
    sort_parked(&mut parked);
    sort_recent(&mut recent);

    let mut omitted_counts: BTreeMap<String, u32> = BTreeMap::new();
    focus = truncate("focus", focus, FOCUS_BOUND, &mut omitted_counts);
    recent = truncate("recent", recent, RECENT_BOUND, &mut omitted_counts);
    blocked = truncate("blocked", blocked, BLOCKED_BOUND, &mut omitted_counts);
    next = truncate("next", next, NEXT_BOUND, &mut omitted_counts);
    parked = truncate("parked", parked, PARKED_BOUND, &mut omitted_counts);

    let health = resolve_health(
        curation.and_then(|curation| curation.health.as_ref()),
        evaluation_date,
        &mut warnings,
    );
    sort_warnings(&mut warnings);

    let curation_stamp = curation_stamp(inputs.curation.as_ref());
    let sources = declared_sources(&inputs.sources);
    let purpose_lite = curation.and_then(|curation| curation.purpose_lite.clone());
    let head_sha = inputs.git.as_ref().and_then(|git| git.head_sha.clone());

    let mut brief = Brief {
        schema_version: SCHEMA_VERSION.to_string(),
        generated_at: inputs.generated_at,
        sources,
        curation: curation_stamp,
        purpose_lite,
        focus,
        in_flight,
        blocked,
        next,
        parked,
        needs_retriage,
        recent,
        health,
        truncation: Truncation {
            applied: !omitted_counts.is_empty(),
            omitted_counts,
        },
        head_sha,
        source_hash: None,
        warnings,
    };
    brief.source_hash = Some(crate::model::source_hash(&brief, brief.head_sha.as_deref()));
    brief
}

/// Tracker items addressed by identifier, for enriching curated entries.
struct Tracker<'a> {
    items: HashMap<&'a str, &'a WorkItem>,
}

impl<'a> Tracker<'a> {
    fn new(beads: Option<&'a BeadsSummary>) -> Self {
        let mut items = HashMap::new();
        if let Some(beads) = beads {
            for item in beads.open.iter().chain(beads.blocked.iter()) {
                items.entry(item.id.as_str()).or_insert(item);
            }
        }
        Self { items }
    }

    fn get(&self, id: &str) -> Option<&'a WorkItem> {
        self.items.get(id).copied()
    }
}

/// Parked entries split into still-parked and expired (needs re-triage).
#[derive(Default)]
struct Parking {
    parked: Vec<ParkedItem>,
    expired: Vec<WorkItem>,
}

fn resolve_parking(
    curation: Option<&CurationFile>,
    tracker: &Tracker<'_>,
    evaluation_date: NaiveDate,
    warnings: &mut Vec<Warning>,
) -> Parking {
    let mut parking = Parking::default();
    let Some(curation) = curation else {
        return parking;
    };
    for entry in &curation.parked {
        let resurface = match entry.resurface.parse::<Resurface>() {
            Ok(resurface) => resurface,
            Err(error) => {
                warnings.push(curation_warning(
                    "curation_invalid_resurface",
                    error.to_string(),
                ));
                continue;
            }
        };
        let confirmed_at = parse_date(entry.confirmed_at.as_deref());
        let Some(parked_at) = parse_date(entry.parked_at.as_deref()).or(confirmed_at) else {
            warnings.push(curation_warning(
                "curation_invalid_entry",
                format!(
                    "parked entry `{}` has no usable parked_at or confirmed_at date",
                    entry.id
                ),
            ));
            continue;
        };
        let confirmed_at = confirmed_at.unwrap_or(parked_at);
        let expired = matches!(&resurface, Resurface::Date(date) if *date <= evaluation_date);
        if expired {
            let mut item = tracker
                .get(&entry.id)
                .cloned()
                .unwrap_or_else(|| blank_work_item(&entry.id));
            item.reason = Some(entry.reason.clone());
            item.confirmed_at = Some(confirmed_at);
            parking.expired.push(item);
        } else {
            parking.parked.push(ParkedItem {
                id: entry.id.clone(),
                reason: entry.reason.clone(),
                parked_at,
                confirmed_at,
                resurface,
            });
        }
    }
    parking
}

fn resolve_health(
    health: Option<&CurationHealth>,
    evaluation_date: NaiveDate,
    warnings: &mut Vec<Warning>,
) -> Health {
    let owner = health.and_then(|health| health.owner.clone());
    let Some(health) = health else {
        return unknown_health(owner);
    };
    let status = parse_health_status(&health.status);
    if status.is_none() && !health.status.eq_ignore_ascii_case("unknown") {
        warnings.push(curation_warning(
            "curation_invalid_health",
            format!(
                "health status `{}` is not green, amber, red, or unknown",
                health.status
            ),
        ));
    }
    let confirmed_at = parse_date(health.confirmed_at.as_deref());
    let fresh = confirmed_at
        .is_some_and(|date| (evaluation_date - date).num_days() <= HUMAN_VERDICT_HORIZON_DAYS);
    match status {
        Some(status) if fresh && status != HealthStatus::Unknown => Health {
            status,
            basis: HealthBasis::Human,
            signals: Vec::new(),
            owner,
            reason: health.reason.clone(),
        },
        _ => unknown_health(owner),
    }
}

fn unknown_health(owner: Option<String>) -> Health {
    Health {
        status: HealthStatus::Unknown,
        basis: HealthBasis::None,
        signals: Vec::new(),
        owner,
        reason: None,
    }
}

fn parse_health_status(raw: &str) -> Option<HealthStatus> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "green" => Some(HealthStatus::Green),
        "amber" => Some(HealthStatus::Amber),
        "red" => Some(HealthStatus::Red),
        "unknown" => Some(HealthStatus::Unknown),
        _ => None,
    }
}

fn curation_stamp(state: Option<&CurationState>) -> CurationStamp {
    let Some(state) = state else {
        return CurationStamp {
            updated_at: None,
            confirmed_at: None,
        };
    };
    let curation = &state.curation;
    let health_date = curation
        .health
        .as_ref()
        .and_then(|health| parse_date(health.confirmed_at.as_deref()));
    let focus_dates = curation
        .focus
        .iter()
        .filter_map(|entry| parse_date(entry.confirmed_at.as_deref()));
    let parked_dates = curation
        .parked
        .iter()
        .filter_map(|entry| parse_date(entry.confirmed_at.as_deref()));
    let confirmed_at = health_date
        .into_iter()
        .chain(focus_dates)
        .chain(parked_dates)
        .max();
    CurationStamp {
        updated_at: state.updated_at,
        confirmed_at: confirmed_at.map(|date| date.and_time(NaiveTime::MIN).and_utc()),
    }
}

fn declared_sources(rows: &[SourceStatus]) -> Vec<SourceStatus> {
    SourceName::DECLARED
        .iter()
        .map(|name| {
            rows.iter()
                .find(|row| row.name == *name)
                .cloned()
                .unwrap_or(SourceStatus {
                    name: *name,
                    status: SourceState::Missing,
                    observed_at: None,
                    staleness: Staleness::Unknown,
                    detail: None,
                })
        })
        .collect()
}

fn sorted_focus_entries(curation: Option<&CurationFile>) -> Vec<&FocusEntry> {
    let mut entries: Vec<&FocusEntry> = curation
        .map(|curation| curation.focus.iter().collect())
        .unwrap_or_default();
    entries.sort_by(|left, right| {
        match (
            parse_date(left.focused_since.as_deref()),
            parse_date(right.focused_since.as_deref()),
        ) {
            (Some(left_since), Some(right_since)) => left_since
                .cmp(&right_since)
                .then_with(|| left.id.cmp(&right.id)),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => left.id.cmp(&right.id),
        }
    });
    entries
}

fn curated_work_item(entry: &FocusEntry, tracker_item: Option<&WorkItem>) -> WorkItem {
    let mut item = tracker_item
        .cloned()
        .unwrap_or_else(|| blank_work_item(&entry.id));
    item.reason = entry.reason.clone();
    item.confirmed_at = parse_date(entry.confirmed_at.as_deref());
    item
}

fn blank_work_item(id: &str) -> WorkItem {
    WorkItem {
        id: id.to_string(),
        title: None,
        status: None,
        priority: None,
        blocked_by: Vec::new(),
        age_days: None,
        reason: None,
        confirmed_at: None,
    }
}

fn parse_date(raw: Option<&str>) -> Option<NaiveDate> {
    raw.and_then(|raw| NaiveDate::parse_from_str(raw.trim(), "%Y-%m-%d").ok())
}

fn curation_warning(code: &str, detail: String) -> Warning {
    Warning {
        source: SourceName::Curation,
        code: code.to_string(),
        detail,
    }
}

fn sort_work_items(items: &mut [WorkItem]) {
    items.sort_by(|left, right| match (left.priority, right.priority) {
        (Some(left_priority), Some(right_priority)) => left_priority
            .cmp(&right_priority)
            .then_with(|| left.id.cmp(&right.id)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left.id.cmp(&right.id),
    });
}

fn sort_parked(items: &mut [ParkedItem]) {
    items.sort_by(|left, right| match (&left.resurface, &right.resurface) {
        (Resurface::Date(left_date), Resurface::Date(right_date)) => left_date
            .cmp(right_date)
            .then_with(|| left.id.cmp(&right.id)),
        (Resurface::Date(_), Resurface::Event(_)) => Ordering::Less,
        (Resurface::Event(_), Resurface::Date(_)) => Ordering::Greater,
        (Resurface::Event(left_event), Resurface::Event(right_event)) => left_event
            .cmp(right_event)
            .then_with(|| left.id.cmp(&right.id)),
    });
}

fn sort_recent(items: &mut [RecentEvent]) {
    items.sort_by(|left, right| {
        right
            .committed_at
            .cmp(&left.committed_at)
            .then_with(|| left.sha.cmp(&right.sha))
    });
}

fn sort_warnings(warnings: &mut [Warning]) {
    warnings.sort_by(|left, right| {
        left.source
            .declaration_order()
            .cmp(&right.source.declaration_order())
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.detail.cmp(&right.detail))
    });
}

fn truncate<T>(
    section: &str,
    items: Vec<T>,
    bound: usize,
    omitted_counts: &mut BTreeMap<String, u32>,
) -> Vec<T> {
    if items.len() <= bound {
        return items;
    }
    let dropped = items.len() - bound;
    omitted_counts.insert(
        section.to_string(),
        u32::try_from(dropped).unwrap_or(u32::MAX),
    );
    items.into_iter().take(bound).collect()
}
