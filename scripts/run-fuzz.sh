#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
toolchain=${KFIND_FUZZ_TOOLCHAIN:-nightly-2026-07-11}
cargo_fuzz_version=${KFIND_CARGO_FUZZ_VERSION:-0.13.2}
seconds=${KFIND_FUZZ_SECONDS:-15}
timeout=${KFIND_FUZZ_TIMEOUT_SECONDS:-5}
rss_limit_mb=${KFIND_FUZZ_RSS_LIMIT_MB:-2048}
targets=(
  query_lexer
  matcher_bytes
  matcher_plan
  user_lexicon
  json_output
  binary_detection
)

shopt -s nullglob

for value in "$seconds" "$timeout" "$rss_limit_mb"; do
  if [[ ! "$value" =~ ^[1-9][0-9]*$ ]]; then
    echo "fuzz budgets must be positive integers" >&2
    exit 2
  fi
done

actual_version=$(cargo fuzz --version)
if [[ "$actual_version" != "cargo-fuzz $cargo_fuzz_version" ]]; then
  echo "expected cargo-fuzz $cargo_fuzz_version, found $actual_version" >&2
  exit 2
fi
rustc +"$toolchain" --version >/dev/null

cd "$repo_root"
corpus_root=$(mktemp -d "${TMPDIR:-/tmp}/kfind-fuzz.XXXXXX")
trap 'rm -rf "$corpus_root"' EXIT

for target in "${targets[@]}"; do
  corpus_dir="$corpus_root/$target"
  mkdir -p "$corpus_dir"
  seeds=("$repo_root/fuzz/corpus/$target"/seed-*)
  if ((${#seeds[@]} > 0)); then
    cp "${seeds[@]}" "$corpus_dir/"
  fi

  echo "running fuzz target $target for ${seconds}s"
  cargo +"$toolchain" fuzz run "$target" "$corpus_dir" -- \
    -max_total_time="$seconds" \
    -timeout="$timeout" \
    -rss_limit_mb="$rss_limit_mb" \
    -verbosity=0 \
    -print_final_stats=1
done
