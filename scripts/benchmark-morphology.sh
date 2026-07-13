#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
IMAGE=${KFIND_MORPH_IMAGE:-kfind-morph-benchmark:local}
OUTPUT_DIR=${1:-target/morph-benchmark}
RUNS=${KFIND_MORPH_RUNS:-5}

case "$OUTPUT_DIR" in
    /*) ;;
    *) OUTPUT_DIR="$ROOT/$OUTPUT_DIR" ;;
esac

mkdir -p "$OUTPUT_DIR"

if [ "${KFIND_MORPH_BLIND:-0}" = "1" ] && [ "${KFIND_MORPH_SMOKE:-0}" = "1" ]; then
    echo "KFIND_MORPH_BLIND and KFIND_MORPH_SMOKE cannot be combined" >&2
    exit 2
elif [ "${KFIND_MORPH_SMOKE:-0}" = "1" ]; then
    set -- --smoke
else
    set --
fi

docker build \
    --file "$ROOT/tools/morph-compare/Dockerfile" \
    --tag "$IMAGE" \
    "$ROOT"

if [ "${KFIND_MORPH_BLIND:-0}" = "1" ]; then
    docker run \
        --rm \
        --network none \
        --user "$(id -u):$(id -g)" \
        --volume "$OUTPUT_DIR:/output" \
        --entrypoint python \
        "$IMAGE" \
        /opt/morph-benchmark/blind_benchmark.py \
        --output /output/report.json
else
    docker run \
        --rm \
        --network none \
        --user "$(id -u):$(id -g)" \
        --volume "$OUTPUT_DIR:/output" \
        "$IMAGE" \
        --runs "$RUNS" \
        "$@" \
        --output /output/report.json
fi
