# CLI reference

Every subcommand and flag, from `podras --help` (v1.1 + roadmap lens).

```
Per-repository project status brief for humans and AI agents

Usage: podras <COMMAND>
```

## `podras brief`

Render the project status brief.

| Flag | Meaning |
|---|---|
| `--json` | Emit the typed JSON envelope |
| `--write` | Write `BRIEF.md` and `BRIEF.json` |
| `--as-of <AS_OF>` | Pin the observation instant (`YYYY-MM-DD`) |
| `--out-dir <OUT_DIR>` | Directory for the render pair; requires `--write` |

Exit 0 always; the envelope carries warnings, never failures, for missing best-effort sources. Identical inputs under `--as-of` produce identical output.

## `podras roadmap`

Render the roadmap lens: identity, bets, and boundaries.

| Flag | Meaning |
|---|---|
| `--json` | Emit the typed JSON envelope |
| `--write` | Write `ROADMAP.md` and `ROADMAP.json` |
| `--as-of <AS_OF>` | Pin the evaluation date (`YYYY-MM-DD`) |
| `--out-dir <OUT_DIR>` | Directory for the render pair; requires `--write` |

Reads `.podras/roadmap.toml`; missing file is a hard error here (unlike `brief`).

## `podras harvest`

Surface mismatch candidates without changing any state.

| Flag | Meaning |
|---|---|
| `--json` | Emit the typed JSON envelope |
| `--as-of <AS_OF>` | Pin the evaluation date (`YYYY-MM-DD`) |

Discovery is not commitment: harvest proposes, humans decide. Idempotent and provenance-preserving.

## `podras focus`

Select the curated focus item.

```
Usage: podras focus <ID>    # issue id to focus
```

Select-or-replace: the previous focus is demoted automatically with a notice. Cap is N=1 by construction (`max_focus` configurable); exceeding it is a health warning, never a block.

## `podras park`

Park an item until a resurface trigger.

```
Usage: podras park --reason <REASON> --resurface <RESURFACE> <ID>
```

| Flag/arg | Meaning |
|---|---|
| `<ID>` | Issue id to park |
| `--reason <REASON>` | Why the item is parked |
| `--resurface <RESURFACE>` | `YYYY-MM-DD` date or `event:<name>` trigger |

Both flags are mandatory; parking without a trigger is a graveyard.

## `podras init`

Scaffold curation state and the AGENTS.md managed block. No flags. Idempotent: creates `.podras/curation.toml` and writes the managed block (drift-checkable, provenance footer) telling agents the house rules — run `brief --json` first, freshness first, beads owns lifecycle, never edit `.podras/*.toml`.

## `podras schema`

Emit the brief JSON Schema. No flags. The published schema for the `schema_version: "brief.v1"` envelope.

## `podras help`

Print the top-level message or the help of any subcommand: `podras help <COMMAND>`.
