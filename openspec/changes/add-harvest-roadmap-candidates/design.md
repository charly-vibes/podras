# Design: add-harvest-roadmap-candidates

## Context

Harvest (brief-v1) extracts typed candidates from whisper notes, git, and the
spec store behind a tracker-vocabulary gate, sorts them into a total order, and
renders both a human narrative and a `--json` envelope. The roadmap lens
(roadmap-v1) owns `.podras/roadmap.toml`, its six-status bet lifecycle, and
nine advisory warning predicates computed over the retained (post-bound) bets
against the full beads export. Stage ⑧ joins the two read paths without
letting either mutate anything.

## Goals / Non-Goals

- Goals: two advisory candidate kinds driven by the lens's own predicates;
  evidence provenance for the new source; degradation that never fails a run;
  deterministic order; render parity.
- Non-Goals: any write path (no curation, tracker, roadmap, artifact); no
  `SignalKind` changes; no lens bound on candidates; no roadmap-v1 Non-Goals
  edit (sibling stage-⑦ change owns that section until archive time).

## Decisions

1. **One new detector module, `src/harvest/roadmap.rs`.** Mirrors the shape of
   `notes.rs`/`git.rs`/`specs.rs`: a pure function of the repository path and
   the evaluation date returning candidates plus an optional degradation
   warning. It sits outside the tracker-vocabulary gate — an identity review
   is independent of whether beads export ids, and gating it would silently
   drop candidates on an unreadable export (a false negative harvest's
   rationale forbids less than a false positive, but here the predicate needs
   no vocabulary at all).
2. **Predicates are extracted, not reimplemented.**
   `circuit_breaker_overrun`, `epic_record`, and `epic_idle_days` move into
   `src/roadmap/warnings.rs` as `pub(crate)` helpers that `bet_warnings` now
   calls, and `src/roadmap.rs` re-exports them. If the 1.25 factor or the
   fourteen-day horizon ever changes, the lens and harvest change together.
3. **Triggers apply to all non-terminal bets, unlike the lens warnings.** The
   lens fires its epic predicates only for active (committed|building) bets
   because the lens is a per-status view; harvest asks a different question —
   "should this bet still exist?" — which a shaping bet with a closed epic
   answers yes to. Documented in the module header and the delta.
4. **Candidates bypass the generic sort, appended in contract order.** The
   detectors' total order (kind, first ref, summary) would scramble the
   bet ordering the locked contract names. Roadmap candidates keep their own
   order (status rank, `admitted_at`, id; identity last) and are appended
   after `sort_and_dedupe`. `CandidateKind` declares `RetireOrReshape` before
   `IdentityReview` so declaration order, the human render groups, and the
   payload order all agree.
5. **Degradation mirrors the brief's D1 philosophy.** `load` already returns
   three domain codes. Absent ⇒ nothing at all; any error ⇒ zero roadmap
   candidates plus one `SourceName::Curation` warning with the domain code and
   a `roadmap: `-prefixed detail, exactly like the brief's lens degradation,
   so tooling sees one shape. The run still exits 0.
6. **Evidence ref is the file, not a per-table anchor.** Roadmap evidence cites
   `.podras/roadmap.toml` plainly: the file is the atomic human decision
   record, and a fragile `#bets.<id>` anchor would resolve against a file
   humans re-edit. Resolution for the idempotence property is file existence.
7. **Render needs no code change beyond the enum.** `render.rs` iterates
   `CandidateKind::ALL`, so both the narrative groups and `--json` carry the
   new kinds automatically; the explicit `none` lines cover the empty case.
8. **No main/envelope/schema changes.** Harvest is on-demand only; the schema
   version stays `harvest.v1` because the candidate union is additive and the
   envelope already carries an open candidates array.

## Risks / Trade-offs

- Wider trigger scope (non-terminal vs active) can fire for a shaping bet a
  human considers fine — accepted: harvest may over-prompt, never act.
- Reading the beads export a second time (after the vocabulary load) costs one
  small file read per harvest run — accepted for module isolation.
