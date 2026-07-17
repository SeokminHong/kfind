#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
. "$repo_root/scripts/lib/benchmark-guard.sh"
guard_benchmark_entrypoint \
  "$repo_root" browser-startup "$repo_root/scripts/benchmark-browser-startup.sh" "$@"

output="${1:-$repo_root/target/browser-startup/report.json}"
runs="${KFIND_BROWSER_STARTUP_RUNS:-5}"
component_source_dir="${KFIND_COMPONENT_RESOURCE_DIR:-$repo_root/target/component-resource}"
full_pos_source_dir="${KFIND_FULL_POS_RESOURCE_DIR:-$repo_root/target/full-pos}"
build_root="$repo_root/target/browser-startup"
release_wasm_dir="$build_root/release-wasm"
benchmark_wasm_dir="$build_root/benchmark-wasm"
full_pos_packed_path="$build_root/full-pos-packed-prototype.bin"

if [[ -z "${KFIND_COMPONENT_RESOURCE_DIR:-}" ]]; then
  "$repo_root/scripts/build-component-resource.sh" "$component_source_dir"
fi
if [[ -z "${KFIND_FULL_POS_RESOURCE_DIR:-}" ]]; then
  "$repo_root/scripts/build-full-pos.sh" "$full_pos_source_dir"
fi

cargo run \
  --manifest-path "$repo_root/Cargo.toml" \
  --release \
  --locked \
  -p kfind-pos-layout-prototype \
  --bin kfind-pos-layout-prototype \
  -- \
  "$full_pos_source_dir/lexicon.bin" \
  "$full_pos_packed_path"

chrome="${KFIND_BROWSER_CHROME:-}"
if [[ -z "$chrome" && -x "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" ]]; then
  chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
fi
if [[ -z "$chrome" ]]; then
  chrome="$(command -v google-chrome || command -v chromium || true)"
fi
if [[ -z "$chrome" || ! -x "$chrome" ]]; then
  echo "Chrome executable not found; set KFIND_BROWSER_CHROME" >&2
  exit 1
fi

rm -rf "$release_wasm_dir" "$benchmark_wasm_dir"
wasm-pack build "$repo_root/crates/kfind-wasm" \
  --target web \
  --out-dir "$release_wasm_dir" \
  --out-name kfind \
  --release
wasm-pack build "$repo_root/crates/kfind-wasm" \
  --target web \
  --out-dir "$benchmark_wasm_dir" \
  --out-name kfind \
  --release \
  -- \
  --features browser-startup-benchmark

node "$repo_root/scripts/benchmark-browser-startup.mjs" \
  "$release_wasm_dir" \
  "$benchmark_wasm_dir" \
  "$component_source_dir/morphology-component-compact.kfc" \
  "$full_pos_source_dir/lexicon.bin" \
  "$full_pos_packed_path" \
  "$repo_root/scripts/benchmark-browser-startup-page.mjs" \
  "$output" \
  "$runs" \
  "$chrome" \
  "$(git -C "$repo_root" rev-parse HEAD)"
