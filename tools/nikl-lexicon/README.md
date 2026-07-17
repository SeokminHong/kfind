# NIKL predicate importer

This tool builds the reviewed `DToL`, `DropS`, `BToWa`, `BToWo`, `DropH`,
`ReuDoubleL`, `Reo`, and `UToEo` predicate layer from pinned National Institute
of Korean Language dictionary snapshots. Same-shape regular conjugations and
regular `EU_DROP` are retained as controls. A regular analysis is emitted only
when independently supported records require it alongside an irregular analysis
for the same lemma and fine POS.

The Python adapter reads the three ZIP snapshots, preserves source-record and
homonym identity, and emits normalized predicate records. The Rust classifier
uses kfind's predicate generator to identify diagnostic conjugations. A record
is promoted only when Korean Basic Dictionary and Standard Korean Language
Dictionary independently support the same analysis. Urimalsaem is retained as
audit evidence. Core duplicates and analyses already covered by productive
suffix rules are recorded in the report but omitted from the enriched artifact.

Run from the repository root:

```sh
scripts/build-enriched-predicates.sh
```

Set `KFIND_NIKL_DOWNLOADS` when the pinned ZIP files are outside `~/Downloads`.
Each snapshot is extracted once under
`${KFIND_NIKL_CACHE:-${XDG_CACHE_HOME:-~/.cache}/kfind/nikl}` and reused while
its SHA-256 stays unchanged. Set `KFIND_NIKL_CACHE` to move this cache.
Raw snapshots and dictionary examples are not copied into the repository.

## Ending catalog

`scripts/audit-nikl-endings.sh` reads the same pinned snapshots and regenerates
`data/rules/nikl-modern-endings.tsv`. Korean Basic Dictionary and Standard
Korean Language Dictionary ending headwords form the runtime catalog;
Urimalsaem identifiers remain provenance evidence. The generated file contains
normalized surfaces, original headwords, grammatical categories, and source
record identifiers. It does not copy definitions or examples.

The runtime compiler treats the catalog as vocabulary, not as permission to
attach every ending to every stem. Stem-final conditions, irregular
alternations, ending order, auxiliary paths, and whole-lemma conflicts remain
separate structural checks.

## Particle catalog

`scripts/audit-nikl-particles.sh` reads the same snapshots and regenerates
`data/rules/nikl-modern-particles.tsv` plus a runtime coverage report. It uses
only structured headword, POS, status, and source-ID fields. Definitions and
examples are not copied or mined for particle chains.

The catalog inventories atomic particle vocabulary. Particle combinations are
accepted only through `data/rules/particles.toml` transitions, so a compound
surface such as `까지도` is represented as `까지 → 도`, not as a new atomic form.
