ARG BASE_IMAGE=kfind-morph-benchmark:local
ARG RUST_VERSION=1.97.0

FROM rust:${RUST_VERSION}-bookworm AS lindera-runner-builder
WORKDIR /src
COPY tools/morph-compare/external/lindera-runner ./lindera-runner
RUN cargo build \
        --locked \
        --release \
        --manifest-path lindera-runner/Cargo.toml \
    && strip lindera-runner/target/release/lindera-benchmark-runner

FROM ${BASE_IMAGE}

COPY --from=lindera-runner-builder \
    /src/lindera-runner/target/release/lindera-benchmark-runner \
    /usr/local/bin/lindera-benchmark-runner

ARG KIWIPIEPY_VERSION=0.23.2
ARG KIWIPIEPY_MODEL_VERSION=0.23.0
ARG MECAB_KO_VERSION=1.0.2
ARG NUMPY_VERSION=2.5.1
ARG PYKOMORAN_VERSION=0.1.6
ARG TQDM_VERSION=4.68.4
ARG KOMORAN_SHA256=ae7301005652ea50126ad1b35ee1b0e1d820b315a469b59fa7603911c3f02a41

USER root
RUN apt-get update \
    && apt-get install --yes --no-install-recommends \
        ca-certificates \
        default-jdk-headless \
    && rm -rf /var/lib/apt/lists/* \
    && python -m pip install \
        --disable-pip-version-check \
        --no-cache-dir \
        "kiwipiepy_model==${KIWIPIEPY_MODEL_VERSION}" \
        "mecab-ko==${MECAB_KO_VERSION}" \
        "PyKomoran==${PYKOMORAN_VERSION}" \
    && python -m pip install \
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

WORKDIR /opt/morph-benchmark/external
COPY tools/morph-compare/python/validation.py /opt/morph-benchmark/python/validation.py
RUN cp /usr/local/lib/python3.12/site-packages/PyKomoran/libs/KOMORAN-3.3.9.jar \
        komoran.jar \
    && echo "${KOMORAN_SHA256}  komoran.jar" | sha256sum --check
COPY tools/morph-compare/external/KomoranRunner.java ./KomoranRunner.java
RUN javac --class-path komoran.jar KomoranRunner.java
COPY tools/morph-compare/external/refresh.py ./refresh.py

USER runner
ENTRYPOINT ["python", "/opt/morph-benchmark/external/refresh.py"]
CMD ["--output", "/snapshot/baselines.json"]
