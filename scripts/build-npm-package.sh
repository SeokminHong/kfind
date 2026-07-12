#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package_dir="$repo_root/packages/kfind"
generated_dir="$package_dir/generated"

rm -rf "$generated_dir"
wasm-pack build "$repo_root/crates/kfind-wasm" \
  --target bundler \
  --out-dir "$generated_dir" \
  --out-name kfind \
  --release

rm -f "$generated_dir/.gitignore" "$generated_dir/package.json" "$generated_dir/README.md"
cp "$repo_root/LICENSE" "$package_dir/LICENSE"
