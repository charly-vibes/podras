//! Purpose: pinned roadmap fixtures and accessors shared by the lens tests.
//!
//! Responsibilities:
//! - build `.podras/roadmap.toml` documents from typed bet/no-go specs;
//! - seed the tracker rows, curation, and commits every scenario reads;
//! - drive the real binary and the library lens from the same repository;
//! - expose exact-section assertions for the roadmap narrative.
//!
//! Rationale: the lens is a separate render path over hand-edited, human-owned
//! state. Fixtures therefore keep every date pinned to one `--as-of` instant
//! and every document either wholly valid or deliberately, narrowly invalid, so
//! a failure names the contract line it broke.

#![allow(dead_code)]

use std::path::Path;

use chrono::NaiveDate;
use podras::brief::BriefInputs;
use podras::model::{Brief, RoadmapView};
use podras::readers::beads::read_records;
use podras::readers::{ReadContext, collect};
use podras::roadmap::file::{ROADMAP_REL_PATH, RoadmapFile, load};
use podras::roadmap::{assemble, attach};
use serde_json::Value;

pub use crate::common::{BeadRow, TempRepo};

/// Pinned evaluation date for every roadmap test.
pub(crate) const AS_OF: &str = "2026-11-18";
/// Epic backing the canonical bet, open and recently updated.
pub(crate) const EPIC_ID: &str = "podras-epic";
/// Epic that was closed while its bet is still active.
pub(crate) const CLOSED_EPIC_ID: &str = "podras-epic-closed";
/// Epic that has not moved for longer than the fourteen-day horizon.
pub(crate) const IDLE_EPIC_ID: &str = "podras-epic-idle";
/// Curated focus item; must stay out of the lens.
pub(crate) const FOCUS_ID: &str = "podras-a";
/// Curated parked item; the lens renders it as "not now".
pub(crate) const PARKED_ID: &str = "podras-f";
/// Curated parked item whose resurface trigger has already expired.
pub(crate) const EXPIRED_ID: &str = "podras-g";
/// Canonical bet id.
pub(crate) const BET_ID: &str = "brief-v2";
/// Canonical no-go id.
pub(crate) const NO_GO_ID: &str = "no-tracker-writes";

/// Parse a fixture date.
pub(crate) fn date(raw: &str) -> NaiveDate {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").expect("fixture date is valid")
}

/// Run the real binary inside a fixture repository.
pub(crate) fn podras_in(repo: &TempRepo, args: &[&str]) -> assert_cmd::assert::Assert {
    assert_cmd::Command::cargo_bin("podras")
        .expect("podras binary is built for tests")
        .args(args)
        .current_dir(repo.root())
        .env("HOME", repo.root())
        .assert()
}

/// Run the binary and return stdout without its trailing newline.
pub(crate) fn stdout(repo: &TempRepo, args: &[&str]) -> String {
    let output = podras_in(repo, args).success().get_output().stdout.clone();
    String::from_utf8(output)
        .expect("podras output is UTF-8")
        .trim_end()
        .to_string()
}

/// Run a JSON verb pinned to the fixture date and return the genesis envelope.
fn envelope(repo: &TempRepo, verb: &str) -> Value {
    serde_json::from_str(&stdout(repo, &[verb, "--json", "--as-of", AS_OF]))
        .expect("verb --json emits a JSON envelope")
}

/// Run `roadmap --json` and return the genesis envelope.
pub(crate) fn roadmap_envelope(repo: &TempRepo) -> Value {
    envelope(repo, "roadmap")
}

/// Run `roadmap --json` and return its typed payload.
pub(crate) fn roadmap_data(repo: &TempRepo) -> Value {
    roadmap_envelope(repo)["data"].clone()
}

/// Run `brief --json` and return the genesis envelope.
pub(crate) fn brief_envelope(repo: &TempRepo) -> Value {
    envelope(repo, "brief")
}

