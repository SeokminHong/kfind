#!/usr/bin/env bash

set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
. "$ROOT/scripts/lib/benchmark-guard.sh"
guard_benchmark_entrypoint \
    "$ROOT" \
    search-baseline \
    "$ROOT/scripts/benchmark-search-baseline.sh" \
    "$@"

RUNS=${KFIND_SEARCH_BASELINE_RUNS:-10}
WARMUP=${KFIND_SEARCH_BASELINE_WARMUP:-2}
REPETITIONS=${KFIND_SEARCH_BASELINE_REPETITIONS:-4096}
SKIP_BUILD=${KFIND_SEARCH_BASELINE_SKIP_BUILD:-0}
KFIND_BIN=${KFIND_SEARCH_BASELINE_KFIND_BIN:-"$ROOT/target/release/kfind"}
REVISION=${KFIND_SEARCH_BASELINE_REVISION:-$(git -C "$ROOT" rev-parse HEAD)}
OUTPUT_DIR=${1:-"$ROOT/target/benchmark/search-baseline/$REVISION"}
FULL_POS_DIR=${KFIND_SEARCH_BASELINE_FULL_POS_DIR:-"$ROOT/target/full-pos"}
COMPONENT_DIR=${KFIND_SEARCH_BASELINE_COMPONENT_DIR:-"$ROOT/target/component-resource"}
DATA_DIR="$OUTPUT_DIR/resources"
FULL_POS_SHA256=012a2ecfc9ee049cb48f655eb240fa2ed6fc739dfde01526078a976549246e88
COMPONENT_SHA256=d7c3c1cea20bf4cdafa6b6b9507cee07be12be672ca1e2c16887ae71a13d4a67

fail() {
    printf 'benchmark-search-baseline: %s\n' "$*" >&2
    exit 2
}

for command in awk cargo git grep install python3 rg; do
    command -v "$command" >/dev/null 2>&1 ||
        fail "required command not found: $command"
done
if command -v shasum >/dev/null 2>&1; then
    sha256_file() {
        shasum -a 256 "$1" | awk '{ print $1 }'
    }
elif command -v sha256sum >/dev/null 2>&1; then
    sha256_file() {
        sha256sum "$1" | awk '{ print $1 }'
    }
else
    fail "required SHA-256 command not found: shasum or sha256sum"
fi
for value in "$RUNS" "$WARMUP" "$REPETITIONS"; do
    [[ "$value" =~ ^[1-9][0-9]*$ ]] ||
        fail "runs, warmup, and repetitions must be positive integers"
done

git -C "$ROOT" cat-file -e "${REVISION}^{commit}" 2>/dev/null ||
    fail "benchmark revision is not a repository commit: $REVISION"
if [[ "$SKIP_BUILD" != 1 ]]; then
    [[ "$REVISION" == "$(git -C "$ROOT" rev-parse HEAD)" ]] ||
        fail "revision must match HEAD when building the benchmark binary"
    git -C "$ROOT" diff --quiet ||
        fail "tracked worktree changes must be committed before measurement"
    git -C "$ROOT" diff --cached --quiet ||
        fail "staged worktree changes must be committed before measurement"
    cargo build \
        --release \
        --locked \
        --package kfind-cli \
        --bin kfind \
        --manifest-path "$ROOT/Cargo.toml"
fi
[[ -x "$KFIND_BIN" ]] || fail "kfind binary is not executable: $KFIND_BIN"

if [[ ! -f "$FULL_POS_DIR/lexicon.bin" ]]; then
    "$ROOT/scripts/build-full-pos.sh" "$FULL_POS_DIR"
fi
if [[ ! -f "$COMPONENT_DIR/morphology-component-compact.kfc" ]]; then
    "$ROOT/scripts/build-component-resource.sh" "$COMPONENT_DIR"
fi
[[ "$(sha256_file "$FULL_POS_DIR/lexicon.bin")" == "$FULL_POS_SHA256" ]] ||
    fail "full POS resource checksum mismatch"
[[ "$(sha256_file "$COMPONENT_DIR/morphology-component-compact.kfc")" == \
    "$COMPONENT_SHA256" ]] ||
    fail "component resource checksum mismatch"

mkdir -p "$DATA_DIR"
install -m 0644 "$FULL_POS_DIR/lexicon.bin" "$DATA_DIR/lexicon.bin"
install -m 0644 \
    "$COMPONENT_DIR/morphology-component-compact.kfc" \
    "$DATA_DIR/morphology-component-compact.kfc"

python3 "$ROOT/tools/search-baseline/benchmark.py" \
    --data-dir "$DATA_DIR" \
    --kfind "$KFIND_BIN" \
    --output "$OUTPUT_DIR" \
    --repetitions "$REPETITIONS" \
    --revision "$REVISION" \
    --runs "$RUNS" \
    --warmup "$WARMUP"