- Shared helpers grow the warnings.rs public surface within the crate —
  contained by `pub(crate)` and the `src/roadmap.rs` re-export point.

## Migration Plan

None: additive candidate kinds in an on-demand verb. Existing fixtures have no
roadmap file, so every existing harvest scenario passes unchanged.

## Open Questions

None — the locked contract (design decisions 4/21/22/23, review fix D7) and
the review-fix list settle scope, order, and degradation.

## Delta Constraints and Properties (archive-time rows)

The openspec delta format does not carry Constraints/Properties sections, so
the rows below belong to the two delta requirements and are pasted verbatim
from this change; they belong in the deployed specs when this change archives
(the brief-v1 rows keep their existing generator names).

### brief-v1 "Harvest candidates are proposals, never decisions"

| id | kind | expr | traces_to |
|----|------|------|-----------|
| harvest_candidates_never_decisions | invariant | `every harvest candidate carries kind + summary + ≥1 git-, notes-, or roadmap-sourced evidence ref; the evidence union extends to {kind: git\|notes\|roadmap, ref}; curation and tracker export bytes stay unchanged; focus/parked stay unchanged; exit == 0 on degraded sources` | [[brief.v1.sources_declared]] |
| harvest_idempotent_provenance | invariant | `two harvest runs over unchanged inputs at one --as-of emit equal candidate sets in the same order; every evidence ref resolves to an existing git object, note entry, or roadmap state file; no repository file changes between runs` | [[brief.v1.sources_declared]] |

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| harvest_candidates_never_decisions | unit | [[brief.v1.sources_declared]] | `harvest_run_on_fixture_repo_with_orphan_commit_and_unlinked_spec_change()` | `every candidate carries kind + summary + ≥1 git-, notes-, or roadmap-sourced evidence ref ∧ curation and tracker export bytes unchanged ∧ focus/parked unchanged ∧ exit == 0` |
| harvest_idempotent_provenance | unit | [[brief.v1.sources_declared]] | `harvest_run_twice_on_fixture_repo()` | `candidate sets equal and ordered across runs ∧ every evidence ref resolves to an existing git object, note entry, or roadmap state file ∧ no repository file changes between runs` |

