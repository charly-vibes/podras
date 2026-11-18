//! Purpose: define the shared, typed brief model produced by `podras brief`.
//!
//! Responsibilities:
//! - declare every enum and struct that crosses the brief --json boundary;
//! - derive serde and JSON Schema so the model is the single schema source;
//! - re-export the semantic payload and source hash used to compare briefs
//!   ignoring volatile time; hashing itself lives in `hash`.
//!
//! Rationale: the envelope is a contract, not prose. Keeping every field typed
//! and shared by readers, health, rendering, and tests prevents parallel
//! representations and keeps `podras schema` honest.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Schema identifier carried by every brief payload.
pub const SCHEMA_VERSION: &str = "brief.v1";

/// Render bound for the focus section.
pub const FOCUS_BOUND: usize = 1;
/// Render bound for the recent section.
pub const RECENT_BOUND: usize = 5;
/// Render bound for the blocked section.
pub const BLOCKED_BOUND: usize = 5;
/// Render bound for the next section.
pub const NEXT_BOUND: usize = 5;
/// Render bound for the parked section.
pub const PARKED_BOUND: usize = 5;

/// Declared data source for a brief reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceName {
    /// Beads issue tracker export.
    Beads,
    /// Git history and working tree.
    Git,
    /// OpenSpec change store.
    Openspec,
    /// Whisper session notes.
    Whisper,
    /// Human-curated `.podras/curation.toml`.
    Curation,
}

impl SourceName {
    /// Every declared source exactly once, in fixed declaration order.
    ///
    /// Freshness threshold coverage tests read this array, so a new variant
    /// without a declared horizon is visible to them.
    pub const ALL: [SourceName; 5] = [
        SourceName::Beads,
        SourceName::Git,
        SourceName::Openspec,
        SourceName::Whisper,
        SourceName::Curation,
    ];

    /// Fixed declaration order: beads, git, openspec, whisper, curation.
    ///
    /// Sources and warnings both sort by this order so two runs over identical
    /// inputs emit identical rows.
    pub const DECLARED: [SourceName; 5] = SourceName::ALL;

    /// Index of this source in the fixed declaration order.
    pub fn declaration_order(self) -> usize {
        match self {
            SourceName::Beads => 0,
            SourceName::Git => 1,
            SourceName::Openspec => 2,
            SourceName::Whisper => 3,
            SourceName::Curation => 4,
        }
    }
}

/// Whether a declared source could be read at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SourceState {
    /// The source exists and was read.
    Present,
    /// The source does not exist for this repository.
    Missing,
    /// The source exists but could not be read or parsed.
    Unreadable,
}

/// Per-source freshness judgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Staleness {
    /// Observed recently enough for this source.
    Fresh,
    /// Observed, but past this source's freshness horizon.
    Stale,
    /// No usable timestamp; freshness cannot be judged.
    Unknown,
}

/// One source row in the brief envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SourceStatus {
    /// Declared source name.
    pub name: SourceName,
    /// Present / missing / unreadable.
    pub status: SourceState,
    /// Timestamp the source data was observed at, when known.
    #[schemars(with = "Option<String>")]
    pub observed_at: Option<DateTime<Utc>>,
    /// Per-source staleness verdict; never project-wide.
    pub staleness: Staleness,
    /// Optional per-source detail or warning text.
    pub detail: Option<String>,
}

/// Health verdict values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    /// A fresh human verdict with no firing signal.
    Green,
    /// At least one signal fires, or a human marked the project amber.
    Amber,
    /// A human marked the project red.
    Red,
    /// No human verdict and no decisive signal; silence is not green.
    Unknown,
}

/// What the health verdict is based on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HealthBasis {
    /// Fresh human verdict from curation.
    Human,
    /// Computed signals.
    Signals,
    /// Neither: verdict unknown.
    None,
}

