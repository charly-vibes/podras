# Change: Render the at-risk no-go in the brief's Health section

## Why

The roadmap lens already flags `no_go_violation_candidate` in `roadmap.warnings`
(decision 23), but the brief's Health narrative — the thing a cold reader
actually re-reads — never names the at-risk boundary. Design decision 16
deferred that render; this change is the deliberate decision to deliver it, as a
render-only concern that keeps the envelope additive (decision 24) and the
warning advisory (exit 0, never a verdict).

## What Changes

- When `roadmap.warnings` contains a warning with code
  `no_go_violation_candidate`, the brief's markdown Health section gains
  exactly one extra line, `- no-go risk: <subject> — <detail>`, taken from the
  first such warning in the lens's deterministic warning order (code, subject,
  detail).
- When no candidate fires — including when no roadmap file exists and the
  brief carries no `roadmap` object — the Health section is byte-identical to
  today's render. Same inputs plus `--as-of` still produce the same bytes.
- No `SignalKind` variant is added, `health.signals` is untouched, exit codes
  are unchanged, and no envelope or schema field changes.

Out of scope: the roadmap lens's own render, curation or roadmap files, and any
enforcement of no-gos.

## Impact

- Affected specs: `brief-v1` — the Health narrative gains a conditional line
  (delta under `specs/brief-v1/spec.md`); `roadmap-v1` — the advisory warning
  contract gains the surfacing rule (delta under `specs/roadmap-v1/spec.md`).
- Affected code: `src/render/markdown.rs` (`render_health` reads
  `brief.roadmap.warnings`); tests in `tests/health.rs`,
  `tests/roadmap/fixtures.rs`, and `tests/determinism.rs`.
