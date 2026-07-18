#!/usr/bin/env bash

set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
. "$ROOT/scripts/lib/benchmark-guard.sh"
guard_benchmark_entrypoint "$ROOT" sorted-output "$ROOT/scripts/benchmark-sorted-output.sh" "$@"

RUNS=${KFIND_BENCH_RUNS:-5}
THREADS=${KFIND_BENCH_THREADS:-12}
HIGH_FILE_COUNT=${KFIND_BENCH_HIGH_FILE_COUNT:-256}
LINES_PER_FILE=${KFIND_BENCH_LINES_PER_FILE:-8192}
LOW_HIT_FILE_COUNT=${KFIND_BENCH_LOW_HIT_FILE_COUNT:-8192}
SKIP_BUILD=${KFIND_BENCH_SKIP_BUILD:-0}
KFIND_BIN=${KFIND_BENCH_KFIND_BIN:-"$ROOT/target/release/kfind"}
REVISION=${KFIND_BENCH_REVISION:-$(git -C "$ROOT" rev-parse HEAD)}
REPORT_DIR=${KFIND_BENCH_REPORT_DIR:-"$ROOT/target/benchmark/sorted-output/$REVISION"}
SYSTEM_NAME=$(uname -s)
TEMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/kfind-sorted-output.XXXXXX")
CORPUS_DIR="$TEMP_DIR/corpus"
RESULTS="$REPORT_DIR/results.tsv"
METADATA="$REPORT_DIR/metadata.json"

cleanup() {
    rm -rf "$TEMP_DIR"
}
trap cleanup EXIT INT TERM

fail() {
    echo "benchmark-sorted-output: $*" >&2
    exit 2
}

for command in cargo git python3 rustc /usr/bin/time awk uname; do
    command -v "$command" >/dev/null 2>&1 || fail "required command not found: $command"
done
[[ "$RUNS" =~ ^[1-9][0-9]*$ ]] || fail "KFIND_BENCH_RUNS must be a positive integer"
[[ "$THREADS" =~ ^[1-9][0-9]*$ ]] || fail "KFIND_BENCH_THREADS must be a positive integer"
git -C "$ROOT" cat-file -e "${REVISION}^{commit}" 2>/dev/null ||
    fail "benchmark revision is not a repository commit: $REVISION"

if [[ "$SKIP_BUILD" != 1 ]]; then
    cargo build --release --locked -p kfind-cli --bin kfind --manifest-path "$ROOT/Cargo.toml"
fi
[[ -x "$KFIND_BIN" ]] || fail "kfind binary is not executable: $KFIND_BIN"
if command -v shasum >/dev/null 2>&1; then
    BINARY_SHA256=$(shasum -a 256 "$KFIND_BIN" | awk '{ print $1 }')
elif command -v sha256sum >/dev/null 2>&1; then
    BINARY_SHA256=$(sha256sum "$KFIND_BIN" | awk '{ print $1 }')
else
    fail "required SHA-256 command not found: shasum or sha256sum"
fi

mkdir -p "$REPORT_DIR"
python3 "$ROOT/tools/sorted-output-benchmark/generate.py" "$CORPUS_DIR" \
    --high-file-count "$HIGH_FILE_COUNT" \
    --lines-per-file "$LINES_PER_FILE" \
    --low-hit-file-count "$LOW_HIT_FILE_COUNT" >"$METADATA"

python3 - "$METADATA" "$REVISION" "$SYSTEM_NAME" "$THREADS" "$RUNS" \
    "$KFIND_BIN" "$BINARY_SHA256" "$(rustc --version)" "$(cargo --version)" <<'PY'
import json
import sys

(
    metadata_path,
    revision,
    system,
    threads,
    runs,
    binary,
    binary_sha256,
    rustc_version,
    cargo_version,
) = sys.argv[1:]
metadata = json.loads(open(metadata_path, encoding="utf-8").read())
metadata["revision"] = revision
metadata["system"] = system
metadata["threads"] = int(threads)
metadata["warmup_runs"] = 1
metadata["measurement_runs"] = int(runs)
metadata["binary"] = binary
metadata["binary_sha256"] = binary_sha256
metadata["rustc"] = rustc_version
metadata["cargo"] = cargo_version
open(metadata_path, "w", encoding="utf-8").write(
    json.dumps(metadata, ensure_ascii=False, indent=2, sort_keys=True) + "\n"
)
PY

printf 'workload\tmode\trun\twall_seconds\trss_bytes\texit_code\n' >"$RESULTS"

run_once() {
    local workload=$1
    local mode=$2
    local run=$3
    local expected_exit=$4
    local query=$5
    local fixture=$6
    local timing="$TEMP_DIR/${workload}-${mode}-${run}.time"
    local exit_code
    local wall
    local rss
    local -a args=(--literal --no-pager --color never --threads "$THREADS")
    local -a time_options=(-f 'real %e\nrss_kib %M')

    if [[ "$mode" == sorted ]]; then
        args+=(--sort path)
    fi
    if [[ "$SYSTEM_NAME" == Darwin ]]; then
        time_options=(-lp)
    fi

    set +e
    /usr/bin/time "${time_options[@]}" -o "$timing" \
        "$KFIND_BIN" "${args[@]}" "$query" "$fixture" >/dev/null 2>/dev/null
    exit_code=$?
    set -e
    [[ "$exit_code" == "$expected_exit" ]] ||
        fail "$workload $mode run $run returned $exit_code, expected $expected_exit"

    wall=$(awk '$1 == "real" { print $2 }' "$timing")
    if [[ "$SYSTEM_NAME" == Darwin ]]; then
        rss=$(awk '/maximum resident set size/ { print $1 }' "$timing")
    else
        rss=$(awk '$1 == "rss_kib" { print $2 * 1024 }' "$timing")
    fi
    [[ -n "$wall" && -n "$rss" ]] || fail "could not parse timing output"
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$workload" "$mode" "$run" "$wall" "$rss" "$exit_code" >>"$RESULTS"
}

for workload in repeated unique low_hit; do
    case "$workload" in
        repeated)
            fixture="$CORPUS_DIR/repeated"
            query=걸어
            expected_exit=0
            ;;
        unique)
            fixture="$CORPUS_DIR/unique"
            query=걸어
            expected_exit=0
            ;;
        low_hit)
            fixture="$CORPUS_DIR/low-hit"
            query=찾기표식
            expected_exit=1
            ;;
    esac
    run_once "$workload" sorted warmup "$expected_exit" "$query" "$fixture"
    run_once "$workload" unsorted warmup "$expected_exit" "$query" "$fixture"
    for ((run = 1; run <= RUNS; run += 1)); do
        if ((run % 2 == 1)); then
            modes=(sorted unsorted)
        else
            modes=(unsorted sorted)
        fi
        for mode in "${modes[@]}"; do
            run_once "$workload" "$mode" "$run" "$expected_exit" "$query" "$fixture"
        done
    done
done

awk -F '\t' '$3 != "warmup"' "$RESULTS" >"$RESULTS.tmp"
mv "$RESULTS.tmp" "$RESULTS"
echo "sorted-output benchmark: $RESULTS"
echo "sorted-output metadata: $METADATA"
