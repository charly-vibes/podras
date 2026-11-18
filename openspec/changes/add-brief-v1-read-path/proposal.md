# Change: Add the brief v1 read path, agent envelope, and focus/park curation

## Why

A human returning cold or an agent starting cold in one of 118 repositories has
no single, current answer to "what is happening here now, and what deserves
attention next". Status is scattered across the tracker, git history, spec
changes, session notes, and curation intent; the corpus census found 1,469 open
tracker issues, 275 of 315 spec changes untouched for more than 14 days, and no
maintained roadmap or status document anywhere. The deployed `brief-v1` spec
states the contract; this change supplies the implementation path for its first
three build stages so the contract can move from red to green.

## What Changes

- Add the `podras` Rust CLI read path: a fixed nine-section brief rendered as
  deterministic Markdown for humans and a typed JSON envelope for agents.
- Add per-source reads for beads, git, the OpenSpec store, whisper session
  notes (latest branch entry only), and the curation file. Each read is
  isolated; a missing or unreadable source produces a per-source warning row
  and never fails the render.
- Add the typed envelope with `schema_version`, `generated_at`, `sources[]`,
  `curation{updated_at, confirmed_at}`, `health{status, basis, signals, owner,
  reason}`, `truncation{applied, omitted_counts}`, `head_sha`, `source_hash`,
  and `warnings[]`, plus a `podras schema` command emitting JSON Schema
  2020-12. There is no project-wide staleness field.
- Add bounded sections with explicit truncation accounting: 1 focus, 5 recent,
  5 blocked, 5 next, 5 parked, and unbounded warnings.
- Add committed renders: `podras brief --write` writes generated `BRIEF.md`
  and `BRIEF.json`. Regeneration overwrites hand edits; it never merges them.
  Whisper note content stays out of the committed render unless the per-repo
  `publish_whisper` flag is enabled.
- Add curation state in `.podras/curation.toml` and the `focus`, `park`, and
  `init` verbs. Focus is select-or-replace under `max_focus` (default 1); park
  requires a reason and a resurface trigger; `init` idempotently scaffolds the
  curation file and the AGENTS.md managed block.
- Add the deterministic health verdict: fresh human verdict over computed
  signals over `unknown`, with seven named signals, and never a green
  synthesized from silence.

Out of scope for this change:

- `harvest` (session-note candidate extraction, build stage 4 — v1.1)
- freshness hardening and the expanded `--as-of` contract (build stage 5 —
  v1.1; the deterministic `--as-of` evaluation clock in v1 is in scope)
- the roadmap lens and portfolio view (gated on the external
  `value-prop-roadmap` synthesis)
- the BRIEF.md regen-diff CI gate
- org-mode render target

## Impact

- Affected specs: `brief-v1` — the delta under
  `openspec/changes/add-brief-v1-read-path/specs/brief-v1/spec.md` adds the
  implementation-facing requirements and carries one `VERIFIES` scenario per
  deployed property. The deployed corpus at `openspec/specs/brief-v1/spec.md`
  is not modified by this change.
- Affected code: new Rust crate (`Cargo.toml`, `src/main.rs`, `src/lib.rs`,
  `src/model.rs`, `src/brief.rs`, `src/error.rs`, `src/curation.rs`,
  `src/health.rs`, `src/freshness.rs`, `src/readers/`, `src/render/`,
  `src/write.rs`, `src/verbs/`), fixture-driven integration tests under
  `tests/`, and `.espectacular/changes/add-brief-v1-read-path/brief-v1/`
  contract stubs.
- Runtime surface: `podras brief [--json] [--write] [--as-of <date>]`,
  `podras focus <id>`, `podras park <id> --reason <text> --resurface
  <date|event:name>`, `podras init`, `podras schema`. Degraded reads exit 0;
  usage errors exit 2; hard errors exit 1.
- Privacy: whisper note content is on-demand only; committed artifacts carry
  per-source freshness, not session content, unless `publish_whisper` is
  explicitly enabled.