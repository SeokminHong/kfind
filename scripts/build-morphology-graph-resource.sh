#!/usr/bin/env bash
set -euo pipefail

readonly GRAPH_SHA256="dbb70e83408f955ca548b6f5db91d0cce1c644ad01dbdfc429d5d6ac172a8c3c"

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
source "${repo_root}/scripts/lib/full-pos-source.sh"
output_directory=${1:-"${repo_root}/target/morphology-graph-resource"}
temporary_directory=$(mktemp -d)
trap 'rm -rf "${temporary_directory}"' EXIT

source_directory=$(fetch_full_pos_source "${temporary_directory}")
stage="${temporary_directory}/output"
resource="${stage}/morphology-component-graph.kfc"
mkdir -p "${stage}/LICENSES"

cd "${repo_root}"
cargo run --release --locked --package kfind-data --bin kfind-data-build-morphology-graph -- \
  "${resource}" \
  "${FULL_POS_SOURCE_SHA256}" \
  "${source_directory}/matrix.def" \
  "${source_directory}/char.def" \
  "${source_directory}/unk.def" \
  "${source_directory}"/*.csv

resource_sha=$(sha256_file "${resource}")
if [[ "${resource_sha}" != "${GRAPH_SHA256}" ]]; then
  echo "morphology graph checksum mismatch: expected ${GRAPH_SHA256}, got ${resource_sha}" >&2
  exit 1
fi

install -m 0644 "${source_directory}/COPYING" "${stage}/LICENSES/mecab-ko-dic-COPYING"
cat >"${stage}/MANIFEST.toml" <<EOF
schema_version = 4
source = "${FULL_POS_SOURCE_NAME}"
source_url = "${FULL_POS_SOURCE_URL}"
source_sha256 = "${FULL_POS_SOURCE_SHA256}"
source_license = "Apache-2.0"
graph_sha256 = "${resource_sha}"
builder = "kfind-data-build-morphology-graph"
EOF

mkdir -p "${output_directory}/LICENSES"
install -m 0644 "${resource}" "${output_directory}/morphology-component-graph.kfc"
install -m 0644 "${stage}/MANIFEST.toml" "${output_directory}/MANIFEST.toml"
install -m 0644 \
  "${stage}/LICENSES/mecab-ko-dic-COPYING" \
  "${output_directory}/LICENSES/mecab-ko-dic-COPYING"

printf 'morphology graph resource: %s\n' "${output_directory}/morphology-component-graph.kfc"
printf 'sha256: %s\n' "${GRAPH_SHA256}"
