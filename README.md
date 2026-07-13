# kfind

[English](README.md) | [한국어](README.ko.md)

Fast Korean lemma and inflection search for code and documents.

`kfind` compiles a Korean lemma or short phrase into bounded surface anchors,
then searches files without running a morphology analyzer over the corpus.

```console
$ kfind 걷다 src docs
docs/guide.md:12: 길을 걸어 갔다.
src/example.txt:8: 손님이 오래 걸었습니다.
```

## Install

Homebrew releases are published through the personal tap:

```sh
brew install seokminhong/brew/kfind
```

To build the current checkout with Rust 1.85 or newer:

```sh
cargo install --locked --path crates/kfind-cli
```

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

Component-aware smart noun searches require explicit initialization. Use
`Engine::with_component_resource` when constructing the engine or call
`load_component_resource` on an existing mutable engine before compiling such a query.

The library and its core dependencies support Rust 1.85's
`wasm32-unknown-unknown` target:

```sh
rustup target add wasm32-unknown-unknown --toolchain 1.85.0
cargo +1.85.0 build --locked --package kfind-wasm --target wasm32-unknown-unknown
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

JavaScript offsets use UTF-16 code units. The package publishes the component
resource as `kfind/assets/morphology-component-compact.kfc`, separate from the
WASM binary. Constructing `Kfind` without it avoids loading the 45.6 MiB asset.
Applications that use component-aware smart noun searches can pass the bytes to
the constructor or call `loadComponentResource` before compiling those queries.
The package has not been published to the registry yet. Its release artifact can
be built and checked locally:

```sh
pnpm --dir packages/kfind run pack:check
```

## Usage

```text
kfind [OPTIONS] <QUERY> [PATH]...
```

For interactive use, part-of-speech tagging is optional. The default auto POS
and `smart` boundary mode favor precise results and use the installed full POS
lexicon when available:

```sh
kfind 걷다 src
kfind 사용자 src docs
kfind 'lit:걸어' data.txt
```

For agent automation, specify the POS of every morphology atom and use `any`,
the embedded lexicon, and JSON Lines output:

```sh
kfind --embedded --boundary any --pos verb --json 걷다 src docs
kfind --embedded --boundary any --json 'n:사용자 v:검증하다' src
```

The agent should inspect context and discard false positives. Narrow the path or
glob, or retry with `smart` boundaries when the candidate set is too large.

When a compiled `smart` plan needs component evidence, the CLI resolves and
validates the installed component resource automatically. Plans that do not
need it leave the asset unloaded. Rust and npm library callers opt in by
supplying the bytes themselves. `--embedded` skips only the full POS lexicon.

Useful search options include `--glob`, `--type`, `--hidden`, `--no-ignore`,
`--encoding`, context flags (`-A`, `-B`, `-C`), `--count`,
`--files-with-matches`, `--quiet`, and `--sort path`. Run `kfind --help` for
the complete CLI contract.

`--explain-query` prints inferred analyses and compiled anchors.
`--explain-match` prints the lemma and rule path behind each match.

## Display language

Human-readable help, errors, diagnostics, and `--explain-*` output follow the
first non-empty value of `LC_ALL`, `LC_MESSAGES`, and `LANG`. A `ko` locale
selects Korean; all other values use English. Option names, accepted values,
JSON fields, and exit codes do not change with the locale.

## Lexicon data

Core irregular predicates and rules are embedded in the binary. Homebrew also
installs the pinned full POS lexicon at `share/kfind/lexicon.bin`; runtime
network access is never required.

Without the full POS file, searches continue with the core lexicon and
heuristics. `--explain-query` reports that preview state. An explicit data
directory can be selected with `--data-dir` or `KFIND_DATA_DIR`.
Use `--embedded` to skip full POS resolution and decoding explicitly.

The full POS artifact is reproducible from the pinned, checksum-verified
`mecab-ko-dic` source:

```sh
scripts/build-full-pos.sh
cargo run --locked -p kfind-testkit --bin verify-gold -- \
  data/generated/full-pos/lexicon.bin
```

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo bench -p kfind-testkit --bench query_matcher
scripts/benchmark-morphology.sh
pnpm --dir packages/kfind run benchmark:startup
pnpm --dir packages/kfind run pack:check
```

The morphology fixture contains 452 positive and negative cases. The Docker
benchmark measures `kfind` on 1,000 cases generated from independent UD
Korean-Kaist and KSL test splits, then compares it with pinned quality and
performance snapshots for Kiwi, Lindera, MeCab-ko, and KOMORAN. External
snapshots are refreshed only when their fixture, adapter, schema, or pinned
version changes. Fuzz targets for query parsing and malformed matcher input
live in `fuzz/`.

The implementation contract and release acceptance criteria are in
[`specs/kfind.md`](specs/kfind.md).

## License

kfind source code and project-authored data are available under the
[MIT License](LICENSE). The Homebrew full POS resource preserves the separate
Apache-2.0 notice from `mecab-ko-dic` under `share/doc/kfind/LICENSES`. UD source
and derived fixtures in the benchmark image remain under CC BY-SA 4.0, with a
per-source notice included in the image.

## Release

Pushing a matching `vX.Y.Z` tag runs the release workflow. It rebuilds and
verifies the full POS resource, publishes source/data/CLI assets, and opens a
Formula PR against `SeokminHong/homebrew-brew`. The tap's `pr-pull` label is
applied only after its Formula tests pass.

The release workflow requires a `TAP_GITHUB_TOKEN` secret with write access to
the tap. It validates the MIT Cargo package metadata before publishing.
