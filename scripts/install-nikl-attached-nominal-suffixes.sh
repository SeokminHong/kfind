#!/bin/sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "${script_dir}/.." && pwd)
candidate=${1:?usage: install-nikl-attached-nominal-suffixes.sh CANDIDATE [OUTPUT]}
output=${2:-"${repo_root}/data/rules/nikl-attached-nominal-suffixes.tsv"}

python3 "${repo_root}/tools/nikl-lexicon/validate_nominal_suffixes.py" \
  "${candidate}" \
  --surface 하

install -d "$(dirname -- "${output}")"
install -m 0644 "${candidate}" "${output}"

printf 'NIKL attached nominal suffixes: %s\n' "${output}"
