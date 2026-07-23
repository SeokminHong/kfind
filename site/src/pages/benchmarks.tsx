import type {
  DurationChartRow,
  QualityChartRow,
} from '../components/quality-chart';

import { DocumentLocale, useDocumentLocale } from '../app/i18n';
import {
  DocumentPage,
  DocumentSection,
  PageIntro,
} from '../components/document';
import { DurationChart, QualityChart } from '../components/quality-chart';
import benchmarkSnapshotJson from '../generated-benchmark/site-morphology.json';
import searchBaselineSnapshotJson from '../generated-benchmark/site-search-baseline.json';

export { createLocationDocumentMeta as meta } from '../app/metadata';

interface RawQuality {
  readonly f1_percent: number;
  readonly fn: number;
  readonly fp: number;
  readonly precision_percent: number;
  readonly recall_percent: number;
  readonly tn: number;
  readonly tp: number;
}

interface ContractQuality {
  readonly contract_f1_percent: number;
  readonly contract_fn: number;
  readonly contract_fp: number;
  readonly contract_precision_percent: number;
  readonly contract_recall_percent: number;
  readonly contract_tn: number;
  readonly contract_tp: number;
  readonly excluded_cases: number;
  readonly reclassified_cases: number;
  readonly reviewed_cases: number;
}

interface QualityResult {
  readonly contract_adjusted?: { readonly overall: ContractQuality };
  readonly overall: RawQuality;
}

enum MorphologyWorkload {
  Canonical = 'canonical',
  QueryMatrix = 'query_matrix',
  Robustness = 'robustness',
}

interface ProfileComparison {
  readonly performance: Readonly<Record<string, PerformanceResult>>;
  readonly profiles: readonly string[];
  readonly quality: Readonly<Record<string, QualityResult>>;
}

interface BenchmarkSnapshot {
  readonly profile_comparisons: Readonly<
    Record<MorphologyWorkload, ProfileComparison>
  >;
  readonly source_report: {
    readonly revision: string;
    readonly sha256: string;
  };
}

interface PerformanceResult {
  readonly cases_per_second: number;
  readonly initialization_seconds: number;
  readonly latency_p95_ms: number;
  readonly peak_rss_kib: number;
  readonly runs: number;
  readonly warmup_runs: number;
}

interface SearchBaselineMetric {
  readonly f1_percent: number;
  readonly fn: number;
  readonly fp: number;
  readonly precision_percent: number;
  readonly recall_percent: number;
  readonly tn: number;
  readonly tp: number;
}

interface SearchBaselineQuality {
  readonly contract_adjusted: SearchBaselineMetric;
  readonly id: string;
  readonly raw: SearchBaselineMetric;
}

interface SearchBaselinePerformance {
  readonly effective_mib_per_second: number;
  readonly id: string;
  readonly max_ms: number;
  readonly median_ms: number;
  readonly min_ms: number;
  readonly p95_ms: number;
}

interface SearchBaselineSnapshot {
  readonly fixture: {
    readonly cases: number;
    readonly contract_negative: number;
    readonly contract_positive: number;
    readonly kind: string;
    readonly queries: number;
    readonly reviewed_cases: number;
    readonly strict_negative: number;
    readonly strict_positive: number;
  };
  readonly performance: {
    readonly bytes: number;
    readonly lines: number;
    readonly methods: readonly SearchBaselinePerformance[];
    readonly runs: number;
    readonly warmup: number;
  };
  readonly quality: readonly SearchBaselineQuality[];
  readonly source_report: {
    readonly revision: string;
    readonly sha256: string;
  };
}

const benchmarkSnapshot = benchmarkSnapshotJson as BenchmarkSnapshot;
const searchBaselineSnapshot =
  searchBaselineSnapshotJson as SearchBaselineSnapshot;

