# Change: Add the bet lifecycle curation verbs — stage ⑦

## Why

Stage ⑥ shipped the roadmap lens as a strictly read-only render over
`.podras/roadmap.toml`. The file's lifecycle is real but hand-edited only:
every admission, deferral, and kill is a date-stamped decision a human must
record, and today the only tool for recording it is a text editor. That is
correct for *who decides* (humans, per decisions 17/22/25/26) but wrong for
*how the record is kept*: hand-edits drift from the decision-17 field order,
forget a `review_or_kill`, or leave a forbidden `revive_when` behind, and
nothing writes atomically. The lens's own lifecycle model
(`bet_lifecycle_closed`, the Model transition table) already names the guards;
stage ⑦ turns them into verbs without moving the decision right.

`docs/design.md` decisions 17/22/25/26 fix the shape:

- **17 (State file)** — `.podras/roadmap.toml` is git-committed, sibling of
  `curation.toml`, with the closed field set and closed enums; verbs and humans
  write it, agents never edit it.
- **22 (Lifecycle)** — admission (shaping → committed) is the dated decision
  recorded in `admitted_at`; deferred bets carry `revive_when`; committed and
  building bets require `review_or_kill`.
- **25 (Agent surface)** — the AGENTS.md block already forbids agents from
  editing the file; the verbs give humans a way to act without editing.
- **26 (Build order)** — `bet add|admit|defer|kill` are stage ⑦, while `start`
  and `ship` stay human hand-edits.

## What Changes

- **`podras bet add --id --title --outcome --horizon --appetite-weeks --owner
  [--epic] [--as-of <date>]`** — appends exactly one `shaping` bet as the last
  `[[bets]]` entry. `admitted_at` stays absent (admission stamps it — decision
  22); `last_reviewed` is stamped with the decision date. The field set is the
  closed decision-17 set: slug `id` (unique), ≤60-char `title`, `outcome`,
  `horizon` (`now|next|later`), `appetite_weeks ≥ 1`, `owner`, optional `epic`.
- **`podras bet admit <id> --epic <bead> --review-or-kill <date>
  [--as-of <date>]`** — the dated shaping → committed decision. Any other
  current status is a domain error naming the bet and the rule
  (`bet_invalid_transition`). Stamps `admitted_at` and `last_reviewed` with the
  decision date and links the bet's single tracker epic.
- **`podras bet defer <id> --revive-when <trigger> [--as-of <date>]`** — legal
  from `shaping`, `committed`, or `building` only: stopping mid-build is a
  deferral, not a fifth state. Sets `status = "deferred"` + `revive_when`,
  stamps `last_reviewed` with the decision date (the field set is closed, so
  deferral adds no new field — decision 17), and clears `review_or_kill`,
  which is forbidden on a deferred bet.
- **`podras bet kill <id> [--reason <text>] [--as-of <date>]`** — legal from
  every status. Sets `status = "killed"`, stamps `last_reviewed`, records
  `killed_reason` exactly when `--reason` is given, and clears `review_or_kill`
  and `revive_when`, which are forbidden on a killed bet.
- **`bet start` and `bet ship` are deliberately absent** (decision 26):
  committed → building and building → shipped stay human hand-edits of
  `status`.
- **Every verb loads before it writes.** The file is human-scaffolded: an
  absent file is a `roadmap_missing` domain error for every verb, including
  `bet add`, and no verb scaffolds state. A file that fails load or validate
  blocks the write with the lens's D3 codes (`roadmap_parse_error`,
  `roadmap_invalid`, `roadmap_unreadable`) — never overwritten. On success the
  mutated file is re-validated and written atomically (temp + rename), re-
  serialized canonically: `[identity]`, `[[bets]]` in existing order with the
  appended bet last, `[[no_gos]]`, fields in decision-17 order, absent optional
  fields omitted. `load(save(x)) == x` semantically, and two identical verb
  runs write identical bytes.
- **Exit codes** follow the existing `Error::Domain` boundary: `0` success,
  `1` domain error (`roadmap_missing`, `roadmap_parse_error`,
  `roadmap_invalid`, `roadmap_unreadable`, `bet_unknown`,
  `bet_invalid_transition`, `bet_invalid`, `bet_invalid_horizon`), `2` clap
  usage error (missing required flag, bad `--as-of`).
- **The read-only rule is scoped, not dropped**: `podras brief` and
  `podras roadmap` still mutate nothing; the bet verbs are the roadmap file's
  only writers and remain agent-excluded (decision 25).

Out of scope for this change:

- `bet start` / `bet ship` verbs — human hand-edits (decision 26).
- Harvest extension (`identity_review`, `retire_or_reshape` candidates) —
  stage ⑧; harvest never mutates roadmap state.
- Any change to curation state, the tracker, AGENTS.md, the brief narrative,
  or the envelope.

## Impact

- Affected specs: `roadmap-v1` — the delta adds the requirement *Bet lifecycle
  verbs are the only roadmap mutations*, modifies the requirement *Read-only
  lens with an agent-facing surface*, the constraint `read_only_no_mutation`,
  and the property `roadmap_read_only` to scope the no-write rule to the read
  verbs, adds the constraint `bet_verbs_only_writers`, ten unit property rows,
  and drops the stage ⑦ Non-Goals line.
- Affected code: `src/roadmap/file.rs` (optional `admitted_at`, canonical
  `save` beside `load`/`validate`), `src/roadmap.rs` (`bet_view`), new
  `src/verbs/bet.rs` (registered in `src/verbs/mod.rs`), `src/commands.rs` and
  `src/main.rs` (`podras bet <subcommand>` wired exactly like `focus`/`park`),
  and `tests/bet_verbs.rs`.
- Runtime surface: `podras bet add|admit|defer|kill` with `--as-of <date>`
  (default today) on every verb; the read verbs and the envelope are
  unchanged.
