# Morphology index benchmark

This development-only tool compares two immutable prefix indexes over the same morphology payload:

- `yada` packed Double-Array trie
- `fst` map

The payload preserves every supported MeCab surface analysis as POS, left context ID, right context
ID, and word cost. It remains separate from the normalized query-side full POS lexicon.

Run the pinned full-scale benchmark from the repository root:

```console
scripts/benchmark-morph-index.sh
```

Artifacts and raw probe reports are written below `target/morph-index-benchmark`. Each container
stores the schema version, source archive SHA-256, entry statistics, section lengths, and section
SHA-256 values. `validate` distinguishes schema, source digest, and content integrity failures.

The script runs each storage/index pair in a first-open process (`cold`) and a second process
(`warm`). It does not flush the operating system page cache, so reports must describe cold numbers
as first-open measurements rather than physical-disk cold-cache measurements.

File-backed mappings are opened read-only. The benchmark requires generated `.kfm` files to remain
immutable for the lifetime of each probe process, as required by the memory mapping API.
