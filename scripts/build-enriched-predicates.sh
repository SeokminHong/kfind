#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "${script_dir}/.." && pwd)
downloads=${KFIND_NIKL_DOWNLOADS:-"${HOME}/Downloads"}
cache_directory=${KFIND_NIKL_CACHE:-"${XDG_CACHE_HOME:-${HOME}/.cache}/kfind/nikl"}
output_directory=${1:-"${repo_root}/data/enriched"}
stage=$(mktemp -d "${TMPDIR:-/tmp}/kfind-enriched.XXXXXX")
trap 'rm -rf "${stage}"' EXIT HUP INT TERM

krdict="${downloads}/전체 내려받기_한국어기초사전_xml_20260619.zip"
stdict="${downloads}/전체 내려받기_표준국어대사전_xml_20260705.zip"
opendict="${downloads}/전체 내려받기_우리말샘_xml_20260702.zip"

python3 "${repo_root}/tools/nikl-lexicon/import_nikl.py" \
  --krdict "${krdict}" \
  --stdict "${stdict}" \
  --opendict "${opendict}" \
  --output "${stage}/records.tsv" \
  --stats "${stage}/MANIFEST.toml" \
  --cache-dir "${cache_directory}"

cargo run --quiet --locked \
  --manifest-path "${repo_root}/tools/nikl-lexicon/classifier/Cargo.toml" -- \
  "${stage}/records.tsv" \
  "${repo_root}/data" \
  "${stage}/classified"

install -d "${output_directory}"
install -m 0644 "${stage}/classified/predicates.tsv" "${output_directory}/predicates.tsv"
install -m 0644 "${stage}/classified/REPORT.tsv" "${output_directory}/REPORT.tsv"
install -m 0644 "${stage}/classified/STATS.toml" "${output_directory}/STATS.toml"
install -m 0644 "${stage}/MANIFEST.toml" "${output_directory}/MANIFEST.toml"

printf 'enriched predicates: %s\n' "${output_directory}/predicates.tsv"
