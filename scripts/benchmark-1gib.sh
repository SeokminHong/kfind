#!/usr/bin/env bash

set -euo pipefail

ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
. "$ROOT/scripts/lib/benchmark-guard.sh"
guard_benchmark_entrypoint "$ROOT" 1gib-scan "$ROOT/scripts/benchmark-1gib.sh" "$@"

SYSTEM_NAME=$(uname -s)
TOTAL_BYTES=${KFIND_BENCH_TOTAL_BYTES:-1073741824}
FILE_COUNT=${KFIND_BENCH_FILE_COUNT:-1024}
SMALL_FILE_COUNT=${KFIND_BENCH_SMALL_FILE_COUNT:-1000}
SMALL_FILE_BYTES=${KFIND_BENCH_SMALL_FILE_BYTES:-65536}
KOREAN_PERCENT=${KFIND_BENCH_KOREAN_PERCENT:-20}
NFD_PERCENT=${KFIND_BENCH_NFD_PERCENT:-50}
SEED=${KFIND_BENCH_SEED:-323301756484}
RUNS=${KFIND_BENCH_RUNS:-3}
SCAN_REPETITIONS=${KFIND_BENCH_SCAN_REPETITIONS:-10}
QUERY=${KFIND_BENCH_QUERY:-kfind-희귀-검색-표식-7d4b}
CORPUS_DIR=${KFIND_BENCH_CORPUS_DIR:-"$ROOT/target/benchmark/1gib-mixed"}
REPORT=${KFIND_BENCH_REPORT:-"$ROOT/docs/benchmarks/$(date +%Y-%m-%d)-1gib-mixed.md"}
KEEP_CORPUS=${KFIND_BENCH_KEEP_CORPUS:-0}
REUSE_CORPUS=${KFIND_BENCH_REUSE_CORPUS:-0}
PROFILE_NAME=${KFIND_BENCH_PROFILE_NAME:-1 GiB mixed low-hit}
SKIP_BUILD=${KFIND_BENCH_SKIP_BUILD:-0}
KFIND_BIN=${KFIND_BENCH_KFIND_BIN:-"$ROOT/target/release/kfind"}
GENERATOR_BIN=${KFIND_BENCH_GENERATOR_BIN:-"$ROOT/target/release/generate-corpus"}
CONFIG_FILE="${CORPUS_DIR}.config"
TEMP_DIR=$(mktemp -d "${TMPDIR:-/tmp}/kfind-benchmark.XXXXXX")
RESULTS="$TEMP_DIR/results.tsv"

cleanup() {
    rm -rf "$TEMP_DIR"
    if [[ "$KEEP_CORPUS" != 1 ]]; then
        rm -rf "$CORPUS_DIR"
        rm -f "$CONFIG_FILE"
    fi
}
trap cleanup EXIT INT TERM

fail() {
    echo "benchmark-1gib: $*" >&2
    exit 2
}

for command in cargo git rg /usr/bin/time awk find sort stat; do
    command -v "$command" >/dev/null 2>&1 || fail "required command not found: $command"
done

revision_commit=${KFIND_BENCH_REVISION:-$(git -C "$ROOT" rev-parse HEAD)}
git -C "$ROOT" cat-file -e "${revision_commit}^{commit}" 2>/dev/null ||
    fail "benchmark revision is not a repository commit: $revision_commit"
if command -v shasum >/dev/null 2>&1; then
    SHA256_COMMAND=(shasum -a 256)
elif command -v sha256sum >/dev/null 2>&1; then
    SHA256_COMMAND=(sha256sum)
else
    fail "required SHA-256 command not found: shasum or sha256sum"
fi

[[ "$RUNS" =~ ^[1-9][0-9]*$ ]] || fail "KFIND_BENCH_RUNS must be a positive integer"
[[ "$SCAN_REPETITIONS" =~ ^[1-9][0-9]*$ ]] || fail \
    "KFIND_BENCH_SCAN_REPETITIONS must be a positive integer"
mkdir -p "$(dirname "$CORPUS_DIR")" "$(dirname "$REPORT")"

config_text=$(cat <<EOF
total_bytes=$TOTAL_BYTES
file_count=$FILE_COUNT
small_file_count=$SMALL_FILE_COUNT
small_file_bytes=$SMALL_FILE_BYTES
korean_percent=$KOREAN_PERCENT
nfd_percent=$NFD_PERCENT
seed=$SEED
EOF
)

