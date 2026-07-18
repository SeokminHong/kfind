# NIKL predicate importer

This tool builds the reviewed `DToL`, `DropS`, `BToWa`, `BToWo`, `DropH`,
`ReuDoubleL`, `Reo`, and `UToEo` predicate layer from pinned National Institute
of Korean Language dictionary snapshots. Same-shape regular conjugations and
regular `EU_DROP` are retained as controls. A regular analysis is emitted only
when independently supported records require it alongside an irregular analysis
for the same lemma and fine POS.

The Python adapter reads the three ZIP snapshots, preserves source-record and
homonym identity, and emits normalized predicate records. It also proposes only
bounded adjective-to-adverb shapes (`-없다`/`-같다` plus `이`, and `르` to
`ㄹ리`) and records them when both the Korean Basic Dictionary and Standard
Korean Language Dictionary independently list the adjective and resulting
adverb. It also reads direct, reciprocal Korean Basic Dictionary derivation
relations whose verb target has a bounded `-이/-히/-리/-기-` voice shape. The
Rust classifier uses kfind's predicate generator to identify diagnostic
conjugations and promotes only cross-dictionary agreement, including independent
Standard Korean Language Dictionary records for both voice lemmas. Urimalsaem is
retained as audit evidence. Core duplicates and analyses already covered by
productive suffix rules are recorded in the report but omitted from the
enriched artifact.

Run from the repository root:

```sh
scripts/build-enriched-predicates.sh
```

Generation writes a reusable candidate under `target/` before validating its
distribution size and installing it. A failed candidate remains there. After
changing only validation policy, rerun validation and installation without
regenerating dictionary data:

```sh
scripts/install-enriched-predicates.sh target/kfind-enriched-candidate.XXXXXX
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

The catalog inventories atomic particle vocabulary and normalized host classes
from Korean Basic Dictionary grammatical annotations and Standard Korean
Language Dictionary `grammar_info`. Definitions and examples are not used for
host classification. Particle combinations are accepted only through the role,
host, and transition fields in `data/rules/particles.toml`, so a compound
surface such as `까지도` is represented as `까지 → 도`, not as a new atomic form.

## Exact lexeme audit

`audit_lexemes.py` looks up an explicit set of headwords in all three pinned
snapshots and writes their structured POS and lexical relations as JSON. It is
intended for failure triage where a dictionary-backed lemma, derivation, or
abbreviation claim must be distinguished from a corpus example or free-text
definition.

```sh
python3 tools/nikl-lexicon/audit_lexemes.py \
  --krdict /path/to/krdict.zip \
  --stdict /path/to/stdict.zip \
  --opendict /path/to/opendict.zip \
  --cache-dir ~/.cache/kfind/nikl \
  --query 누구 --query 누군가 --query 무어 --query 무언가 \
  --output target/nikl-lexemes.json
```

The report includes pinned snapshot hashes and exact missing-headword lists by
source. It preserves only headword, POS, status, source identifier, and explicit
relation fields; definitions and examples are excluded.

## Attached nominal suffix catalog

`scripts/audit-nikl-attached-nominal-suffixes.sh` generates one reusable catalog
candidate from the pinned snapshots. A separate installer validates the schema,
reviewed surface set, and Korean Basic Dictionary plus Standard Korean Language
Dictionary evidence before copying that candidate into `data/rules`. Urimalsaem
identifiers remain additional provenance. Rerun validation after policy changes
without reading the dictionary snapshots again:

```sh
scripts/install-nikl-attached-nominal-suffixes.sh \
  target/kfind-attached-nominal-suffix-candidate.XXXXXX/suffixes.tsv
```

Adding a surface to the catalog does not generate another matcher candidate. The
runtime validator still requires aligned nominal-prefix, suffix, and particle
paths.
