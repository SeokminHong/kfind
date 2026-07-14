#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$repo_root/scripts/lib/benchmark-guard.sh"
guard_benchmark_entrypoint \
  "$repo_root" npm-startup "$repo_root/scripts/benchmark-npm-startup.sh" "$@"

node_output="$repo_root/target/npm-startup-node"
output="${1:-$repo_root/target/npm-startup/report.json}"
runs="${KFIND_NPM_STARTUP_RUNS:-5}"
component_source_dir="${KFIND_COMPONENT_RESOURCE_DIR:-$repo_root/target/component-resource}"
full_pos_source_dir="${KFIND_FULL_POS_RESOURCE_DIR:-$repo_root/target/full-pos}"

if [[ -z "${KFIND_FULL_POS_RESOURCE_DIR:-}" ]]; then
  "$repo_root/scripts/build-full-pos.sh" "$full_pos_source_dir"
fi

"$repo_root/scripts/build-npm-package.sh"

rm -rf "$node_output"
wasm-pack build "$repo_root/crates/kfind-wasm" \
  --target nodejs \
  --out-dir "$node_output" \
  --out-name kfind \
  --release

node --expose-gc "$repo_root/scripts/benchmark-npm-startup.cjs" \
  "$node_output/kfind.js" \
  "$component_source_dir/morphology-component-compact.kfc" \
  "$full_pos_source_dir/lexicon.bin" \
  "$output" \
  "$runs"