/// Named health signals; each firing signal carries its own reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SignalKind {
    /// The curated focus has not been confirmed recently.
    StaleFocus,
    /// More focused items than `max_focus`.
    WipOverCap,
    /// No meaningful project progress for too long.
    StuckProject,
    /// The curated focus item is closed in the tracker.
    FocusClosed,
    /// A parked item passed its resurface trigger.
    ParkedExpired,
    /// Commits exist that no issue claims.
    OrphanCommits,
    /// Spec changes changed without a linked bead.
    SpecChangeWithoutBead,
}

/// A firing health signal with a human-readable reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Signal {
    /// Which signal fired.
    pub kind: SignalKind,
    /// Why it fired.
    pub reason: String,
}

/// Layered health verdict plus its evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Health {
    /// Final verdict.
    pub status: HealthStatus,
    /// Which layer decided the verdict.
    pub basis: HealthBasis,
    /// Signals that fired while computing the verdict.
    pub signals: Vec<Signal>,
    /// Human owner of the verdict, when declared.
    pub owner: Option<String>,
    /// Optional human explanation.
    pub reason: Option<String>,
}

/// Curation timestamps exposed by the brief.
///
/// `updated_at` is the file mtime of the last verb write; `confirmed_at` is the
/// newest human `confirmed_at` among health and focus/parked entries. Keeping
/// them separate is what makes "unchanged" differ from "unconfirmed".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CurationStamp {
    /// When the curation file last changed.
    #[schemars(with = "Option<String>")]
    pub updated_at: Option<DateTime<Utc>>,
    /// When a human last confirmed curation state.
    #[schemars(with = "Option<String>")]
    pub confirmed_at: Option<DateTime<Utc>>,
}

/// Declared truncation of bounded sections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Truncation {
    /// True when any bound removed at least one item.
    pub applied: bool,
    /// Per-section count of removed items; only sections that dropped items.
    pub omitted_counts: BTreeMap<String, u32>,
}

/// A unit of work read from a source or curated as intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkItem {
    /// Stable item identifier.
    pub id: String,
    /// Human-readable title, when known.
    pub title: Option<String>,
    /// Tracker status as reported, when known.
    pub status: Option<String>,
    /// Tracker priority, lower is more urgent; absent sorts last.
    pub priority: Option<u8>,
    /// Identifiers blocking this item.
    pub blocked_by: Vec<String>,
    /// Days since the item was created, when the source reports it.
    pub age_days: Option<u32>,
    /// Curation reason when the item is curated.
    pub reason: Option<String>,
    /// Human confirmation date when the item is curated.
    #[schemars(with = "Option<String>")]
    pub confirmed_at: Option<NaiveDate>,
}

/// A parked item, owned exclusively by curation state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ParkedItem {
    /// Parked item identifier.
    pub id: String,
    /// Why it is parked.
    pub reason: String,
    /// When it was parked.
    #[schemars(with = "String")]
    pub parked_at: NaiveDate,
    /// When a human last confirmed the park.
    #[schemars(with = "String")]
    pub confirmed_at: NaiveDate,
    /// Mandatory date-or-event resurface trigger.
    #[schemars(with = "String")]
    pub resurface: Resurface,
}

/// The trigger that returns a parked item to attention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resurface {
    /// Return on or after this calendar date.
    Date(NaiveDate),
    /// Return when a named event fires.
    Event(String),
}

/// Why a resurface trigger could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResurfaceParseError {
    /// The raw trigger text that failed to parse.
    pub raw: String,
}

impl fmt::Display for ResurfaceParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "resurface trigger `{}` is neither YYYY-MM-DD nor event:<name>",
            self.raw
        )
    }
}

impl std::error::Error for ResurfaceParseError {}

impl fmt::Display for Resurface {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Resurface::Date(date) => write!(formatter, "{date}"),
            Resurface::Event(name) => write!(formatter, "event:{name}"),
        }
    }
}

impl FromStr for Resurface {
    type Err = ResurfaceParseError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        if let Some(name) = raw.strip_prefix("event:") {
            if name.is_empty() {
                return Err(ResurfaceParseError {
                    raw: raw.to_string(),
                });
            }
            return Ok(Resurface::Event(name.to_string()));
        }
        NaiveDate::parse_from_str(raw, "%Y-%m-%d")
            .map(Resurface::Date)
            .map_err(|_| ResurfaceParseError {
                raw: raw.to_string(),
            })
    }
}

