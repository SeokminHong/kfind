#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
. "$ROOT/scripts/lib/benchmark-guard.sh"
guard_benchmark_entrypoint \
    "$ROOT" real-corpus-blind "$ROOT/tools/morph-compare/real_corpus/run.sh" "$@"

OUTPUT_DIR=${1:-"$ROOT/target/real-corpus-blind"}
REPORT_DIR=${2:-"$OUTPUT_DIR"}
CASES="$ROOT/tools/morph-compare/real_corpus/cases.jsonl"
SOURCES="$ROOT/tools/morph-compare/real_corpus/sources.json"
FULL_POS_DIR="$ROOT/target/full-pos"
COMPONENT_DIR="$ROOT/target/component-resource"
RUNNER="$ROOT/tools/morph-compare/runner/target/release/morph-benchmark-runner"
REVISION=$(git -C "$ROOT" rev-parse HEAD)

mkdir -p "$OUTPUT_DIR" "$REPORT_DIR"
if [ ! -f "$FULL_POS_DIR/lexicon.bin" ]; then
    "$ROOT/scripts/build-full-pos.sh" "$FULL_POS_DIR"
fi
if [ ! -f "$COMPONENT_DIR/morphology-component-compact.kfc" ]; then
    "$ROOT/scripts/build-component-resource.sh" "$COMPONENT_DIR"
fi

cargo build \
    --release \
    --locked \
    --manifest-path "$ROOT/tools/morph-compare/runner/Cargo.toml"

KFIND_FULL_POS_LEXICON="$FULL_POS_DIR/lexicon.bin" \
KFIND_ENRICHED_PREDICATES="$ROOT/data/enriched/predicates.tsv" \
KFIND_COMPONENT_RESOURCE="$COMPONENT_DIR/morphology-component-compact.kfc" \
    "$RUNNER" boundary kfind-embedded any "$CASES" "$OUTPUT_DIR/agent.json"

KFIND_FULL_POS_LEXICON="$FULL_POS_DIR/lexicon.bin" \
KFIND_ENRICHED_PREDICATES="$ROOT/data/enriched/predicates.tsv" \
KFIND_COMPONENT_RESOURCE="$COMPONENT_DIR/morphology-component-compact.kfc" \
    "$RUNNER" untagged kfind-full-pos smart "$CASES" "$OUTPUT_DIR/user.json"

python3 "$ROOT/tools/morph-compare/real_corpus/evaluate.py" \
    --cases "$CASES" \
    --sources "$SOURCES" \
    --profile "agent=$OUTPUT_DIR/agent.json" \
    --profile "user=$OUTPUT_DIR/user.json" \
    --revision "$REVISION" \
    --output-json "$REPORT_DIR/2026-07-14-real-corpus-blind.json" \
    --output-markdown "$REPORT_DIR/2026-07-14-real-corpus-blind.md"
