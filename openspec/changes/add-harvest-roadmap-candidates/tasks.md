# Tasks: add-harvest-roadmap-candidates

Checklist maps stage ⑧ of the roadmap build order (`podras-1jj` proposal,
`podras-0k4` implementation) to its deliverable files and scenarios. Test
function names MUST equal the property generators in the roadmap-v1 delta; the
brief-v1 delta keeps the existing generator names because their tests already
exist and must keep passing. Delivered TDD: `tests/harvest_roadmap.rs` was
written first and watched failing (candidate-emitting scenarios red on the
empty candidate set), then the detector landed and the file went green.

## 0. Change artifacts

- [x] 0.1 (podras-1jj) Delta specs: `specs/brief-v1/spec.md` (MODIFIED
  "Harvest candidates are proposals, never decisions", evidence wording only,
  with the MODIFIED Constraints/Properties rows whose generator names are
  unchanged) and `specs/roadmap-v1/spec.md` (ADDED "Harvest proposes roadmap
  candidates, never decisions" with five scenarios, Constraints, and
  Properties). Scenario: all five roadmap-v1 scenarios.
- [x] 0.2 (podras-1jj) `proposal.md` and `design.md` under
  `openspec/changes/add-harvest-roadmap-candidates/`, recording the no-lens-
  bound choice, the shared-predicate extraction, and the D1-mirroring
  degradation. Scenario: —.
- [x] 0.3 (podras-1jj) Note recorded for the sibling stage-⑦ change: the
  roadmap-v1 Non-Goals section still lists the harvest extension and is
  cleaned up at archive time, not in this change. Scenario: —.

## 1. Shared predicates

- [x] 1.1 (podras-0k4) `src/roadmap/warnings.rs`: extract
  `circuit_breaker_overrun(admitted_at, appetite_weeks, evaluation_date)`,
  `epic_record(epic, records)`, and `epic_idle_days(record, evaluation_date)`
  as `pub(crate)` helpers; `bet_warnings` calls them so the lens semantics are
  byte-for-byte unchanged. Scenario: `retire_or_reshape_fires_per_trigger_with_terminal_bets_exempt` (and the
  existing lens tests, which must stay green).
- [x] 1.2 (podras-0k4) `src/roadmap.rs`: `pub(crate) use` the three helpers so
  harvest imports them without reaching into the private module. Scenario: —.

## 2. Candidate contract

- [x] 2.1 (podras-0k4) red: `tests/harvest_roadmap.rs` — the seven scenario
  tests (passed/absent/future `next_review`; each trigger; terminal exemptions;
  write-nothing; idempotent double run; malformed roadmap) fail on the empty
  roadmap candidate set. Scenario: all five.
- [x] 2.2 (podras-0k4) `src/harvest.rs`: `CandidateKind::{RetireOrReshape,
  IdentityReview}` (declaration order = render and payload order),
  `EvidenceKind::Roadmap`, `ALL` extended to five, `as_str` arms
  `retire_or_reshape` / `identity_review` / `roadmap`. Scenario:
  `harvest_roadmap_run_is_deterministic`.
- [x] 2.3 (podras-0k4) `src/harvest/roadmap.rs`: the detector — identity
  candidate iff `next_review` strictly earlier (absent/fresh ⇒ none), one
  retire-or-reshape per non-terminal bet over every bet in the file (no lens
  bound) with shared predicates and reasons joined in trigger order, lens
  status rank → `admitted_at` → id ordering with identity last, absent file ⇒
  nothing, malformed/unreadable ⇒ zero candidates + one
  `SourceName::Curation` warning with the domain code. Scenario: all five.
- [x] 2.4 (podras-0k4) `src/harvest.rs::assemble`: append the roadmap
  candidates after `sort_and_dedupe` (they carry their own contract order) and
  extend the warnings with at most the one degradation warning. Scenario:
  `harvest_roadmap_run_writes_nothing`, `malformed_roadmap_degrades_harvest`.
- [x] 2.5 (podras-0k4) `src/harvest/render.rs`: verified both the human
  narrative (grouped by `CandidateKind::ALL`, explicit `none` lines) and the
  `--json` envelope carry the new kinds with no code change. Scenario:
  `harvest_roadmap_run_is_deterministic`.

## 3. Gates

- [x] 3.1 (podras-0k4) `cargo test --test harvest_roadmap` green (7/7).
  Scenario: all five.
- [x] 3.2 (podras-0k4) Existing scoped tests stay green:
  `cargo test --test harvest` and the lens unit tests in `src/roadmap.rs` run
  with the shared helpers in place. Scenario:
  `harvest_roadmap_run_writes_nothing`.
- [x] 3.3 (podras-0k4) `openspec validate add-harvest-roadmap-candidates
  --strict` exits 0. Scenario: all five.

## Out of scope (declared exclusions)

- Roadmap-v1 Non-Goals cleanup — archive time, owned by the stage-⑦ sibling.
- Any write path from harvest; new flags; schema or `SignalKind` changes;
  per-repo threshold configuration.

## Scenario coverage ledger

| scenario | owning ticket | test owner |
|---|---|---|
| `identity_review_fires_only_when_next_review_passed` | podras-0k4 | `tests/harvest_roadmap.rs` (`harvest_runs_on_fixture_repo_with_passed_next_review`, `harvest_runs_on_fixtures_with_absent_and_future_next_review`) |
| `retire_or_reshape_fires_per_trigger_with_terminal_bets_exempt` | podras-0k4 | `tests/harvest_roadmap.rs` (`harvest_runs_on_fixture_repo_with_each_retire_or_reshape_trigger`, `harvest_runs_on_fixture_repo_with_terminal_bets_exempt`) |
| `harvest_roadmap_run_writes_nothing` | podras-0k4 | `tests/harvest_roadmap.rs` (`harvest_roadmap_run_writes_nothing`) |
| `harvest_roadmap_run_is_deterministic` | podras-0k4 | `tests/harvest_roadmap.rs` (`harvest_roadmap_run_twice_on_fixture_repo`) |
| `malformed_roadmap_degrades_harvest` | podras-0k4 | `tests/harvest_roadmap.rs` (`harvest_runs_on_fixture_repo_with_malformed_roadmap`) |

## 6. Review fixes (Rule of 5, post-implementation)

- [x] 6.1 RF-1/RF-2 red→green: unadmitted-shaping-bet test, fallback sort key, admission-gated circuit breaker (`src/harvest/roadmap.rs`, `tests/harvest_roadmap.rs`)
- [x] 6.2 RF-3: module doc states the admission gating
- [x] 6.3 RF-4: `bet admit` prints a note when replacing an existing epic (`src/verbs/bet.rs`)
- [x] 6.4 RF-5: staged property rows for the absent/future and unadmitted-bet tests; invariant row notes the gating
