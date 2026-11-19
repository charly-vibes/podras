## MODIFIED Requirements

### Requirement: Read-only lens with an agent-facing surface

`podras roadmap` SHALL re-emit the brief envelope. A malformed or unreadable
`.podras/roadmap.toml` SHALL be a hard domain error there, an absent one
`roadmap_missing`; `podras brief` SHALL omit `roadmap` and exit 0,
appending one curation-sourced warning. The read verbs SHALL never write
the file; the bet verbs (`bet add|admit|defer|kill`) are its only writers,
and agents never edit it.
The lens SHALL render enums and constraints, not prose. `podras init` SHALL
add two roadmap AGENTS.md lines.

#### Scenario: roadmap_read_only

- **WHEN** `podras roadmap --json --as-of 2026-11-18` and `podras roadmap --write --as-of 2026-11-18` each run twice on an unchanged fixture repository with a roadmap file, then the file is corrupted and both `podras brief` and `podras roadmap` run, then the file is removed and both run again, and `podras init` runs twice
- **THEN** the two payloads are equal after removing `generated_at`, the two renders differ only in `generated_at`, `ROADMAP.md` carries the lens sections only while `ROADMAP.json` carries the full brief envelope, no repository byte outside the `--write` targets changes, the corrupted file is a `roadmap_parse_error` or `roadmap_invalid` domain error from `podras roadmap` while `podras brief` omits `roadmap`, appends one curation-sourced warning carrying that code, leaves the five declared `sources[]` rows unchanged, and exits 0, the absent file reports `roadmap_missing` from `podras roadmap` while `podras brief` omits the object and exits 0, and the AGENTS.md block contains both roadmap lines while the second init is byte-identical
- **VERIFIES** [[roadmap.v1.roadmap_read_only]]
