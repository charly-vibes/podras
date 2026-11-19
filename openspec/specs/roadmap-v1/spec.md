---
id: roadmap.v1
kind: intent
statement: "WHEN a human returns cold or an agent starts cold in a git repository, THE roadmap-v1 capability SHALL render a separate read-only lens over identity, admitted bets, and binding no-gos from the git-committed .podras/roadmap.toml, with stored horizons, closed lifecycles, bounded deterministic output, and advisory warnings that never become a health verdict."
---

# roadmap-v1 Specification

## Purpose

podras keeps two questions apart. The brief's now-lens answers *what is
happening in this project now*; the roadmap lens answers the commitment layer —
WHO the work is for (identity), WHY each bet was admitted (outcome, appetite,
owner, one tracker epic), and WHAT is refused (no-gos with a reason, an
enforcement tier, and a revoke path). Separation is the first design decision
(`docs/design.md` decision 16): the lens renders identity + bets + boundaries in
its own render path and never mixes them into the brief's focus or parked
columns. `focus` remains curation intent; a bet is a commitment with appetite,
outcome, and one tracker epic. Neither is inferred from the other (corpus C1;
brief v1's `curation_beats_inference`).

The capability is stage ⑥ of the roadmap build order (decision 26) and is
strictly read-only: `podras roadmap [--json] [--as-of <date>] [--write]` reuses
the brief read path, emits the same envelope, and writes committed `ROADMAP.md`
/ `ROADMAP.json` renders via `--write`. State lives in `.podras/roadmap.toml`, a
git-committed sibling of `curation.toml` that verbs and humans write and agents
never edit (decision 17, enforced textually by the AGENTS.md managed block,
decision 25). No verb writes it in this stage: `bet add|admit|defer|kill` are
stage ⑦ and the harvest extension is stage ⑧.

Design gate: the corpus topic `value-prop-roadmap` (12 providers, synthesized
2026-11-18) resolves the three open structural questions. Parked tasks stay in
`curation.toml` (decision 18) — a roadmap-side parked list would be the
duplication the corpus names the cardinal sin. Task parking (`podras park` plus
a resurface trigger) and bet deferral (`status = "deferred"` plus `revive_when`)
are distinct objects. The brief's frozen narrative does not change here; its
envelope merely gains one optional object.

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| separate_lens | invariant | `the roadmap lens renders identity, bets, and no-gos in its own render path and never mixes them into the brief's focus or parked columns; its not-now column re-renders curation's parked entries and then parked entries whose resurface trigger expired, each flagged as needing re-triage; focus is curation intent and a bet is a commitment, and neither is inferred from the other — the brief's frozen narrative stays as v1 left it` | [[roadmap.v1]] |
| roadmap_state_file | invariant | `roadmap state lives in one git-committed file, .podras/roadmap.toml, sibling of curation.toml, holding exactly [identity], [[bets]] and [[no_gos]]; parked tasks stay in curation.toml (no second parked list); verbs and humans write it and agents never edit it; an absent file is legal and omits the lens; a malformed or unreadable file is a hard error for podras roadmap, while podras brief omits the object, warns once with the same code, and exits 0` | [[roadmap.v1]] |
| bet_lifecycle_closed | invariant | `bet status and horizon are closed stored enums (shaping|committed|building|deferred|shipped|killed; now|next|later); horizon is authoritative for rendering and never computed from observed work; admission (shaping to committed) is the dated decision recorded in admitted_at; committed and building bets require review_or_kill; deferred bets require revive_when and every other status omits it` | [[roadmap.v1]] |
| identity_linked_never_generated | invariant | `the lens renders identity.path, last_reviewed, next_review and staleness only — identity prose is linked, never generated or rewritten; staleness is decided by next_review alone: absent means unknown and never fresh, passed means stale, equal or later means fresh; identity_review_overdue is advisory whenever next_review is absent or has passed, its detail naming which` | [[roadmap.v1]] |
| warnings_never_verdict | invariant | `roadmap warnings are RoadmapWarningKind values carried in roadmap.warnings, never SignalKind variants in health.signals; every code is advisory — exit is always 0 and no warning auto-kills, auto-promotes, or rewrites state; bet_epic_missing covers committed, building, and shipped bets, horizon_epic_mismatch only non-terminal ones, and every warning is computed after the bound so none names a dropped bet; a no-go with empty match_terms never fires no_go_violation_candidate` | [[roadmap.v1]] |
| additive_optional_envelope | invariant | `roadmap{} is an optional top-level object added to the existing brief envelope and schema_version stays the literal brief.v1; an absent roadmap.toml omits it while a present file with zero bets is a present lens (bets: []); it is bounded (5 bets, 5 no-gos, all warnings) and deterministically ordered — bets by status rank building/committed/deferred/shaping/shipped/killed, then admitted_at, then id; no-gos by id; truncation counts bets and no-gos while parked omissions stay brief-level` | [[roadmap.v1]] |
| read_only_no_mutation | invariant | `podras roadmap and podras brief read the roadmap state and mutate nothing; no verb writes .podras/roadmap.toml in this stage; identical inputs under --as-of produce identical semantic output, with only generated_at varying, and no roadmap warning enters the health verdict` | [[roadmap.v1]] |

## Non-Goals

- Bet lifecycle verbs (`bet add|admit|defer|kill`) — stage ⑦, a deliberate later change.
- Harvest extension (`identity_review`, `retire_or_reshape` candidates) — stage ⑧; harvest never mutates roadmap state here.
- A second parked list, or any roadmap-side copy of curation's parked entries.
- Brief narrative changes, new `SignalKind` variants, `schema_version` bumps, portfolio/org renders, or per-repo roadmap thresholds.

## Model

Bet lifecycle (one commitment in the roadmap lens):

### States

- `shaping`
- `committed`
- `building`
- `deferred`
- `shipped`
- `killed`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| admit | shaping | committed | [[roadmap.v1.bet_lifecycle_closed]] ∧ `human records the dated admission decision in admitted_at` |
| start | committed | building | [[roadmap.v1.bet_lifecycle_closed]] ∧ `work on the bet's epic has begun` |
| ship | building | shipped | [[roadmap.v1.bet_lifecycle_closed]] ∧ `the outcome is delivered` |
| defer_shaping | shaping | deferred | [[roadmap.v1.bet_lifecycle_closed]] ∧ `a shaped bet was not selected and carries a revive_when trigger` |
| defer_committed | committed | deferred | [[roadmap.v1.bet_lifecycle_closed]] ∧ `a committed bet is set aside with a revive_when trigger` |
| defer_building | building | deferred | [[roadmap.v1.bet_lifecycle_closed]] ∧ `stopping mid-build is a deferral, not a fifth state, and requires revive_when` |
| kill_shaping | shaping | killed | [[roadmap.v1.bet_lifecycle_closed]] ∧ `human decision; killed_reason may record why` |
| kill_committed | committed | killed | [[roadmap.v1.bet_lifecycle_closed]] ∧ `human decision; killed_reason may record why` |
| kill_building | building | killed | [[roadmap.v1.bet_lifecycle_closed]] ∧ `human decision; killed_reason may record why` |
| kill_deferred | deferred | killed | [[roadmap.v1.bet_lifecycle_closed]] ∧ `human decision; killed_reason may record why` |
| kill_shipped | shipped | killed | [[roadmap.v1.bet_lifecycle_closed]] ∧ `human decision; killed_reason may record why` |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| lens_separate_from_now_lens | unit | [[roadmap.v1.separate_lens]] | `fixture_repo_with_curation_focus_and_a_now_bet()` | `roadmap rows and focus/parked rows are disjoint ∧ no bet renders as focus ∧ no focus entry renders as a bet` |
| roadmap_file_shape | unit | [[roadmap.v1.roadmap_state_file]] | `roadmap_toml_at_default_path_with_identity_bets_and_no_gos()` | `parsed identity/bets/no_gos round-trip equal to written entries ∧ [identity], [[bets]], [[no_gos]] are the only tables` |
| bet_lifecycle_enums | unit | [[roadmap.v1.bet_lifecycle_closed]] | `roadmap_toml_with_every_status_and_horizon()` | `every status and horizon member round-trips ∧ committed/building carry review_or_kill ∧ deferred carries revive_when and no other status does` |
| identity_staleness_never_fresh | unit | [[roadmap.v1.identity_linked_never_generated]] | `roadmap_toml_without_or_past_next_review()` | `absent next_review ⇒ staleness == unknown ∧ identity_review_overdue present ∧ detail says none is declared ∧ passed next_review ⇒ staleness == stale ∧ identity_review_overdue present ∧ detail names the date ∧ neither is fresh` |
| no_go_violation_flags_only | unit | [[roadmap.v1.warnings_never_verdict]] | `bead_titles_matching_a_no_go_match_term()` | `one no_go_violation_candidate per whole-token match names the offending bead and term ∧ near-token text is not flagged ∧ typographic hyphens normalize so a U+2010 text matches a hyphenated term ∧ empty match_terms fires nothing ∧ nothing written` |
| roadmap_warnings_not_verdict | unit | [[roadmap.v1.warnings_never_verdict]] | `roadmap_toml_with_a_bet_past_appetite()` | `the warning code appears in roadmap.warnings ∧ health.signals keeps its v1 members ∧ exit == 0` |
| roadmap_envelope_additive_bounded | unit | [[roadmap.v1.additive_optional_envelope]] | `roadmap_toml_with_more_than_five_bets_and_no_gos()` | `schema_version == brief.v1 ∧ absent file omits roadmap ∧ zero bets keeps a present lens ∧ exactly 5 bets and 5 no-gos survive in declared order ∧ truncation.omitted_counts names bets/no_gos ∧ no warning or review_due names a dropped bet` |
| roadmap_read_only | unit | [[roadmap.v1.read_only_no_mutation]] | `podras_roadmap_run_twice_on_fixture_repo()` | `no repository byte changes between reads ∧ payloads identical under the same --as-of ∧ no verb writes roadmap.toml ∧ wherever podras roadmap errors on a present roadmap file, podras brief omits the lens with one warning and exits 0` |

## Requirements

### Requirement: Roadmap lens is a separate render path

`podras roadmap` SHALL render the identity row, the bets, and the no-gos as its
own lens and SHALL NOT mix them into the brief's focus or parked columns. Its
not-now column SHALL render the curation parked entries and then every parked
entry whose resurface trigger expired, flagged as needing re-triage. `focus`
SHALL stay curation intent and a bet a commitment with outcome, appetite,
owner, horizon, and at most one epic; neither SHALL be inferred from the other.

#### Scenario: lens_separate_from_now_lens

- **WHEN** `podras roadmap --json` and `podras brief --json` run on a fixture repository that has a curation focus, a parked entry whose resurface date has passed, and a `now` bet
- **THEN** the roadmap lens renders the identity row, the bets, and the no-gos, and its not-now column lists the curation parked entries and then the expired entry marked `(needs re-triage: resurface trigger expired)`; the brief's focus and parked sections render only curation entries; no bet appears as focus, no focus entry appears as a bet, and both commands exit 0
- **VERIFIES** [[roadmap.v1.lens_separate_from_now_lens]]

#### Scenario: bet_in_brief_sections_is_a_violation

- **WHEN** `podras brief --json` runs on a repository that also has a `now` bet
- **THEN** the brief's focus and parked sections render only curation entries and no bet id appears in any computed brief column — rendering a bet as focus or parked is a lens-separation violation
- **VERIFIES** [[roadmap.v1.lens_separate_from_now_lens]]

### Requirement: Roadmap state file shape and closed lifecycles

`podras roadmap` SHALL read `.podras/roadmap.toml` holding exactly `[identity]`,
`[[bets]]`, and `[[no_gos]]`. `horizon`, `status`, and `enforcement` SHALL be
members of their closed enums; `review_or_kill` SHALL be present exactly for
`committed` and `building` bets and `revive_when` exactly for `deferred` bets. A
TOML syntax error or unknown key or enum member SHALL fail as
`roadmap_parse_error`; every semantic violation SHALL aggregate into one
`roadmap_invalid` error.

#### Scenario: roadmap_file_shape

- **WHEN** a roadmap file at the default path with `[identity]`, `[[bets]]`, and `[[no_gos]]` entries is parsed and written back, and a second file has a duplicate bet id, a non-slug id, a 61-character title, and `appetite_weeks = 0`
- **THEN** the parsed identity, bets, and no-gos round-trip equal to the written entries; the invalid file is rejected as one `roadmap_invalid` error naming every violation; and a file with an unknown key is rejected as `roadmap_parse_error`
- **VERIFIES** [[roadmap.v1.roadmap_file_shape]]

#### Scenario: bet_lifecycle_enums

- **WHEN** roadmap files carry every declared `status` and `horizon` member, then one committed bet without `review_or_kill`, one deferred bet without `revive_when`, and one committed bet carrying `revive_when`
- **THEN** the valid file parses with the declared enum members intact, and each invalid file is rejected with `roadmap_invalid` naming the offending bet and rule
- **VERIFIES** [[roadmap.v1.bet_lifecycle_enums]]

### Requirement: Identity is linked, never generated

The lens SHALL render `identity.path`, `last_reviewed`, `next_review`, and
`staleness` only; no verb SHALL rewrite identity prose. Staleness SHALL be
decided by `next_review` alone: absent means `unknown`, earlier than the
evaluation date means `stale`, equal or later means `fresh`, and `fresh` SHALL
never arise otherwise. The `identity_review_overdue` advisory SHALL fire when
`next_review` is absent or has passed, and its detail SHALL name which.

#### Scenario: identity_staleness_never_fresh

- **WHEN** a roadmap file without `next_review`, one whose `next_review` is before the evaluation date, and one whose `next_review` equals or follows it are evaluated
- **THEN** the first reports `staleness = unknown` with an `identity_review_overdue` advisory whose detail says no next review is declared, the second reports `stale` with the advisory naming the passed date, the third reports `fresh` with no such advisory, and nothing without a future `next_review` reports `fresh`
- **VERIFIES** [[roadmap.v1.identity_staleness_never_fresh]]

#### Scenario: fresh_is_never_reported_without_next_review

- **WHEN** an identity row carries a description field and no `next_review`
- **THEN** the lens renders only `identity.path`, `last_reviewed`, `next_review`, and `staleness` — the prose is never rendered or rewritten — and `staleness` is `unknown`; reporting `fresh` without a future `next_review` is a staleness violation
- **VERIFIES** [[roadmap.v1.identity_staleness_never_fresh]]

### Requirement: No-gos are boundaries, flagged and never enforced by mutation

`[[no_gos]]` entries SHALL carry `id`, `reason`, `scope`, `enforcement`
(`automated` only with a real check, else `human_review`), `revoke_by`, and
`match_terms`. Terms SHALL match case-insensitively as whole tokens: token
characters are Unicode alphanumeric, `-`, or `_`, and typographic hyphens
(U+2010..U+2015, U+2212) normalize to `-`. The first match in file order SHALL
emit one `no_go_violation_candidate`; empty `match_terms` SHALL never fire it,
and nothing SHALL be mutated.

#### Scenario: no_go_violation_flags_only

- **WHEN** a fixture tracker has an open bead titled `auto-close`, another titled `autoclose`, a third titled `auto-close` with U+2010 in place of the hyphen, and a no-go whose `match_terms` is `auto-close`, plus a no-go with empty `match_terms`
- **THEN** exactly two `no_go_violation_candidate` warnings name the two whole-token matches and their term, the `autoclose` bead is not flagged, the empty-term no-go fires nothing, and no repository or tracker byte changes
- **VERIFIES** [[roadmap.v1.no_go_violation_flags_only]]

### Requirement: Roadmap health is named warnings, never a verdict

Warnings SHALL be `RoadmapWarningKind` values in `roadmap.warnings`, never
`SignalKind` variants in `health.signals`; the verdict SHALL NOT change.
`bet_epic_missing` SHALL cover `committed`, `building`, and `shipped` bets;
`horizon_epic_mismatch` SHALL apply only to non-terminal bets (`shaping`,
`committed`, `building`, `deferred`). Warnings SHALL sort by code, then subject,
SHALL never name a bet dropped by the bound, and SHALL stay advisory: exit 0, no
auto-kill or auto-promote.

#### Scenario: roadmap_warnings_not_verdict

- **WHEN** small fixture repositories, each with at most five bets so no bet is truncated, respectively hold a building bet past `appetite_weeks` times 1.25, another building bet past `appetite_weeks` but below that factor, a committed bet without an epic, a shipped bet without an epic, an active bet whose epic is closed, an active bet whose epic has not changed for 15 days, a `now` bet with no open epic, a `shipped` bet whose epic sits under `later`, an active bet whose `review_or_kill` has passed, and an identity whose `next_review` has passed
- **THEN** each repository's corresponding code appears in `roadmap.warnings`, the shipped bet with a mismatched horizon fires no `horizon_epic_mismatch`, `health.signals` carries no roadmap code and the verdict is unchanged by them, every command exits 0, and no roadmap, curation, or tracker byte changes
- **VERIFIES** [[roadmap.v1.roadmap_warnings_not_verdict]]

#### Scenario: promoting_a_warning_to_the_verdict_is_a_violation

- **WHEN** any roadmap warning fires
- **THEN** `health.signals` keeps its v1 members and the verdict is unchanged — promoting a roadmap warning into the health verdict is a layering violation, and the exit stays 0
- **VERIFIES** [[roadmap.v1.roadmap_warnings_not_verdict]]

### Requirement: Additive, bounded, deterministic envelope

The brief envelope SHALL gain an optional `roadmap{}` (`identity`, `bets`,
`no_gos`, `review_due`, `warnings`, `truncation`) while `schema_version` stays
`"brief.v1"`. An absent file omits it; zero bets render a lens. Bets and no-gos
SHALL be bounded to 5 each; `truncation` SHALL count only those, never the
brief's parked omissions. `review_due` and every warning SHALL be computed over
the retained bets. `--write` SHALL emit a lens-only `ROADMAP.md` and
full-envelope `ROADMAP.json`.

#### Scenario: roadmap_envelope_additive_bounded

- **WHEN** `podras brief --json` runs on a repository with no roadmap file, on one with a roadmap file whose bets are empty, and on one with seven bets and seven no-gos where the bound drops a `shipped` bet without an epic (which would otherwise fire `bet_epic_missing`) and a `killed` bet, twice each at the same `--as-of`
- **THEN** the first envelope carries no `roadmap` object, the second carries a present lens with `bets: []` and an identity row, the third keeps exactly five bets in the declared status-rank then `admitted_at` then `id` order and five no-gos by `id` with `truncation.applied = true` and `omitted_counts` naming bets and no_gos, no warning names a dropped bet, `review_due` reflects the retained bets only, every envelope reports `schema_version = "brief.v1"`, and the two runs of each case are identical apart from `generated_at`
- **VERIFIES** [[roadmap.v1.roadmap_envelope_additive_bounded]]

#### Scenario: rows_exceeding_the_bound_are_counted

- **WHEN** the roadmap file holds seven bets and seven no-gos
- **THEN** exactly five bets and five no-gos survive in the declared order and `truncation.omitted_counts` names bets and no_gos; dropping rows without accounting would violate the additive-bounded contract
- **VERIFIES** [[roadmap.v1.roadmap_envelope_additive_bounded]]

### Requirement: Read-only lens with an agent-facing surface

`podras roadmap` SHALL re-emit the brief envelope. A malformed or unreadable
`.podras/roadmap.toml` SHALL be a hard domain error there, an absent one
`roadmap_missing`; `podras brief` SHALL instead omit `roadmap` and exit 0,
appending one curation-sourced warning with the domain code (or
`roadmap_unreadable`) for a malformed or unreadable file. No verb SHALL write
the file; the lens SHALL render enums and constraints, not prose. `podras init`
SHALL add two roadmap AGENTS.md lines.

#### Scenario: roadmap_read_only

- **WHEN** `podras roadmap --json --as-of 2026-11-18` and `podras roadmap --write --as-of 2026-11-18` each run twice on an unchanged fixture repository with a roadmap file, then the file is corrupted and both `podras brief` and `podras roadmap` run, then the file is removed and both run again, and `podras init` runs twice
- **THEN** the two payloads are equal after removing `generated_at`, the two renders differ only in `generated_at`, `ROADMAP.md` carries the lens sections only while `ROADMAP.json` carries the full brief envelope, no repository byte outside the `--write` targets changes, the corrupted file is a `roadmap_parse_error` or `roadmap_invalid` domain error from `podras roadmap` while `podras brief` omits `roadmap`, appends one curation-sourced warning carrying that code, leaves the five declared `sources[]` rows unchanged, and exits 0, the absent file reports `roadmap_missing` from `podras roadmap` while `podras brief` omits the object and exits 0, and the AGENTS.md block contains both roadmap lines while the second init is byte-identical
- **VERIFIES** [[roadmap.v1.roadmap_read_only]]
