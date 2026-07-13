#!/usr/bin/env bash
set -euo pipefail

readonly COMPONENT_SHA256="5fc46a151e41485dc4b4a3a931135c0f490913f2c2c908b9d87adb87a7c14efd"

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
source "${repo_root}/scripts/lib/full-pos-source.sh"
output_directory=${1:-"${repo_root}/target/component-shadow-resource"}
temporary_directory=$(mktemp -d)
trap 'rm -rf "${temporary_directory}"' EXIT

source_directory=$(fetch_full_pos_source "${temporary_directory}")
stage="${temporary_directory}/output"
manifest="${repo_root}/tools/morph-index-benchmark/Cargo.toml"

cd "${repo_root}"
cargo run --release --locked --manifest-path "${manifest}" -- build-components \
  --source-sha256 "${FULL_POS_SOURCE_SHA256}" \
  --output "${stage}" \
  --matrix "${source_directory}/matrix.def" \
  --char-def "${source_directory}/char.def" \
  --unk-def "${source_directory}/unk.def" \
  "${source_directory}"/*.csv

resource="${stage}/morphology-component-compact.kfc"
resource_sha=$(sha256_file "${resource}")
if [[ "${resource_sha}" != "${COMPONENT_SHA256}" ]]; then
  echo "component checksum mismatch: expected ${COMPONENT_SHA256}, got ${resource_sha}" >&2
  exit 1
fi

mkdir -p "${output_directory}"
install -m 0644 "${resource}" "${output_directory}/morphology-component-compact.kfc"
cat >"${output_directory}/MANIFEST.toml" <<EOF
schema_version = 1
source = "${FULL_POS_SOURCE_NAME}"
source_url = "${FULL_POS_SOURCE_URL}"
source_sha256 = "${FULL_POS_SOURCE_SHA256}"
source_license = "Apache-2.0"
component_sha256 = "${resource_sha}"
builder = "morph-index-benchmark build-components"
EOF

printf 'component shadow resource: %s\n' "${output_directory}/morphology-component-compact.kfc"
printf 'sha256: %s\n' "${COMPONENT_SHA256}"
