# Independent morphology benchmark

[한국어](README.ko.md)

This development tool runs the `kfind` embedded/full-POS profiles on the same
held-out cases and compares them with pinned quality and performance snapshots
from Kiwi, Lindera, MeCab-ko, and KOMORAN. External analyzers and corpora are not part of
the product binary or default search path.

Fixtures are generated from the Universal Dependencies 2.18 Korean-Kaist and
Korean-KSL test and development splits. Their URLs, SHA-256 digests, and CC
BY-SA 4.0 licenses are pinned in `sources.json`. The scored 1,000-case fixture
uses only manually reviewed Korean-Kaist sentences and keeps 500 POS-stratified
positives paired with 500 deterministic negatives. `sentence-reviews.json`
pins every reviewed dev/test sentence pool and records rejected sentences as an
unscored robustness-candidate registry. Korean-KSL uses a separate manually
reviewed sentence registry, and its confirmed noisy sentences form a separate
scored 500-case Robust fixture. Development uses the development fixture; the
test fixture remains the regression baseline.
The image also builds a separate 1,000-case human-usage fixture. Its queries omit
POS, and each negative excludes the query lemma under every supported POS.

The same image derives a query matrix from the canonical positive sentences. It
selects up to three aligned present queries per sentence and pairs every positive
with an absent query of the same coarse POS in that sentence. The explicit-POS
matrix evaluates kfind and all four external analyzers, while a separate untagged
matrix evaluates the human kfind profiles. The `query_matrix` report section keeps
query-level quality separate from all-present-query sentence coverage. It reports
strict and contract-adjusted confusion matrices, sentence coverage, and
sentence-cluster bootstrap 95% intervals in parallel. The fixed-size 1,000-case
regression baseline remains separate.

The report also measures Korean-KSL as a separate scored 500-case Robust
workload with 250 positives and 250 negatives. Every source-signal and supplement
sentence in the pre-review pool is manually inspected, and every selected query,
POS, expected result, and raw byte span is reviewed. The explicit-POS fixture
compares precision, recall, F1, error-scope recall, and performance for kfind and
the pinned external defaults. The untagged fixture measures the kfind human
workflow separately. Both keep robustness mode off, run in fresh processes after
one warm-up, and remain separate from the manually verified standard-orthography
canonical score.

```sh
scripts/benchmark-morphology.sh
```

The default run performs one warm-up and five measured runs per kfind profile.
It does not execute external analyzers; it reads only the version-controlled
snapshot bound to the test fixture SHA-256. Results are written to
`target/morph-benchmark/report.json` and `report.md`.
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

## Real technical-corpus blind evaluation

The `real_corpus` fixture evaluates 25 Korean README, source-code comment, and
technical-document excerpts from pinned revisions with the Agent and User
product profiles. Its source manifest preserves licenses and original-file
SHA-256 digests. The evaluator checks canonical text uniqueness, gold byte
spans, and all required slices.

```sh
python3 tools/morph-compare/real_corpus/verify_sources.py
```

This command downloads the pinned URLs and verifies each file digest and
excerpt line range.

```sh
KFIND_BENCH_REVISION=$(git rev-parse HEAD) \
  tools/morph-compare/real_corpus/run.sh
```

Results are written to `target/real-corpus-blind` by default. This fixture does
not replace the UD regression fixture and is not used to select product rules.

Refresh the external snapshot explicitly only when the test fixture,
performance schema, or pinned external tool and adapter configuration changes.
The default benchmark fails with this command when the fixture or snapshot
schema does not match. The default image builds only the `kfind` runner; external
analyzers and their dedicated runners are built only in the separate refresh image:

```sh
scripts/refresh-morph-baselines.sh
```

Render the committed report charts from the same JSON:

```sh
python3 tools/morph-compare/render_charts.py \
  target/morph-benchmark/report.json docs/benchmarks/assets \
  --prefix smart-component-
```

See the [benchmark contract](../../docs/benchmarks/README.md) for the current
measurement and reporting requirements.

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
with six slices. Strict corpus-gold TP/FP/TN/FN always remain visible. Reviewed
`contract_expected` annotations additionally produce contract-adjusted TPᶜ,
FPᶜ, TNᶜ, FNᶜ, precision, recall, and F1 without replacing the strict metrics.
Each kfind false negative records an automatic `primary_cause` and its evidence.
The report also records each `kfind` profile and artifact SHA-256, plus separate
lists of recovered, still-missed, and newly regressed false negatives.
Shadow verification records raw anchor hits, verified program hits, structural
candidates, and unique structural windows per case outside the timed evaluation.
The product decision uses the compact structural resource. A separate full-lattice
diagnostic can record historical cost paths, but its availability or conclusion
does not gate the product decision.
The Agent precision shadow separately records query POS, provenance,
core/token/whole-token spans, exact analyses, and bounded-lattice include/exclude
path presence for `embedded + any` matches. Cost ordering is not used for its
projections.

Current performance covers kfind's end-to-end query-to-decision workload and
reports the median and min/max across measured runs. The product-persona
comparison uses the same explicit-POS fixture and gold for Agent, User, and all
four external analyzers. Agent and the external adapters keep explicit POS;
User removes POS from the same query and runs full-POS + smart. It is a
persona-adjusted product comparison, not an identical-input backend ranking.
The full test report also compares smart, token, and any for both kfind lexicon
profiles; only smart loads the component resource. A
separate startup table compares resource-less embedded and full-POS engines with
the same engines after explicit component loading.
Each startup profile runs in a fresh process after one warm-up and records at
least three initialization-time and peak-RSS samples.

The `Human untagged search` section separately compares embedded/full-POS with
smart/any. It reports binary quality and performance plus intended-POS plan
coverage, multi-POS plan rate, and literal fallback rate. Its F1 is not combined
with the explicit-POS task because the negative definition differs.

The `Product workflows` section first presents recall, throughput, and false-
positive candidate count for agent use with `embedded + any + explicit POS`,
then precision, recall, and plan coverage for human use with
`full-POS + smart + untagged`. The library keeps a resource-less embedded engine
as its default and exposes full-POS and component resources as explicit costs.
The workflows are not combined into one score.

The `Product CLI use cases` section runs both workflows as independent CLI
processes over a fixed 100 MiB, 1,000-file corpus. Wall time includes startup,
query compilation, filesystem walking, scanning, verification, and output
serialization. It reports throughput and peak RSS alongside wall time, while
library resource initialization remains a separate cost. The generated
`product-use-cases.svg` preserves that separation.
The generated `product-workflows.svg` places profile precision, recall, F1, and
false-positive candidates beside actual CLI wall time, throughput, and peak RSS
while labeling their separate fixture and corpus units.
The generated `product-external-comparison.svg` compares Agent, User, Kiwi,
Lindera, MeCab-ko, and KOMORAN on precision, recall, F1, initialization,
throughput, p95, and peak RSS. Its row labels contain only persona or backend
names; the input conditions are documented alongside the chart.
The generated `robustness-quality.svg` compares the product defaults on the same
natural noisy fixture, including target-span and context-only recall.
`robustness-performance.svg` shows the corresponding initialization, throughput,
p95, and peak RSS medians from one warm-up and five measured fresh processes.
