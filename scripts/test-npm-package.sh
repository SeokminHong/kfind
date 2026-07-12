#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
node_output="$repo_root/target/npm-node"

"$repo_root/scripts/build-npm-package.sh"

rm -rf "$node_output"
wasm-pack build "$repo_root/crates/kfind-wasm" \
  --target nodejs \
  --out-dir "$node_output" \
  --out-name kfind \
  --release

node "$repo_root/scripts/test-npm-package.cjs" \
  "$node_output/kfind.js" \
  "$repo_root/packages/kfind/generated/kfind.d.ts"
