# Design: add-bet-lifecycle-verbs (stage 7)

Cites `docs/design.md` decisions 17 (state file, closed field set), 22
(lifecycle and dated decisions), 25 (agent surface), and 26 (build order).

## D1 — Load before every write; the file is never scaffolded

Every bet verb begins with `roadmap::file::load`. An absent file is
`roadmap_missing` for all four verbs, including `bet add`: the file is
human-scaffolded (decision 17), and a verb that created it would invent an
`[identity]` table the human never wrote. A file that fails load
(`roadmap_parse_error`, `roadmap_unreadable`) or validate (`roadmap_invalid`,
one aggregated message) fails the verb before any byte is written, mirroring
the lens's D3 table. Rejected alternative: `bet add` scaffolding a file (the
lens already renders "absent is legal"; a verb-created skeleton would let an
agent-era verb create human decision state).

## D2 — Transitions are guarded in the verb, re-validated in the writer

Guards:

- `admit`: shaping only. Any other status is `bet_invalid_transition` naming
  the bet, its current status, and the rule.
- `defer`: shaping, committed, or building — stopping mid-build is a deferral,
  not a fifth state (decision 22's `defer_building`). `deferred`, `shipped`,
  and `killed` bets are rejected.
- `kill`: legal from every status.

After mutation the writer re-runs `roadmap::file::validate`, so the fields a
verb just set (slug ids, unique ids, dates, `review_or_kill` presence on
committed/building, `revive_when` exactly on deferred) are checked before the
atomic rename; an invalid result is `roadmap_invalid` and the file keeps its
previous bytes.

## D3 — Stamps and clearings per verb (the field set is closed — decision 17)

| verb | status | `admitted_at` | `last_reviewed` | `epic` | `review_or_kill` | `revive_when` | `killed_reason` |
|---|---|---|---|---|---|---|---|
| `add` | shaping | absent (untouched) | decision date | `--epic` or absent | cleared (absent) | cleared | cleared |
| `admit` | committed | decision date | decision date | `--epic` | `--review-or-kill` | cleared | cleared |
| `defer` | deferred | untouched | decision date | untouched | cleared | `--revive-when` | untouched |
| `kill` | killed | untouched | decision date | untouched | cleared | cleared | `--reason` exactly when given, else cleared |

Deferral adds no new field: `last_reviewed` is the dated decision (decision
22). `admitted_at` becomes an optional field in the state structs: a shaping
bet has none, admission stamps it, and later transitions leave it untouched.
Kill re-records `killed_reason` on every kill (given ⇒ set, omitted ⇒ cleared)
because a kill is a fresh decision, not an edit of a previous one.

## D4 — Canonical atomic serialization

`roadmap::file::save(repo, file)` sits beside `load`/`validate` (signatures
unchanged) and writes `toml::to_string_pretty` of the struct through a
`.podras/.roadmap.toml.<pid>.tmp` sibling plus `rename`, exactly like
`curation::save`. Declaration order gives the canonical field order (decision
17); `skip_serializing_if = "Option::is_none"` omits absent optional fields,
so `load(save(x)) == x` semantically and two identical runs write identical
bytes. Round-trip property: the file round-trips and existing entries,
including their table order, are preserved — the appended bet is last.

## D5 — `--as-of` is the decision date

Every verb accepts `--as-of <YYYY-MM-DD>` (clap `NaiveDate`, default today)
and stamps it into `last_reviewed` (and `admitted_at` for `admit`). No clock
is read inside the library; the binary owns the default, matching the lens's
pinned-evaluation-date rule.

## D6 — CLI wiring and exit codes

`podras bet <subcommand>` is wired exactly like `focus`/`park`: one library
verb module (`src/verbs/bet.rs`) behind one command function per subcommand in
`src/commands.rs`, dispatched in `src/main.rs`. Errors flow through the
existing `Error::Domain` → miette boundary: exit `0` on success, `1` on domain
error (`roadmap_missing`, `roadmap_parse_error`, `roadmap_invalid`,
`roadmap_unreadable`, `bet_unknown`, `bet_invalid_transition`, `bet_invalid`,
`bet_invalid_horizon`, `roadmap_serialize_error`), `2` on clap usage error
(missing required flag, malformed `--as-of`).

## D7 — Non-goals

`bet start` and `bet ship` stay human hand-edits (decision 26): committed →
building and building → shipped are observations about work in flight, not
dated curation decisions, and adding verbs for them would invite automation of
the tracker-facing half of the lifecycle. Curation state, the tracker, and
AGENTS.md are untouched (decision 25's lines already shipped in stage 6).

## D8 — Spec tables applied at archive

The rows below apply to `openspec/specs/roadmap-v1/spec.md` when this change
archives: the constraint and property tables are edited (the
`read_only_no_mutation` constraint and `roadmap_read_only` property are scoped
to the read verbs), the Non-Goals list drops its stage ⑦ line, and the new
rows are added.

### Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| read_only_no_mutation | invariant | `the read verbs, podras brief and podras roadmap, read the roadmap state and mutate nothing; the stage 7 bet verbs are the only writers of .podras/roadmap.toml and agents never edit it; identical read inputs under --as-of produce identical semantic output, with only generated_at varying, and no roadmap warning enters the health verdict` | [[roadmap.v1]] |
| bet_verbs_only_writers | invariant | `podras bet add|admit|defer|kill are the only writers of .podras/roadmap.toml; every verb loads the human-scaffolded file first — an absent file is roadmap_missing for every verb including add, and a file failing load or validate (roadmap_parse_error, roadmap_invalid, roadmap_unreadable) blocks the write byte-for-byte; add appends exactly one shaping bet with admitted_at absent; admit guards shaping to committed only and stamps admitted_at; defer guards from shaping, committed, and building only, requires revive_when, and clears review_or_kill; kill is legal from every status, records killed_reason exactly when --reason is given, and clears review_or_kill and revive_when; every verb stamps the --as-of decision date into last_reviewed, re-validates, writes atomically, and re-serializes canonically so load(write(x)) == x semantically; start and ship stay human hand-edits` | [[roadmap.v1]] |

### Non-Goals

- Harvest extension (`identity_review`, `retire_or_reshape` candidates) — stage ⑧; harvest never mutates roadmap state here.
- A second parked list, or any roadmap-side copy of curation's parked entries.
- Brief narrative changes, new `SignalKind` variants, `schema_version` bumps, portfolio/org renders, or per-repo roadmap thresholds.
- `bet start` and `bet ship` verbs — committed → building and building → shipped stay human hand-edits.

### Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| lens_separate_from_now_lens | unit | [[roadmap.v1.separate_lens]] | `fixture_repo_with_curation_focus_and_a_now_bet()` | `roadmap rows and focus/parked rows are disjoint ∧ no bet renders as focus ∧ no focus entry renders as a bet` |
| roadmap_file_shape | unit | [[roadmap.v1.roadmap_state_file]] | `roadmap_toml_at_default_path_with_identity_bets_and_no_gos()` | `parsed identity/bets/no_gos round-trip equal to written entries ∧ [identity], [[bets]], [[no_gos]] are the only tables` |
| bet_lifecycle_enums | unit | [[roadmap.v1.bet_lifecycle_closed]] | `roadmap_toml_with_every_status_and_horizon()` | `every status and horizon member round-trips ∧ committed/building carry review_or_kill ∧ deferred carries revive_when and no other status does` |
| identity_staleness_never_fresh | unit | [[roadmap.v1.identity_linked_never_generated]] | `roadmap_toml_without_or_past_next_review()` | `absent next_review ⇒ staleness == unknown ∧ identity_review_overdue present ∧ detail says none is declared ∧ passed next_review ⇒ staleness == stale ∧ identity_review_overdue present ∧ detail names the date ∧ neither is fresh` |
| no_go_violation_flags_only | unit | [[roadmap.v1.warnings_never_verdict]] | `bead_titles_matching_a_no_go_match_term()` | `one no_go_violation_candidate per whole-token match names the offending bead and term ∧ near-token text is not flagged ∧ typographic hyphens normalize so a U+2010 text matches a hyphenated term ∧ empty match_terms fires nothing ∧ nothing written` |
| roadmap_warnings_not_verdict | unit | [[roadmap.v1.warnings_never_verdict]] | `roadmap_toml_with_a_bet_past_appetite()` | `the warning code appears in roadmap.warnings ∧ health.signals keeps its v1 members ∧ exit == 0` |
| roadmap_envelope_additive_bounded | unit | [[roadmap.v1.additive_optional_envelope]] | `roadmap_toml_with_more_than_five_bets_and_no_gos()` | `schema_version == brief.v1 ∧ absent file omits roadmap ∧ zero bets keeps a present lens ∧ exactly 5 bets and 5 no-gos survive in declared order ∧ truncation.omitted_counts names bets/no_gos ∧ no warning or review_due names a dropped bet` |
| roadmap_read_only | unit | [[roadmap.v1.read_only_no_mutation]] | `podras_roadmap_run_twice_on_fixture_repo()` | `the read verbs change no repository byte ∧ payloads identical under the same --as-of ∧ the stage ⑦ bet verbs are the only writers of roadmap.toml ∧ wherever podras roadmap errors on a present roadmap file, podras brief omits the lens with one warning and exits 0` |
| bet_add_appends_a_shaping_bet_that_round_trips | unit | [[roadmap.v1.bet_verbs_only_writers]] | `bet_add_appends_a_shaping_bet_that_round_trips()` | `the appended shaping bet is last with admitted_at absent and last_reviewed stamped ∧ the existing bet, no_gos, and identity survive field-for-field ∧ the file loads and validates` |
| bet_admit_moves_only_shaping_to_committed | unit | [[roadmap.v1.bet_verbs_only_writers]] | `bet_admit_moves_only_shaping_to_committed()` | `admit from shaping stamps admitted_at, last_reviewed, epic, and review_or_kill ∧ admit from any other status is bet_invalid_transition naming the bet and rule ∧ the file is untouched` |
| bet_defer_requires_a_revive_trigger_and_stamps_the_review | unit | [[roadmap.v1.bet_verbs_only_writers]] | `bet_defer_requires_a_revive_trigger_and_stamps_the_review()` | `defer from shaping/committed/building sets deferred with revive_when, stamps last_reviewed, and clears review_or_kill ∧ defer from deferred or shipped is bet_invalid_transition` |
| bet_kill_is_legal_from_every_status | unit | [[roadmap.v1.bet_verbs_only_writers]] | `bet_kill_is_legal_from_every_status()` | `kill from all six statuses sets killed and stamps last_reviewed ∧ killed_reason is recorded exactly when --reason is given ∧ review_or_kill and revive_when are cleared ∧ each file loads and validates` |
| bet_verbs_reject_unknown_ids | unit | [[roadmap.v1.bet_verbs_only_writers]] | `bet_verbs_reject_unknown_ids()` | `admit, defer, and kill on an unknown id each fail bet_unknown ∧ the file is byte-identical` |
| bet_verbs_refuse_to_write_a_malformed_file | unit | [[roadmap.v1.bet_verbs_only_writers]] | `bet_verbs_refuse_to_write_a_malformed_file()` | `unknown key ⇒ roadmap_parse_error ∧ semantic violation ⇒ roadmap_invalid ∧ unreadable file ⇒ roadmap_unreadable ∧ each file keeps its exact previous bytes` |
| bet_add_is_deterministic_byte_for_byte | unit | [[roadmap.v1.bet_verbs_only_writers]] | `bet_add_is_deterministic_byte_for_byte()` | `two identical add runs write identical bytes ∧ the same admit-then-defer sequence keeps the files byte-identical` |
| bet_as_of_controls_the_stamped_dates | unit | [[roadmap.v1.bet_verbs_only_writers]] | `bet_as_of_controls_the_stamped_dates()` | `add stamps only last_reviewed ∧ admit stamps admitted_at and last_reviewed ∧ defer and kill stamp last_reviewed ∧ every stamp equals the passed --as-of` |
| bet_verbs_require_a_scaffolded_roadmap_file | unit | [[roadmap.v1.bet_verbs_only_writers]] | `bet_verbs_require_a_scaffolded_roadmap_file()` | `every verb on an absent file fails roadmap_missing ∧ no file is scaffolded` |
| bet_verbs_exit_zero_on_success_and_one_on_domain_error | unit | [[roadmap.v1.bet_verbs_only_writers]] | `bet_verbs_exit_zero_on_success_and_one_on_domain_error()` | `success and legal transitions exit 0 ∧ a guarded transition exits 1 ∧ a missing required flag exits 2` |