const backendLabels: Readonly<Record<string, string>> = {
  'kfind-embedded-any': 'kfind embedded · any',
  'kfind-embedded-smart': 'kfind embedded · smart',
  'kfind-full-pos-any': 'kfind full POS · any',
  'kfind-full-pos-smart': 'kfind full POS · smart',
  kiwi: 'Kiwi',
  lindera: 'Lindera',
  'mecab-ko': 'MeCab-ko',
  komoran: 'KOMORAN',
};

const searchStrategyLabels: Readonly<
  Record<DocumentLocale, Readonly<Record<string, string>>>
> = {
  [DocumentLocale.Korean]: {
    kfind_any: 'kfind full POS · any',
    kfind_smart: 'kfind full POS · smart',
    regex_enumerated: '활용형 열거 정규식',
    regex_stem: '짧은 어간 정규식',
    rg_enumerated: 'rg · 활용형 열거',
    grep_enumerated: 'grep · 활용형 열거',
    rg_stem: 'rg · 짧은 어간',
    grep_stem: 'grep · 짧은 어간',
  },
  [DocumentLocale.English]: {
    kfind_any: 'kfind full POS · any',
    kfind_smart: 'kfind full POS · smart',
    regex_enumerated: 'Enumerated-surface regex',
    regex_stem: 'Short-stem regex',
    rg_enumerated: 'rg · enumerated surfaces',
    grep_enumerated: 'grep · enumerated surfaces',
    rg_stem: 'rg · short stems',
    grep_stem: 'grep · short stems',
  },
};

