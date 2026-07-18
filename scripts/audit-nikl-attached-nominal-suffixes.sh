#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "${script_dir}/.." && pwd)
downloads=${KFIND_NIKL_DOWNLOADS:-"${HOME}/Downloads"}
cache_directory=${KFIND_NIKL_CACHE:-"${XDG_CACHE_HOME:-${HOME}/.cache}/kfind/nikl"}
output=${1:-"${repo_root}/data/rules/nikl-attached-nominal-suffixes.tsv"}
install -d "${repo_root}/target"
candidate_directory=$(mktemp -d "${repo_root}/target/kfind-attached-nominal-suffix-candidate.XXXXXX")
candidate="${candidate_directory}/suffixes.tsv"

printf 'attached nominal suffix candidate: %s\n' "${candidate}"

python3 "${repo_root}/tools/nikl-lexicon/audit_nominal_suffixes.py" \
  --krdict "${downloads}/전체 내려받기_한국어기초사전_xml_20260619.zip" \
  --stdict "${downloads}/전체 내려받기_표준국어대사전_xml_20260705.zip" \
  --opendict "${downloads}/전체 내려받기_우리말샘_xml_20260702.zip" \
  --surface 하 \
  --output "${candidate}" \
  --cache-dir "${cache_directory}"

"${script_dir}/install-nikl-attached-nominal-suffixes.sh" "${candidate}" "${output}"
