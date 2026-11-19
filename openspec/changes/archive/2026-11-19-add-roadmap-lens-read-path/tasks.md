# Tasks: add-roadmap-lens-read-path

Checklist maps stage ⑥ of the roadmap build order (`podras-5y8`) to its
deliverable files and the scenario it owns. Test function names MUST equal
scenario ids; each contract's cargo filter is the scenario id. All items
in sections 0–3 are delivered: `cargo test`, `cargo fmt --check`,
`cargo clippy -D warnings`, `ah check --changes add-roadmap-lens-read-path
--run-tests` (27 passed), `openspec validate --strict`, and `spk lint` are
green. Section 4 stays out of scope for this change; section 5 records the
Rule-of-5 review fixes and re-runs those gates at landing.

## 0. Change artifacts and gates

- [x] 0.1 (podras-5y8) Delta spec at `openspec/changes/add-roadmap-lens-read-path/specs/roadmap-v1/spec.md`: seven ADDED requirements for lens separation, the state file shape and closed lifecycles, identity staleness, no-go boundaries, advisory warnings, the additive bounded envelope, and the read-only agent surface. Scenario: — (eight scenarios cover the change).
- [x] 0.2 (podras-5y8) `proposal.md`, `design.md`, `tasks.md` under `openspec/changes/add-roadmap-lens-read-path/`. Scenario: —.
- [x] 0.3 (podras-5y8) Eight contracts at `.espectacular/roadmap-v1/<scenario>.toml`, deployed alongside the `roadmap-v1` spec because implementation and tests are green; `[tests]` cargo flags = scenario id: `lens-separate-from-now-lens`, `roadmap-file-shape`, `bet-lifecycle-enums`, `identity-staleness-never-fresh`, `no-go-violation-flags-only`, `roadmap-warnings-not-verdict`, `roadmap-envelope-additive-bounded`, `roadmap-read-only`. Scenario: all eight.
- [x] 0.4 (gate) `openspec validate add-roadmap-lens-read-path --strict` exits 0, `openspec validate --specs --strict` exits 0, and `spk lint openspec/specs` exits 0. Scenario: `corpus_self_lint` (deployed v1 contract, rerun here).
- [x] 0.5 (gate) `ah check` reports 0 findings and `ah check --changes add-roadmap-lens-read-path --run-tests` reports 27 passed. Scenario: all eight.

## 1. State file, model, and validation

- [x] 1.1 red: `src/roadmap/file.rs::roadmap_file_shape` — fixture file with `[identity]`, `[[bets]]`, `[[no_gos]]` round-trips through parse+write-back; duplicate bet id, non-slug id, 61-character title, and `appetite_weeks = 0` aggregate into one `roadmap_invalid`; unknown key is `roadmap_parse_error`.
- [x] 1.2 red: `src/roadmap/file.rs::bet_lifecycle_enums` — every `status` and `horizon` member round-trips; committed bet without `review_or_kill`, deferred bet without `revive_when`, and non-deferred bet with `revive_when` are each rejected with `roadmap_invalid`; unknown enum members are `roadmap_parse_error`.
- [x] 1.3 `src/model.rs`: `Horizon`, `BetStatus`, `Enforcement`, `RoadmapWarningKind` (declaration order = sort order), `IdentityView`, `BetView`, `NoGoView`, `ReviewDue`, `RoadmapWarning`, `RoadmapView`, plus `ROADMAP_BETS_BOUND = 5` / `ROADMAP_NO_GOS_BOUND = 5`; `Brief.roadmap: Option<RoadmapView>` with `skip_serializing_if`; `SCHEMA_VERSION` stays `"brief.v1"`.
- [x] 1.4 `src/roadmap/file.rs`: `ROADMAP_REL_PATH`, the `#[serde(deny_unknown_fields)]` file structs (dates as `String`), `load(repo) -> Result<Option<RoadmapFile>>` (absent ⇒ `Ok(None)`), manual `^[a-z0-9][a-z0-9-]*$` slug check, date parsing via `crate::curation::parse_human_date`, and aggregate semantic validation joined by `"; "` with the `docs/design.md` decision-17 suggestion.
- [x] 1.5 `src/lib.rs` + `src/brief/sections.rs`: register `pub mod roadmap;` with its re-exports; the section `Brief { .. }` literal sets `roadmap: None` so every existing caller keeps v1 behaviour.

## 2. Warnings and tracker records

