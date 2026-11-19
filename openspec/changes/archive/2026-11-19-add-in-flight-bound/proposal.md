# Change: Bound the in-flight section at 15 with declared truncation

## Why

`in_flight` is the only computed brief queue without a declared bound; the
`bounded_defaults` list never mentions it and `never_silently_truncate` calls
unbounded "the exception", so the status quo is an undeclared exception, not a
decision. Fleet evidence (18 repos, committed envelopes as-of 2026-11-18): 376
in-flight rows, median 13, max 83 (wai) / 73 (dont), 65% of rows ≥30 days old —
verbosity is an outlier tail, and wai/dont cold returns fail the brief's own
10-minute orientation test. This ratifies option B of
docs/research/2026-11-18-in-flight-bound-or-page.md (podras-eg4).

## What Changes

- `in_flight` gains a declared bound of 15, slicing the existing
  priority-then-id total order (a deterministic prefix; no new `--as-of`
  boundary, committed renders stay flag-free and drift-gate-clean).
- The existing truncation contract carries the accounting: the narrative prints
  "(N more omitted)" like every other bounded section and the envelope records
  `omitted_counts.in_flight`. No schema change; `schema_version` stays
  `brief.v1`.
- The fleet sweep declares the bound and asserts it; docs/design.md bound list
  updated.

## Impact

- Affected specs: brief-v1 ("Bounded sections with declared truncation")
- Affected code: src/model.rs, src/brief/ordering.rs, src/render/markdown.rs,
  scripts/validation/fleet_sweep.py, docs/design.md
- Known cost accepted: per-item visibility of the stale tail becomes an honest
  count; the tracker stays the declared full-list authority. A paired
  stale-WIP/in_flight_over_bound health advisory is deliberately NOT bundled —
  it needs its own ratification if wanted.
