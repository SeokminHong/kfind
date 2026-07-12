# Independent morphology benchmark

[한국어](README.ko.md)

This development tool runs the same held-out cases through the `kfind`
embedded/full-POS profiles, Kiwi, and Lindera. External analyzers and corpora
are not part of the product binary or default search path.

Fixtures are generated from the Universal Dependencies 2.18 Korean-Kaist and
Korean-KSL test and development splits. Their URLs, SHA-256 digests, and CC
BY-SA 4.0 licenses are pinned in `sources.json`. For each split, the generator
selects 250 POS-stratified positive cases from each source and pairs each with a
deterministic negative from the same source, producing 1,000 cases. Development
uses the development fixture; the test fixture remains the regression baseline.

```sh
scripts/benchmark-morphology.sh
```

The default run performs one warm-up and five measured runs per backend. Results
are written to `target/morph-benchmark/report.json` and `report.md`.
After the image is built, the container runs with `--network none`.
`scripts/compare-morphology.sh` is an alias for the same benchmark.
The image build creates the pinned full-POS artifact and fails if its checksum
cannot be verified. Benchmark execution never falls back to the embedded
profile when that artifact is unavailable.

The deterministic CI smoke set selects the first development case for every
source/POS/expected combination:

```sh
KFIND_MORPH_SMOKE=1 KFIND_MORPH_RUNS=1 scripts/benchmark-morphology.sh
```

Render the committed report charts from the same JSON:

```sh
python3 tools/morph-compare/render_charts.py \
  target/morph-benchmark/report.json docs/benchmarks/assets
```

See the [comparison analysis](../../docs/benchmarks/2026-07-12-morphology-comparison.md)
and [improvement handoff](../../docs/benchmarks/2026-07-12-morphology-handoff.md).

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
initialization, throughput, latency, and peak RSS measurements. The test report
also embeds development results and a version-controlled hard-negative fixture
with five slices. Each kfind false negative records an automatic `primary_cause`
and its evidence.
The report also records each `kfind` profile and artifact SHA-256, plus separate
lists of recovered, still-missed, and newly regressed false negatives.
Shadow verification records raw anchor hits, verified branch hits, local-lattice
candidates, and unique analysis windows per case outside the timed evaluation.

Performance covers each backend's end-to-end query-to-decision workload and
reports the median and min/max across measured runs. It is not a tokenizer-only
throughput comparison.
