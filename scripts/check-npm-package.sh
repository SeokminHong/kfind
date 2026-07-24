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
npm_license="$(node -p "require('$package_dir/package.json').license")"
npm_bin="$(node -p "require('$package_dir/package.json').bin.kfind")"
test "$cargo_version" = "$npm_version"
test "$npm_name" = "@kfind/kfind"
test "$npm_license" = "SEE LICENSE IN LICENSES.md"
test "$npm_bin" = "bin/kfind.js"

npm pack --ignore-scripts --dry-run --json "$package_dir" | node -e '
  const fs = require("node:fs");
  const report = JSON.parse(fs.readFileSync(0, "utf8"))[0];
  const files = new Map(report.files.map((file) => [file.path, file.size]));
  const asset = "assets/morphology-component-compact.kfc";
  if (files.get(asset) !== 37103813) {
    throw new Error("missing or invalid " + asset);
  }
  const enriched = "assets/predicates.enriched.tsv";
  if (files.get(enriched) !== 42910) {
    throw new Error("missing or invalid " + enriched);
  }
  for (const required of [
    "assets.d.ts",
    "assets.js",
    "bin/kfind.js",
    "node/kfind.js",
    "node/kfind_bg.wasm",
    "node/package.json",
    "assets/MANIFEST.toml",
    "assets/LICENSES/mecab-ko-dic-COPYING",
    "assets/predicates.enriched.MANIFEST.toml",
    "assets/LICENSES/NIKL-derived-data-NOTICE.md",
    "LICENSES.md",
  ]) {
    if (!files.has(required)) {
      throw new Error("missing " + required);
    }
  }
  const executable = report.files.find((file) => file.path === "bin/kfind.js");
  if (executable.mode !== 0o755) {
    throw new Error("bin/kfind.js is not executable");
  }
'

grep -Fq "원자료 저작자·제공자는 국립국어원입니다." \
  "$package_dir/assets/LICENSES/NIKL-derived-data-NOTICE.md"
grep -Fq "CC BY-SA 2.0 KR" \
  "$package_dir/assets/LICENSES/NIKL-derived-data-NOTICE.md"

packed_consumer_dir="$(
  mktemp -d "${TMPDIR:-/tmp}/kfind-packed-consumer.XXXXXX"
)"

cleanup_packed_consumer() {
  if [[ -d "$packed_consumer_dir" ]]; then
    rm -rf -- "$packed_consumer_dir"
  fi
}
trap cleanup_packed_consumer EXIT

npm pack \
  --ignore-scripts \
  --pack-destination "$packed_consumer_dir" \
  "$package_dir" >/dev/null

shopt -s nullglob
packed_tarballs=("$packed_consumer_dir"/*.tgz)
test "${#packed_tarballs[@]}" -eq 1

mkdir "$packed_consumer_dir/consumer"
npm install \
  --ignore-scripts \
  --no-audit \
  --no-fund \
  --prefix "$packed_consumer_dir/consumer" \
  "${packed_tarballs[0]}" >/dev/null
cp "$repo_root/scripts/test-npm-assets.mjs" \
  "$packed_consumer_dir/consumer/test-npm-assets.mjs"
node "$packed_consumer_dir/consumer/test-npm-assets.mjs"
