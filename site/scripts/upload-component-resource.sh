#!/usr/bin/env bash

set -euo pipefail

site_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
repo_root="$(cd "$site_dir/.." && pwd)"
component_dir="$repo_root/target/site-component-resource"
component_asset="$component_dir/morphology-component-compact.kfc"

"$repo_root/scripts/build-component-resource.sh" "$component_dir"

wrangler r2 object put \
  "kfind-assets/morphology-component-compact.kfc" \
  --file "$component_asset" \
  --content-type "application/octet-stream" \
  --cache-control "public, max-age=3600" \
  --remote
