#!/usr/bin/env bash
set -euo pipefail

readonly MORPHOLOGY_SHA256="50bbaa64b06a080c7fa09c13e21090388a1c0f5109ed413546e0004ce7794f23"

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
source "${repo_root}/scripts/lib/full-pos-source.sh"
output_directory=${1:-"${repo_root}/target/morphology-resource"}
temporary_directory=$(mktemp -d)
trap 'rm -rf "${temporary_directory}"' EXIT

source_directory=$(fetch_full_pos_source "${temporary_directory}")
resource="${output_directory}/morphology.bin"

cd "${repo_root}"
cargo run --release --locked --package kfind-data --bin kfind-data-build-morphology -- \
  "${resource}" \
  "${FULL_POS_SOURCE_SHA256}" \
  "${source_directory}/matrix.def" \
  "${source_directory}/char.def" \
  "${source_directory}/unk.def" \
  "${source_directory}"/*.csv

resource_sha=$(sha256_file "${resource}")
if [[ "${resource_sha}" != "${MORPHOLOGY_SHA256}" ]]; then
  echo "morphology checksum mismatch: expected ${MORPHOLOGY_SHA256}, got ${resource_sha}" >&2
  exit 1
fi
cat >"${output_directory}/MANIFEST.toml" <<EOF
schema_version = 3
source = "${FULL_POS_SOURCE_NAME}"
source_url = "${FULL_POS_SOURCE_URL}"
source_sha256 = "${FULL_POS_SOURCE_SHA256}"
source_license = "Apache-2.0"
morphology_sha256 = "${resource_sha}"
builder = "kfind-data-build-morphology"
EOF

printf 'morphology resource: %s\n' "${resource}"
printf 'sha256: %s\n' "${MORPHOLOGY_SHA256}"
