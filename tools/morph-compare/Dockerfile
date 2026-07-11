ARG RUST_VERSION=1.97.0
ARG PYTHON_VERSION=3.12.13

FROM rust:${RUST_VERSION}-bookworm AS lindera-builder
ARG LINDERA_VERSION=4.0.0
RUN cargo install \
    --locked \
    --version "${LINDERA_VERSION}" \
    --no-default-features \
    --features embed-ko-dic \
    --root /opt/lindera \
    lindera-cli \
    && strip /opt/lindera/bin/lindera

FROM rust:${RUST_VERSION}-bookworm AS kfind-builder
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY data ./data
RUN cargo build --locked --release --package kfind-cli --bin kfind \
    && strip target/release/kfind

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

COPY --from=lindera-builder /opt/lindera/bin/lindera /usr/local/bin/lindera
COPY --from=kfind-builder /src/target/release/kfind /usr/local/bin/kfind
COPY data/fixtures/morphology_cases.tsv /opt/kfind/data/fixtures/morphology_cases.tsv
COPY tools/morph-compare/compare.py tools/morph-compare/test_compare.py /opt/morph-compare/

WORKDIR /opt/morph-compare
RUN python -m unittest discover --start-directory . --pattern 'test_*.py' \
    && useradd --create-home --uid 10001 runner \
    && mkdir /output \
    && chown runner:runner /output

USER runner
ENTRYPOINT ["python", "/opt/morph-compare/compare.py"]
CMD ["--output", "/output/report.json"]
