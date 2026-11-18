# Tasks: add-brief-v1-read-path

Checklist maps every board ticket to its deliverable files and the deployed
property it owns. Test function names MUST equal property ids; the contract
stub's cargo filter is the property id. Nothing here is complete yet — this
change is the proposal/contract phase. Mark items only when the Rule-of-5
review for that ticket has passed.

## 0. Change artifacts and gates

- [x] 0.1 (podras-d2e) Delta spec at `openspec/changes/add-brief-v1-read-path/specs/brief-v1/spec.md`: ADDED requirements for the read path, envelope, committed render, curation file/verbs, init, health, and source degradation. Property: — (14 `VERIFIES` scenarios cover every deployed property).
- [x] 0.2 (podras-d2e) `proposal.md`, `design.md`, `tasks.md` under `openspec/changes/add-brief-v1-read-path/`. Property: —.
- [x] 0.3 (podras-d2e) Contract stubs at `.espectacular/changes/add-brief-v1-read-path/brief-v1/*.toml`, one per property, with `[tests]` cargo flags = property id. Property: — (all 14).
- [x] 0.4 (podras-d2e) `.espectacular/config.toml` declares `cargo = ["cargo", "test"]` under `[runners]`. Property: —.
- [x] 0.5 (spk gate) `spk lint openspec/specs` passes with exit 0; the `corpus_self_lint` shell contract runs exactly that command. Property: `corpus_self_lint`.
- [x] 0.6 (podras-d2e) `openspec validate add-brief-v1-read-path --strict` exits 0 and `ah check --changes add-brief-v1-read-path` reports no structural findings. Property: —.

## 1. Scaffold and model

- [x] 1.1 (podras-t9y) Create the crate: `Cargo.toml` (genesis-vibes 0.13, clap 4 derive, serde/serde_json/toml, schemars, miette/thiserror, chrono; dev: tempfile), `src/main.rs` clap wiring, `src/lib.rs`, `src/error.rs`, `justfile`. Property: —.
- [x] 1.2 (podras-xqr) `src/model.rs`: `Brief`, `SourceStatus`, `SourceName`, `SourceState`, `Staleness`, `CurationStamp`, `WorkItem`, `ParkedItem`, `RecentEvent`, `Health`, `HealthStatus`, `HealthBasis`, `Signal`, `SignalKind`, `Truncation`, `Warning`; deterministic ordering keys and `source_hash` semantics per design D4/D5. Property: `determinism_payload`.

## 2. Read path

- [x] 2.1 (podras-l21) `src/readers/beads.rs`: parse `.beads/issues.jsonl`; missing → `missing` + `beads_missing` warning; malformed lines → `beads_parse_error`; exit 0. Property: —.
- [x] 2.2 (podras-ocl) `src/readers/git.rs`: HEAD sha, current branch, bounded log window, `observed_at` from HEAD commit; missing HEAD → `unreadable` + warning; exit 0. Property: —.
- [x] 2.3 (podras-3hw) `src/readers/specs.rs`: best-effort openspec observation; missing `openspec/` → `openspec` source status `missing` + per-source warning, remaining sections render, exit 0. Property: `missing_source_warns`.
- [x] 2.4 (podras-c5w) `src/readers/whisper.rs`: latest branch entry timestamp and first `## Next action` line only; missing notes → `missing` + warning; extracted line is on-demand-only. Property: —.
- [x] 2.5 (podras-xqr) `src/readers/mod.rs` + `src/brief.rs`: independent per-reader dispatch (SCOPE-002 resolution, design D1); collect sources/warnings; never propagate reader failure. Property: `determinism_payload` (read path stability).

## 3. Curation

- [x] 3.1 (podras-i45) `src/curation.rs`: load/save/round-trip `.podras/curation.toml`; defaults when absent; reject unknown keys with `unreadable` warning; atomic writes; `Resurface` date/event parsing. Property: `curation_file_shape`.
- [x] 3.2 (podras-3xw) `src/verbs/focus.rs`: select-or-replace, demote notice, `max_focus` cap, open-item resolution; over-cap yields `wip_over_cap` health warning and never a block. Property: `focus_cap_signal`.
- [x] 3.3 (podras-l4v) `src/verbs/park.rs`: mandatory `--reason` and `--resurface`; expired trigger moves the item to `needs_retriage` and out of `parked`; parked never rendered as next/ready. Property: `park_expired_surfaces`.

## 4. Health and freshness

