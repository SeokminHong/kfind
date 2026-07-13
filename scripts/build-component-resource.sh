#!/usr/bin/env bash
set -euo pipefail

readonly COMPONENT_SHA256="5fc46a151e41485dc4b4a3a931135c0f490913f2c2c908b9d87adb87a7c14efd"

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
source "${repo_root}/scripts/lib/full-pos-source.sh"
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
  "${source_directory}/matrix.def" \
  "${source_directory}/char.def" \
  "${source_directory}/unk.def" \
  "${source_directory}"/*.csv

resource_sha=$(sha256_file "${resource}")
if [[ "${resource_sha}" != "${COMPONENT_SHA256}" ]]; then
  echo "component checksum mismatch: expected ${COMPONENT_SHA256}, got ${resource_sha}" >&2
  exit 1
fi

install -m 0644 "${source_directory}/COPYING" "${stage}/LICENSES/mecab-ko-dic-COPYING"
cat >"${stage}/MANIFEST.toml" <<EOF
schema_version = 1
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
