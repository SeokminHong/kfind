#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "${script_dir}/.." && pwd)
candidate_directory=${1:?usage: install-enriched-predicates.sh CANDIDATE [OUTPUT]}
output_directory=${2:-"${repo_root}/data/enriched"}
classified_directory="${candidate_directory}/classified"

python3 "${repo_root}/tools/nikl-lexicon/validate_enriched.py" "${classified_directory}"

install -d "${output_directory}"
install -m 0644 "${classified_directory}/predicates.tsv" "${output_directory}/predicates.tsv"
install -m 0644 "${classified_directory}/REPORT.tsv" "${output_directory}/REPORT.tsv"
install -m 0644 "${classified_directory}/STATS.toml" "${output_directory}/STATS.toml"
install -m 0644 "${candidate_directory}/MANIFEST.toml" "${output_directory}/MANIFEST.toml"

printf 'enriched predicates: %s\n' "${output_directory}/predicates.tsv"
