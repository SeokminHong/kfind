#!/usr/bin/env bash
set -euo pipefail

readonly LEXICON_SHA256="012a2ecfc9ee049cb48f655eb240fa2ed6fc739dfde01526078a976549246e88"

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
source "${repo_root}/scripts/lib/full-pos-source.sh"
output_directory=${1:-"${repo_root}/data/generated/full-pos"}
temporary_directory=$(mktemp -d)
trap 'rm -rf "${temporary_directory}"' EXIT

source_directory=$(fetch_full_pos_source "${temporary_directory}")
stage="${temporary_directory}/output"
mkdir -p "${stage}/LICENSES"

cd "${repo_root}"
cargo run --locked --package kfind-data --bin kfind-data-extract-mecab -- \
  "${stage}/lexicon.bin" \
  "${source_directory}"/*.csv
cargo run --locked --package kfind-data --bin kfind-data-inspect-pos -- \
  "${stage}/lexicon.bin" >"${stage}/STATS.toml"

actual_lexicon_sha=$(sha256_file "${stage}/lexicon.bin")
if [[ "${actual_lexicon_sha}" != "${LEXICON_SHA256}" ]]; then
  echo "lexicon checksum mismatch: expected ${LEXICON_SHA256}, got ${actual_lexicon_sha}" >&2
  exit 1
fi
stats_sha=$(sha256_file "${stage}/STATS.toml")

install -m 0644 "${source_directory}/COPYING" "${stage}/LICENSES/mecab-ko-dic-COPYING"
cat >"${stage}/MANIFEST.toml" <<EOF
schema_version = 1
source = "${FULL_POS_SOURCE_NAME}"
source_url = "${FULL_POS_SOURCE_URL}"
source_sha256 = "${FULL_POS_SOURCE_SHA256}"
source_license = "Apache-2.0"
lexicon_sha256 = "${LEXICON_SHA256}"
stats_sha256 = "${stats_sha}"
extractor = "kfind-data-extract-mecab"
EOF

mkdir -p "${output_directory}/LICENSES"
install -m 0644 "${stage}/lexicon.bin" "${output_directory}/lexicon.bin"
install -m 0644 "${stage}/MANIFEST.toml" "${output_directory}/MANIFEST.toml"
install -m 0644 "${stage}/STATS.toml" "${output_directory}/STATS.toml"
install -m 0644 \
  "${stage}/LICENSES/mecab-ko-dic-COPYING" \
  "${output_directory}/LICENSES/mecab-ko-dic-COPYING"

printf 'full POS lexicon: %s\n' "${output_directory}/lexicon.bin"
printf 'sha256: %s\n' "${LEXICON_SHA256}"
cat "${stage}/STATS.toml"
