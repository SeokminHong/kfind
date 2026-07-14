#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 8 ]]; then
  echo "usage: render-homebrew-formula.sh VERSION RUST_TOOLCHAIN SOURCE_SHA256 FULL_POS_SHA256 COMPONENT_SHA256 ASSETS_SHA256 LICENSE OUTPUT" >&2
  exit 2
fi

version=$1
rust_toolchain=$2
source_sha256=$3
full_pos_sha256=$4
component_sha256=$5
assets_sha256=$6
project_license=$7
output=$8
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

if [[ ! "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.]+)?$ ]]; then
  echo "invalid release version: ${version}" >&2
  exit 2
fi
if [[ ! "${rust_toolchain}" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "invalid Rust toolchain: ${rust_toolchain}" >&2
  exit 2
fi
for checksum in \
  "${source_sha256}" \
  "${full_pos_sha256}" \
  "${component_sha256}" \
  "${assets_sha256}"; do
  if [[ ! "${checksum}" =~ ^[0-9a-f]{64}$ ]]; then
    echo "invalid sha256: ${checksum}" >&2
    exit 2
  fi
done
if [[ -z "${project_license}" || "${project_license}" == "null" ]]; then
  echo "project license must be finalized before rendering the formula" >&2
  exit 2
fi
case "${project_license}" in
  "MIT OR Apache-2.0")
    formula_license='license any_of: ["MIT", "Apache-2.0"]'
    ;;
  *[!A-Za-z0-9.+-]*)
    echo "unsupported Homebrew license expression: ${project_license}" >&2
    exit 2
    ;;
  *)
    formula_license="license \"${project_license}\""
    ;;
esac

escape_sed() {
  printf '%s' "$1" | sed 's/[&|\\]/\\&/g'
}

mkdir -p "$(dirname "${output}")"
sed \
  -e "s|@VERSION@|$(escape_sed "${version}")|g" \
  -e "s|@RUST_TOOLCHAIN@|${rust_toolchain}|g" \
  -e "s|@SOURCE_SHA256@|${source_sha256}|g" \
  -e "s|@FULL_POS_SHA256@|${full_pos_sha256}|g" \
  -e "s|@COMPONENT_SHA256@|${component_sha256}|g" \
  -e "s|@ASSETS_SHA256@|${assets_sha256}|g" \
  -e "s|@FORMULA_LICENSE@|$(escape_sed "${formula_license}")|g" \
  "${repo_root}/packaging/homebrew/kfind.rb.in" >"${output}"