const copy = {
  [DocumentLocale.Korean]: {
    eyebrow: '근거 · 품질과 성능',
    title: '벤치마크',
    summary:
      '형태 검색 품질과 실행 비용을 서로 다른 workload로 측정합니다. 모든 품질 비교는 raw와 contract-adjusted 결과를 함께 표시하며, 성능은 초기화·처리량·지연 시간·메모리를 별도 단위로 유지합니다.',
    scopeTitle: '평가 범위',
    scopeParagraphs: [
      'Canonical은 사람이 표준 맞춤법을 확인한 500개 양성·500개 음성 사례입니다. 제품과 Kiwi, Lindera, MeCab-ko, KOMORAN은 같은 표제어·품사·문장 과제를 수행합니다.',
      'Robust는 실제 오류 문장 250개 양성·250개 음성 사례입니다. 표준문 결과와 합산하지 않으며 robustness 설정, 오류 위치와 입력 분모를 별도로 기록합니다.',
      'Query matrix는 같은 문장에 여러 양성·음성 질의를 적용해 부분 회수와 문장 안 오탐을 측정합니다. 제품 계약 검토 registry가 strict corpus gold와 다른 기대값을 선언할 수 있는 평가군입니다.',
    ],
    metricTitle: 'Raw와 contract-adjusted 지표',
    metricParagraphs: [
      'Raw는 원본 corpus gold를 그대로 사용합니다. TP, FP, TN, FN과 여기서 계산한 precision, recall, F1을 수정하지 않습니다.',
      'Contract-adjusted는 제품 실행 전에 고정한 registry를 같은 예측에 적용합니다. 의미로 구분할 수 없는 동형이의, source에 정렬된 내부 성분과 gold span 오류를 재분류하고, 제품 입력 계약에 속하지 않는 비표준 입력만 제외합니다. 구현이 어렵거나 아직 지원하지 않는 문법은 제외하지 않습니다.',
      'Contract-adjusted confusion matrix는 raw 약어 오른쪽 위에 c를 붙여 TPᶜ, FPᶜ, TNᶜ, FNᶜ로 표기합니다. 예를 들어 FNᶜ는 제품 계약 안의 false negative입니다.',
      'Contract review가 없는 평가군은 raw 기대값을 그대로 사용합니다. 이때 두 결과가 같고 reviewed cases가 0이라는 사실도 결과에 포함합니다. 보정 결과만 남기거나 raw 결과를 숨기지 않습니다.',
    ],
    fnExample:
      'Full-POS query matrix의 raw FN 4와 FNᶜ 0은 같은 실행 결과를 두 계약으로 읽은 값입니다. Raw FN 4는 strict gold span을 회수하지 못한 사례입니다. Registry는 그중 의미 구조로 구분할 수 없는 동형이의와 source에 정렬된 검색 성분을 제품 목표의 양성으로 선언합니다. 따라서 FNᶜ 0은 네 사례가 제품 목표 밖의 false negative라는 뜻이며, 제품이 네 오류를 수정했다는 뜻이 아닙니다.',
    canonicalTitle: 'Canonical 품질·성능',
    canonicalCaption:
      '같은 1,000개 explicit-POS 사례에서 kfind의 embedded/full POS와 any/smart 조합 4종, 외부 분석기 고정 설정의 F1입니다.',
    queryMatrixTitle: 'Query matrix 품질·성능',
    queryMatrixDescription:
      'Query matrix는 한 source 문장에서 최대 세 개의 “있어야 하는” 표제어·품사·span 질의를 고르고, 각 질의마다 같은 품사의 “없어야 하는” 질의를 짝지은 진단 fixture입니다. 아래 값은 개별 질의 단위로 집계하며 Canonical 회귀선과 합치거나 대체하지 않습니다.',
    queryMatrixCaption:
      '같은 query matrix에서 kfind profile 4종과 외부 분석기 고정 설정의 예측에 strict gold와 고정 contract review registry를 각각 적용한 F1입니다.',
    robustTitle: 'Robust 품질·성능',
    robustCaption:
      '같은 500개 실제 오류 문장에서 kfind profile 4종과 외부 분석기 고정 설정의 F1입니다. Contract review가 없으므로 raw와 contract-adjusted 결과가 같습니다.',
    searchTitle: '형태 질의와 정규식 검색 기준선',
    searchParagraphs: [
      '명시적 품사를 붙인 full-POS 형태 질의 7개를 any와 smart로 각각 실행하고, 사람이 활용형을 열거한 정규식 및 짧은 어간만 열거한 정규식과 비교합니다. 각 질의는 positive 8개와 negative 8개를 가지며 전체 112개 case입니다.',
      '이 constructed fixture는 같은 질의에서 coverage와 경계 trade-off를 보여 주는 진단입니다. Held-out 품질 benchmark나 일반적인 한국어 검색 품질의 순위가 아닙니다.',
      'Contract-adjusted는 각 방법의 같은 예측에 사전 고정한 contract expectation을 적용한 값입니다. Any와 smart의 오탐 차이는 그대로 유지하며 보정 결과만으로 raw 오류를 숨기지 않습니다.',
    ],
    searchQualityCaption:
      '같은 112개 case에 strict gold와 고정 contract expectation을 적용한 F1입니다.',
    searchChartDescription:
      'kfind full POS any/smart, 활용형 열거 정규식과 짧은 어간 정규식의 raw 및 contract-adjusted F1 비교',
    searchConfusionTitle: '검색 전략 confusion matrix',
    strategy: '검색 전략',
    searchRawCounts: 'Raw TP / TN / FP / FN',
    searchAdjustedCounts: 'TPᶜ / TNᶜ / FPᶜ / FNᶜ',
    searchPerformanceTitle: '7-query batch 시간',
    searchPerformanceParagraph:
      '13.54 MiB 단일 파일을 각 질의가 fresh process로 한 번씩, 전체 7회 스캔한 wall time입니다. 방법 순서를 순환하며 warm-up 2회 뒤 10회 측정했습니다. 품질과 시간은 하나의 점수로 합치지 않습니다.',
    searchDurationDescription:
      'kfind와 rg, grep 정규식 조합별 7-query fresh-process batch 중앙값',
    searchDurationCaption:
      '막대는 batch wall time 중앙값이며 낮을수록 짧습니다. 정확한 min, max와 p95는 아래 표에 표시합니다.',
    median: '중앙값',
    minimum: '최솟값',
    maximum: '최댓값',
    effectiveThroughput: '유효 처리량',
    searchSource: '검색 기준선 source',
    chartDescription: '프로필별 raw와 contract-adjusted F1 막대 비교',
    rawLabel: 'Raw',
    adjustedLabel: 'Contract-adjusted',
    metricLabel: 'F1',
    confusionTitle: 'Confusion matrix',
    backend: '프로필·제품',
    rawCounts: 'Raw TP / TN / FP / FN',
    adjustedCounts: 'TPᶜ / TNᶜ / FPᶜ / FNᶜ',
    performanceTitle: '동일 workload 성능',
    performanceParagraph:
      '위 품질 fixture를 fresh process에서 warm-up 1회 뒤 5회 측정한 중앙값입니다. kfind profile 4종과 외부 분석기의 고정 설정을 같은 workload 안에서 비교하며 품질과 하나의 점수로 합치지 않습니다.',
    initialization: '초기화',
    throughput: 'cases/s',
    latency: 'p95',
    memory: 'peak RSS',
    sourcesTitle: '원본 자료',
    sourceParagraph:
      '승인 snapshot은 source report의 Git revision과 SHA-256을 보존합니다. 측정 환경, fixture checksum, 도구 버전과 개별 실패 사례는 원본 보고서에서 확인할 수 있습니다.',
    reportLink: '벤치마크 계약과 보고서',
  },
  [DocumentLocale.English]: {
    eyebrow: 'EVIDENCE · QUALITY AND PERFORMANCE',
    title: 'Benchmarks',
    summary:
      'Morphology quality and execution cost are measured as separate workloads. Every quality comparison includes raw and contract-adjusted results, while initialization, throughput, latency, and memory retain their own units.',
    scopeTitle: 'Evaluation scope',
    scopeParagraphs: [
      'Canonical contains 500 positive and 500 negative sentences manually reviewed for standard Korean. kfind, Kiwi, Lindera, MeCab-ko, and KOMORAN perform the same lemma, POS, and sentence task.',
      'Robust contains 250 positive and 250 negative natural sentences with real errors. Its results are not combined with Canonical, and the robustness setting, error location, and denominator are recorded separately.',
      'The query matrix applies multiple positive and negative queries to each sentence. It measures partial recovery and within-sentence false positives, and it is the dataset with a versioned product-contract review registry.',
    ],
    metricTitle: 'Raw and contract-adjusted metrics',
    metricParagraphs: [
      'Raw metrics preserve the source corpus gold. TP, FP, TN, FN, precision, recall, and F1 are reported without modification.',
      'Contract-adjusted metrics apply a registry fixed before product execution to the same predictions. It may reclassify semantically indistinguishable homographs, source-aligned internal components, and gold-span errors. Only nonstandard input outside the product contract may be excluded. Unsupported or expensive grammar remains in the denominator.',
      'A contract-adjusted confusion matrix adds a superscript c to each raw abbreviation: TPᶜ, FPᶜ, TNᶜ, and FNᶜ. For example, FNᶜ means an in-contract false negative.',
      'A dataset without contract reviews uses its raw expectation unchanged. The report still includes the identical adjusted result and a reviewed-case count of zero. Raw evidence is never replaced by the adjusted view.',
    ],
    fnExample:
      'The full-POS query matrix has four raw false negatives and zero contract-adjusted false negatives. The execution result is identical in both views. The four raw misses target strict gold spans, while the registry classifies their indistinguishable homographs and source-aligned components as positive under the product contract. FNᶜ = 0 therefore means that the four misses are outside the product false-negative objective; it does not mean that the implementation fixed four errors.',
    canonicalTitle: 'Canonical quality and performance',
    canonicalCaption:
      'F1 on the same 1,000 explicit-POS cases for four kfind embedded/full-POS and any/smart combinations plus fixed external-analyzer settings.',
    queryMatrixTitle: 'Query-matrix quality and performance',
    queryMatrixDescription:
      'The query matrix selects up to three lemma-POS-span queries that should match in one source sentence and pairs each with a same-POS query that should not match. The values below are aggregated per query and remain separate from the Canonical regression baseline.',
    queryMatrixCaption:
      'F1 for four kfind profiles and fixed external-analyzer settings, evaluated against strict gold and the fixed contract-review registry.',
    robustTitle: 'Robust quality and performance',
    robustCaption:
      'F1 for four kfind profiles and fixed external-analyzer settings on the same 500 natural noisy sentences. No contract review is applied, so raw and adjusted results are identical.',
    searchTitle: 'Morphology queries and regex baselines',
    searchParagraphs: [
      'Seven full-POS morphology queries with explicit POS run once with any and once with smart. Both are compared with a regex that manually enumerates inflected surfaces and a regex that lists only short stems. Each query has eight positives and eight negatives, for 112 cases.',
      'This constructed fixture diagnoses coverage and boundary trade-offs for the same queries. It is neither a held-out quality benchmark nor a general ranking of Korean search quality.',
      'Contract-adjusted values apply expectations fixed before execution to each method’s same predictions. The false-positive difference between any and smart remains visible; adjusted results never replace raw errors.',
    ],
    searchQualityCaption:
      'F1 from strict gold and fixed contract expectations on the same 112 cases.',
    searchChartDescription:
      'Raw and contract-adjusted F1 for kfind full POS any/smart, an enumerated-surface regex, and a short-stem regex',
    searchConfusionTitle: 'Search-strategy confusion matrix',
    strategy: 'Search strategy',
    searchRawCounts: 'Raw TP / TN / FP / FN',
    searchAdjustedCounts: 'TPᶜ / TNᶜ / FPᶜ / FNᶜ',
    searchPerformanceTitle: 'Seven-query batch time',
    searchPerformanceParagraph:
      'Each query scans the same 13.54 MiB file once in a fresh process, for seven scans per batch. Methods rotate order; two warm-ups precede ten measurements. Quality and time are not combined into one score.',
    searchDurationDescription:
      'Median seven-query fresh-process batch time for kfind and each rg or grep regex combination',
    searchDurationCaption:
      'Bars show median batch wall time; lower is shorter. The table gives exact minimum, maximum, and p95 values.',
    median: 'Median',
    minimum: 'Minimum',
    maximum: 'Maximum',
    effectiveThroughput: 'Effective throughput',
    searchSource: 'Search-baseline source',
    chartDescription: 'Raw and contract-adjusted F1 bars for each profile',
    rawLabel: 'Raw',
    adjustedLabel: 'Contract-adjusted',
    metricLabel: 'F1',
    confusionTitle: 'Confusion matrix',
    backend: 'Profile or product',
    rawCounts: 'Raw TP / TN / FP / FN',
    adjustedCounts: 'TPᶜ / TNᶜ / FPᶜ / FNᶜ',
    performanceTitle: 'Same-workload performance',
    performanceParagraph:
      'The quality fixture above runs in fresh processes with one warm-up followed by five measurements. The table compares four kfind profiles with fixed external-analyzer settings in the same workload and does not combine cost with quality.',
    initialization: 'Initialization',
    throughput: 'cases/s',
    latency: 'p95',
    memory: 'Peak RSS',
    sourcesTitle: 'Source evidence',
    sourceParagraph:
      'The approved snapshot records the source report Git revision and SHA-256. The report contains the environment, fixture checksums, tool versions, and case-level failures.',
    reportLink: 'Benchmark contract and reports',
  },
} as const;

