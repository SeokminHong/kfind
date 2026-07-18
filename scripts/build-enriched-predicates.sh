#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "${script_dir}/.." && pwd)
downloads=${KFIND_NIKL_DOWNLOADS:-"${HOME}/Downloads"}
cache_directory=${KFIND_NIKL_CACHE:-"${XDG_CACHE_HOME:-${HOME}/.cache}/kfind/nikl"}
output_directory=${1:-"${repo_root}/data/enriched"}
install -d "${repo_root}/target"
candidate_directory=$(mktemp -d "${repo_root}/target/kfind-enriched-candidate.XXXXXX")

printf 'enriched candidate: %s\n' "${candidate_directory}"

krdict="${downloads}/전체 내려받기_한국어기초사전_xml_20260619.zip"
stdict="${downloads}/전체 내려받기_표준국어대사전_xml_20260705.zip"
opendict="${downloads}/전체 내려받기_우리말샘_xml_20260702.zip"

python3 "${repo_root}/tools/nikl-lexicon/import_nikl.py" \
  --krdict "${krdict}" \
  --stdict "${stdict}" \
  --opendict "${opendict}" \
  --output "${candidate_directory}/records.tsv" \
  --stats "${candidate_directory}/MANIFEST.toml" \
  --cache-dir "${cache_directory}"

cargo run --quiet --locked \
  --manifest-path "${repo_root}/tools/nikl-lexicon/classifier/Cargo.toml" -- \
  "${candidate_directory}/records.tsv" \
  "${repo_root}/data" \
  "${candidate_directory}/classified"

"${script_dir}/install-enriched-predicates.sh" "${candidate_directory}" "${output_directory}"
