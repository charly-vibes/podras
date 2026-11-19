## ADDED Requirements

### Requirement: Harvest proposes roadmap candidates, never decisions

`podras harvest` SHALL emit advisory, typed roadmap candidates and SHALL NOT
auto-kill, auto-promote, or rewrite any file. `identity_review` SHALL fire only
when `identity.next_review` is strictly earlier than the evaluation date;
`retire_or_reshape` SHALL fire once per non-terminal bet that trips a predicate
shared with the lens warnings, over every bet in the file. A malformed or
unreadable roadmap SHALL degrade the run to zero roadmap candidates with one
warning at exit 0.

#### Scenario: identity_review_fires_only_when_next_review_passed

- **WHEN** `podras harvest --json --as-of 2026-11-18` runs on fixture repositories whose roadmap declares `next_review = 2026-11-01`, no `next_review`, and `next_review = 2027-05-18`
- **THEN** only the first repository emits an `identity_review` candidate, exactly one, carrying roadmap evidence and a summary naming the passed date, and it is ordered last among all candidates
- **VERIFIES** [[roadmap.v1.harvest_candidates_proposed_not_decided]]

#### Scenario: retire_or_reshape_fires_per_trigger_with_terminal_bets_exempt

- **WHEN** `podras harvest` runs on a fixture repository holding a committed bet past `appetite_weeks`×7×1.25, a non-terminal bet whose epic has not changed for more than fourteen days, a shaping bet whose epic is closed, and shipped and killed bets tripping the same shapes
- **THEN** each non-terminal bet yields exactly one `retire_or_reshape` candidate naming its bet id and triggering reason(s), the terminal bets yield none, and the candidates order by lens status rank, then `admitted_at`, then id
- **VERIFIES** [[roadmap.v1.harvest_candidates_proposed_not_decided]]

#### Scenario: harvest_roadmap_run_writes_nothing

- **WHEN** `podras harvest` runs on a fixture repository with a roadmap whose bets trip every trigger
- **THEN** `.podras/roadmap.toml`, `.podras/curation.toml`, and `.beads/issues.jsonl` keep their exact bytes — promoting, retiring, or rewriting a bet from a harvest run violates candidates-never-decisions
- **VERIFIES** [[roadmap.v1.harvest_candidates_proposed_not_decided]]

#### Scenario: harvest_roadmap_run_is_deterministic

- **WHEN** `podras harvest --json --as-of 2026-11-18` runs twice over unchanged inputs
- **THEN** both runs emit the same candidate set in the same order, carrying `identity_review` and `retire_or_reshape` kinds with resolvable roadmap evidence refs
- **VERIFIES** [[roadmap.v1.harvest_candidates_proposed_not_decided]]

#### Scenario: malformed_roadmap_degrades_harvest

- **WHEN** `podras harvest --json` runs on a fixture repository whose `.podras/roadmap.toml` is malformed TOML
- **THEN** the run exits 0, emits zero roadmap candidates, appends one degradation warning carrying the domain code, and changes no repository byte
- **VERIFIES** [[roadmap.v1.harvest_candidates_proposed_not_decided]]

