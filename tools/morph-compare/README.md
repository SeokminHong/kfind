# Independent morphology benchmark

[한국어](README.ko.md)

This development tool runs the same held-out cases through `kfind`, Kiwi, and
Lindera. External analyzers and corpora are not part of the product binary or
default search path.

Fixtures are generated from the Universal Dependencies 2.18 Korean-Kaist and
Korean-KSL test splits. Their URLs, SHA-256 digests, and CC BY-SA 4.0 licenses
are pinned in `sources.json`. The generator selects 250 POS-stratified positive
cases from each source and pairs each with a deterministic negative from the
same source, producing 1,000 cases.

```sh
scripts/benchmark-morphology.sh
```

Results are written to `target/morph-benchmark/report.json` and `report.md`.
After the image is built, the container runs with `--network none`.
`scripts/compare-morphology.sh` is an alias for the same benchmark.

To run the image directly:

```sh
docker build -f tools/morph-compare/Dockerfile -t kfind-morph-benchmark:local .
mkdir -p target/morph-benchmark
docker run --rm --network none \
  --user "$(id -u):$(id -g)" \
  -v "$PWD/target/morph-benchmark:/output" \
  kfind-morph-benchmark:local
```

Every backend predicts whether the gold lemma and POS exist in the sentence. A
positive prediction must overlap the gold eojeol span; returning the same
lemma/POS anywhere in a negative sentence is a false positive. Reports include
accuracy, precision, recall, F1, source/POS breakdowns, failure spans, and
initialization, throughput, latency, and peak RSS measurements.

Performance covers each backend's end-to-end query-to-decision workload after
one initialization. It is not a tokenizer-only throughput comparison.