/// Run `brief --json` and return its typed payload.
pub(crate) fn brief_data(repo: &TempRepo) -> Value {
    brief_envelope(repo)["data"].clone()
}

/// Run the roadmap narrative pinned to the fixture date.
pub(crate) fn roadmap_narrative(repo: &TempRepo) -> String {
    stdout(repo, &["roadmap", "--as-of", AS_OF])
}

/// The brief's reader output for the fixture repository.
pub(crate) fn brief_inputs(repo: &TempRepo) -> BriefInputs {
    let context = ReadContext {
        repo_root: repo.root().to_path_buf(),
        as_of: Some(date(AS_OF)),
        branch: None,
    };
    collect(&context).expect("every source collects for the fixture")
}

/// Attach a lens to a brief at the library boundary.
pub(crate) fn attached(repo: &TempRepo, with_lens: bool) -> Brief {
    let brief = podras::brief::assemble(&brief_inputs(repo));
    let view = if with_lens { Some(view(repo)) } else { None };
    attach(brief, view)
}

/// Assemble the lens from the repository's validated file and tracker export.
pub(crate) fn view(repo: &TempRepo) -> RoadmapView {
    let evaluation = date(AS_OF);
    let file = load_required(repo.root());
    let records = read_records(repo.root());
    assemble(&file, &records, evaluation)
}

/// Load the roadmap file that a fixture must have written.
pub(crate) fn load_required(repo: &Path) -> RoadmapFile {
    load(repo)
        .expect("fixture roadmap parses and validates")
        .expect("fixture roadmap exists")
}

/// Load a roadmap document written into a throwaway directory.
pub(crate) fn load_text(toml: &str) -> podras::Result<Option<RoadmapFile>> {
    let dir = tempfile::tempdir().expect("create temporary roadmap directory");
    std::fs::create_dir_all(dir.path().join(".podras")).expect("create fixture .podras");
    std::fs::write(dir.path().join(ROADMAP_REL_PATH), toml).expect("write fixture roadmap");
    load(dir.path())
}

/// The stable domain code of a rejected roadmap, when the error carries one.
pub(crate) fn error_code(error: &podras::Error) -> Option<&str> {
    match error {
        podras::Error::Domain { code, .. } => Some(*code),
        _ => None,
    }
}

/// Contents of one `## ` section, without the heading and trailing blanks.
pub(crate) fn section<'a>(markdown: &'a str, heading: &str) -> &'a str {
    let marker = format!("## {heading}\n\n");
    let start = markdown
        .find(&marker)
        .unwrap_or_else(|| panic!("render has section `{heading}`: {markdown}"))
        + marker.len();
    let rest = &markdown[start..];
    let end = rest.find("\n## ").unwrap_or(rest.len());
    rest[..end].trim_end_matches('\n')
}

/// Every `## ` heading, in render order.
pub(crate) fn headings(markdown: &str) -> Vec<&str> {
    markdown
        .lines()
        .filter_map(|line| line.strip_prefix("## "))
        .collect()
}

/// Assert a section renders exactly the expected non-blank lines.
///
/// The lens states its bets bound once, after the final bet group, so the last
/// section may carry one `(<n> more omitted)` accounting line after its
/// content. Nothing else may follow the declared lines.
pub(crate) fn assert_section(markdown: &str, heading: &str, expected: &[&str]) {
    let body = section(markdown, heading);
    let actual: Vec<&str> = body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    let mut content = actual.clone();
    if content.last().is_some_and(|line| is_omission(line)) {
        content.pop();
    }
    assert_eq!(
        content, expected,
        "section `{heading}` must render exactly {expected:?} plus at most one omission line: {markdown}"
    );
}

/// True for the `(<n> more omitted)` accounting line.
pub(crate) fn is_omission(line: &str) -> bool {
    let Some(count) = line
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(" more omitted)"))
    else {
        return false;
    };
    !count.is_empty() && count.chars().all(|digit| digit.is_ascii_digit())
}