impl Serialize for Resurface {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Resurface {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

/// A recent git change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RecentEvent {
    /// Commit sha.
    pub sha: String,
    /// Commit timestamp.
    #[schemars(with = "String")]
    pub committed_at: DateTime<Utc>,
    /// One-line commit subject.
    pub subject: String,
    /// Outcome the commit delivered, when known.
    pub outcome: Option<String>,
}

/// A stable per-source warning; warnings are never truncated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Warning {
    /// Source that produced the warning.
    pub source: SourceName,
    /// Stable machine-readable code.
    pub code: String,
    /// Human-readable detail.
    pub detail: String,
}

/// Render bound for the roadmap's bet list.
pub const ROADMAP_BETS_BOUND: usize = 5;
/// Render bound for the roadmap's no-go list.
pub const ROADMAP_NO_GOS_BOUND: usize = 5;

/// Time horizon a bet is committed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Horizon {
    /// The work being done right now.
    Now,
    /// The next bet after the current one.
    Next,
    /// Explicitly not now; never a requirement.
    Later,
}

/// Lifecycle state of a bet.
///
/// Declaration order is the state machine's own order, not the render order;
/// `roadmap` sorts by status rank, which promotes `building` and `committed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BetStatus {
    /// Being shaped; not yet admitted.
    Shaping,
    /// Admitted with a dated decision.
    Committed,
    /// Active and in flight.
    Building,
    /// Paused with a mandatory revive trigger.
    Deferred,
    /// Delivered.
    Shipped,
    /// Abandoned, with a reason.
    Killed,
}

/// Declared enforcement tier of a no-go.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Enforcement {
    /// A real automated check exists.
    Automated,
    /// A human review is the enforcement.
    HumanReview,
}

/// Identity row of the roadmap lens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct IdentityView {
    /// Link to the prose that states what the project ought to be.
    pub path: String,
    /// When a human last reviewed the identity.
    #[schemars(with = "String")]
    pub last_reviewed: NaiveDate,
    /// When the next review is due; absent means unscheduled.
    #[schemars(with = "Option<String>")]
    pub next_review: Option<NaiveDate>,
    /// Staleness decided by `next_review` alone.
    pub staleness: Staleness,
}

/// One bet as the lens renders it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BetView {
    /// Slug identifier, unique among bets.
    pub id: String,
    /// Imperative title, at most 60 characters.
    pub title: String,
    /// What changes if the bet lands.
    pub outcome: String,
    /// Declared horizon; authoritative, never computed.
    pub horizon: Horizon,
    /// Appetite in weeks; the circuit breaker escalates past 1.25x.
    pub appetite_weeks: u32,
    /// Human owner.
    pub owner: String,
    /// The single tracker epic backing the bet, when declared.
    pub epic: Option<String>,
    /// Lifecycle state.
    pub status: BetStatus,
    /// Admission decision date.
    #[schemars(with = "String")]
    pub admitted_at: NaiveDate,
    /// When a human last reviewed the bet.
    #[schemars(with = "String")]
    pub last_reviewed: NaiveDate,
    /// Dated review-or-kill decision; required while committed or building.
    #[schemars(with = "Option<String>")]
    pub review_or_kill: Option<NaiveDate>,
    /// Trigger that revives a deferred bet.
    pub revive_when: Option<String>,
    /// Why the bet was killed, when it was.
    pub killed_reason: Option<String>,
}

/// One no-go as the lens renders it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct NoGoView {
    /// Slug identifier, unique among no-gos.
    pub id: String,
    /// Why the boundary exists.
    pub reason: String,
    /// What the boundary covers.
    pub scope: String,
    /// Declared enforcement tier.
    pub enforcement: Enforcement,
    /// What reversal requires; never an edit of this entry.
    pub revoke_by: String,
    /// Whole-token terms matched against bead titles and labels.
    pub match_terms: Vec<String>,
}

