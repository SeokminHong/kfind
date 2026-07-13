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

npm pack --dry-run --json "$package_dir" | node -e '
  const fs = require("node:fs");
  const report = JSON.parse(fs.readFileSync(0, "utf8"))[0];
  const files = new Map(report.files.map((file) => [file.path, file.size]));
  const asset = "assets/morphology-component-compact.kfc";
  if (files.get(asset) !== 47859711) {
    throw new Error("missing or invalid " + asset);
  }
  for (const required of [
    "assets/MANIFEST.toml",
    "assets/LICENSES/mecab-ko-dic-COPYING",
  ]) {
    if (!files.has(required)) {
      throw new Error("missing " + required);
    }
  }
'
