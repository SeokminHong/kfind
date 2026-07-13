#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
source "${repo_root}/scripts/lib/full-pos-source.sh"
output_directory=${1:-"${repo_root}/target/morph-index-benchmark"}
temporary_directory=$(mktemp -d)
trap 'rm -rf "${temporary_directory}"' EXIT

source_directory=$(fetch_full_pos_source "${temporary_directory}")
manifest="${repo_root}/tools/morph-index-benchmark/Cargo.toml"

cd "${repo_root}"
cargo run --release --locked --manifest-path "${manifest}" -- build \
  --source-sha256 "${FULL_POS_SOURCE_SHA256}" \
  --output "${output_directory}" \
  "${source_directory}"/*.csv

cargo run --release --locked --manifest-path "${manifest}" -- build-components \
  --source-sha256 "${FULL_POS_SOURCE_SHA256}" \
  --output "${output_directory}" \
  --matrix "${source_directory}/matrix.def" \
  --char-def "${source_directory}/char.def" \
  --unk-def "${source_directory}/unk.def" \
  "${source_directory}"/*.csv

binary="${repo_root}/tools/morph-index-benchmark/target/release/morph-index-benchmark"
queries="${output_directory}/queries.json"
for kind in double-array fst; do
  artifact="${output_directory}/morphology-${kind}.kfm"
  for storage in resident mmap; do
    for phase in cold warm; do
      "${binary}" probe \
        --source-sha256 "${FULL_POS_SOURCE_SHA256}" \
        --storage "${storage}" \
        --iterations 100 \
        --queries "${queries}" \
        "${artifact}" >"${output_directory}/${kind}-${storage}-${phase}.json"
    done
  done
done

component_queries="${output_directory}/component-queries.json"
for format in full compact; do
  case "${format}" in
    full) artifact="${output_directory}/morphology-full.kfm" ;;
    compact) artifact="${output_directory}/morphology-component-compact.kfc" ;;
  esac
  for storage in resident mmap; do
    for phase in cold warm; do
      "${binary}" probe-component \
        --source-sha256 "${FULL_POS_SOURCE_SHA256}" \
        --format "${format}" \
        --storage "${storage}" \
        --iterations 100 \
        --queries "${component_queries}" \
        "${artifact}" >"${output_directory}/component-${format}-${storage}-${phase}.json"
    done
  done
done

printf 'morphology index benchmark: %s\n' "${output_directory}"