/// Earliest outstanding review per source; never a computed deadline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ReviewDue {
    /// Identity's declared next review, when declared.
    #[schemars(with = "Option<String>")]
    pub identity: Option<NaiveDate>,
    /// Earliest `review_or_kill` among active bets.
    #[schemars(with = "Option<String>")]
    pub roadmap: Option<NaiveDate>,
}

/// Named roadmap warnings; each is advisory, never a verdict.
///
/// Declaration order is the deterministic sort order of the warning list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoadmapWarningKind {
    /// An active bet outlived its appetite.
    BetAgeExceedsAppetite,
    /// An active bet passed 1.25x its appetite.
    CircuitBreakerDue,
    /// A committed, building, or shipped bet has no tracker epic.
    BetEpicMissing,
    /// A bet is active while its epic is closed.
    BetEpicClosedNotRetired,
    /// A bet's epic has not changed in fourteen days.
    BetNoTransition,
    /// A non-terminal bet's horizon contradicts the epic's state.
    HorizonEpicMismatch,
    /// An open bead matches a no-go's terms.
    NoGoViolationCandidate,
    /// Identity's next review is absent or has passed.
    IdentityReviewOverdue,
    /// An active bet's `review_or_kill` date has passed.
    RoadmapReviewOverdue,
}

/// One roadmap warning with its subject and evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RoadmapWarning {
    /// Which warning fired.
    pub code: RoadmapWarningKind,
    /// Bet, no-go, or `identity`.
    pub subject: String,
    /// Why it fired.
    pub detail: String,
}

/// The roadmap lens: identity, bets, boundaries, and advisory warnings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RoadmapView {
    /// What the project ought to be, and how stale that review is.
    pub identity: IdentityView,
    /// Bounded, deterministically ordered bets.
    pub bets: Vec<BetView>,
    /// Bounded, deterministically ordered no-gos.
    pub no_gos: Vec<NoGoView>,
    /// Earliest outstanding review per source.
    pub review_due: ReviewDue,
    /// Advisory warnings over the retained bets and no-gos; never truncated,
    /// and never naming an entry the bounds dropped.
    pub warnings: Vec<RoadmapWarning>,
    /// Declared truncation of `bets` and `no_gos`; parked omissions stay in the
    /// brief's own `truncation`.
    pub truncation: Truncation,
}

/// The complete typed brief envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Brief {
    /// Envelope schema identifier.
    pub schema_version: String,
    /// Evaluation clock pinning every date-sensitive decision.
    #[schemars(with = "String")]
    pub as_of: NaiveDate,
    /// Generation timestamp; the only volatile field.
    #[schemars(with = "String")]
    pub generated_at: DateTime<Utc>,
    /// Declared source rows in fixed declaration order.
    pub sources: Vec<SourceStatus>,
    /// Curation timestamps.
    pub curation: CurationStamp,
    /// One-line project purpose.
    pub purpose_lite: Option<String>,
    /// Curated focus.
    pub focus: Vec<WorkItem>,
    /// Work currently in flight.
    pub in_flight: Vec<WorkItem>,
    /// Blocked or waiting items.
    pub blocked: Vec<WorkItem>,
    /// Ready candidates, excluding focus and parked items.
    pub next: Vec<WorkItem>,
    /// Currently parked items.
    pub parked: Vec<ParkedItem>,
    /// Parked items whose date trigger has passed.
    pub needs_retriage: Vec<WorkItem>,
    /// Recent commit events, newest first.
    pub recent: Vec<RecentEvent>,
    /// Layered health verdict.
    pub health: Health,
    /// Declared truncation of bounded sections.
    pub truncation: Truncation,
    /// Repository HEAD sha, when known.
    pub head_sha: Option<String>,
    /// SHA-256 of the canonical semantic payload.
    pub source_hash: Option<String>,
    /// Stable per-source warnings; never truncated.
    pub warnings: Vec<Warning>,
    /// Roadmap lens, present only when `.podras/roadmap.toml` loads and
    /// validates; a malformed or unreadable file leaves it absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roadmap: Option<RoadmapView>,
}

pub use crate::hash::{semantic_payload, source_hash};
