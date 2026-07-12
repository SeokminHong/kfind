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

The `kfind` crate exposes the same query compiler and morphology matcher for
in-memory UTF-8 input:

```rust
use kfind::{CompileOptions, Engine};

let engine = Engine::embedded().expect("embedded data should be valid");
let matcher = engine
    .compile("걷다", &CompileOptions::default())
    .expect("query should compile");
let text = "길을 걸어 갔다.";
let matches = matcher.find_all(text.as_bytes());

assert_eq!(&text[matches[0].span.clone()], "걸어");
```

The library and its core dependencies support Rust 1.85's
`wasm32-unknown-unknown` target:

```sh
rustup target add wasm32-unknown-unknown --toolchain 1.85.0
cargo +1.85.0 build --locked --package kfind --target wasm32-unknown-unknown
```

This target support is for Rust library consumers. JavaScript bindings and
package generation are not included yet.

## Usage

```text
kfind [OPTIONS] <QUERY> [PATH]...
```

Queries may use explicit part-of-speech tags:

```sh
kfind 'n:사용자 v:검증하다' .
kfind 'lit:걸어' data.txt
kfind 걷다 --expand inflection --json .
```

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
```

The morphology fixture contains 413 positive and negative cases. The Docker
benchmark runs 1,000 cases generated from independent UD Korean-Kaist and KSL
test splits through `kfind`, Kiwi, and Lindera. Fuzz
targets for query parsing and malformed matcher input live in `fuzz/`.

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
