#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
. "$repo_root/scripts/lib/benchmark-guard.sh"
selector=${1:-all}
if (($# > 0)); then
  shift
fi

command=(cargo bench -p kfind-testkit --bench query_matcher)
if [[ "$selector" != all ]]; then
  command+=(-- "$selector" "$@")
fi

guard_benchmark_entrypoint "$repo_root" "criterion-$selector" "${command[@]}"
exec "${command[@]}"
