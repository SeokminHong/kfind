#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package_dir="$repo_root/packages/kfind"
generated_dir="$package_dir/generated"
node_dir="$package_dir/node"
assets_dir="$package_dir/assets"
component_source_dir="${KFIND_COMPONENT_RESOURCE_DIR:-$repo_root/target/component-resource}"

if [[ -z "${KFIND_COMPONENT_RESOURCE_DIR:-}" ]]; then
  "$repo_root/scripts/build-component-resource.sh" "$component_source_dir"
fi

rm -rf "$generated_dir"
rm -rf "$node_dir"
wasm-pack build "$repo_root/crates/kfind-wasm" \
  --target bundler \
  --out-dir "$generated_dir" \
  --out-name kfind \
  --release

wasm-pack build "$repo_root/crates/kfind-wasm" \
  --target nodejs \
  --out-dir "$node_dir" \
  --out-name kfind \
  --release

rm -f "$generated_dir/.gitignore" "$generated_dir/package.json" "$generated_dir/README.md"
rm -f "$node_dir/.gitignore" "$node_dir/README.md"
rm -rf "$assets_dir"
mkdir -p "$assets_dir/LICENSES"
install -m 0644 \
  "$component_source_dir/morphology-component-compact.kfc" \
  "$assets_dir/morphology-component-compact.kfc"
install -m 0644 "$component_source_dir/MANIFEST.toml" "$assets_dir/MANIFEST.toml"
install -m 0644 \
  "$component_source_dir/LICENSES/mecab-ko-dic-COPYING" \
  "$assets_dir/LICENSES/mecab-ko-dic-COPYING"
install -m 0644 \
  "$repo_root/data/enriched/predicates.tsv" \
  "$assets_dir/predicates.enriched.tsv"
install -m 0644 \
  "$repo_root/data/enriched/MANIFEST.toml" \
  "$assets_dir/predicates.enriched.MANIFEST.toml"
install -m 0644 \
  "$repo_root/data/enriched/NOTICE.md" \
  "$assets_dir/LICENSES/NIKL-derived-data-NOTICE.md"
cp "$repo_root/LICENSE" "$package_dir/LICENSE"
