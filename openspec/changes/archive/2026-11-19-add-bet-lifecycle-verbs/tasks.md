# Tasks: add-bet-lifecycle-verbs

Checklist maps stage 7 of the roadmap build order to its deliverable files.
Test function names in `tests/bet_verbs.rs` equal the property-row generators.

- [x] 0.1 (podras-o89) `proposal.md` citing decisions 17/22/25/26; `design.md` with verb decisions D1-D7.
- [x] 0.2 (podras-o89) Delta spec `specs/roadmap-v1/spec.md`: ADDED requirement "Bet lifecycle verbs are the only roadmap mutations" (nine scenarios), MODIFIED requirement "Read-only lens with an agent-facing surface", MODIFIED constraints (`read_only_no_mutation` scoped to the read verbs; new `bet_verbs_only_writers`), MODIFIED Non-Goals (stage 7 line dropped, stage 8 kept), MODIFIED properties (scoped `roadmap_read_only`, ten new unit rows whose generators equal the test fns).
- [x] 1.1 `src/roadmap/file.rs`: `admitted_at` is `Option<String>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`), absent until admission; `bet_violations` checks it only when present.
- [x] 1.2 `src/roadmap/file.rs`: canonical atomic `save(repo, file)` beside `load`/`validate` (their signatures unchanged); `skip_serializing_if` on every absent-able option.
- [x] 1.3 `src/roadmap.rs::bet_view` reads the optional `admitted_at`.
- [x] 2.1 `tests/bet_verbs.rs::bet_add_appends_a_shaping_bet_that_round_trips`.
- [x] 2.2 `tests/bet_verbs.rs::bet_admit_moves_only_shaping_to_committed`.
- [x] 2.3 `tests/bet_verbs.rs::bet_defer_requires_a_revive_trigger_and_stamps_the_review`.
- [x] 2.4 `tests/bet_verbs.rs::bet_kill_is_legal_from_every_status`.
- [x] 2.5 `tests/bet_verbs.rs::bet_verbs_reject_unknown_ids`.
- [x] 2.6 `tests/bet_verbs.rs::bet_verbs_refuse_to_write_a_malformed_file`.
- [x] 2.7 `tests/bet_verbs.rs::bet_add_is_deterministic_byte_for_byte`.
- [x] 2.8 `tests/bet_verbs.rs::bet_as_of_controls_the_stamped_dates`.
- [x] 2.9 `tests/bet_verbs.rs::bet_verbs_require_a_scaffolded_roadmap_file`.
- [x] 2.10 `tests/bet_verbs.rs::bet_verbs_exit_zero_on_success_and_one_on_domain_error`.
- [x] 2.11 `src/verbs/bet.rs`: add/admit/defer/kill with load, guard, mutate, stamp, re-validate, atomic save; domain codes `bet_unknown`, `bet_invalid_transition`, `bet_invalid`, `bet_invalid_horizon`; registered in `src/verbs/mod.rs`.
- [x] 3.1 `src/commands.rs`: `bet_add`, `bet_admit`, `bet_defer`, `bet_kill` wrappers.
- [x] 3.2 `src/main.rs`: `podras bet add|admit|defer|kill` wired like `focus`/`park`, each verb with `--as-of` (clap `NaiveDate`, default today).
- [x] 4.1 Out of scope: harvest extension (stage 8); `bet start`/`bet ship` (human hand-edits); curation state, tracker, AGENTS.md, brief narrative, and envelope untouched.

## Scenario coverage ledger

Every scenario in the delta spec maps to the same-named test function in
`tests/bet_verbs.rs`, owned by podras-e09 (tasks 2.1-2.10 above).
