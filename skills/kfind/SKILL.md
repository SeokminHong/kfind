---
name: kfind
description: Search Korean lemmas and inflected forms in code or documents with kfind. Use for Korean text search that needs morphology-aware candidates, explicit spans, or rule provenance.
---

<!-- managed by kfind init -->

# Search Korean source text with kfind

Use `kfind` instead of literal grep when a Korean lemma may appear with particles,
endings, irregular inflections, or registered derivations. Treat it as a
query-directed text matcher, not semantic search: `v:검증하다` does not match a
paraphrase such as `검증을 수행했다`; search `n:검증` separately when needed.

## Run the agent workflow

1. Choose a part of speech for every morphology atom.
2. Start with `--embedded --boundary any --json` for reproducible, recall-oriented
   candidates.
3. Limit paths, `--glob`, or `--type` before broadening the query.
4. Parse JSON Lines from stdout and diagnostics from stderr.
5. Inspect matched context and discard false positives. Retry with `--boundary smart`
   when the candidate set is too large.

Use `--pos` for a single-part-of-speech query:

```sh
kfind --embedded --boundary any --pos verb --json '검증하다' src docs
```

Tag each atom in a mixed phrase:

```sh
kfind --embedded --boundary any --json 'n:권한 v:검증하다' src
```

Use `--literal` for surface-only matching without morphology expansion:

```sh
kfind --literal --boundary any --json '검증했다' src
```

## Choose parts of speech

Pass the long value to `--pos`, or prefix an atom with its short tag:

| `--pos` value | Atom tag | Part of speech |
| --- | --- | --- |
| `auto` | — | infer candidates; interactive use only |
| `noun` | `n:` | noun |
| `pronoun` | `pro:` | pronoun |
| `numeral` | `num:` | numeral |
| `verb` | `v:` | verb |
| `adjective` | `adj:` | adjective |
| `determiner` | `det:` | determiner |
| `adverb` | `adv:` | adverb |
| `particle` | `j:` | particle (조사) |
| `interjection` | `intj:` | interjection |
| `literal` | `lit:` | literal surface |

Do not combine a global `--pos` with a different atom tag. Quote whitespace to keep
it inside one literal atom, and use backslash to escape the next character.

```sh
kfind --embedded --boundary any --json 'det:새 n:기능' docs
kfind --embedded --boundary any --json 'n:권한 "접근 제어" v:검증하다' src
```

Phrase atoms must occur in order on one line. Use `--max-gap N` to change the
maximum Unicode-scalar gap between adjacent verified tokens; the default is `24`.
Use `--expand derivation` only when registered derivations are also required;
the default `inflection` expansion covers particles and inflected endings.

## Control scope and interpret output

Pass files or directories after the query. Without a path, `kfind` searches the
current directory, or stdin when input is piped. Directory walks honor ignore files
and skip hidden paths by default.

```sh
kfind --embedded --boundary any --pos noun --json \
  --glob '*.rs' --glob '!target/**' '사용자' crates
```

Read records with `type: "match"` as matches. Each match contains `path`, `line`,
`text`, and `spans`. Each span identifies its phrase `atom`, byte ranges for `core`
and completed `token`, matched `surface`, and `origins` containing lemma, POS, and
rule provenance. Respect `offset_unit`; non-UTF-8 paths or text use Base64 fields.
With context options, ignore or separately process `context` and `context_break`
records.

Exit status `0` means at least one match, `1` means no match, and `2` means a usage,
query, data, I/O, or search error. Use `--explain-query` without `--json` to inspect
query planning when a search behaves unexpectedly.