/// Ids of a JSON array of objects.
pub(crate) fn array_ids(items: &Value) -> Vec<String> {
    items
        .as_array()
        .expect("value is an array")
        .iter()
        .map(|item| {
            item["id"]
                .as_str()
                .expect("array element carries an id")
                .to_string()
        })
        .collect()
}

/// Codes of the lens's warnings, in payload order.
pub(crate) fn warning_codes(data: &Value) -> Vec<String> {
    data["roadmap"]["warnings"]
        .as_array()
        .expect("roadmap payload carries warnings")
        .iter()
        .map(|warning| {
            warning["code"]
                .as_str()
                .expect("warning carries a code")
                .to_string()
        })
        .collect()
}

/// One warning's subject and detail, joined for provenance assertions.
pub(crate) fn warning_text(warning: &Value) -> String {
    format!(
        "{} {}",
        warning["subject"].as_str().unwrap_or_default(),
        warning["detail"].as_str().unwrap_or_default()
    )
}

/// Sorted object keys of a JSON value.
pub(crate) fn object_keys(value: &Value) -> Vec<String> {
    let mut keys: Vec<String> = value
        .as_object()
        .expect("value is a JSON object")
        .keys()
        .cloned()
        .collect();
    keys.sort();
    keys
}

/// One bet entry of a roadmap document.
#[derive(Clone)]
pub(crate) struct BetSpec<'a> {
    /// Slug id.
    pub id: &'a str,
    /// Imperative title.
    pub title: &'a str,
    /// What lands if the bet succeeds.
    pub outcome: &'a str,
    /// Declared horizon.
    pub horizon: &'a str,
    /// Lifecycle status.
    pub status: &'a str,
    /// Admission date.
    pub admitted_at: &'a str,
    /// Last human review date.
    pub reviewed_at: &'a str,
    /// Appetite in weeks.
    pub appetite_weeks: u32,
    /// Tracker epic, when declared.
    pub epic: Option<&'a str>,
    /// Dated review-or-kill decision.
    pub review_or_kill: Option<&'a str>,
    /// Revive trigger for deferred bets.
    pub revive_when: Option<&'a str>,
    /// Why the bet was killed.
    pub killed_reason: Option<&'a str>,
}

impl<'a> BetSpec<'a> {
    /// A bet with the fixture defaults; dates are pinned, nothing warns.
    pub(crate) fn new(id: &'a str, title: &'a str, status: &'a str, horizon: &'a str) -> Self {
        Self {
            id,
            title,
            outcome: "The bet lands",
            horizon,
            status,
            admitted_at: "2026-11-01",
            reviewed_at: "2026-11-01",
            appetite_weeks: 6,
            epic: None,
            review_or_kill: None,
            revive_when: None,
            killed_reason: None,
        }
    }

    /// Set the outcome sentence.
    pub(crate) fn with_outcome(mut self, outcome: &'a str) -> Self {
        self.outcome = outcome;
        self
    }

    /// Link exactly one tracker epic.
    pub(crate) fn with_epic(mut self, epic: &'a str) -> Self {
        self.epic = Some(epic);
        self
    }

    /// Set the admission and review dates together.
    pub(crate) fn admitted(mut self, admitted_at: &'a str, reviewed_at: &'a str) -> Self {
        self.admitted_at = admitted_at;
        self.reviewed_at = reviewed_at;
        self
    }

    /// Set the appetite in weeks.
    pub(crate) fn with_appetite(mut self, weeks: u32) -> Self {
        self.appetite_weeks = weeks;
        self
    }

    /// Set the dated review-or-kill decision.
    pub(crate) fn with_review_or_kill(mut self, date: &'a str) -> Self {
        self.review_or_kill = Some(date);
        self
    }

    /// Set the revive trigger.
    pub(crate) fn with_revive_when(mut self, trigger: &'a str) -> Self {
        self.revive_when = Some(trigger);
        self
    }

    /// Set the killed reason.
    pub(crate) fn with_killed_reason(mut self, reason: &'a str) -> Self {
        self.killed_reason = Some(reason);
        self
    }

