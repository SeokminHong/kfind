#!/usr/bin/env bash

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
package_dir="$repo_root/packages/kfind"

"$repo_root/scripts/test-npm-package.sh"

cargo_version="$(
  cargo metadata --no-deps --format-version 1 |
    jq -r '.packages[] | select(.name == "kfind-wasm") | .version'
)"
npm_version="$(node -p "require('$package_dir/package.json').version")"
npm_name="$(node -p "require('$package_dir/package.json').name")"
test "$cargo_version" = "$npm_version"
test "$npm_name" = "kfind"

npm pack --dry-run --json "$package_dir"