### roadmap-v1 "Harvest proposes roadmap candidates, never decisions"

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| harvest_candidates_proposed_not_decided | invariant | `harvest emits advisory roadmap candidates — identity_review only when identity.next_review is strictly earlier than the evaluation date (absent ⇒ unknown ⇒ none; equal or later ⇒ fresh ⇒ none; at most one) and retire_or_reshape once per non-terminal bet (shaping|committed|building|deferred; shipped and killed exempt) tripping the shared circuit-breaker (evaluation − admitted_at > appetite_weeks×7×1.25, admitted bets only — a bet without an admitted_at has no appetite clock and stays reachable through the epic predicates), idle-epic (> fourteen days), or closed-epic predicate, the summary naming the bet id and every triggering reason; candidates compute over every bet in the file (the lens 5-bet bound does not apply) ordered by lens status rank, then admitted_at, then id — unadmitted bets sort by last_reviewed in that rank — with the identity candidate last; a run writes nothing and no candidate auto-kills, auto-promotes, or rewrites state` | [[roadmap.v1]] |
| harvest_roadmap_degrade_not_fail | invariant | `an absent roadmap.toml yields no roadmap candidates and no warning; a malformed or unreadable roadmap.toml degrades harvest to zero roadmap candidates with one warning carrying the domain code while the run exits 0 — a broken human-owned state file never blocks a read-only run` | [[roadmap.v1]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| identity_review_fires_only_when_next_review_passed | unit | [[roadmap.v1.harvest_candidates_proposed_not_decided]] | `harvest_runs_on_fixture_repo_with_passed_next_review()` | `passed next_review ⇒ exactly one identity_review candidate with roadmap evidence naming the passed date and ordered last` |
| identity_review_absent_or_future_next_review | unit | [[roadmap.v1.harvest_candidates_proposed_not_decided]] | `harvest_runs_on_fixtures_with_absent_and_future_next_review()` | `absent next_review ⇒ no identity_review candidate ∧ future next_review ⇒ none` |
| unadmitted_shaping_bet_still_fires_epic_triggers | unit | [[roadmap.v1.harvest_candidates_proposed_not_decided]] | `harvest_runs_on_fixture_repo_with_unadmitted_shaping_bet()` | `a shaping bet without admitted_at whose epic is closed yields exactly one retire_or_reshape candidate naming the bet id and bet_epic_closed_not_retired, with roadmap evidence` |
| retire_or_reshape_fires_per_trigger_with_terminal_bets_exempt | unit | [[roadmap.v1.harvest_candidates_proposed_not_decided]] | `harvest_runs_on_fixture_repo_with_each_retire_or_reshape_trigger()` | `each non-terminal trigger yields one candidate naming bet id + reason ∧ candidates order by lens status rank, then admitted_at, then id` |
| terminal_bets_exempt | unit | [[roadmap.v1.harvest_candidates_proposed_not_decided]] | `harvest_runs_on_fixture_repo_with_terminal_bets_exempt()` | `shipped and killed bets tripping every trigger shape yield zero retire_or_reshape candidates` |
| harvest_roadmap_run_writes_nothing | unit | [[roadmap.v1.harvest_candidates_proposed_not_decided]] | `harvest_roadmap_run_writes_nothing()` | `roadmap.toml, curation.toml, and .beads bytes identical before and after a run` |
| harvest_roadmap_run_is_deterministic | unit | [[roadmap.v1.harvest_candidates_proposed_not_decided]] | `harvest_roadmap_run_twice_on_fixture_repo()` | `two runs at one --as-of emit identical payloads apart from generated_at ∧ both roadmap kinds present ∧ identity_review ordered last ∧ evidence refs resolve` |
| malformed_roadmap_degrades_harvest | unit | [[roadmap.v1.harvest_roadmap_degrade_not_fail]] | `harvest_runs_on_fixture_repo_with_malformed_roadmap()` | `exit == 0 ∧ zero roadmap candidates ∧ one degradation warning ∧ no repository byte changes` |

## Review fixes (Rule of 5)

A Rule-of-5 review of the delivered commit (`7655e7a`) produced five findings,
all applied by this change. The TypeSafe verification pass verified the two
HIGH findings (`jev-1.14.0`, confidence 0.96 and 0.93 against the 0.8 gate);
MEDIUM/LOW findings are self-reported.

- **RF-1 — an unadmitted bet keeps its epic triggers (CORR-001, verified).**
  `retire_or_reshape` dropped the whole candidate for any bet whose
  `admitted_at` was absent — exactly what `bet add` writes — even when the
  closed- or idle-epic predicate fired. The circuit breaker now ages admitted
  bets only, the epic predicates reach every non-terminal bet, and the sort
  key falls back to `last_reviewed` (renamed `sort_key`; the order stays
  total and deterministic).
- **RF-2 — the fixture models `bet add` output (EDGE-001, verified).**
  `unadmitted_bet()` writes a shaping bet with no `admitted_at`, and the new
  `harvest_runs_on_fixture_repo_with_unadmitted_shaping_bet` test asserts the
  epic trigger still fires.
- **RF-3 — the module doc states the gating (CLAR-001).** The doc claimed the
  triggers "apply to every non-terminal status" without the admission caveat;
  it now says the circuit breaker ages admitted bets only.
- **RF-4 — the admit notice (CORR-002).** `podras bet admit` prints a note
  when it replaces an epic the shaping bet already carried.
- **RF-5 — the generator ledger (CLAR-002).** The absent/future branch and the
  unadmitted-bet test each get their own property row, so archive-time
  generator-to-test matching stays one-to-one.
- Not applied: CORR-003 (checking the epic id against the tracker export on
  admit) — it adds tracker reads to a human verb, contract surface beyond the
  locked decisions; recorded as a CONSIDER for a deliberate later change.
