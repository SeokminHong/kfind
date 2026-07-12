ARG RUST_VERSION=1.97.0
ARG PYTHON_VERSION=3.12.13

FROM python:${PYTHON_VERSION}-slim-bookworm AS corpus-builder
WORKDIR /build
COPY tools/morph-compare/sources.json ./sources.json
COPY tools/morph-compare/python ./python
RUN python -m unittest discover --start-directory python --pattern 'test_*.py'
RUN python python/fetch_sources.py --manifest sources.json --output /corpora \
    && python python/dataset.py \
        --manifest sources.json \
        --sources /corpora \
        --output /benchmark/cases.jsonl \
        --metadata /benchmark/metadata.json \
        --split test \
    && python python/dataset.py \
        --manifest sources.json \
        --sources /corpora \
        --output /benchmark/dev-cases.jsonl \
        --metadata /benchmark/dev-metadata.json \
        --split dev \
    && python python/local_context_dataset.py \
        --manifest sources.json \
        --sources /corpora \
        --output /benchmark/local-context-cases.jsonl \
        --metadata /benchmark/local-context-metadata.json

FROM rust:${RUST_VERSION}-bookworm AS runner-builder
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY data ./data
COPY scripts/build-full-pos.sh ./scripts/build-full-pos.sh
COPY scripts/build-morphology-resource.sh ./scripts/build-morphology-resource.sh
COPY scripts/lib/full-pos-source.sh ./scripts/lib/full-pos-source.sh
COPY tools/morph-compare/runner ./tools/morph-compare/runner
RUN scripts/build-full-pos.sh /benchmark/full-pos \
    && scripts/build-morphology-resource.sh /benchmark/morphology \
    && cargo build \
    --locked \
    --release \
    --manifest-path tools/morph-compare/runner/Cargo.toml \
    && strip tools/morph-compare/runner/target/release/morph-benchmark-runner

FROM python:${PYTHON_VERSION}-slim-bookworm AS runtime
ARG KIWIPIEPY_VERSION=0.23.2
ARG KIWIPIEPY_MODEL_VERSION=0.23.0
ARG NUMPY_VERSION=2.5.1
ARG TQDM_VERSION=4.68.4
RUN python -m pip install \
    --disable-pip-version-check \
    --no-cache-dir \
    "kiwipiepy_model==${KIWIPIEPY_MODEL_VERSION}"
RUN python -m pip install \
    --disable-pip-version-check \
    --no-cache-dir \
    --only-binary=:all: \
    "numpy==${NUMPY_VERSION}" \
    "tqdm==${TQDM_VERSION}" \
    && python -m pip install \
        --disable-pip-version-check \
        --no-cache-dir \
        --no-deps \
        --only-binary=:all: \
        "kiwipiepy==${KIWIPIEPY_VERSION}"

COPY --from=runner-builder \
    /src/tools/morph-compare/runner/target/release/morph-benchmark-runner \
    /usr/local/bin/morph-benchmark-runner
COPY --from=corpus-builder /benchmark /opt/morph-benchmark/data
COPY --from=corpus-builder /corpora/*LICENSE.txt /opt/morph-benchmark/licenses/
COPY --from=runner-builder /benchmark/full-pos /opt/morph-benchmark/full-pos
COPY --from=runner-builder /benchmark/morphology /opt/morph-benchmark/morphology
COPY tools/morph-compare/benchmark.py /opt/morph-benchmark/benchmark.py
COPY tools/morph-compare/hard-negatives.jsonl /opt/morph-benchmark/hard-negatives.jsonl
COPY tools/morph-compare/python /opt/morph-benchmark/python
COPY tools/morph-compare/sources.json /opt/morph-benchmark/sources.json

WORKDIR /opt/morph-benchmark
RUN python -m unittest discover --start-directory python --pattern 'test_*.py' \
    && useradd --create-home --uid 10001 runner \
    && mkdir /output \
    && chown runner:runner /output

USER runner
ENTRYPOINT ["python", "/opt/morph-benchmark/benchmark.py"]
CMD ["--output", "/output/report.json"]
