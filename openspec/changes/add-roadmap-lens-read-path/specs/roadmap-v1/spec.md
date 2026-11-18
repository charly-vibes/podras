## ADDED Requirements

### Requirement: Roadmap lens is a separate render path

`podras roadmap` SHALL render the identity row, the bets, and the no-gos as its
own lens and SHALL NOT mix them into the brief's focus or parked columns.
`focus` SHALL remain curation intent and a bet SHALL remain a commitment with
outcome, appetite, owner, horizon, and at most one tracker epic. Neither SHALL
be inferred from the other: a bet SHALL NOT become focus and a curation focus
SHALL NOT become a bet. Both reads SHALL exit 0.

#### Scenario: lens_separate_from_now_lens

- **WHEN** `podras roadmap --json` and `podras brief --json` run on a fixture repository that has a curation focus and a `now` bet
- **THEN** the roadmap lens renders the identity row, the bets, and the no-gos while the brief's focus and parked sections render only curation entries; no bet appears as focus, no focus entry appears as a bet, and both commands exit 0
- **VERIFIES** [[roadmap.v1.separate_lens]]

### Requirement: Roadmap state file shape and closed lifecycles

`podras roadmap` SHALL read `.podras/roadmap.toml` holding exactly `[identity]`,
`[[bets]]`, and `[[no_gos]]`. `horizon`, `status`, and `enforcement` SHALL be
members of their closed enums; `review_or_kill` SHALL be present exactly for
`committed` and `building` bets and `revive_when` exactly for `deferred` bets. A
TOML syntax error or unknown key/enum member SHALL fail as `roadmap_parse_error`;
every semantic violation SHALL aggregate into one `roadmap_invalid` error.

#### Scenario: roadmap_file_shape

- **WHEN** a roadmap file at the default path with `[identity]`, `[[bets]]`, and `[[no_gos]]` entries is parsed and written back, and a second file has a duplicate bet id, a non-slug id, a 61-character title, and `appetite_weeks = 0`
- **THEN** the parsed identity, bets, and no-gos round-trip equal to the written entries; the invalid file is rejected as one `roadmap_invalid` error naming every violation; and a file with an unknown key is rejected as `roadmap_parse_error`
- **VERIFIES** [[roadmap.v1.roadmap_state_file]]

#### Scenario: bet_lifecycle_enums

- **WHEN** roadmap files carry every declared `status` and `horizon` member, then one committed bet without `review_or_kill`, one deferred bet without `revive_when`, and one committed bet carrying `revive_when`
- **THEN** the valid file parses with the declared enum members intact, and each invalid file is rejected with `roadmap_invalid` naming the offending bet and rule
- **VERIFIES** [[roadmap.v1.bet_lifecycle_closed]]

### Requirement: Identity is linked, never generated

The lens SHALL render `identity.path`, `last_reviewed`, `next_review`, and
`staleness` only; no verb SHALL rewrite identity prose. Staleness SHALL be
decided by `next_review` alone: absent `next_review` SHALL mean `unknown` plus
the `identity_review_overdue` advisory (subject `identity`) and SHALL NEVER be
`fresh`; a `next_review` before the evaluation date SHALL mean `stale`; a
`next_review` equal to or after it SHALL mean `fresh`.

#### Scenario: identity_staleness_never_fresh

- **WHEN** a roadmap file without `next_review`, one whose `next_review` is before the evaluation date, and one whose `next_review` equals or follows it are evaluated
- **THEN** the first reports `staleness = unknown` with `identity_review_overdue`, the second `stale`, the third `fresh`, and no evaluation without `next_review` reports `fresh`
- **VERIFIES** [[roadmap.v1.identity_linked_never_generated]]

### Requirement: No-gos are boundaries, flagged and never enforced by mutation

`[[no_gos]]` entries SHALL carry `id`, `reason`, `scope`, `enforcement`
(`automated` only with a real check, else `human_review`), `revoke_by`, and
`match_terms`. Terms SHALL match case-insensitively as whole tokens — a string
edge or a character outside `[a-z0-9_-]` — across every open bead's title and
labels. The first match in file order SHALL emit one
`no_go_violation_candidate` naming bead and term; empty `match_terms` SHALL
never fire it; nothing SHALL be mutated.

#### Scenario: no_go_violation_flags_only

