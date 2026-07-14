#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
node_output="$repo_root/target/npm-node"
component_asset="$repo_root/packages/kfind/assets/morphology-component-compact.kfc"
enriched_asset="$repo_root/packages/kfind/assets/predicates.enriched.tsv"

"$repo_root/scripts/build-npm-package.sh"

rm -rf "$node_output"
wasm-pack build "$repo_root/crates/kfind-wasm" \
  --target nodejs \
  --out-dir "$node_output" \
  --out-name kfind \
  --release

node "$repo_root/scripts/test-npm-package.cjs" \
  "$node_output/kfind.js" \
  "$repo_root/packages/kfind/generated/kfind.d.ts" \
  "$component_asset" \
  "$enriched_asset"

wasm_bytes=$(wc -c <"$repo_root/packages/kfind/generated/kfind_bg.wasm")
component_bytes=$(wc -c <"$component_asset")
if ((wasm_bytes >= component_bytes)); then
  echo "WASM binary must not contain the component asset" >&2
  exit 1
fi
