ARG RUST_VERSION=1.97.0
ARG PYTHON_VERSION=3.12.13

FROM python:${PYTHON_VERSION}-slim-bookworm AS corpus-builder
WORKDIR /build
COPY tools/morph-compare/sources.json ./sources.json
COPY tools/morph-compare/sentence-reviews.json ./sentence-reviews.json
COPY tools/morph-compare/robustness-reviews.json ./robustness-reviews.json
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
    && python python/dataset.py \
        --manifest sources.json \
        --sources /corpora \
        --output /benchmark/human-untagged-cases.jsonl \
        --metadata /benchmark/human-untagged-metadata.json \
        --split test \
        --query-mode untagged \
    && python python/robustness_dataset.py \
        --manifest sources.json \
        --sources /corpora \
        --output /benchmark/robustness-cases.jsonl \
        --metadata /benchmark/robustness-metadata.json \
        --query-mode explicit-pos \
    && python python/robustness_dataset.py \
        --manifest sources.json \
        --sources /corpora \
        --output /benchmark/robustness-untagged-cases.jsonl \
        --metadata /benchmark/robustness-untagged-metadata.json \
        --query-mode untagged \
    && python python/review_registry.py \
        --manifest sources.json \
        --sources /corpora \
        --output /benchmark/robustness-reviewed-sentences.jsonl \
        --metadata /benchmark/robustness-reviewed-sentences-metadata.json \
    && python python/query_matrix.py \
        --manifest sources.json \
        --sources /corpora \
        --canonical-cases /benchmark/cases.jsonl \
        --output /benchmark/query-matrix-cases.jsonl \
        --metadata /benchmark/query-matrix-metadata.json \
        --split test \
        --query-mode explicit-pos \
    && python python/query_matrix.py \
        --manifest sources.json \
        --sources /corpora \
        --canonical-cases /benchmark/dev-cases.jsonl \
        --output /benchmark/query-matrix-dev-cases.jsonl \
        --metadata /benchmark/query-matrix-dev-metadata.json \
        --split dev \
        --query-mode explicit-pos \
    && python python/query_matrix.py \
        --manifest sources.json \
        --sources /corpora \
        --canonical-cases /benchmark/cases.jsonl \
        --output /benchmark/query-matrix-untagged-cases.jsonl \
        --metadata /benchmark/query-matrix-untagged-metadata.json \
        --split test \
        --query-mode untagged

FROM rust:${RUST_VERSION}-bookworm AS runner-builder
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY skills/kfind ./skills/kfind
COPY data ./data
COPY scripts/build-full-pos.sh ./scripts/build-full-pos.sh
COPY scripts/build-morphology-resource.sh ./scripts/build-morphology-resource.sh
COPY scripts/build-component-resource.sh ./scripts/build-component-resource.sh
COPY scripts/lib/full-pos-source.sh ./scripts/lib/full-pos-source.sh
COPY tools/morph-compare/runner ./tools/morph-compare/runner
RUN scripts/build-full-pos.sh /benchmark/full-pos \
    && scripts/build-morphology-resource.sh /benchmark/morphology \
    && scripts/build-component-resource.sh /benchmark/component \
    && cargo build \
    --locked \
    --release \
    -p kfind-cli \
    --bin kfind \
    -p kfind-testkit \
    --bin generate-corpus \
    && cargo build \
    --locked \
    --release \
    --manifest-path tools/morph-compare/runner/Cargo.toml \
    && strip \
    target/release/kfind \
    target/release/generate-corpus \
    tools/morph-compare/runner/target/release/morph-benchmark-runner

FROM python:${PYTHON_VERSION}-slim-bookworm AS runtime
COPY --from=runner-builder \
    /src/tools/morph-compare/runner/target/release/morph-benchmark-runner \
    /usr/local/bin/morph-benchmark-runner
COPY --from=runner-builder /src/target/release/kfind /usr/local/bin/kfind
COPY --from=runner-builder \
    /src/target/release/generate-corpus \
    /usr/local/bin/generate-corpus
COPY --from=corpus-builder /benchmark /opt/morph-benchmark/data
COPY --from=corpus-builder /corpora/*LICENSE.txt /opt/morph-benchmark/licenses/
COPY --from=runner-builder /benchmark/full-pos /opt/morph-benchmark/full-pos
COPY --from=runner-builder /benchmark/morphology /opt/morph-benchmark/morphology
COPY --from=runner-builder /benchmark/component /opt/morph-benchmark/component
COPY --from=runner-builder \
    /benchmark/full-pos/lexicon.bin \
    /opt/morph-benchmark/product-data/lexicon.bin
COPY data/enriched/predicates.tsv /opt/morph-benchmark/enriched/predicates.tsv
COPY data/enriched/predicates.tsv \
    /opt/morph-benchmark/product-data/predicates.enriched.tsv
COPY --from=runner-builder \
    /benchmark/component/morphology-component-compact.kfc \
    /opt/morph-benchmark/product-data/morphology-component-compact.kfc
COPY tools/morph-compare/benchmark.py /opt/morph-benchmark/benchmark.py
COPY tools/morph-compare/hard-negatives.jsonl /opt/morph-benchmark/hard-negatives.jsonl
COPY tools/morph-compare/external/baselines.json /opt/morph-benchmark/external-baselines.json
COPY tools/morph-compare/external/query-matrix-baselines.json \
    /opt/morph-benchmark/query-matrix-external-baselines.json
COPY tools/morph-compare/external/robustness-baselines.json \
    /opt/morph-benchmark/robustness-external-baselines.json
COPY tools/morph-compare/python /opt/morph-benchmark/python
COPY tools/morph-compare/sources.json /opt/morph-benchmark/sources.json
COPY tools/morph-compare/sentence-reviews.json \
    /opt/morph-benchmark/sentence-reviews.json

WORKDIR /opt/morph-benchmark
RUN python -m unittest discover --start-directory python --pattern 'test_*.py' \
    && useradd --create-home --uid 10001 runner \
    && mkdir /output \
    && chown runner:runner /output

USER runner
ENTRYPOINT ["python", "/opt/morph-benchmark/benchmark.py"]
CMD ["--output", "/output/report.json"]