- [x] 2.1 red: `src/roadmap/warnings.rs::roadmap_warnings_not_verdict` — a building bet past `appetite_weeks * 1.25`, a committed bet without an epic, a closed epic under an active bet, a 15-day-idle epic, a `now` bet with no open epic, a deferred bet with an open epic, a passed `review_or_kill`, and an identity without `next_review` each emit their code; `health.signals` stays v1; exit 0; no bytes change.
- [x] 2.2 red: `src/roadmap/warnings.rs::no_go_violation_flags_only` — `auto-close` matches a bead titled `auto-close` but not `autoclose`; one candidate per no-go per bead with `subject = no_go.id` and `detail = "<bead> matched `<term>`"`; empty `match_terms` never fires; nothing is written.
- [x] 2.3 red: `src/roadmap/warnings.rs::identity_staleness_never_fresh` — absent `next_review` ⇒ `unknown` + `identity_review_overdue` (subject `identity`) and never `fresh`; passed ⇒ `stale` and also the advisory (revised in 5.1); equal to the evaluation date ⇒ `fresh`.
- [x] 2.4 `src/roadmap/warnings.rs`: `CIRCUIT_BREAKER_FACTOR: f64 = 1.25` documented as `roadmap.circuit_breaker_factor`; the nine predicates; whole-token matching over bead titles and labels; 14-day `bet_no_transition` window reused from `crate::health::links::SIGNAL_HORIZON_DAYS`; earliest outstanding review per source; warning sort by code, subject, detail.
- [x] 2.5 `src/readers/beads.rs`: `IssueRecord { id, title, status, labels, updated_at, open }` and `read_records(repo) -> Vec<IssueRecord>` (best-effort: absent/unreadable ⇒ empty), `RawIssue.labels` with `#[serde(default)]`, `open = !matches!(status, Some("closed" | "done"))`. `BeadsSummary` and existing reader signatures unchanged.

## 3. Assembler, CLI, and renders

- [x] 3.1 red: `tests/roadmap.rs::lens_separate_from_now_lens` — a fixture with a curation focus and a `now` bet; roadmap rows and focus/parked rows are disjoint, no bet renders as focus and no focus entry renders as a bet, both commands exit 0.
- [x] 3.2 red: `tests/roadmap.rs::roadmap_envelope_additive_bounded` — no file ⇒ no `roadmap` object; zero-bet file ⇒ present lens (`bets: []`) with identity row; 7 bets + 7 no-gos ⇒ exactly five of each in declared order with `truncation.applied = true` and `omitted_counts` naming bets/no_gos; `schema_version = "brief.v1"`; two runs at one `--as-of` identical apart from `generated_at`.
- [x] 3.3 red: `tests/roadmap.rs::roadmap_read_only` — `podras roadmap --json --as-of` and `--write --as-of` twice each: payload equality after removing `generated_at`, `ROADMAP.json` renders differ only in `generated_at`, reads change no repository bytes, a corrupted file yields `roadmap_parse_error`/`roadmap_invalid` from `podras roadmap` while `podras brief` omits `roadmap` and appends one warning (revised in 5.3), and two `podras init` runs are byte-identical with the two roadmap AGENTS.md lines present.
- [x] 3.4 `src/roadmap.rs`: `assemble(&BriefInputs, &RoadmapFile, &[IssueRecord], NaiveDate) -> RoadmapView` (pure: no clock, filesystem, or env; the unused `BriefInputs` argument is dropped in 5.2), status rank then `admitted_at` then `id`, no-go `id` order, bounds applied after sort, `review_due` as the earliest outstanding review per source, `attach(Brief, Option<RoadmapView>) -> Brief` setting `roadmap` and recomputing `source_hash`.
- [x] 3.5 `src/commands.rs` + `src/main.rs` + `src/render/roadmap.rs` + `src/write.rs`: `podras roadmap [--json] [--as-of <date>] [--write]` reusing the brief read path and envelope; `roadmap_missing` when the verb runs alone without the file; human narrative renders enums and constraints with explicit `none` lines; `write_roadmap_renders_to` emits `ROADMAP.md` / `ROADMAP.json` with the brief's generated-artifact semantics.
- [x] 3.6 Docs: README documents `podras roadmap`, the state file's human ownership, and the advisory-only warning rule; `src/verbs/init.rs` managed block gains exactly the two decision-25 lines and a second `podras init` stays byte-identical.

## 4. Out of scope for this change (declared exclusions — not delivered here; no beads tickets exist as of 2026-11-19)

- 4.1 (stage ⑦) Curation verbs `bet add|admit|defer|kill`; no verb writes `.podras/roadmap.toml` here. Scenario: —.
- 4.2 (stage ⑧) Harvest extension: `identity_review` when `next_review` passes, `retire_or_reshape` as a candidate row, never a mutation. Scenario: —.
- 4.3 A second parked list, or any roadmap-side copy of curation's parked entries. Scenario: —.
- 4.4 Brief narrative change, new `SignalKind` variants, `schema_version` bump, portfolio/org renders, per-repo roadmap thresholds. Scenario: —.

## 5. Rule-of-5 review fixes

A Rule-of-5 review of commit `7d8c12c` produced fixes D1–D14, recorded in
`design.md` under "Review fixes (Rule of 5)". Sections 0–3 above describe the
change as first delivered; the items below amend it. Gate results for this
section are re-run at landing and recorded in 5.9.

