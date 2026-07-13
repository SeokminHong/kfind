# Morphology index benchmark

This development-only tool compares two immutable prefix indexes over the same morphology payload:

- `yada` packed Double-Array trie
- `fst` map

The payload preserves every supported MeCab surface analysis as POS, left context ID, right context
ID, and word cost. It remains separate from the normalized query-side full POS lexicon.

The same run also compares the full schema 3 lattice resource with a decision-equivalent compact
component projection. The compact artifact preserves every source node's POS, context IDs, word
cost, connection matrix, and unknown-word definitions. It omits source analysis metadata that the
component cost decision does not read. The build fails unless both artifacts return identical
analysis counts and scoring checksums for the shared exact and common-prefix workload.

Run the pinned full-scale benchmark from the repository root:

```console
scripts/benchmark-morph-index.sh
```

Artifacts and raw probe reports are written below `target/morph-index-benchmark`. Each container
stores the schema version, source archive SHA-256, entry statistics, section lengths, and section
SHA-256 values. `validate` distinguishes schema, source digest, and content integrity failures.
`component-build-report.json` records full/compact sizes and lookup equivalence. The
`component-*-{resident,mmap}-{cold,warm}.json` probes record initialization, lookup latency, analysis
hits, checksum, and peak RSS.

The script runs each storage/index pair in a first-open process (`cold`) and a second process
(`warm`). It does not flush the operating system page cache, so reports must describe cold numbers
as first-open measurements rather than physical-disk cold-cache measurements.

File-backed mappings are opened read-only. The benchmark requires generated `.kfm` files to remain
immutable for the lifetime of each probe process, as required by the memory mapping API.