corpus_bytes() {
    find "$CORPUS_DIR" -type f -name 'corpus-*.txt' |
        while IFS= read -r file; do
            stat_size "$file"
        done |
        awk '{ total += $1 } END { print total + 0 }'
}

stat_size() {
    if [[ "$SYSTEM_NAME" == Darwin ]]; then
        stat -f '%z' "$1"
    else
        stat -c '%s' "$1"
    fi
}

has_reusable_corpus=0
if [[ "$REUSE_CORPUS" == 1 && -d "$CORPUS_DIR" && -f "$CONFIG_FILE" ]]; then
    if [[ "$(cat "$CONFIG_FILE")" == "$config_text" && "$(corpus_bytes)" == "$TOTAL_BYTES" ]]; then
        has_reusable_corpus=1
    fi
fi

if [[ "$has_reusable_corpus" != 1 ]]; then
    rm -rf "$CORPUS_DIR"
    rm -f "$CONFIG_FILE"
    available_kib=$(df -Pk "$(dirname "$CORPUS_DIR")" | awk 'NR == 2 { print $4 }')
    required_kib=$((TOTAL_BYTES / 1024 + 524288))
    (( available_kib >= required_kib )) || fail \
        "insufficient disk space: need at least ${required_kib} KiB, have ${available_kib} KiB"

    if [[ "$SKIP_BUILD" != 1 ]]; then
        echo "Building release binaries..." >&2
        cargo build --release -p kfind-cli --bin kfind -p kfind-testkit --bin generate-corpus \
            --manifest-path "$ROOT/Cargo.toml"
    fi
    [[ -x "$KFIND_BIN" ]] || fail "kfind binary is not executable: $KFIND_BIN"
    [[ -x "$GENERATOR_BIN" ]] || fail "corpus generator is not executable: $GENERATOR_BIN"
    echo "Generating $TOTAL_BYTES-byte corpus..." >&2
    "$GENERATOR_BIN" "$CORPUS_DIR" \
        --total-bytes "$TOTAL_BYTES" \
        --files "$FILE_COUNT" \
        --small-files "$SMALL_FILE_COUNT" \
        --small-file-bytes "$SMALL_FILE_BYTES" \
        --korean-percent "$KOREAN_PERCENT" \
        --nfd-percent "$NFD_PERCENT" \
        --seed "$SEED"
    printf '%s\n' "$config_text" >"$CONFIG_FILE"
else
    echo "Reusing corpus at $CORPUS_DIR" >&2
    if [[ "$SKIP_BUILD" != 1 ]]; then
        cargo build --release -p kfind-cli --bin kfind --manifest-path "$ROOT/Cargo.toml"
    fi
fi
[[ -x "$KFIND_BIN" ]] || fail "kfind binary is not executable: $KFIND_BIN"

actual_bytes=$(corpus_bytes)
[[ "$actual_bytes" == "$TOTAL_BYTES" ]] || fail \
    "corpus size mismatch: expected $TOTAL_BYTES, got $actual_bytes"

corpus_checksum=$(
    while IFS= read -r file; do
        cat "$file"
    done < <(find "$CORPUS_DIR" -type f -name 'corpus-*.txt' | LC_ALL=C sort) |
        "${SHA256_COMMAND[@]}" | awk '{ print $1 }'
)

KFIND_COMMAND=("$KFIND_BIN" --literal --quiet --no-ignore "$QUERY" "$CORPUS_DIR")
RG_COMMAND=(rg -F --quiet --no-ignore "$QUERY" "$CORPUS_DIR")
printf 'engine\trun\twall_seconds\tthroughput_mib_s\tmax_rss_bytes\texit_code\n' >"$RESULTS"

run_unmeasured() {
    set +e
    "$@" >/dev/null 2>/dev/null
    local exit_code=$?
    set -e
    [[ "$exit_code" == 1 ]] || fail \
        "warm-up must return the expected no-match exit code 1, got $exit_code: $*"
}

