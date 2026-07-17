# kfind

[English](README.md) | [한국어](README.ko.md) |
[Documentation & playground](https://kfind.pages.dev)

Fast Korean lemma and inflection search for code and documents.

`kfind` analyzes the query once, compiles bounded candidate programs,
and scans files without running a morphology analyzer over the corpus. It finds
inflected forms while retaining grep-like path filtering, context, and output
modes.

```console
$ kfind -n 걷다 src docs
docs/guide.md:12: 길을 걸어 갔다.
src/example.txt:8: 손님이 오래 걸었습니다.
```

## Purpose

`kfind` is a query-directed text matcher for agents and interactive search. It
turns a short Korean lemma or phrase into a bounded search plan, then returns
candidate spans and morphology provenance from files or in-memory text. Agents
can retrieve a broad candidate set quickly and use surrounding context for the
final judgment; people can choose the more selective default workflow.

Morphology is a means of planning and verifying a search, not the product's
output. `kfind` does not analyze every sentence in the input corpus.

## Goals and non-goals

Goals:

- Compile short queries into bounded plans and scan large text collections with
  low overhead.
- Provide tested recall and precision for the supported Korean morphology while
  preserving matched spans, lemmas, POS, and rule provenance.
- Offer reproducible offline behavior through the CLI, Rust library, and
  JavaScript/WebAssembly package.

Non-goals:

- A general-purpose sentence tokenizer or morphology analyzer, or a backend
  optimized to lead morphology-analyzer throughput rankings.
- Semantic search, synonym or paraphrase expansion, and semantic homonym
  disambiguation.
- Complete reverse analysis of arbitrary surface forms or unrestricted coverage
  of every Korean construction.

## Features

- Finds noun-particle combinations, predicate endings, irregular inflections,
  and selected productive derivations from a lemma.
- Searches ordered, same-line phrases with per-atom part-of-speech tags.
- Offers `smart` boundaries for interactive precision and `any` boundaries for
  recall-oriented automation.
- Walks files in parallel with ignore rules, globs, named file types, hidden-file
  control, stdin, and explicit encodings.
- Produces terminal text, context, counts, file lists, JSON Lines, and query or
  match provenance.
- Runs offline. Core rules are embedded; Homebrew installs the optional full POS
  and morphology-component resources plus the agent skill.
- Exposes the query compiler and matcher through Rust and WebAssembly libraries.

## Install

Homebrew releases are published through the personal tap:

```sh
brew install seokminhong/brew/kfind
```

`brew install` and `brew upgrade` install the component resource built for the
same kfind version and run an integrity check after installation. Run
`kfind --check-data` to repeat that check manually.

To build the current checkout with Rust 1.97 or newer:

```sh
cargo install --locked --path crates/kfind-cli
```

## Agent skill setup

Run `kfind --init` in a project directory. In a terminal it opens a checkbox
selector for Claude Code, Codex, Gemini CLI, and custom stdout output. The
project destinations are `.claude/skills/kfind`, `.agents/skills/kfind`, and
`.gemini/skills/kfind`.

```sh
# Interactive checkbox selection.
kfind --init

# Reproducible one-liner.
kfind --init --agent codex --agent claude-code

# Non-interactive stdin selection.
printf 'codex\ngemini\n' | kfind --init

# Write only SKILL.md content to stdout for another agent.
kfind --init --agent custom > path/to/kfind/SKILL.md
```

Homebrew installs the canonical skill under `share/kfind` with the binary, but it
cannot choose a project or agent on the user's behalf. Run `kfind --init` once in
each project. That initialization links to Homebrew's stable `opt/kfind` path, so
later `brew upgrade kfind` runs update those project skills automatically. A source
or Cargo installation writes a managed copy; rerun `kfind --init` to update it.
Existing skills without the kfind management marker are never overwritten.

## Quick start

```sh
# Infer the part of speech and find inflections.
kfind 걷다 src docs

# Search a lemma as a noun and consume valid particles.
kfind --pos noun 사용자 src

# Search an ordered phrase. Each atom can have its own part of speech.
kfind 'n:권한 v:검증하다' src --max-gap 24

# Search bytes as a literal without morphology expansion.
kfind --literal '걸어' data.txt

# Restrict files and print two lines of context.
kfind 걷다 . --type-add 'docs:*.{md,mdx,txt}' --type docs -C 2

# Emit stable machine-readable records for automation.
kfind --embedded --boundary any --pos verb --json 걷다 src docs
```

With no `PATH`, `kfind` reads piped stdin or searches `.` when stdin is a
terminal. `-` selects stdin explicitly.

## Search model

### Morphology expansion

The default `inflection` mode includes noun plurals and particle chains,
predicate endings, copula forms, and the irregular classes covered by the
versioned rules and lexicon, plus cross-checked dictionary conjugations.
`derivation` adds registered productive forms such as `-적`, `-하다`, `-되다`,
and `-시키다`, along with dictionary-linked derived forms. `literal` disables
morphology expansion.

The query is expanded; the corpus is not fully tokenized or analyzed. This keeps
file scanning fast, but it is not semantic search. For example,
`v:검증하다` does not match a paraphrase such as `검증을 수행했다`; search
`n:검증` separately when that wording matters. A surface form such as `걸어`
also is not reverse-analyzed into every possible lemma unless the lemma or POS is
given explicitly.

### Query language

Atoms are separated by whitespace. Quotes keep a phrase inside one literal
atom, and backslashes escape the next character. The supported POS tags are:

| Tag | Part of speech |
| --- | --- |
| `n:` | noun |
| `pro:` | pronoun |
| `num:` | numeral |
| `v:` | verb |
| `adj:` | adjective |
| `det:` | determiner |
| `adv:` | adverb |
| `j:` | particle |
| `intj:` | interjection |
| `lit:` | literal |

```sh
kfind 'n:권한 "접근 제어" v:검증하다' src
kfind 'det:새 n:기능' docs
kfind 'lit:걸어' data.txt
```

Phrase atoms must appear in order on the same line. `--max-gap` measures the
Unicode scalar distance from the end of one verified token to the start of the
next. A global `--pos` may be combined with atom tags only when they name the
same POS.

### Boundary policies

| Policy | Behavior | Typical use |
| --- | --- | --- |
| `smart` | Applies POS-aware verification and checks the completed token span. It can use the optional structural resource to prove exact POS/component spans and adjacent-token arrangements. | Interactive search; default |
| `token` | Requires token boundaries around every core and completed token span. | Strict standalone tokens |
| `any` | Does not require left or right token boundaries. | Recall-oriented automation with downstream context review |

A one-syllable query remains conservative under `smart`. Explicit particle POS
can expand registered allomorphs such as `은/는`, `이/가`, and `으로/로`; an
untagged query searches only the particle surface that was written.

Semantic ambiguity is deliberately retained: both `걷다` and `걸다` may match
`걸었고`. This differs from structural POS evidence. Under `smart`, adjacent
token arrangement selects `매일/MAG` in `매일 보고 싶어` and rejects `n:매`;
the copular structure in `독수리가 아니라 매일 수도 있어` selects
`매/NNG + 이/VCP + ㄹ/ETM` and rejects `adv:매일`. If structure remains
ambiguous, kfind keeps supported candidates for recall.

### Human and agent workflows

For interactive use, omit the POS. The default `auto` POS and `smart` boundary
favor precision and use the installed full POS lexicon when available:

```sh
kfind 걷다 src
kfind 사용자 src docs
```

For agent automation, specify every morphology atom, use `any`, the embedded
lexicon, and JSON Lines:

```sh
kfind --embedded --boundary any --pos verb --json 걷다 src docs
kfind --embedded --boundary any --json 'n:사용자 v:검증하다' src
```

The agent workflow returns a broader candidate set. Inspect the surrounding
text, narrow paths or globs, or retry with `smart` if the set is too large.

## CLI reference

```text
kfind [OPTIONS] <QUERY> [PATH]...
kfind --init [--agent <AGENT>]...
```

### Query and compilation

| Option | Values and default | Description |
| --- | --- | --- |
| `--pos <POS>` | `auto` (default), `noun`, `pronoun`, `numeral`, `verb`, `adjective`, `determiner`, `adverb`, `particle`, `interjection`, `literal` | Forces one POS for the entire query. |
| `--expand <LEVEL>` | `inflection` (default), `literal`, `derivation` | Chooses the morphology expansion level. `derivation` includes inflection. |
| `--boundary <POLICY>` | `smart` (default), `token`, `any` | Chooses match-boundary verification. |
| `--literal` | off | Shortcut for `--expand literal --pos literal`; conflicting `--expand` or `--pos` values are errors. |
| `--embedded` | off | Skips full POS discovery and decoding. A `smart` plan may still require the component resource. |
| `--max-gap <NUM>` | `24` | Sets the maximum Unicode scalar gap between adjacent phrase atoms. |
| `--unicode-normalization <MODE>` | `nfc` (default), `canonical`, `none` | Uses NFC only, generates NFC and NFD patterns, or matches input bytes without normalization. |

### Files and input

| Option | Values and default | Description |
| --- | --- | --- |
| `--encoding <ENCODING>` | `auto` (default), `utf-8`, `utf-16le`, `utf-16be`, `euc-kr` | Selects input decoding. `auto` detects BOM-marked UTF-16 and otherwise uses UTF-8; it does not guess EUC-KR. |
| `--glob <GLOB>` | repeatable | Adds an include glob or an exclude glob prefixed with `!`. |
| `--type <TYPE>` | repeatable | Searches only files in a named type. |
| `--type-add <NAME:GLOB>` | repeatable | Defines or extends a named file type. |
| `--hidden` | off | Includes hidden files and directories. |
| `--no-ignore` | off | Disables `.gitignore`, `.ignore`, global Git ignore, and parent ignore rules. |
| `--threads <NUM>` | automatic | Sets the number of file-search worker threads. |

Directory walks exclude hidden and ignored entries by default and do not follow
symbolic links. An explicitly named file is searched even when an ignore rule
would exclude it. Input stops at the first NUL byte and treats the file as
binary.

### Output and diagnostics

| Option | Default | Description |
| --- | --- | --- |
| `-n`, `--line-number` | off | Prints one-based line numbers. |
| `-H`, `--with-filename` | automatic | Always prints file names; conflicts with `-h`. |
| `-h`, `--no-filename` | automatic | Never prints file names; conflicts with `-H`. |
| `-C`, `--context <NUM>` | `0` | Prints `NUM` lines before and after each match. |
| `-B`, `--before-context <NUM>` | context value | Overrides the number of lines before each match. |
| `-A`, `--after-context <NUM>` | context value | Overrides the number of lines after each match. |
| `-l`, `--files-with-matches` | off | Prints each matching file once and stops that file after its first match; conflicts with `--count`, `--quiet`, and `--json`. |
| `-c`, `--count` | off | Prints the number of lines with at least one verified match per file; conflicts with `--quiet` and `--json`. |
| `-q`, `--quiet` | off | Prints no matches and stops globally after the first match; conflicts with `--json`. |
| `--json` | off | Writes one JSON object per match or context record; conflicts with `--explain-query`. |
| `--color <WHEN>` | `auto`; `auto`, `always`, `never` | Controls terminal highlighting. `auto` enables color only for standard output to a terminal. |
| `--no-pager` | off | Bypasses the pager when writing standard text results to a terminal. |
| `--column` | off | Prints a one-based Unicode scalar column and implies line-number output. |
| `--explain-query` | off | Prints inferred analyses, candidate programs, consumption states, normalization, and lexicon status before results. |
| `--explain-match` | off | Adds the lemma and rule path behind each text match. JSON already includes origin metadata. |
| `--sort path` | unsorted parallel stream | Buffers completed file results and emits path order; this uses memory proportional to results and can reduce parallel throughput. |

File names are printed automatically when searching a directory or multiple
inputs. Match and context lines use `:` and `-` separators respectively. Standard
text results with terminal stdin and stdout use a built-in TUI that opens when
the search starts and adds completed result rows progressively. A long match line expands
to one row per verified match, and each row truncates both sides so its target
remains visible at a position balanced by the original before/after ratio. The
layout is recomputed on terminal resize. Scrolling stops when the final row reaches
the bottom of the content area. Navigation remains active while searching.
Use `↑`/`↓` or `k`/`j` to move and `q` or `Esc` to exit and stop the remaining search.
Redirects and pipes, JSON Lines, count, file summaries, quiet mode,
and `--no-pager` retain the direct stdout stream. If the TUI cannot start, output
falls back to standard text on stdout.

Repeated navigation is frame-paced by the content viewport size without dropping
movement. Larger viewports combine more held-key input in each frame to limit
terminal scroll operations.

The pager index scales with completed source lines and expanded match rows. Use
`--no-pager` for a bounded stream when large interactive result sets are not
needed.

JSON Lines records contain `type`, path, line, optional column, text, spans,
core and token byte ranges, matched surface, lemma/POS origins, rule paths, and
an `offset_unit`. Non-UTF-8 paths and text use Base64 fields rather than lossy
conversion.

### Data and command information

| Option | Default | Description |
| --- | --- | --- |
| `--data-dir <PATH>` | automatic discovery | Reads `lexicon.bin`, optional `predicates.enriched.tsv`, and `morphology-component-compact.kfc` from one explicit directory. |
| `--check-data` | off | Validates the installed full POS and component resources, including the exact component package version, then exits. Supports `--json` and `--data-dir`. |
| `--user-lexicon <PATH>` | XDG config path | Loads a TOML user lexicon instead of the default config lookup. |
| `--init` | off | Initializes the kfind skill in the current directory without a query. |
| `--agent <AGENT>` | TTY selection or stdin; repeatable | Selects `claude-code`, `codex`, `gemini`, or `custom`; requires `--init`. |
| `--help` | — | Prints localized command help. `-h` is reserved for `--no-filename`. |
| `-V`, `--version` | — | Prints the version. |

The CLI checks `--user-lexicon`, `KFIND_USER_LEXICON`, and then
`$XDG_CONFIG_HOME/kfind/lexicon.toml` or `$HOME/.config/kfind/lexicon.toml`:

```toml
[[predicate]]
lemma = "플러그인하다"
pos = "verb"
alternation = "Ha"

[[nominal]]
surface = "LLM"
```

Entries extend the bundled data. Set `replace = true` on an entry to replace
existing analyses in the same morphology category for that lemma.

### Exit status and display language

| Code | Meaning |
| ---: | --- |
| `0` | At least one match was found, or initialization/data validation succeeded. |
| `1` | No match was found. |
| `2` | Usage, query compilation, data, I/O, or search error. |

Human-readable help, errors, diagnostics, and `--explain-*` output follow the
first non-empty value of `LC_ALL`, `LC_MESSAGES`, and `LANG`. A `ko` locale
selects Korean; all other values use English. Option names, accepted values,
JSON fields, and exit codes do not change with the locale.

## Lexicon data

Core irregular predicates and rules are embedded in the binary. Homebrew also
installs the pinned full POS lexicon, CC BY-SA enriched predicate metadata and
surfaces, and compact morphology-component resource under `share/kfind`;
runtime network access is never required.

The component header records its kfind package version. A mismatched binary and
component fail during decoding instead of falling back to a stale resource.
Package upgrades replace both artifacts; kfind never updates them in the
background.

Without the full POS file, searches continue with the core lexicon and
heuristics. `--explain-query` reports that preview state. `--data-dir` or
`KFIND_DATA_DIR` selects an explicit resource directory. Outside `--embedded`,
`predicates.enriched.tsv` is loaded when present. `--embedded` skips full POS and
enriched predicate resolution. A compiled `smart` plan that requires component
evidence still resolves and validates the component resource; plans that do not
need it leave it unloaded.

The external lexicon data are reproducible from pinned, checksum-verified
`mecab-ko-dic` and NIKL dictionary snapshots:

```sh
scripts/build-full-pos.sh
cargo run --locked -p kfind-testkit --bin verify-gold -- \
  data/generated/full-pos/lexicon.bin
scripts/build-enriched-predicates.sh
```

Of 12,888 conjugations supported by two NIKL dictionaries, the enriched
generator stores only the 130 that productive rules cannot generate. It also
stores 153 predicate-to-adverb forms whose Korean Basic Dictionary entry IDs
agree in both directions. The resulting TSV has 283 surface-only rows and is
27,707 bytes. Conjugations are available in the default `inflection` mode;
derived adverbs require `derivation`.

## Benchmarks

kfind measures morphology quality, end-to-end CLI throughput, resource startup,
and literal scanning as separate workloads. The benchmark contract defines the
reproducible commands, inputs, warm-up and run counts, and report requirements.
Measurements and comparisons remain in their individual reports.

- [Benchmark contract](docs/benchmarks/README.md)

## Library

### Rust

The `kfind` crate exposes the same query compiler and morphology matcher for
in-memory UTF-8 input:

```rust
use kfind::{CompileOptions, Engine};

let engine = Engine::new()?;
let matcher = engine
    .compile("걷다", &CompileOptions::default())
    .expect("query should compile");
let text = "길을 걸어 갔다.";
let matches = matcher.find_all(text.as_bytes());

assert_eq!(&text[matches[0].span.clone()], "걸어");
```

Build the same dictionary-quality profile as the CLI with `ResourceBundle` and
`Engine::with_resources`. The bundle accepts optional full POS binary, enriched
predicate TSV, and component bytes. Existing individual constructors delegate
to the same initialization path. Component bytes can also be installed later
with `load_component_resource` before compiling a plan that needs them.

The 1.x stable facade consists of engine construction, compile options and
errors, matching, and match provenance at the crate root. Caller-assembled
lexicons and query-plan inspection require an explicit `kfind::expert` import;
workspace implementation crates are not published separately.

The library and its core dependencies support Rust 1.97's
`wasm32-unknown-unknown` target:

```sh
rustup target add wasm32-unknown-unknown --toolchain 1.97.0
cargo +1.97.0 build --locked --package kfind-wasm --target wasm32-unknown-unknown
```

### JavaScript

The unscoped `kfind` npm package provides ESM WebAssembly bindings and generated
TypeScript declarations for browser bundlers:

```js
import { Kfind } from "kfind";

const engine = new Kfind();
const matcher = engine.compile("걷다");
const text = "😀 길을 걸어 갔다.";
const matches = matcher.findAll(text);

console.log(text.slice(matches[0].start, matches[0].end)); // 걸어
```

JavaScript offsets use UTF-16 code units. `Kfind.withResources` accepts optional
`fullPos`, `enrichedPredicates`, and `component` fields as one profile. The
package publishes the enriched TSV as `kfind/assets/predicates.enriched.tsv`
and the component resource as
`kfind/assets/morphology-component-compact.kfc`, both separate from the WASM
binary. Constructing `Kfind` without resources loads neither external asset.
Component bytes can also be installed later with `loadComponentResource`.
The package `prepack` check rebuilds the WASM and version-matched component,
runs Node and TypeScript smoke tests, and verifies the packed asset list.

The package has not been published to the registry yet. Its release artifact
can be built and checked locally:

```sh
pnpm --dir packages/kfind run pack:check
```

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/benchmark-criterion.sh
scripts/benchmark-morphology.sh
pnpm --dir packages/kfind run benchmark:startup
pnpm --dir packages/kfind run pack:check
```

The morphology fixture contains 588 positive and negative regression cases. The
Docker benchmark measures `kfind` on 1,000 manually reviewed cases sampled from
UD Korean-Kaist, then compares it with pinned Kiwi, Lindera, MeCab-ko, and
KOMORAN snapshots. Rejected Korean-Kaist sentences and Korean-KSL remain
unscored robustness candidates. Fuzz targets and their fixed seed corpora live
in `fuzz/`. CI runs every target for 15 seconds through `scripts/run-fuzz.sh`.

The implementation contract and release acceptance criteria are in
[`specs/kfind.md`](specs/kfind.md).

## License

kfind source code and project-authored data are available under the
[MIT License](LICENSE). The Homebrew full POS and component resources preserve
the Apache-2.0 notice from `mecab-ko-dic`, while enriched predicate data
preserves its CC BY-SA 2.0 Korea notice under `share/doc/kfind/LICENSES`.
The Formula uses `license :cannot_represent` because this combination cannot be
expressed as an SPDX license expression. UD source and derived fixtures in the
benchmark image remain under CC BY-SA 4.0, with a per-source notice included in
the image.

## Release

Pushing a matching `vX.Y.Z` tag runs the release workflow. It rebuilds and
verifies the full POS and component resources, publishes source/data/CLI assets
and the npm package, and opens a Formula PR against `SeokminHong/homebrew-brew`.
Prereleases use the npm `next` tag and stable releases use `latest`. The tap's
`pr-pull` label is applied only after its Formula tests pass.

The release workflow requires a `TAP_GITHUB_TOKEN` secret with write access to
the tap. It validates the MIT Cargo package metadata before publishing.
