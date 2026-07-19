#!/usr/bin/env bash

set -euo pipefail

site_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(cd "$site_dir/.." && pwd)"
wasm_dir="$site_dir/src/generated-wasm"
benchmark_dir="$site_dir/src/generated-benchmark"
benchmark_snapshot="$repo_root/docs/benchmarks/site-morphology.json"

rm -rf "$wasm_dir" "$benchmark_dir"
mkdir -p "$wasm_dir" "$benchmark_dir"

wasm-pack build "$repo_root/crates/kfind-wasm" \
  --target web \
  --out-dir "$wasm_dir" \
  --out-name kfind \
  --release

rm -f \
  "$wasm_dir/.gitignore" \
  "$wasm_dir/package.json" \
  "$wasm_dir/README.md"

install -m 0644 \
  "$benchmark_snapshot" \
  "$benchmark_dir/site-morphology.json"

wasm_bytes="$(wc -c < "$wasm_dir/kfind_bg.wasm")"
pages_file_limit=$((25 * 1024 * 1024))
if ((wasm_bytes >= pages_file_limit)); then
  echo "site WASM exceeds the Cloudflare Pages 25 MiB file limit" >&2
  exit 1
fi
