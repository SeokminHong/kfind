#!/usr/bin/env bash
set -euo pipefail

readonly SOURCE_NAME="mecab-ko-dic-2.1.1-20180720"
readonly SOURCE_URL="https://bitbucket.org/eunjeon/mecab-ko-dic/downloads/${SOURCE_NAME}.tar.gz"
readonly SOURCE_SHA256="fd62d3d6d8fa85145528065fabad4d7cb20f6b2201e71be4081a4e9701a5b330"
readonly LEXICON_SHA256="dbf7f8f282e14cef7b4962dd217bda89456dc908aed3e59f7b5e4a58edbb3a79"

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
output_directory=${1:-"${repo_root}/data/generated/full-pos"}
temporary_directory=$(mktemp -d)
trap 'rm -rf "${temporary_directory}"' EXIT

sha256_file() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}

archive="${temporary_directory}/${SOURCE_NAME}.tar.gz"
curl --fail --location --silent --show-error --output "${archive}" "${SOURCE_URL}"

actual_source_sha=$(sha256_file "${archive}")
if [[ "${actual_source_sha}" != "${SOURCE_SHA256}" ]]; then
  echo "source checksum mismatch: expected ${SOURCE_SHA256}, got ${actual_source_sha}" >&2
  exit 1
fi

tar -xzf "${archive}" -C "${temporary_directory}"
source_directory="${temporary_directory}/${SOURCE_NAME}"
stage="${temporary_directory}/output"
mkdir -p "${stage}/LICENSES"

cd "${repo_root}"
cargo run --locked --package kfind-data --bin kfind-data-extract-mecab -- \
  "${stage}/lexicon.bin" \
  "${source_directory}"/*.csv

actual_lexicon_sha=$(sha256_file "${stage}/lexicon.bin")
if [[ "${actual_lexicon_sha}" != "${LEXICON_SHA256}" ]]; then
  echo "lexicon checksum mismatch: expected ${LEXICON_SHA256}, got ${actual_lexicon_sha}" >&2
  exit 1
fi

install -m 0644 "${source_directory}/COPYING" "${stage}/LICENSES/mecab-ko-dic-COPYING"
cat >"${stage}/MANIFEST.toml" <<EOF
schema_version = 1
source = "${SOURCE_NAME}"
source_url = "${SOURCE_URL}"
source_sha256 = "${SOURCE_SHA256}"
source_license = "Apache-2.0"
lexicon_sha256 = "${LEXICON_SHA256}"
extractor = "kfind-data-extract-mecab"
EOF

mkdir -p "${output_directory}/LICENSES"
install -m 0644 "${stage}/lexicon.bin" "${output_directory}/lexicon.bin"
install -m 0644 "${stage}/MANIFEST.toml" "${output_directory}/MANIFEST.toml"
install -m 0644 \
  "${stage}/LICENSES/mecab-ko-dic-COPYING" \
  "${output_directory}/LICENSES/mecab-ko-dic-COPYING"

printf 'full POS lexicon: %s\n' "${output_directory}/lexicon.bin"
printf 'sha256: %s\n' "${LEXICON_SHA256}"