function adjusted(result: QualityResult): ContractQuality {
  const metrics = result.contract_adjusted?.overall;

  if (metrics !== undefined) {
    return metrics;
  }

  return {
    contract_f1_percent: result.overall.f1_percent,
    contract_fn: result.overall.fn,
    contract_fp: result.overall.fp,
    contract_precision_percent: result.overall.precision_percent,
    contract_recall_percent: result.overall.recall_percent,
    contract_tn: result.overall.tn,
    contract_tp: result.overall.tp,
    excluded_cases: 0,
    reclassified_cases: 0,
    reviewed_cases: 0,
  };
}

function requiredEntry<Value>(
  values: Readonly<Record<string, Value>>,
  key: string,
  family: string,
): Value {
  const value = values[key];

  if (value === undefined) {
    throw new Error(`${family} result is unavailable for ${key}`);
  }

  return value;
}

function chartRows(
  backends: readonly string[],
  quality: Readonly<Record<string, QualityResult>>,
): QualityChartRow[] {
  return backends.map((backend) => {
    const result = requiredEntry(quality, backend, 'quality');

    return {
      adjusted: adjusted(result).contract_f1_percent,
      label: backendLabels[backend] ?? backend,
      raw: result.overall.f1_percent,
    };
  });
}

interface MorphologyComparisonProps {
  readonly adjustedCounts: string;
  readonly adjustedLabel: string;
  readonly backend: string;
  readonly caption: string;
  readonly chartDescription: string;
  readonly comparison: ProfileComparison;
  readonly confusionTitle: string;
  readonly initialization: string;
  readonly latency: string;
  readonly locale: DocumentLocale;
  readonly memory: string;
  readonly metricLabel: string;
  readonly performanceParagraph: string;
  readonly performanceTitle: string;
  readonly rawCounts: string;
  readonly rawLabel: string;
  readonly throughput: string;
  readonly title: string;
}

