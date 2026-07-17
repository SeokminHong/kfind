#!/usr/bin/env bash
set -euo pipefail

readonly COMPONENT_SHA256="d3a7eb486eef5faa92e006dd72e5ff72b63befb4bb102013f2d19e5fc32ff00a"

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
source "${repo_root}/scripts/lib/full-pos-source.sh"
component_version=$(cargo pkgid -p kfind-data | sed -E 's/.*[#@]//')
output_directory=${1:-"${repo_root}/target/component-resource"}
temporary_directory=$(mktemp -d)
trap 'rm -rf "${temporary_directory}"' EXIT

source_directory=$(fetch_full_pos_source "${temporary_directory}")
stage="${temporary_directory}/output"
resource="${stage}/morphology-component-compact.kfc"
mkdir -p "${stage}/LICENSES"

cd "${repo_root}"
cargo run --release --locked --package kfind-data --bin kfind-data-build-component -- \
  "${resource}" \
  "${FULL_POS_SOURCE_SHA256}" \
  "${source_directory}"/*.csv

resource_sha=$(sha256_file "${resource}")
if [[ "${resource_sha}" != "${COMPONENT_SHA256}" ]]; then
  echo "component checksum mismatch: expected ${COMPONENT_SHA256}, got ${resource_sha}" >&2
  exit 1
fi

install -m 0644 "${source_directory}/COPYING" "${stage}/LICENSES/mecab-ko-dic-COPYING"
cat >"${stage}/MANIFEST.toml" <<EOF
schema_version = 5
kfind_version = "${component_version}"
source = "${FULL_POS_SOURCE_NAME}"
source_url = "${FULL_POS_SOURCE_URL}"
source_sha256 = "${FULL_POS_SOURCE_SHA256}"
source_license = "Apache-2.0"
component_sha256 = "${resource_sha}"
builder = "kfind-data-build-component"
EOF

mkdir -p "${output_directory}/LICENSES"
install -m 0644 "${resource}" "${output_directory}/morphology-component-compact.kfc"
install -m 0644 "${stage}/MANIFEST.toml" "${output_directory}/MANIFEST.toml"
install -m 0644 \
  "${stage}/LICENSES/mecab-ko-dic-COPYING" \
  "${output_directory}/LICENSES/mecab-ko-dic-COPYING"

printf 'component resource: %s\n' "${output_directory}/morphology-component-compact.kfc"
printf 'sha256: %s\n' "${COMPONENT_SHA256}"