    /// Render the `[[bets]]` table.
    pub(crate) fn render(&self) -> String {
        let mut out = format!(
            "[[bets]]\nid = \"{}\"\ntitle = \"{}\"\noutcome = \"{}\"\nhorizon = \"{}\"\n\
             appetite_weeks = {}\nowner = \"charly\"\n",
            self.id, self.title, self.outcome, self.horizon, self.appetite_weeks,
        );
        if let Some(epic) = self.epic {
            out.push_str(&format!("epic = \"{epic}\"\n"));
        }
        out.push_str(&format!(
            "status = \"{}\"\nadmitted_at = \"{}\"\nlast_reviewed = \"{}\"\n",
            self.status, self.admitted_at, self.reviewed_at,
        ));
        if let Some(date) = self.review_or_kill {
            out.push_str(&format!("review_or_kill = \"{date}\"\n"));
        }
        if let Some(trigger) = self.revive_when {
            out.push_str(&format!("revive_when = \"{trigger}\"\n"));
        }
        if let Some(reason) = self.killed_reason {
            out.push_str(&format!("killed_reason = \"{reason}\"\n"));
        }
        out.push('\n');
        out
    }
}

/// Render one `[[no_gos]]` table.
pub(crate) fn no_go(
    id: &str,
    reason: &str,
    scope: &str,
    enforcement: &str,
    revoke_by: &str,
    terms: &[&str],
) -> String {
    let terms: Vec<String> = terms.iter().map(|term| format!("\"{term}\"")).collect();
    format!(
        "[[no_gos]]\nid = \"{id}\"\nreason = \"{reason}\"\nscope = \"{scope}\"\n\
         enforcement = \"{enforcement}\"\nrevoke_by = \"{revoke_by}\"\n\
         match_terms = [{}]\n\n",
        terms.join(", "),
    )
}

/// A no-go with the human-review defaults and the given terms.
pub(crate) fn review_no_go(id: &str, reason: &str, terms: &[&str]) -> String {
    no_go(
        id,
        reason,
        "all verbs",
        "human_review",
        "a new design decision",
        terms,
    )
}

/// Render the `[identity]` table, optionally with a scheduled next review.
pub(crate) fn identity(next_review: Option<&str>) -> String {
    let mut out =
        String::from("[identity]\npath = \"README.md#why\"\nlast_reviewed = \"2026-11-01\"\n");
    if let Some(next_review) = next_review {
        out.push_str(&format!("next_review = \"{next_review}\"\n"));
    }
    out.push('\n');
    out
}

/// Join an identity, bets, and no-gos into one document.
pub(crate) fn roadmap_file(identity: &str, bets: &[String], no_gos: &[String]) -> String {
    format!("{identity}{}{}", bets.join(""), no_gos.join(""))
}

/// Curation selecting a focus, parking an item, and leaving health unknown.
pub(crate) const CURATION: &str = r#"purpose_lite = "podras: project status for a cold return"
max_focus = 1
publish_whisper = false

[health]
status = "unknown"
owner = "@sasha"
confirmed_at = "2026-11-15"

[[focus]]
id = "podras-a"
reason = "finish the brief read path"
focused_since = "2026-11-10"
confirmed_at = "2026-11-15"

[[parked]]
id = "podras-f"
reason = "harvest moves to v1.1"
resurface = "2027-01-01"
parked_at = "2026-11-10"
confirmed_at = "2026-11-10"
"#;

/// The canonical lens: one building bet, one automated no-go, fresh identity.
pub(crate) fn full_roadmap() -> String {
    let bet = BetSpec::new(BET_ID, "Ship the roadmap lens", "building", "now")
        .with_outcome("A cold reader predicts the next bet from the lens alone")
        .with_epic(EPIC_ID)
        .with_review_or_kill("2026-12-30");
    let no_go = no_go(
        NO_GO_ID,
        "podras never mutates the tracker",
        "all verbs",
        "automated",
        "a new design decision",
        &["auto-close", "auto-promote"],
    );
    roadmap_file(&identity(Some("2027-05-18")), &[bet.render()], &[no_go])
}

