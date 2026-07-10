#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 6 ]]; then
  echo "usage: render-homebrew-formula.sh VERSION SOURCE_SHA256 FULL_POS_SHA256 ASSETS_SHA256 LICENSE OUTPUT" >&2
  exit 2
fi

version=$1
source_sha256=$2
full_pos_sha256=$3
assets_sha256=$4
project_license=$5
output=$6
repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

if [[ ! "${version}" =~ ^[0-9]+\.[0-9]+\.[0-9]+([-.][0-9A-Za-z.]+)?$ ]]; then
  echo "invalid release version: ${version}" >&2
  exit 2
fi
for checksum in "${source_sha256}" "${full_pos_sha256}" "${assets_sha256}"; do
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
  -e "s|@SOURCE_SHA256@|${source_sha256}|g" \
  -e "s|@FULL_POS_SHA256@|${full_pos_sha256}|g" \
  -e "s|@ASSETS_SHA256@|${assets_sha256}|g" \
  -e "s|@FORMULA_LICENSE@|$(escape_sed "${formula_license}")|g" \
  "${repo_root}/packaging/homebrew/kfind.rb.in" >"${output}"
