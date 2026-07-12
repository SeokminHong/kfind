#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
IMAGE=${KFIND_MORPH_IMAGE:-kfind-morph-benchmark:local}
OUTPUT_DIR=${1:-target/morph-benchmark}

case "$OUTPUT_DIR" in
    /*) ;;
    *) OUTPUT_DIR="$ROOT/$OUTPUT_DIR" ;;
esac

mkdir -p "$OUTPUT_DIR"

docker build \
    --file "$ROOT/tools/morph-compare/Dockerfile" \
    --tag "$IMAGE" \
    "$ROOT"

docker run \
    --rm \
    --network none \
    --user "$(id -u):$(id -g)" \
    --volume "$OUTPUT_DIR:/output" \
    "$IMAGE"