/// A present lens with an identity row and zero bets.
pub(crate) fn empty_roadmap() -> String {
    roadmap_file(&identity(Some("2027-05-18")), &[], &[])
}

/// Seven bets spanning every status, admitted in rank-defeating order.
pub(crate) fn seven_bets_roadmap() -> String {
    let bets = [
        BetSpec::new("bet-build", "Build the lens", "building", "now")
            .with_epic(EPIC_ID)
            .with_review_or_kill("2026-12-30")
            .admitted("2026-11-10", "2026-11-10"),
        BetSpec::new("bet-commit", "Commit the lens", "committed", "next")
            .with_epic(EPIC_ID)
            .with_review_or_kill("2026-12-30")
            .admitted("2026-11-09", "2026-11-09"),
        BetSpec::new("bet-defer", "Defer the verbs", "deferred", "next")
            .with_revive_when("when the lens ships")
            .admitted("2026-11-08", "2026-11-08"),
        BetSpec::new("shape-b", "Shape the harvest", "shaping", "later")
            .admitted("2026-11-07", "2026-11-07"),
        BetSpec::new("shape-a", "Shape the verbs", "shaping", "next")
            .admitted("2026-11-07", "2026-11-07"),
        BetSpec::new("bet-ship", "Ship the cold start", "shipped", "later")
            .admitted("2026-11-01", "2026-11-01"),
        BetSpec::new("bet-kill", "Kill the graph view", "killed", "later")
            .with_killed_reason("no value-prop evidence")
            .admitted("2026-10-01", "2026-10-01"),
    ];
    let rendered: Vec<String> = bets.iter().map(|bet| bet.render()).collect();
    let no_gos: Vec<String> = (1..=7)
        .map(|index| {
            review_no_go(
                &format!("no-go-{index:02}"),
                "never mutate the tracker",
                &[],
            )
        })
        .collect();
    roadmap_file(&identity(Some("2027-05-18")), &rendered, &no_gos)
}

/// Four bets that fire eight of the nine advisory codes between them.
pub(crate) fn warnings_roadmap() -> String {
    let aged = BetSpec::new("bet-aged", "Review the read path", "committed", "next")
        .with_epic(EPIC_ID)
        .with_appetite(2)
        .with_review_or_kill("2026-12-15");
    let breaker = BetSpec::new("bet-breaker", "Build the graph view", "building", "now")
        .with_epic(CLOSED_EPIC_ID)
        .with_appetite(1)
        .with_review_or_kill("2026-10-01")
        .admitted("2026-09-01", "2026-09-01");
    let idle = BetSpec::new("bet-stale-epic", "Wire the harvest", "building", "next")
        .with_epic(IDLE_EPIC_ID)
        .with_review_or_kill("2026-12-30")
        .admitted("2026-11-10", "2026-11-10");
    let missing = BetSpec::new("bet-missing-epic", "Admit the verbs", "committed", "next")
        .with_review_or_kill("2026-12-30")
        .admitted("2026-11-10", "2026-11-10");
    let bets: Vec<String> = [aged, breaker, idle, missing]
        .iter()
        .map(|bet| bet.render())
        .collect();
    roadmap_file(&identity(None), &bets, &[])
}

/// Curation with the canonical parked item plus one expired resurface trigger.
pub(crate) fn curation_with_expired(resurface: &str) -> String {
    format!(
        "{CURATION}\n[[parked]]\nid = \"{EXPIRED_ID}\"\nreason = \"harvest waits for demand\"\n\
         resurface = \"{resurface}\"\nparked_at = \"2026-11-01\"\nconfirmed_at = \"2026-11-01\"\n"
    )
}

