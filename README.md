# podras

> "Tal vez te olvides de mí, tal vez me olvide de ti…" — from *No podrás*, Cristian Castro

Per-repository project status briefs — a deterministic CLI that answers "what's happening in this project?" for humans coming back cold and for AI coding agents, from one git-committed source.

> **Why:** Returning to a project after weeks means 15 minutes of archaeology across the tracker, openspec changes, git log, and session notes — and AI coding agents either distrust that scattered state or invent it. Ad-hoc status docs fail because they're maintained by hand, so they rot: stale AGENTS.md conventions and hopeful prose diverge from reality within days. podras instead *projects* status from evidence that already exists (beads, openspec, git history, whisper notes) plus explicit human curation, and commits the rendered brief so cloning a repo carries its orientation context.
>
> **Status:** [experimental](docs/src/status.md) · pre-release: works through v1.1, no crates.io publish or GitHub Release yet · [Motivation & design](docs/src/index.md)

## When to use

- Use `podras` when you (or an agent) are returning cold to a single repo and want one trustworthy brief: current focus, next action, blockers, parked items — with freshness stamps.
- Use it when an AI coding agent needs typed, machine-readable state (`podras brief --json`) instead of guessing from stale docs.
- Use it when you want explicit intent recorded: what you're committed to now (`focus`) and what you're deliberately not doing (`park`).
- Use it in CI to catch drift between the committed brief and live evidence.

