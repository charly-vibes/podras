# Tasks: add-no-go-health-line

## 1. Change artifacts

- [x] 1.1 `proposal.md`, `design.md`, and the `brief-v1` / `roadmap-v1` spec deltas under `openspec/changes/add-no-go-health-line/`. Scenario: —.
- [x] 1.2 `openspec validate add-no-go-health-line --strict` exits 0. Scenario: —.

## 2. Tests (red first)

- [x] 2.1 red: `tests/health.rs::health_no_go_line` — rendering a brief whose roadmap carries one `no_go_violation_candidate` yields exactly one `- no-go risk: <subject> — <detail>` line after the signal lines; a brief with no candidate (and one with no roadmap at all) renders a byte-identical Health section. Scenario: `health_no_go_line`.
- [x] 2.2 red: `tests/roadmap/fixtures.rs` gains a no-go-matching fixture (`candidate_roadmap`) whose match term hits a seeded open bead. Scenario: `health_no_go_line`.
- [x] 2.3 red: `tests/determinism.rs::health_no_go_line_determinism` — two `podras brief --write --as-of` runs on the candidate fixture produce `BRIEF.md` renders identical apart from the freshness stamp, both carrying the single line. Scenario: `health_no_go_line_determinism`.

## 3. Implementation

- [x] 3.1 `src/render/markdown.rs`: `render_health` reads `brief.roadmap.warnings`, emits the first `no_go_violation_candidate` as `- no-go risk: <subject> — <detail>` after the signal lines; no change when absent. Scenario: `health_no_go_line`.
- [x] 3.2 Targeted gates green: `cargo test health`, `cargo test determinism`, `cargo test envelope`, and `openspec validate add-no-go-health-line --strict`. Scenario: both.