/// A committed lens with one still-parked and one expired parked entry.
///
/// `title` seeds the expired entry's tracker row; `None` leaves the entry
/// untracked, which is the `none` half of the lens's `<title or none>` field.
pub(crate) fn expired_park_fixture(title: Option<&str>) -> TempRepo {
    let repo = TempRepo::new();
    let mut rows = vec![BeadRow {
        id: PARKED_ID,
        title: "Harvest v1.1",
        status: "open",
        labels: &[],
        updated_at: "2026-11-10T00:00:00Z",
    }];
    if let Some(title) = title {
        rows.push(BeadRow {
            id: EXPIRED_ID,
            title,
            status: "open",
            labels: &[],
            updated_at: "2026-11-10T00:00:00Z",
        });
    }
    repo.beads_jsonl_rows(&rows);
    repo.curation(&curation_with_expired("2026-11-01"));
    repo.roadmap(&empty_roadmap());
    repo.commit("expired park fixture");
    repo
}

/// Terminal bets plus the two statuses that still owe an epic.
///
/// `shipped-closed` and `killed-closed` are terminal while their epic is closed;
/// `deferred-closed` is non-terminal with the same epic. Only the last may fire
/// `horizon_epic_mismatch`, and only `building`/`shipped` may report a missing
/// epic — `killed` is exempt.
pub(crate) fn terminal_bets_roadmap() -> String {
    let bets = [
        BetSpec::new("shipped-closed", "Ship the graph view", "shipped", "later")
            .with_epic(CLOSED_EPIC_ID),
        BetSpec::new("killed-closed", "Kill the graph view", "killed", "later")
            .with_epic(CLOSED_EPIC_ID)
            .with_killed_reason("no value-prop evidence"),
        BetSpec::new(
            "deferred-closed",
            "Defer the graph view",
            "deferred",
            "later",
        )
        .with_epic(CLOSED_EPIC_ID)
        .with_revive_when("when the graph asks for itself"),
        BetSpec::new("building-no-epic", "Build the lens", "building", "next")
            .with_review_or_kill("2026-12-30"),
        BetSpec::new("shipped-no-epic", "Ship the lens", "shipped", "later"),
    ];
    let rendered: Vec<String> = bets.iter().map(|bet| bet.render()).collect();
    roadmap_file(&identity(Some("2027-05-18")), &rendered, &[])
}

/// Seven active bets whose review dates all passed, with the earliest review
/// dates on the two bets the 5/5 bound drops last.
///
/// This proves truncation precedes `review_due` and the warnings: a
/// pre-truncation computation would report `2026-09-01` and name `bet-06` and
/// `bet-07`, which the lens never shows.
pub(crate) fn review_passed_roadmap() -> String {
    let bets = [
        BetSpec::new("bet-01", "Hold the first bet", "building", "now")
            .with_epic(EPIC_ID)
            .with_appetite(12)
            .with_review_or_kill("2026-10-10")
            .admitted("2026-10-01", "2026-10-01"),
        BetSpec::new("bet-02", "Hold the second bet", "building", "now")
            .with_epic(EPIC_ID)
            .with_appetite(12)
            .with_review_or_kill("2026-10-11")
            .admitted("2026-10-02", "2026-10-02"),
        BetSpec::new("bet-03", "Hold the third bet", "building", "now")
            .with_epic(EPIC_ID)
            .with_appetite(12)
            .with_review_or_kill("2026-10-12")
            .admitted("2026-10-03", "2026-10-03"),
        BetSpec::new("bet-04", "Hold the fourth bet", "building", "now")
            .with_epic(EPIC_ID)
            .with_appetite(12)
            .with_review_or_kill("2026-10-13")
            .admitted("2026-10-04", "2026-10-04"),
        BetSpec::new("bet-05", "Hold the fifth bet", "building", "now")
            .with_epic(EPIC_ID)
            .with_appetite(12)
            .with_review_or_kill("2026-10-14")
            .admitted("2026-10-05", "2026-10-05"),
        BetSpec::new("bet-06", "Hold the sixth bet", "building", "now")
            .with_epic(EPIC_ID)
            .with_appetite(12)
            .with_review_or_kill("2026-09-01")
            .admitted("2026-10-06", "2026-10-06"),
        BetSpec::new("bet-07", "Hold the seventh bet", "building", "now")
            .with_epic(EPIC_ID)
            .with_appetite(12)
            .with_review_or_kill("2026-09-02")
            .admitted("2026-10-07", "2026-10-07"),
    ];
    let rendered: Vec<String> = bets.iter().map(|bet| bet.render()).collect();
    roadmap_file(&identity(Some("2027-05-18")), &rendered, &[])
}