run_timed() {
    local engine=$1
    local run=$2
    shift 2
    local timing="$TEMP_DIR/${engine}-${run}.time"
    local exit_code
    local wall_total
    local wall
    local rss_bytes
    local -a time_options

    set +e
    time_options=(-f 'real %e\nrss_kib %M')
    if [[ "$SYSTEM_NAME" == Darwin ]]; then
        time_options=(-lp)
    fi
    /usr/bin/time "${time_options[@]}" -o "$timing" bash -c '
        repetitions=$1
        shift
        for ((iteration = 1; iteration <= repetitions; iteration += 1)); do
            "$@" >/dev/null 2>/dev/null
            exit_code=$?
            if [[ "$exit_code" != 1 ]]; then
                exit 64
            fi
        done
    ' benchmark-repeat "$SCAN_REPETITIONS" "$@"
    exit_code=$?
    set -e
    [[ "$exit_code" == 0 ]] || fail \
        "$engine run $run did not produce only no-match exit codes"

    if [[ "$SYSTEM_NAME" == Darwin ]]; then
        wall_total=$(awk '$1 == "real" { print $2 }' "$timing")
        rss_bytes=$(awk '/maximum resident set size/ { print $1 }' "$timing")
    else
        wall_total=$(awk '$1 == "real" { print $2 }' "$timing")
        rss_bytes=$(awk '$1 == "rss_kib" { print $2 * 1024 }' "$timing")
    fi
    [[ -n "$wall_total" && -n "$rss_bytes" ]] || fail "could not parse timing output for $engine"
    wall=$(awk -v total="$wall_total" -v repetitions="$SCAN_REPETITIONS" \
        'BEGIN { printf "%.4f", total / repetitions }')
    throughput=$(awk -v bytes="$TOTAL_BYTES" -v seconds="$wall" \
        'BEGIN { if (seconds <= 0) print "inf"; else printf "%.2f", bytes / 1048576 / seconds }')
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$engine" "$run" "$wall" "$throughput" "$rss_bytes" 1 >>"$RESULTS"
    echo "$engine run $run: ${wall}s, ${throughput} MiB/s, RSS ${rss_bytes} bytes" >&2
}

echo "Warming filesystem cache..." >&2
run_unmeasured "${KFIND_COMMAND[@]}"
run_unmeasured "${RG_COMMAND[@]}"

for ((run = 1; run <= RUNS; run += 1)); do
    if (( run % 2 == 1 )); then
        run_timed kfind "$run" "${KFIND_COMMAND[@]}"
        run_timed rg "$run" "${RG_COMMAND[@]}"
    else
        run_timed rg "$run" "${RG_COMMAND[@]}"
        run_timed kfind "$run" "${KFIND_COMMAND[@]}"
    fi
done

median_column() {
    local engine=$1
    local column=$2
    awk -F '\t' -v engine="$engine" -v column="$column" \
        'NR > 1 && $1 == engine { print $column }' "$RESULTS" |
        sort -n |
        awk '{ values[NR] = $1 } END {
            middle = int((NR + 1) / 2)
            if (NR % 2 == 1) print values[middle]
            else printf "%.3f", (values[middle] + values[middle + 1]) / 2
        }'
}

kfind_wall=$(median_column kfind 3)
rg_wall=$(median_column rg 3)
kfind_throughput=$(median_column kfind 4)
rg_throughput=$(median_column rg 4)
kfind_rss=$(median_column kfind 5)
rg_rss=$(median_column rg 5)
wall_ratio=$(awk -v kfind="$kfind_wall" -v rg="$rg_wall" 'BEGIN { printf "%.3f", kfind / rg }')
throughput_ratio=$(awk -v kfind="$kfind_throughput" -v rg="$rg_throughput" \
    'BEGIN { printf "%.1f", kfind / rg * 100 }')
rss_mib=$(awk -v bytes="$kfind_rss" 'BEGIN { printf "%.2f", bytes / 1048576 }')
wall_result=$(awk -v ratio="$wall_ratio" 'BEGIN { print (ratio <= 1.5 ? "PASS" : "FAIL") }')
throughput_result=$(awk -v ratio="$throughput_ratio" 'BEGIN { print (ratio >= 70 ? "PASS" : "FAIL") }')
rss_result=$(awk -v mib="$rss_mib" 'BEGIN { print (mib <= 40 ? "PASS" : "FAIL") }')

architecture=$(uname -m)
if [[ "$SYSTEM_NAME" == Darwin ]]; then
    cpu=$(sysctl -n machdep.cpu.brand_string)
    memory_bytes=$(sysctl -n hw.memsize)
    os="macOS $(sw_vers -productVersion) ($(sw_vers -buildVersion))"
    storage=$(system_profiler SPNVMeDataType -detailLevel mini 2>/dev/null |
        awk -F ': ' '/Model: / && !found { print $2; found = 1 }')
    storage=${storage:-"APFS $(df -P "$CORPUS_DIR" | awk 'NR == 2 { print $1 }')"}
    rss_source='/usr/bin/time -l (bytes)'