- [x] 5.1 (add-roadmap-lens-read-path) Semantics: `bet_epic_missing` covers `committed`, `building`, and `shipped` (D4); `horizon_epic_mismatch` only non-terminal bets (D5); `review_due` and every warning are computed after the 5/5 bounds over retained bets only (D6); `identity_review_overdue` also fires when `next_review` has passed, its detail naming which case (D7); no-go tokens accept Unicode alphanumerics plus `-`/`_` with U+2010..U+2015 and U+2212 normalized to `-` on both text and term (D8). Scenario: `roadmap_warnings_not_verdict`, `no_go_violation_flags_only`, `identity_staleness_never_fresh`.
- [x] 5.2 (add-roadmap-lens-read-path) Errors and signatures: a non-`NotFound` IO failure loading the state file is `Error::Domain { code: "roadmap_unreadable", … }` with a suggestion, not `Error::Io` (D9); `roadmap::assemble(&RoadmapFile, &[IssueRecord], NaiveDate)` and `write::write_roadmap_renders_to(&Path, &Brief)` drop their unused parameters (D10, D11); `src/roadmap.rs` re-exports only `load`, `RoadmapFile`, `ROADMAP_REL_PATH` and the file structs drop `JsonSchema` (D12); `brief::sort_warnings_in_place` delegates to a `pub(crate)` `ordering::sort_warnings` with the key unchanged (D13). Scenario: `roadmap_read_only`.
- [x] 5.3 (add-roadmap-lens-read-path) CLI degradation: `podras brief` omits `roadmap`, appends exactly one `SourceName::Curation` warning (`code` = the domain code or `roadmap_unreadable`, `detail` prefixed `roadmap: `), keeps the five `sources[]` rows, and exits 0 on a malformed or unreadable file; `podras roadmap` keeps the hard domain error (D1, D2). Scenario: `roadmap_read_only`.
- [x] 5.4 (add-roadmap-lens-read-path) Lens parked column: `ROADMAP.md`'s "Not now (parked)" renders curation parked bullets first, then `needs_retriage` entries as `- <id> — <title or none> (needs re-triage: resurface trigger expired)`; the parked omission marker is unchanged (D3). Scenario: `lens_separate_from_now_lens`.
- [x] 5.5 (add-roadmap-lens-read-path) Specs: D1–D8 are stated in the deployed `roadmap-v1` spec and the delta; their requirement and scenario sections are byte-identical and every requirement paragraph is at most 500 characters, with constraints and scenarios carrying the detail. Scenario: all eight.
- [x] 5.6 (add-roadmap-lens-read-path) Tests: the eight contracts' tests cover the amended semantics, including brief degradation and the parked `needs_retriage` line. Scenario: all eight.
- [x] 5.7 (add-roadmap-lens-read-path) Docs: README gains the "Roadmap state" section (example `.podras/roadmap.toml`, `ROADMAP.md` lens-only versus `ROADMAP.json` full envelope) and the `roadmap_missing` suggestion points at `README.md#roadmap-state` (D14). Scenario: —.
- [x] 5.8 (gate) Archive-duplication check: re-run `openspec archive add-roadmap-lens-read-path --yes` on a scratch copy of the landed tree — `Specs already in sync; no files changed`, 7 requirements before and after (measured false positive: no duplication or dropped text, so no fix was made). Scenario: `corpus_self_lint`.
- [x] 5.9 (gate) Re-run after every section-5 edit, recorded: `cargo fmt --check` clean; `cargo clippy --all-targets -D warnings` clean; `cargo test` 24/24 test binaries (49 lib unit tests); `ah check` 0 findings; `ah check --changes add-roadmap-lens-read-path --run-tests` 27 passed; `openspec validate --strict` exits 0 for the change and for specs; `spk lint openspec/specs` exits 0. Scenario: all eight.

## Scenario coverage ledger

| scenario | owning ticket | contract | test owner |
|---|---|---|---|
| `lens_separate_from_now_lens` | podras-5y8 | `lens-separate-from-now-lens.toml` | `tests/roadmap.rs` |
| `roadmap_file_shape` | podras-5y8 | `roadmap-file-shape.toml` | `tests/roadmap.rs` |
| `bet_lifecycle_enums` | podras-5y8 | `bet-lifecycle-enums.toml` | `tests/roadmap.rs` |
| `identity_staleness_never_fresh` | podras-5y8 | `identity-staleness-never-fresh.toml` | `tests/roadmap.rs` |
| `no_go_violation_flags_only` | podras-5y8 | `no-go-violation-flags-only.toml` | `tests/roadmap.rs` |
| `roadmap_warnings_not_verdict` | podras-5y8 | `roadmap-warnings-not-verdict.toml` | `tests/roadmap.rs` |
| `roadmap_envelope_additive_bounded` | podras-5y8 | `roadmap-envelope-additive-bounded.toml` | `tests/roadmap.rs` |
| `roadmap_read_only` | podras-5y8 | `roadmap-read-only.toml` | `tests/roadmap.rs` |