/// Tracker export for the canonical lens: one open, recently moved epic.
pub(crate) fn seed_full(repo: &TempRepo) {
    repo.beads_jsonl_rows(&[
        BeadRow {
            id: EPIC_ID,
            title: "Roadmap lens epic",
            status: "open",
            labels: &[],
            updated_at: "2026-11-17T00:00:00Z",
        },
        BeadRow {
            id: FOCUS_ID,
            title: "Brief read path",
            status: "open",
            labels: &[],
            updated_at: "2026-11-10T00:00:00Z",
        },
        BeadRow {
            id: PARKED_ID,
            title: "Harvest v1.1",
            status: "open",
            labels: &[],
            updated_at: "2026-11-10T00:00:00Z",
        },
    ]);
}

/// Tracker export with the open, closed, and idle epics the warnings use.
pub(crate) fn seed_epic_states(repo: &TempRepo) {
    repo.beads_jsonl_rows(&[
        BeadRow {
            id: EPIC_ID,
            title: "Roadmap lens epic",
            status: "open",
            labels: &[],
            updated_at: "2026-11-17T00:00:00Z",
        },
        BeadRow {
            id: CLOSED_EPIC_ID,
            title: "Graph view epic",
            status: "closed",
            labels: &[],
            updated_at: "2026-11-17T00:00:00Z",
        },
        BeadRow {
            id: IDLE_EPIC_ID,
            title: "Harvest epic",
            status: "open",
            labels: &[],
            updated_at: "2026-10-01T00:00:00Z",
        },
        BeadRow {
            id: FOCUS_ID,
            title: "Brief read path",
            status: "open",
            labels: &[],
            updated_at: "2026-11-10T00:00:00Z",
        },
        BeadRow {
            id: PARKED_ID,
            title: "Harvest v1.1",
            status: "open",
            labels: &[],
            updated_at: "2026-11-10T00:00:00Z",
        },
    ]);
}

/// A committed repository carrying the canonical lens.
pub(crate) fn full_fixture() -> TempRepo {
    let repo = TempRepo::new();
    seed_full(&repo);
    repo.curation(CURATION);
    repo.roadmap(&full_roadmap());
    repo.commit("roadmap: canonical lens fixture");
    repo
}

/// A committed repository carrying the given roadmap document.
pub(crate) fn fixture_with_roadmap(document: &str) -> TempRepo {
    let repo = TempRepo::new();
    seed_full(&repo);
    repo.curation(CURATION);
    repo.roadmap(document);
    repo.commit("roadmap: lens fixture");
    repo
}

/// A committed repository with the given identity row and no bets.
pub(crate) fn fixture_with_identity(next_review: Option<&str>) -> TempRepo {
    fixture_with_roadmap(&roadmap_file(&identity(next_review), &[], &[]))
}

/// The exact bet bullet lines the canonical lens must render.
pub(crate) fn canonical_bet_lines() -> [&'static str; 3] {
    [
        "- brief-v2 — Ship the roadmap lens (status: building; horizon: now; appetite: 6w; owner: charly; epic: podras-epic)",
        "  - outcome: A cold reader predicts the next bet from the lens alone",
        "  - admitted: 2026-11-01; last reviewed: 2026-11-01; review or kill: 2026-12-30",
    ]
}