function MorphologyComparison({
  adjustedCounts,
  adjustedLabel,
  backend,
  caption,
  chartDescription,
  comparison,
  confusionTitle,
  initialization,
  latency,
  locale,
  memory,
  metricLabel,
  performanceParagraph,
  performanceTitle,
  rawCounts,
  rawLabel,
  throughput,
  title,
}: MorphologyComparisonProps): React.JSX.Element {
  return (
    <>
      <QualityChart
        adjustedLabel={adjustedLabel}
        caption={caption}
        description={chartDescription}
        metricLabel={metricLabel}
        rawLabel={rawLabel}
        rows={chartRows(comparison.profiles, comparison.quality)}
        title={title}
      />

      <h3>{confusionTitle}</h3>
      <div className="table-scroll">
        <table>
          <thead>
            <tr>
              <th scope="col">{backend}</th>
              <th scope="col">{rawCounts}</th>
              <th scope="col">{adjustedCounts}</th>
            </tr>
          </thead>
          <tbody>
            {comparison.profiles.map((profile) => {
              const result = requiredEntry(
                comparison.quality,
                profile,
                'quality',
              );
              const contract = adjusted(result);

              return (
                <tr key={profile}>
                  <th scope="row">{backendLabels[profile] ?? profile}</th>
                  <td>
                    {result.overall.tp} / {result.overall.tn} /{' '}
                    {result.overall.fp} / {result.overall.fn}
                  </td>
                  <td>
                    {contract.contract_tp} / {contract.contract_tn} /{' '}
                    {contract.contract_fp} / {contract.contract_fn}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <h3>{performanceTitle}</h3>
      <p>{performanceParagraph}</p>
      <div className="table-scroll">
        <table>
          <thead>
            <tr>
              <th scope="col">{backend}</th>
              <th scope="col">{initialization}</th>
              <th scope="col">{throughput}</th>
              <th scope="col">{latency}</th>
              <th scope="col">{memory}</th>
            </tr>
          </thead>
          <tbody>
            {comparison.profiles.map((profile) => {
              const result = requiredEntry(
                comparison.performance,
                profile,
                'performance',
              );

              return (
                <tr key={profile}>
                  <th scope="row">{backendLabels[profile] ?? profile}</th>
                  <td>{result.initialization_seconds.toFixed(4)} s</td>
                  <td>{result.cases_per_second.toLocaleString(locale)}</td>
                  <td>{result.latency_p95_ms.toFixed(4)} ms</td>
                  <td>{(result.peak_rss_kib / 1024).toFixed(1)} MiB</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </>
  );
}

function searchQualityRows(locale: DocumentLocale): readonly QualityChartRow[] {
  return searchBaselineSnapshot.quality.map((result) => ({
    adjusted: result.contract_adjusted.f1_percent,
    label: searchStrategyLabels[locale][result.id] ?? result.id,
    raw: result.raw.f1_percent,
  }));
}

function durationRows(locale: DocumentLocale): readonly DurationChartRow[] {
  return searchBaselineSnapshot.performance.methods.map((result) => ({
    label: searchStrategyLabels[locale][result.id] ?? result.id,
    milliseconds: result.median_ms,
  }));
}

export default function BenchmarksPage(): React.JSX.Element {
  const locale = useDocumentLocale();
  const text = copy[locale];

  return (
    <DocumentPage>
      <PageIntro
        eyebrow={text.eyebrow}
        title={text.title}
        summary={text.summary}
      />

      <DocumentSection id="evaluation-scope" title={text.scopeTitle}>
        {text.scopeParagraphs.map((paragraph) => (
          <p key={paragraph}>{paragraph}</p>
        ))}
      </DocumentSection>

      <DocumentSection id="quality-contract" title={text.metricTitle}>
        {text.metricParagraphs.map((paragraph) => (
          <p key={paragraph}>{paragraph}</p>
        ))}
        <p>{text.fnExample}</p>
      </DocumentSection>

      <DocumentSection id="canonical-quality" title={text.canonicalTitle}>
        <MorphologyComparison
          adjustedCounts={text.adjustedCounts}
          adjustedLabel={text.adjustedLabel}
          backend={text.backend}
          caption={text.canonicalCaption}
          chartDescription={text.chartDescription}
          comparison={
            benchmarkSnapshot.profile_comparisons[MorphologyWorkload.Canonical]
          }
          confusionTitle={text.confusionTitle}
          initialization={text.initialization}
          latency={text.latency}
          locale={locale}
          memory={text.memory}
          metricLabel={text.metricLabel}
          performanceParagraph={text.performanceParagraph}
          performanceTitle={text.performanceTitle}
          rawCounts={text.rawCounts}
          rawLabel={text.rawLabel}
          throughput={text.throughput}
          title={text.canonicalTitle}
        />
      </DocumentSection>

      <DocumentSection id="query-matrix-quality" title={text.queryMatrixTitle}>
        <p>{text.queryMatrixDescription}</p>
        <MorphologyComparison
          adjustedCounts={text.adjustedCounts}
          adjustedLabel={text.adjustedLabel}
          backend={text.backend}
          caption={text.queryMatrixCaption}
          chartDescription={text.chartDescription}
          comparison={
            benchmarkSnapshot.profile_comparisons[
              MorphologyWorkload.QueryMatrix
            ]
          }
          confusionTitle={text.confusionTitle}
          initialization={text.initialization}
          latency={text.latency}
          locale={locale}
          memory={text.memory}
          metricLabel={text.metricLabel}
          performanceParagraph={text.performanceParagraph}
          performanceTitle={text.performanceTitle}
          rawCounts={text.rawCounts}
          rawLabel={text.rawLabel}
          throughput={text.throughput}
          title={text.queryMatrixTitle}
        />
      </DocumentSection>

      <DocumentSection id="robust-quality" title={text.robustTitle}>
        <MorphologyComparison
          adjustedCounts={text.adjustedCounts}
          adjustedLabel={text.adjustedLabel}
          backend={text.backend}
          caption={text.robustCaption}
          chartDescription={text.chartDescription}
          comparison={
            benchmarkSnapshot.profile_comparisons[MorphologyWorkload.Robustness]
          }
          confusionTitle={text.confusionTitle}
          initialization={text.initialization}
          latency={text.latency}
          locale={locale}
          memory={text.memory}
          metricLabel={text.metricLabel}
          performanceParagraph={text.performanceParagraph}
          performanceTitle={text.performanceTitle}
          rawCounts={text.rawCounts}
          rawLabel={text.rawLabel}
          throughput={text.throughput}
          title={text.robustTitle}
        />
      </DocumentSection>

      <DocumentSection id="search-strategy-baseline" title={text.searchTitle}>
        {text.searchParagraphs.map((paragraph) => (
          <p key={paragraph}>{paragraph}</p>
        ))}
        <QualityChart
          adjustedLabel={text.adjustedLabel}
          caption={text.searchQualityCaption}
          description={text.searchChartDescription}
          metricLabel={text.metricLabel}
          rawLabel={text.rawLabel}
          rows={searchQualityRows(locale)}
          title={text.searchTitle}
        />

        <h3>{text.searchConfusionTitle}</h3>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">{text.strategy}</th>
                <th scope="col">{text.searchRawCounts}</th>
                <th scope="col">{text.searchAdjustedCounts}</th>
              </tr>
            </thead>
            <tbody>
              {searchBaselineSnapshot.quality.map((result) => (
                <tr key={result.id}>
                  <th scope="row">
                    {searchStrategyLabels[locale][result.id] ?? result.id}
                  </th>
                  <td>
                    {result.raw.tp} / {result.raw.tn} / {result.raw.fp} /{' '}
                    {result.raw.fn}
                  </td>
                  <td>
                    {result.contract_adjusted.tp} /{' '}
                    {result.contract_adjusted.tn} /{' '}
                    {result.contract_adjusted.fp} /{' '}
                    {result.contract_adjusted.fn}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        <h3>{text.searchPerformanceTitle}</h3>
        <p>{text.searchPerformanceParagraph}</p>
        <DurationChart
          caption={text.searchDurationCaption}
          description={text.searchDurationDescription}
          rows={durationRows(locale)}
          title={text.searchPerformanceTitle}
        />
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">{text.strategy}</th>
                <th scope="col">{text.median}</th>
                <th scope="col">{text.minimum}</th>
                <th scope="col">{text.maximum}</th>
                <th scope="col">{text.latency}</th>
                <th scope="col">{text.effectiveThroughput}</th>
              </tr>
            </thead>
            <tbody>
              {searchBaselineSnapshot.performance.methods.map((result) => (
                <tr key={result.id}>
                  <th scope="row">
                    {searchStrategyLabels[locale][result.id] ?? result.id}
                  </th>
                  <td>{result.median_ms.toFixed(2)} ms</td>
                  <td>{result.min_ms.toFixed(2)} ms</td>
                  <td>{result.max_ms.toFixed(2)} ms</td>
                  <td>{result.p95_ms.toFixed(2)} ms</td>
                  <td>
                    {result.effective_mib_per_second.toLocaleString(locale)}{' '}
                    MiB/s
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        <p className="source-identifiers">
          {text.searchSource}:{' '}
          <code>{searchBaselineSnapshot.source_report.revision}</code> ·{' '}
          <code>{searchBaselineSnapshot.source_report.sha256}</code>
        </p>
        <p className="reference-link">
          <a href="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-24-search-strategy-baseline.md">
            {text.reportLink}
          </a>
        </p>
      </DocumentSection>

      <DocumentSection id="source-evidence" title={text.sourcesTitle}>
        <p>{text.sourceParagraph}</p>
        <p className="source-identifiers">
          <code>{benchmarkSnapshot.source_report.revision}</code> ·{' '}
          <code>{benchmarkSnapshot.source_report.sha256}</code>
        </p>
        <p className="reference-link">
          <a href="https://github.com/SeokminHong/kfind/tree/main/docs/benchmarks">
            {text.reportLink}
          </a>
        </p>
      </DocumentSection>
    </DocumentPage>
  );
}
