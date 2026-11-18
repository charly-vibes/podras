# Getting started

This walkthrough covers the whole lifecycle in one repo. Every command below exits 0 and on-demand renders carry the envelope contract (`"ok": true` in `--json` mode).

## 1. Scaffold curation state

```console
$ podras init
```

`init` is idempotent: it scaffolds `.podras/curation.toml` (all keys commented out) and writes a managed block into `AGENTS.md` telling agents how to use podras: run `podras brief --json` first, freshness first, beads owns task lifecycle, focus is intent not permission, never silently change focus or unpark, never edit `.podras/*.toml`. Commit both files — per-repo, git-committed state means a fresh clone already has its orientation context.

## 2. Render the brief

```console
$ podras brief              # narrative for humans
$ podras brief --json       # typed JSON envelope for agents
```

The brief reads beads, git, openspec (best-effort), and whisper session notes, then layers your curation on top. Nine fixed sections: purpose, focus, in flight, blocked/waiting, next/ready, parked, recent, health, sources & freshness. Missing inputs produce per-source warnings, never failures.

Pin the observation instant for reproducible output:

```console
$ podras brief --as-of 2026-11-18
```

Identical inputs under `--as-of` produce identical output.

## 3. Record intent: focus and park

```console
$ podras focus podras-abc
$ podras park podras-def --reason "blocked on the sync design" --resurface 2027-01-15
```

`focus` is select-or-replace: the previous focus item is demoted automatically with a notice, and the cap is N=1 by construction (`max_focus` is configurable). `park` requires a reason and a resurface trigger — a `YYYY-MM-DD` date or `event:<name>`. Parked items never appear as ready; ones past their resurface date are flagged for re-triage.

Both verbs write `.podras/curation.toml`, which humans may also hand-edit. Agents never edit it.

## 4. Commit the render pair

```console
$ podras brief --write
```

Writes `BRIEF.md` and `BRIEF.json` at the repo root — generated-only, freshness-stamped, visible to non-cloners and tool-less agents. The committed envelope differs from the on-demand render only in `generated_at`. Re-run and commit whenever status changes; CI can regen-and-diff as a drift gate.

## 5. The roadmap lens

```console
$ podras roadmap --write
```

The roadmap lens is a separate why-lens: identity, bets, and boundaries (no-gos). It reads `.podras/roadmap.toml`, a hand-written, git-committed decision record — write it yourself; **agents never edit it** (see [Configuration](configuration.md) for the full shape). `--write` emits:

- `ROADMAP.md` — the lens narrative only;
- `ROADMAP.json` — the full brief envelope with the `roadmap` object attached, so an agent also gets `sources[]`, `health`, `truncation`, and `source_hash`.

If the file is missing, `brief` simply carries no `roadmap` object; `roadmap` itself is a hard error. A malformed file degrades: `brief` omits the object and adds exactly one curation warning.

## House rules

- Reads are side-effect free; harvest is idempotent; **curation beats inference**.
- Freshness is per-source; `unknown` is never turned into green.
- podras never mutates the tracker — harvest proposes, humans commit.