else
    cpu=$(awk -F ': ' '/model name/ { print $2; exit }' /proc/cpuinfo)
    memory_bytes=$(awk '/MemTotal/ { print $2 * 1024; exit }' /proc/meminfo)
    os=$(awk -F= '/^PRETTY_NAME=/ { gsub(/"/, "", $2); print $2 }' /etc/os-release)
    storage="$(df -PT "$CORPUS_DIR" | awk 'NR == 2 { print $2 " on " $1 }')"
    rss_source='/usr/bin/time -v (KiB converted to bytes)'
fi
memory_gib=$(awk -v bytes="$memory_bytes" 'BEGIN { printf "%.1f GiB", bytes / 1073741824 }')
revision=$revision_commit
if [[ -z "${KFIND_BENCH_REVISION:-}" && -n "$(git -C "$ROOT" status --porcelain)" ]]; then
    revision="$revision (working tree changes)"
fi
generated_at=$(date -u '+%Y-%m-%dT%H:%M:%SZ')
rust_version=$(rustc --version)
rg_version=$(rg --version | head -n 1)

{
    cat <<EOF
# kfind 1 GiB mixed corpus benchmark

측정 시각: $generated_at
프로필: $PROFILE_NAME
cache 상태: 한 번의 사전 scan 뒤 warm cache 측정. cold cache는 측정하지 않음.

## 환경

| 항목 | 값 |
| --- | --- |
| kfind Git revision | \`$revision\` |
| CPU | $cpu ($architecture) |
| Memory | $memory_gib ($memory_bytes bytes) |
| Storage | $storage |
| OS | $os |
| Rust | \`$rust_version\` |
| kfind | \`$($KFIND_BIN --version)\` |
| ripgrep | \`$rg_version\` |
| RSS 수집 | $rss_source |

## corpus

| 설정 | 값 |
| --- | ---: |
| 전체 크기 | $TOTAL_BYTES bytes |
| 파일 수 | $FILE_COUNT |
| 작은 파일 | $SMALL_FILE_COUNT x $SMALL_FILE_BYTES bytes |
| 큰 파일 | $((FILE_COUNT - SMALL_FILE_COUNT)); 남은 bytes 균등 분배 |
| 한글 line 선택 비율 | $KOREAN_PERCENT% |
| 한글 line 중 NFD 선택 비율 | $NFD_PERCENT% |
| Seed | $SEED |
| Run당 scan 반복 | $SCAN_REPETITIONS |
| 연결 content SHA-256 | \`$corpus_checksum\` |

파일 끝의 exact-size padding은 ASCII다. 생성 corpus는 보고서에 포함하지 않는다.

## 명령

\`\`\`console
$ target/release/kfind --literal --quiet --no-ignore '$QUERY' '<corpus>'
$ rg -F --quiet --no-ignore '$QUERY' '<corpus>'
\`\`\`

두 명령의 종료 코드 1은 고정 literal이 없는 no-match 결과이며 전체 corpus를 scan한다. 각 run은 ${SCAN_REPETITIONS}회 scan의 합산 시간을 1회당 평균으로 환산했고, 실행 순서는 run마다 교대했다.

## 결과

| 도구 | Run | Wall (s) | Throughput (MiB/s) | Maximum RSS (bytes) | Exit |
| --- | ---: | ---: | ---: | ---: | ---: |
EOF
    awk -F '\t' 'NR > 1 { printf "| %s | %s | %s | %s | %s | %s |\n", $1, $2, $3, $4, $5, $6 }' "$RESULTS"
    cat <<EOF

| 도구 | Median wall (s) | Median throughput (MiB/s) | Median maximum RSS (bytes) |
| --- | ---: | ---: | ---: |
| kfind | $kfind_wall | $kfind_throughput | $kfind_rss |
| rg -F | $rg_wall | $rg_throughput | $rg_rss |

## v0.1 목표 판정

| 목표 | 측정 | 판정 |
| --- | ---: | :---: |
| kfind wall <= rg -F x 1.5 | x $wall_ratio | $wall_result |
| kfind throughput >= rg -F 70% | $throughput_ratio% | $throughput_result |
| kfind RSS <= 40 MiB | $rss_mib MiB | $rss_result |

이 비교는 기능의 절대 우열이 아니라 동일한 low-hit I/O 경로의 회귀 기준이다.
EOF
} >"$REPORT"

echo "Benchmark report: $REPORT" >&2
