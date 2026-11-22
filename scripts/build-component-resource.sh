#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
component_checksum_file="${repo_root}/data/generated/morphology-component-compact.sha256"
COMPONENT_SHA256=$(<"${component_checksum_file}")
refresh_checksum=0

if [[ "${1:-}" == "--refresh-checksum" ]]; then
  refresh_checksum=1
  shift
fi

if [[ ! "${COMPONENT_SHA256}" =~ ^[0-9a-f]{64}$ ]]; then
  echo "invalid component checksum: ${component_checksum_file}" >&2
  exit 1
fi

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
  "${repo_root}/data/rules" \
  "${source_directory}"/*.csv \
  "${repo_root}/data/rules/stdict-nominal-prefix-components.csv" \
  "${repo_root}/data/rules/attested-nominal-compound-components.csv"

resource_sha=$(sha256_file "${resource}")
if [[ "${refresh_checksum}" == 1 ]]; then
  printf '%s\n' "${resource_sha}" >"${component_checksum_file}"
  COMPONENT_SHA256=${resource_sha}
fi
if [[ "${resource_sha}" != "${COMPONENT_SHA256}" ]]; then
  echo "component checksum mismatch: expected ${COMPONENT_SHA256}, got ${resource_sha}" >&2
  exit 1
fi

install -m 0644 "${source_directory}/COPYING" "${stage}/LICENSES/mecab-ko-dic-COPYING"
cat >"${stage}/MANIFEST.toml" <<EOF
schema_version = 7
kfind_version = "${component_version}"
source = "${FULL_POS_SOURCE_NAME}"
source_url = "${FULL_POS_SOURCE_URL}"
source_sha256 = "${FULL_POS_SOURCE_SHA256}"
source_license = "Apache-2.0"
nominal_roles_source = "krdict"
nominal_roles_source_sha256 = "a8ab7d044d4f6341e0f217db63f38f4d18beed3e1f153130f6cb4e9494fea1d6"
nominal_roles_license = "CC-BY-SA-2.0-KR"
nominal_prefixes_source = "stdict"
nominal_prefixes_source_sha256 = "880b31447146df5879c076012b21d4cc3c0c24e70fd91be7fc73f7ff7da34d52"
nominal_prefixes_license = "CC-BY-SA-2.0-KR"
nominal_suffixes_source = "stdict"
nominal_suffixes_source_sha256 = "880b31447146df5879c076012b21d4cc3c0c24e70fd91be7fc73f7ff7da34d52"
nominal_suffixes_license = "CC-BY-SA-2.0-KR"
component_sha256 = "${resource_sha}"
builder = "kfind-data-build-component"
EOF

mkdir -p "${output_directory}/LICENSES"
install -m 0644 "${resource}" "${output_directory}/morphology-component-compact.kfc"
install -m 0644 "${stage}/MANIFEST.toml" "${output_directory}/MANIFEST.toml"
install -m 0644 "${repo_root}/data/enriched/NOTICE.md" \
  "${output_directory}/LICENSES/NIKL-derived-data-NOTICE.md"
install -m 0644 \
  "${stage}/LICENSES/mecab-ko-dic-COPYING" \
  "${output_directory}/LICENSES/mecab-ko-dic-COPYING"

printf 'component resource: %s\n' "${output_directory}/morphology-component-compact.kfc"
printf 'sha256: %s\n' "${COMPONENT_SHA256}"
