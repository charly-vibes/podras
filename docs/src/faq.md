# FAQ

## Why is the output deterministic? What does `--as-of` do?

Status must be trustworthy across runs and across consumers, so podras separates volatile metadata from semantics: `--as-of <YYYY-MM-DD>` pins the observation instant, and identical inputs under `--as-of` produce identical output. The committed render (`brief --write`) differs from the on-demand render only in `generated_at`. Envelope content is bounded with a deterministic order (e.g. 5 bets by status rank then `admitted_at` then `id`; all warnings kept).

## Which files does podras read?

`brief` harvests from sources that already exist: the beads tracker and git history (mandatory), openspec changes (best-effort — missing gives a per-source warning, never a failure), whisper session notes (light read of the latest branch entry), and your curation files (`.podras/curation.toml`, read-if-present; `.podras/roadmap.toml` for the lens object). It reads all of them; it writes only what `--write` (or the `focus`/`park`/`init` verbs) produce. It never mutates the tracker.

## Can agents edit `.podras/roadmap.toml` (or `curation.toml`)?

No. Both are human-owned: `focus` and `park` write curation; `roadmap.toml` is written by hand (or by the `bet` verbs once stage ⑦ lands). `podras init` writes an AGENTS.md managed block that states the rules explicitly — never edit `.podras/*.toml`, run `podras brief --json` first, and treat roadmap `later`/`shaping`/`deferred` items as non-requirements and no-gos as hard constraints.

## Why does `brief` degrade silently on a malformed roadmap file?

Deliberate asymmetry: a broken `roadmap.toml` makes `podras brief` omit the `roadmap` object and add exactly one curation warning instead of failing — the now-lens should still work. `podras roadmap` reports the same file as a hard error, because there the lens itself is what you asked for. In both directions the brief never manufactures a coherent false narrative.

## Are whisper session notes private?

Yes, by default. Whisper *content* never appears in the committed render (`brief --write`) — on-demand renders may include it, and the committed render carries only the freshness stamp. Publishing notes into the committed render is a per-repo opt-in: `publish_whisper = true` in `.podras/curation.toml`.

## How does CI consume `BRIEF.json`?

It is a typed JSON envelope — `schema_version: "brief.v1"`, per-source `observed_at`, typed enums, explicit unknowns, bounded sections — with the published schema emitted by `podras schema`. `brief --write` keeps a committed `BRIEF.md`/`BRIEF.json` pair at the repo root, so non-cloning consumers and tool-less agents can read status directly, and a CI regen-and-diff check can gate drift. Per-source freshness means a stale input never masquerades as healthy: `staleness` is `fresh|stale|unknown` and unknown is never turned into green.

## Is there a portfolio / multi-repo view?

It is mapped in the design but computed on the fly, never stored, and its timing was left unvalidated — no portfolio command ships today.
