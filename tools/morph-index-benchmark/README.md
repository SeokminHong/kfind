# Morphology index benchmark

This development-only tool compares two immutable prefix indexes over the same morphology payload:

- `yada` packed Double-Array trie
- `fst` map

The payload preserves every supported MeCab surface analysis as POS, left context ID, right context ID, and word cost. It remains separate from the normalized query-side full POS lexicon.

The same run compares the full schema 3 lattice resource, the decision-equivalent compact schema 1 component projection, and the policy-neutral schema 3 analysis graph. The compact artifact preserves the fields read by the current component cost decision. The graph also preserves source analysis type, source positions, normalized expression relations, components, and source-derived categorical POS transitions. The build fails unless the full and graph resources have identical source analyses, relation components, connection matrix and unknown definitions, and unless all three artifacts return identical analysis counts and scoring checksums for the shared exact and common-prefix workload.

Run the pinned full-scale benchmark from the repository root:

```console
scripts/benchmark-morph-index.sh
```

Artifacts and raw probe reports are written below `target/morph-index-benchmark`. Each container stores the schema version, source archive SHA-256, entry statistics, section lengths, and section SHA-256 values. `validate` distinguishes schema, source digest, and content integrity failures. `component-build-report.json` records full, compact and graph sizes, graph component count and lookup equivalence. The `component-*-{resident,mmap}-{cold,warm}.json` probes record initialization, lookup latency, analysis hits, checksum, and peak RSS. Graph probes currently use resident storage only because the validated graph resource owns its bytes.

The script runs each storage/index pair in a first-open process (`cold`) and a second process (`warm`). It does not flush the operating system page cache, so reports must describe cold numbers as first-open measurements rather than physical-disk cold-cache measurements.

File-backed mappings are opened read-only. The benchmark requires generated `.kfm` files to remain immutable for the lifetime of each probe process, as required by the memory mapping API.
