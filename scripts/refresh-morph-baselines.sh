#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$ROOT/scripts/lib/benchmark-guard.sh"
guard_benchmark_entrypoint \
    "$ROOT" morphology-baseline-refresh "$ROOT/scripts/refresh-morph-baselines.sh" "$@"

BASE_IMAGE=${KFIND_MORPH_IMAGE:-kfind-morph-benchmark:local}
REFRESH_IMAGE=${KFIND_MORPH_REFRESH_IMAGE:-kfind-morph-baseline-refresh:local}
BACKENDS=${KFIND_MORPH_BASELINE_BACKENDS:-kiwi,lindera,mecab-ko,komoran}

docker build \
    --file "$ROOT/tools/morph-compare/Dockerfile" \
    --tag "$BASE_IMAGE" \
    "$ROOT"
docker build \
    --build-arg "BASE_IMAGE=$BASE_IMAGE" \
    --file "$ROOT/tools/morph-compare/external/Dockerfile" \
    --tag "$REFRESH_IMAGE" \
    "$ROOT"
docker run \
    --rm \
    --network none \
    --user "$(id -u):$(id -g)" \
    --volume "$ROOT/tools/morph-compare/external:/snapshot" \
    "$REFRESH_IMAGE" \
    --backends "$BACKENDS" \
    --output /snapshot/baselines.json
docker run \
    --rm \
    --network none \
    --user "$(id -u):$(id -g)" \
    --volume "$ROOT/tools/morph-compare/external:/snapshot" \
    "$REFRESH_IMAGE" \
    --cases /opt/morph-benchmark/data/query-matrix-cases.jsonl \
    --metadata /opt/morph-benchmark/data/query-matrix-metadata.json \
    --backends "$BACKENDS" \
    --output /snapshot/query-matrix-baselines.json
