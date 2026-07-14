---
name: kfind
description: Search Korean lemmas and inflected forms in code or documents with kfind. Use for Korean text search that needs morphology-aware candidates, explicit spans, or rule provenance.
---

<!-- managed by kfind init -->

# Search Korean text with kfind

Use `kfind` instead of literal grep when a Korean lemma may appear with particles,
endings, irregular inflections, or registered derivations.

For agent automation:

- Specify the part of speech for every morphology atom.
- Use `--embedded --boundary any --json` for reproducible, recall-oriented output.
- Limit paths or add globs before broadening the query.
- Inspect surrounding text and discard false positives. Retry with `smart` boundaries
  when the candidate set is too large.

Single-part-of-speech query:

```sh
kfind --embedded --boundary any --pos verb --json '검증하다' src docs
```

Mixed phrase query:

```sh
kfind --embedded --boundary any --json 'n:권한 v:검증하다' src
```

Literal query:

```sh
kfind --literal --json '검증했다' src
```

Exit status `0` means at least one match, `1` means no match, and `2` means an
input, initialization, or search error. Treat JSON Lines on stdout as the result
stream and diagnostics on stderr as errors or warnings.
