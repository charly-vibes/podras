## ADDED Requirements

### Requirement: Health narrative surfaces the at-risk no-go

When the brief carries a `roadmap` object whose `warnings` include a
`no_go_violation_candidate` warning, the markdown Health section SHALL gain
exactly one extra line, `- no-go risk: <subject> — <detail>`, naming the first
candidate in the lens's deterministic warning order; no `SignalKind` SHALL be
added and `health.signals` SHALL stay unchanged. When no candidate fires,
including when no `roadmap` object is present, the Health section SHALL be
byte-identical to the render without the rule.

#### Scenario: health_no_go_line

- **WHEN** `podras brief --as-of 2026-11-18` runs on a fixture repository whose roadmap file declares a no-go with a match term matching an open bead's title, and again on the same fixture without that matching bead
- **THEN** the first run's Health section contains exactly one `- no-go risk:` line naming the no-go id as subject and the bead-and-term evidence as detail, placed after the signal lines, `health.signals` carries no roadmap code, and the second run's Health section contains no `no-go risk:` line and is byte-identical to a brief rendered with no roadmap file at all
- **VERIFIES** [[brief.v1.health_no_go_line]]

#### Scenario: health_no_go_line_determinism

- **WHEN** `podras brief --write --as-of 2026-11-18` runs twice on the candidate fixture
- **THEN** the two `BRIEF.md` renders are identical apart from the `generated_at` freshness stamp and both carry the single `- no-go risk:` line, and the envelope keeps `schema_version = "brief.v1"` with no new field
- **VERIFIES** [[brief.v1.health_no_go_line]]
