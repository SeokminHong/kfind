ARG BASE_IMAGE=kfind-morph-benchmark:local
FROM ${BASE_IMAGE}

ARG MECAB_KO_VERSION=1.0.2
ARG PYKOMORAN_VERSION=0.1.6
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
        "mecab-ko==${MECAB_KO_VERSION}" \
        "PyKomoran==${PYKOMORAN_VERSION}"

WORKDIR /opt/morph-benchmark/external
RUN cp /usr/local/lib/python3.12/site-packages/PyKomoran/libs/KOMORAN-3.3.9.jar \
        komoran.jar \
    && echo "${KOMORAN_SHA256}  komoran.jar" | sha256sum --check
COPY tools/morph-compare/external/KomoranRunner.java ./KomoranRunner.java
RUN javac --class-path komoran.jar KomoranRunner.java
COPY tools/morph-compare/external/refresh.py ./refresh.py

USER runner
ENTRYPOINT ["python", "/opt/morph-benchmark/external/refresh.py"]
CMD ["--output", "/snapshot/baselines.json"]
