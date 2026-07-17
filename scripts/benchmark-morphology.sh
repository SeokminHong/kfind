#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$ROOT/scripts/lib/benchmark-guard.sh"
guard_benchmark_entrypoint "$ROOT" morphology "$ROOT/scripts/benchmark-morphology.sh" "$@"

IMAGE=${KFIND_MORPH_IMAGE:-kfind-morph-benchmark:local}
OUTPUT_DIR=${1:-target/morph-benchmark}
OUTPUT_DISPLAY=$OUTPUT_DIR
RUNS=${KFIND_MORPH_RUNS:-5}
VERBOSE=${KFIND_MORPH_VERBOSE:-0}

case "$VERBOSE" in
    0|1) ;;
    *)
        printf 'benchmark-morphology: KFIND_MORPH_VERBOSE must be 0 or 1\n' >&2
        exit 2
        ;;
esac

case "$OUTPUT_DIR" in
    /*) ;;
    *) OUTPUT_DIR="$ROOT/$OUTPUT_DIR" ;;
esac

mkdir -p "$OUTPUT_DIR"

if [ "${KFIND_MORPH_SMOKE:-0}" = "1" ]; then
    set -- --smoke
else
    set --
fi

set -- "$@" --progress

if [ "$VERBOSE" = "1" ]; then
    set -- "$@" --print-report
fi

printf '[morphology] building benchmark image\n'
if [ "$VERBOSE" = "1" ]; then
    docker build \
        --file "$ROOT/tools/morph-compare/Dockerfile" \
        --tag "$IMAGE" \
        "$ROOT"
else
    BUILD_LOG=$(mktemp "${TMPDIR:-/tmp}/kfind-morph-build.XXXXXX")
    trap 'rm -f "$BUILD_LOG"' EXIT HUP INT TERM
    if docker build \
        --progress plain \
        --file "$ROOT/tools/morph-compare/Dockerfile" \
        --tag "$IMAGE" \
        "$ROOT" \
        >"$BUILD_LOG" 2>&1; then
        :
    else
        BUILD_STATUS=$?
        printf 'benchmark-morphology: image build failed\n' >&2
        tail -n 80 "$BUILD_LOG" >&2
        exit "$BUILD_STATUS"
    fi
fi

printf '[morphology] running benchmark with %s measured run(s)\n' "$RUNS"
docker run \
    --rm \
    --network none \
    --user "$(id -u):$(id -g)" \
    --volume "$OUTPUT_DIR:/output" \
    "$IMAGE" \
    --runs "$RUNS" \
    "$@" \
    --output /output/report.json

printf '[morphology] reports: %s, %s\n' \
    "$OUTPUT_DISPLAY/report.json" \
    "$OUTPUT_DISPLAY/report.md"