**Don't use it when** you want a fleet-wide, many-repo view — that's [`ddl`](https://github.com/charly-vibes/ddl) (portfolio status/registry across repos; podras is per-repo, deterministic, and git-committed). Don't use it to track session/pipeline context — that's [`wai`](https://github.com/charly-vibes/wai), a complement, not a competitor. **Assumption:** podras reads git-committed evidence — a beads tracker, openspec changes, git history, optional whisper notes — plus optional hand-curated files in `.podras/`; with none of those it has nothing to project.

## Install

Pre-release: no GitHub Release or crates.io publish yet (both land with the first release). Until then, build from source:

```sh
cargo install --path .
# → `podras --version` prints e.g. podras 1.1.0
```

Once the first release ships, the prebuilt asset from [charly-vibes/podras/releases](https://github.com/charly-vibes/podras/releases) becomes the primary path (linux/darwin/windows, amd64/arm64 + checksums), with `cargo install podras` as the source fallback.

## Quick start

```sh
podras init
podras brief
# → exit 0; prints the rendered brief: what's happening, what's next, what's blocked, what's parked
podras brief --json
# → exit 0; prints the typed JSON envelope, starting with {"ok":true, …}
```

`podras init` scaffolds `.podras/curation.toml` and the AGENTS.md managed block. No dev recipes here — see [Development](#development).

## Common workflows

1. **Cold return** — clone the repo, read the committed `BRIEF.md`: focus, next action, blockers, parked items, freshness per source. No archaeology.
2. **Agent orientation** — `podras brief --json` → typed envelope with `sources[]`, `health`, `truncation`, `source_hash`, explicit unknowns. A contract, not a prompt; agents read it, never edit `.podras/` files.
3. **Drift check in CI** — run `scripts/guards/brief-drift.sh` (or `podras brief --write` and diff) → non-zero exit if the committed brief no longer matches live evidence.

## Commands

| Command | What it does | When |
|---|---|---|
| `podras brief` | Renders the status brief; `--json` emits the typed envelope; `--write` commits `BRIEF.md` + `BRIEF.json` | Coming back cold; orienting an agent |
| `podras roadmap` | Renders the roadmap lens read-only from `.podras/roadmap.toml` (identity, bets, no-gos); `--write` emits `ROADMAP.md`/`ROADMAP.json` | Reviewing or sharing the why-lens |
| `podras focus <ID>` | Records the one curated focus item | Declaring what you're committed to now |
| `podras park --reason <REASON> --resurface <DATE\|event:<name>> <ID>` | Parks an item until a resurface trigger | Deliberately deferring work |
| `podras harvest` | Surfaces mismatch candidates without changing any state | Deciding what curation needs updating |
| `podras init` | Scaffolds curation state and the AGENTS.md managed block | First setup in a repo |
| `podras schema` | Emits the brief JSON Schema | Wiring agents/validators to the envelope |

Full reference: [docs/src/cli.md](docs/src/cli.md).

## FAQ

### Is it deterministic?
Yes. Same repo state + same `--as-of` pin → byte-identical output. The brief is a projection, not a document anyone edits.

### What does it read?
Evidence that already exists: beads tracker, openspec changes, git history, optional whisper session notes — plus optional human curation in `.podras/`. It never mutates the tracker; harvest proposes only.

### Can agents edit the `.podras/` files?
No. Curation and the roadmap are human decision records — write them yourself. Agents read the brief and the envelope.

### What do "fresh" and "stale" mean?
Every source in the envelope carries a freshness stamp; a stale source is still shown, just marked, so the reader knows which claims rest on old evidence.

### Where do agents get the schema?
`podras schema` emits the brief JSON Schema; `podras brief --json` is the contract it validates.

### Is whisper content exposed publicly?
The brief surfaces only what the committed sources contain; session notes feed in the same way as any other source — check your repo before publishing it anywhere public.

### How do I use it in CI?
Run `scripts/guards/brief-drift.sh` as a gate: it fails when the committed render diverges from live evidence.

More: [docs/src/faq.md](docs/src/faq.md).

## Docs / status / license

- [Docs (book)](https://charly-vibes.github.io/podras/) · [Getting started](docs/src/getting-started.md) · [CLI reference](docs/src/cli.md)
- [Status & maturity](docs/src/status.md) · [Motivation & design](docs/src/index.md) · [FAQ](docs/src/faq.md)
- License: [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0)

[![wai-tracked](https://img.shields.io/badge/wai-tracked-8A2BE2)](https://github.com/charly-vibes/wai)
[![Test & Build](https://github.com/charly-vibes/podras/actions/workflows/ci.yml/badge.svg)](https://github.com/charly-vibes/podras/actions/workflows/ci.yml)
[![Release](https://github.com/charly-vibes/podras/actions/workflows/release.yml/badge.svg)](https://github.com/charly-vibes/podras/actions/workflows/release.yml)
[![Docs](https://github.com/charly-vibes/podras/actions/workflows/docs.yml/badge.svg)](https://github.com/charly-vibes/podras/actions/workflows/docs.yml)
[![crates.io](https://img.shields.io/crates/v/podras.svg)](https://crates.io/crates/podras)
[![docs.rs](https://docs.rs/podras/badge.svg)](https://docs.rs/podras)
[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://www.apache.org/licenses/LICENSE-2.0)

## Design principle

**Project status is a projection of evidence plus explicit human intent — never a document people maintain independently.** Beads stays authoritative for tasks; curation stays authoritative for intent; the brief is a trustworthy view, not another source of truth. Portfolio view (many repos) is computed on the fly, never stored.

## Value proposition

| Who | Problem today | What podras gives them |
|---|---|---|
| Human returning to a project | 15 min of archaeology across tracker, specs, git log, session notes | One brief: current focus, next action, blockers, health — with freshness stamps |
| Person showing the project to someone else | Ad-hoc screenshots/prose | A generated, shareable brief artifact |
| AI coding agent (pi, Claude Code, Codex) | Distrusts or invents state; stale AGENTS.md conventions | A typed JSON envelope with per-source freshness, stable IDs, explicit unknowns — a contract, not a prompt |

## Roadmap state

The roadmap lens reads one hand-written, git-committed file: `.podras/roadmap.toml`, the sibling of `.podras/curation.toml`. It is a human decision record — **agents never edit it**. A missing file is legal (the brief simply carries no `roadmap` object); a malformed file degrades the brief with one warning, while `podras roadmap` reports it as a hard error. Dates are ISO-8601; horizons are `now|next|later`; bet statuses are `shaping|committed|building|deferred|shipped|killed`.

The full annotated example — `[identity]`, `[[bets]]`, `[[no_gos]]` with all required and optional fields — lives in [docs/src/getting-started.md](docs/src/getting-started.md).

## Development

```sh
just ci    # lint + test + build
just test  # tests only
cargo install --path .  # run your local build
```

## Research record

The repo's research record is a kaya ledger (`.kaya/ledger.jsonl`). Its
projections — `docs/research/` and `docs/public/` — are generated build
outputs: **do not hand-edit them**. Capture new evidence with
`just record-capture` (skips on a dirty tree) and gate with `just record-check`
(locally the lag gate defaults to warn; CI fails on main). The nightly Record
workflow (`.github/workflows/record.yml`) captures scheduled
snapshots, and docs Pages deploys the dossier alongside the mdBook.

## Naming

Named with a nod to *No podrás* (Cristian Castro) — "tal vez te olvides de mí…" — forgetting is fine; coming back cold should still tell you what's happening. Deterministic, per-repo, a lens you point at a project. (Resembles *podrá* — "it will be able to"; the tool tells you what the project can do next.)
