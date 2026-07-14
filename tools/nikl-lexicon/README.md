# NIKL predicate importer

This tool builds the reviewed `ReuDoubleL`, `Reo`, and regular `EU_DROP`
predicate layer from pinned National Institute of Korean Language dictionary
snapshots.

The Python adapter reads the three ZIP snapshots, preserves source-record and
homonym identity, and emits normalized predicate records. The Rust classifier
uses kfind's predicate generator to identify diagnostic conjugations. A record
is promoted only when Korean Basic Dictionary and Standard Korean Language
Dictionary independently support the same analysis. Urimalsaem is retained as
audit evidence.

Run from the repository root:

```sh
scripts/build-enriched-predicates.sh
```

Set `KFIND_NIKL_DOWNLOADS` when the pinned ZIP files are outside `~/Downloads`.
Each snapshot is extracted once under
`${KFIND_NIKL_CACHE:-${XDG_CACHE_HOME:-~/.cache}/kfind/nikl}` and reused while
its SHA-256 stays unchanged. Set `KFIND_NIKL_CACHE` to move this cache.
Raw snapshots and dictionary examples are not copied into the repository.
