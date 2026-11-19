## 1. Tests (red first)
- [x] 1.1 tests/envelope.rs `in_flight_truncated`: 18 in-flight → 15 kept, `omitted_counts.in_flight == 3`, `truncation.applied` true
- [x] 1.2 tests/brief.rs `in_flight_truncated`: narrative "In flight" renders "(3 more omitted)" plus the declared priority-then-id prefix (open-0 kept, open-7 dropped)
- [x] 1.3 tests/envelope.rs `bounds_declared`: in_flight under bound stays whole (7, no omitted entry)

## 2. Implementation
- [x] 2.1 src/model.rs: `IN_FLIGHT_BOUND: usize = 15`
- [x] 2.2 src/brief/ordering.rs: `apply_bounds` truncates in_flight under key `"in_flight"`
- [x] 2.3 src/render/markdown.rs: "In flight" section wires `omitted(brief, "in_flight")`
- [x] 2.4 scripts/validation/fleet_sweep.py: `BOUNDS` gains `in_flight: 15`; docstring/comment updated

## 3. Docs
- [x] 3.1 docs/design.md bounded defaults line gains "15 in flight"

## 4. Spec artifacts
- [x] 4.1 .espectacular/brief-v1/in-flight-truncated.toml maps the property to cargo flags `in_flight_truncated`
- [x] 4.2 `openspec validate add-in-flight-bound --strict` green

## 5. Gates
- [x] 5.1 `cargo test` full suite green (146 tests)
- [x] 5.2 `ah check` green on deployed specs after archive (overlay mode conflicts by design on MODIFIED scenarios; the change is archived in this session)
- [x] 5.3 brief drift gate green: podras' committed renders regenerated (`podras brief --write --as-of 2026-11-18`, installed 0.1.0 == HEAD semantics; podras' 4 in-flight rows are below the bound so the feature cannot alter these bytes)

## 6. Archive-time (after human sign-off on the ratified bound)
- [x] 6.1 openspec/specs/brief-v1/spec.md: update `bounded_defaults` invariant row, `bounds_declared` property predicate, add `in_flight_truncated` property row
- [x] 6.2 `openspec archive add-in-flight-bound --yes`; `openspec validate --strict` green