- [x] 4.1 (podras-j13) `src/health.rs`: seven named signals with reasons; layering fresh human verdict > firing signals > `unknown`; green only from fresh human green with no signal; expired human verdicts treated as absent. Property: `silence_is_unknown`.
- [x] 4.2 (podras-msq) `src/freshness.rs`: per-source `fresh|stale|unknown` from `observed_at`; no project-wide staleness field; curation `updated_at` vs `confirmed_at`. Property: `stale_source_flagged`.

## 5. Render

- [x] 5.1 (podras-ci3) `src/render/markdown.rs`: nine fixed sections in cold-return order; focus/parked only from curation; parked absent from next; explicit `none` lines. Property: `focus_not_inferred`.
- [x] 5.2 (podras-msq) `src/render/envelope.rs`: typed `Brief` serialization; bounds 1/5/5/5/5 with unbounded warnings; `truncation.applied` + `omitted_counts`; `schema/brief-envelope.schema.json` via `podras schema` (2020-12) validating every emitted envelope. Property: `bounds_declared`, `truncation_flagged`, `envelope_schema_conform`.

## 6. Committed render

- [x] 6.1 (podras-8ha) `src/write.rs`: `brief --write` writes `BRIEF.md` + `BRIEF.json` atomically; regeneration overwrites hand edits, never merges; semantic payload equals on-demand except `generated_at`; freshness stamp present. Property: `hand_edit_render_overwritten`.
- [x] 6.2 (podras-8ha) `src/write.rs` + `src/render/markdown.rs`: whisper content excluded from committed files unless `publish_whisper = true`; whisper `observed_at` always present; next-action line only on demand. Property: `whisper_privacy`.

## 7. Init

- [x] 7.1 (podras-jza) `src/verbs/init.rs`: idempotent `.podras/curation.toml` scaffold (preserve existing bytes) and AGENTS.md managed block via genesis `managed_block` with provenance footer; second run changes nothing. Property: —.

## 8. Validation experiments

- [x] 8.1 (podras-4i2) `tests/common/mod.rs` fixture builders plus `tests/experiments/`: amnesia, agent cold-start, and staleness fault injection (close focus silently, kill tracker, edit spec post-read); each discrepancy must be exposed. Property: — (gates the change's done claim).

## 9. Out of scope for this change (tracked, not delivered here)

- [x] 9.1 (podras-o47 — delivered under add-brief-v1-1) v1.1 harvest: `src/harvest.rs` candidates, never decisions. Property: —.
- [x] 9.2 (podras-7jd — delivered under add-brief-v1-1) v1.1 freshness hardening and expanded `--as-of`; `src/freshness.rs`, `src/render/envelope.rs`. Property: —.
- [x] 9.3 (podras-mgd — delivered under add-brief-v1-1) v1.1 BRIEF.md regen-diff CI gate: `scripts/guards/brief-drift.sh`, `lefthook.yml`. Property: —.
- [x] 9.4 (podras-cs6 — delivered) Tooling chore: refresh the pinned spk binary in `versions.ddl.toml`. Property: —.

`podras-5y8` (roadmap-lens contract) remains gated on the external
`value-prop-roadmap` synthesis and is out of scope for this change; it appears
on the board but no v1 deliverable depends on it.

## Property coverage ledger

| deployed property | owning ticket | contract stub |
|---|---|---|
| `determinism_payload` | podras-xqr | `determinism-payload.toml` |
| `missing_source_warns` | podras-3hw | `missing-source-warns.toml` |
| `whisper_privacy` | podras-8ha | `whisper-privacy.toml` |
| `focus_cap_signal` | podras-3xw | `focus-cap-signal.toml` |
| `park_expired_surfaces` | podras-l4v | `park-expired-surfaces.toml` |
| `truncation_flagged` | podras-msq | `truncation-flagged.toml` |
| `silence_is_unknown` | podras-j13 | `silence-is-unknown.toml` |
| `hand_edit_render_overwritten` | podras-8ha | `hand-edit-render-overwritten.toml` |
| `stale_source_flagged` | podras-msq | `stale-source-flagged.toml` |
| `envelope_schema_conform` | podras-msq | `envelope-schema-conform.toml` |
| `focus_not_inferred` | podras-ci3 | `focus-not-inferred.toml` |
| `curation_file_shape` | podras-i45 | `curation-file-shape.toml` |
| `bounds_declared` | podras-msq | `bounds-declared.toml` |
| `corpus_self_lint` | spk gate | `corpus-self-lint.toml` |