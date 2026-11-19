## ADDED Requirements

### Requirement: Bet lifecycle verbs are the only roadmap mutations

`podras bet add|admit|defer|kill` SHALL be the only writers of
`.podras/roadmap.toml`. Every verb SHALL load the human-scaffolded file
first, refuse a malformed one, re-validate, and write atomically and
canonically. Guards, stamps, clearings, and error codes are stated per verb
in the scenarios. `bet start` and `bet ship` SHALL remain human hand-edits.

#### Scenario: bet_add_appends_a_shaping_bet_that_round_trips

- **WHEN** `podras bet add --id --title --outcome --horizon --appetite-weeks --owner [--epic]` runs on a fixture repository whose roadmap file holds one building bet and one no-go
- **THEN** the appended bet is the last `[[bets]]` entry with status `shaping`, no `admitted_at`, and `last_reviewed` equal to the decision date; the existing bet, `[[no_gos]]`, and `[identity]` survive the rewrite field-for-field; and the file loads and validates
- **VERIFIES** [[roadmap.v1.bet_verbs_only_writers]]

#### Scenario: bet_admit_moves_only_shaping_to_committed

- **WHEN** `podras bet admit <id> --epic <bead> --review-or-kill <date>` runs on a shaping bet, then runs again on that now-committed bet
- **THEN** the first run sets status `committed` and stamps `admitted_at` and `last_reviewed` with the decision date and `review_or_kill` with the flag, and the second run is a `bet_invalid_transition` domain error naming the bet and the rule that leaves the file byte-identical
- **VERIFIES** [[roadmap.v1.bet_verbs_only_writers]]

#### Scenario: bet_defer_requires_a_revive_trigger_and_stamps_the_review

- **WHEN** `podras bet defer <id> --revive-when <trigger>` runs on a committed bet, then on that deferred bet, and on a shipped bet
- **THEN** the first run sets status `deferred` with the revive trigger, stamps `last_reviewed` with the decision date, and clears `review_or_kill`; the second and third runs are `bet_invalid_transition` domain errors
- **VERIFIES** [[roadmap.v1.bet_verbs_only_writers]]

#### Scenario: bet_kill_is_legal_from_every_status

- **WHEN** `podras bet kill <id> [--reason <text>]` runs once per fixture bet in each of the six statuses
- **THEN** every run sets status `killed` and stamps `last_reviewed` with the decision date; `killed_reason` is recorded exactly for the runs given `--reason`; `review_or_kill` and `revive_when` are cleared on every run; and each file loads and validates
- **VERIFIES** [[roadmap.v1.bet_verbs_only_writers]]

#### Scenario: bet_verbs_reject_unknown_ids

- **WHEN** `bet admit`, `bet defer`, and `bet kill` each run with an id no bet carries
- **THEN** each is a `bet_unknown` domain error and the file is byte-identical
- **VERIFIES** [[roadmap.v1.bet_verbs_only_writers]]

#### Scenario: bet_verbs_refuse_to_write_a_malformed_file

- **WHEN** any bet verb runs on a repository whose roadmap file has an unknown key, then on one violating a semantic rule, then on one that cannot be read
- **THEN** the verbs fail with `roadmap_parse_error`, `roadmap_invalid`, and `roadmap_unreadable` respectively and each file keeps its exact previous bytes
- **VERIFIES** [[roadmap.v1.bet_verbs_only_writers]]

#### Scenario: bet_add_is_deterministic_byte_for_byte

- **WHEN** two identical `bet add` runs execute in two identical fixture repositories, followed by the same `admit` and `defer` sequence in each
- **THEN** the two files are byte-identical after the adds and byte-identical after the sequence
- **VERIFIES** [[roadmap.v1.bet_verbs_only_writers]]

#### Scenario: bet_verbs_require_a_scaffolded_roadmap_file

- **WHEN** every bet verb runs in a repository whose `.podras/roadmap.toml` has been removed
- **THEN** each fails with `roadmap_missing`, no file is scaffolded, and the next read verb reports the lens as absent
- **VERIFIES** [[roadmap.v1.bet_verbs_only_writers]]

#### Scenario: bet_as_of_controls_the_stamped_dates

- **WHEN** `add`, `admit`, `defer`, and `kill` each run with an explicit `--as-of`
- **THEN** `add` stamps only `last_reviewed` with the decision date, `admit` stamps `admitted_at` and `last_reviewed`, and `defer` and `kill` stamp `last_reviewed`, each exactly at the passed date
- **VERIFIES** [[roadmap.v1.bet_verbs_only_writers]]

## MODIFIED Requirements

### Requirement: Read-only lens with an agent-facing surface

`podras roadmap` SHALL re-emit the brief envelope. A malformed or unreadable
`.podras/roadmap.toml` SHALL be a hard domain error there, an absent one
`roadmap_missing`; `podras brief` SHALL instead omit `roadmap` and exit 0,
appending one curation-sourced warning with the domain code (or
`roadmap_unreadable`) for a malformed or unreadable file. The read verbs
(`podras brief`, `podras roadmap`) SHALL never write the file; the stage ⑦
bet verbs are its only writers, and agents remain excluded from editing it.
The lens SHALL render enums and constraints, not prose. `podras init` SHALL
add two roadmap AGENTS.md lines.

#### Scenario: roadmap_read_only

- **WHEN** `podras roadmap --json --as-of 2026-11-18` and `podras roadmap --write --as-of 2026-11-18` each run twice on an unchanged fixture repository with a roadmap file, then the file is corrupted and both `podras brief` and `podras roadmap` run, then the file is removed and both run again, and `podras init` runs twice
- **THEN** the two payloads are equal after removing `generated_at`, the two renders differ only in `generated_at`, `ROADMAP.md` carries the lens sections only while `ROADMAP.json` carries the full brief envelope, no repository byte outside the `--write` targets changes, the corrupted file is a `roadmap_parse_error` or `roadmap_invalid` domain error from `podras roadmap` while `podras brief` omits `roadmap`, appends one curation-sourced warning carrying that code, leaves the five declared `sources[]` rows unchanged, and exits 0, the absent file reports `roadmap_missing` from `podras roadmap` while `podras brief` omits the object and exits 0, and the AGENTS.md block contains both roadmap lines while the second init is byte-identical
- **VERIFIES** [[roadmap.v1.read_only_no_mutation]]