- **WHEN** a fixture tracker has an open bead whose title or label contains a no-go's `match_terms` entry and another bead whose title merely embeds the term's characters (`auto-close` versus `autoclose`), plus a no-go with empty `match_terms`
- **THEN** exactly one `no_go_violation_candidate` names the offending bead and the matched term, the near-token bead is not flagged, the empty-term no-go fires nothing, and no repository or tracker byte changes
- **VERIFIES** [[roadmap.v1.warnings_never_verdict]]

### Requirement: Roadmap health is named warnings, never a verdict

Warnings SHALL be `RoadmapWarningKind` values in `roadmap.warnings`, never
`SignalKind` variants in `health.signals`; the verdict SHALL NOT change. Codes:
`bet_age_exceeds_appetite`, `circuit_breaker_due`, `bet_epic_missing`,
`bet_epic_closed_not_retired`, `bet_no_transition`, `horizon_epic_mismatch`,
`no_go_violation_candidate`, `identity_review_overdue`, `roadmap_review_overdue`.
Warnings SHALL sort by code, then subject; all advisory: exit 0, no auto-kill or
auto-promote.

#### Scenario: roadmap_warnings_not_verdict

- **WHEN** a fixture repository holds a building bet past `appetite_weeks` times 1.25, another building bet past `appetite_weeks` but below that factor, a committed bet without an epic, an active bet whose epic is closed, an active bet whose epic has not changed for 15 days, a `now` bet with no open epic, a `later` bet whose epic is closed, an active bet whose `review_or_kill` has passed, and an identity without `next_review`
- **THEN** the corresponding codes appear in `roadmap.warnings`, `health.signals` carries no roadmap code and the verdict is unchanged by them, the command exits 0, and no roadmap, curation, or tracker byte changes
- **VERIFIES** [[roadmap.v1.warnings_never_verdict]]

### Requirement: Additive, bounded, deterministic envelope

The brief envelope SHALL gain an optional top-level `roadmap{}` (`identity`,
`bets`, `no_gos`, `review_due`, `warnings`, `truncation`) while `schema_version`
stays `"brief.v1"`. An absent file SHALL omit it; zero bets SHALL still render a
lens. Bets and no-gos SHALL be bounded to 5 each, ordered by status rank, then
`admitted_at`, then `id`; no-gos by `id`. `review_due` SHALL hold the earliest
outstanding review or `null`; `--write` SHALL emit `ROADMAP.md` and
`ROADMAP.json`.

#### Scenario: roadmap_envelope_additive_bounded

- **WHEN** `podras brief --json` runs on a repository with no roadmap file, on one with a roadmap file whose bets are empty, and on one with seven bets and seven no-gos, twice each at the same `--as-of`
- **THEN** the first envelope carries no `roadmap` object, the second carries a present lens with `bets: []` and an identity row, the third keeps exactly five bets in the declared status-rank then `admitted_at` then `id` order and five no-gos by `id` with `truncation.applied = true` and `omitted_counts` naming bets and no_gos, every envelope reports `schema_version = "brief.v1"`, and the two runs of each case are identical apart from `generated_at`
- **VERIFIES** [[roadmap.v1.additive_optional_envelope]]

### Requirement: Read-only lens with an agent-facing surface

`podras roadmap [--json] [--as-of <date>] [--write]` SHALL re-emit the brief
envelope. A malformed or invalid `.podras/roadmap.toml` SHALL be a hard error in
both commands; an absent file SHALL omit the object in `podras brief` and report
`roadmap_missing` in `podras roadmap`. No verb SHALL write `.podras/roadmap.toml`.
The lens SHALL render enums and constraints, not prose or actionable `later`
items. `podras init`'s AGENTS.md managed block SHALL gain exactly two roadmap
lines.

#### Scenario: roadmap_read_only

- **WHEN** `podras roadmap --json --as-of 2026-11-18` and `podras roadmap --write --as-of 2026-11-18` each run twice on an unchanged fixture repository with a roadmap file, then the file is corrupted and both `podras brief` and `podras roadmap` run, and `podras init` runs twice
- **THEN** the two payloads are equal after removing `generated_at`, the two `ROADMAP.json` renders differ only in `generated_at`, no repository byte outside the `--write` targets changes, the corrupted file yields `roadmap_parse_error` or `roadmap_invalid` from both commands, and the AGENTS.md block contains both roadmap lines while the second init is byte-identical
- **VERIFIES** [[roadmap.v1.read_only_no_mutation]]