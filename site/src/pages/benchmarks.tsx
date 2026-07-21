import type { QualityChartRow } from '../components/quality-chart';

import { DocumentLocale, useDocumentLocale } from '../app/i18n';
import { createDocumentMeta } from '../app/metadata';
import { RoutePath } from '../app/navigation';
import {
  DocumentPage,
  DocumentSection,
  PageIntro,
} from '../components/document';
import { QualityChart } from '../components/quality-chart';
import benchmarkSnapshotJson from '../generated-benchmark/site-morphology.json';

export const meta = createDocumentMeta(RoutePath.Benchmarks);

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

interface BenchmarkSnapshot {
  readonly backends: readonly string[];
  readonly performance: Readonly<Record<string, PerformanceResult>>;
  readonly quality: Readonly<Record<string, QualityResult>>;
  readonly query_matrix: {
    readonly explicit_pos: {
      readonly dataset: {
        readonly cases: number;
        readonly contract_review: {
          readonly excluded_cases: number;
          readonly reclassified_cases: number;
          readonly registry_sha256: string;
          readonly reviewed_cases: number;
        };
      };
      readonly quality: Readonly<Record<string, QualityResult>>;
    };
  };
  readonly robustness: {
    readonly explicit_pos: {
      readonly backends: readonly string[];
      readonly quality: Readonly<Record<string, QualityResult>>;
    };
  };
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

const benchmarkSnapshot = benchmarkSnapshotJson as BenchmarkSnapshot;

const backendLabels: Readonly<Record<string, string>> = {
  'kfind-embedded': 'kfind embedded',
  'kfind-full-pos': 'kfind full POS',
  kiwi: 'Kiwi',
  lindera: 'Lindera',
  'mecab-ko': 'MeCab-ko',
  komoran: 'KOMORAN',
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
      'Contract review가 없는 평가군은 raw 기대값을 그대로 사용합니다. 이때 두 결과가 같고 reviewed cases가 0이라는 사실도 결과에 포함합니다. 보정 결과만 남기거나 raw 결과를 숨기지 않습니다.',
    ],
    fnExample:
      'Full-POS query matrix의 raw FN 4와 FNᶜ 0은 같은 실행 결과를 두 계약으로 읽은 값입니다. Raw FN 4는 strict gold span을 회수하지 못한 사례입니다. Registry는 그중 의미 구조로 구분할 수 없는 동형이의와 source에 정렬된 검색 성분을 제품 목표의 양성으로 선언합니다. 따라서 FNᶜ 0은 네 사례가 제품 목표 밖의 false negative라는 뜻이며, 제품이 네 오류를 수정했다는 뜻이 아닙니다.',
    canonicalTitle: 'Canonical 품질',
    canonicalCaption:
      '같은 1,000개 explicit-POS 사례의 F1입니다. Review registry가 없는 행은 raw와 contract-adjusted 값이 같고 review 수가 0입니다.',
    queryMatrixTitle: 'Query matrix 품질',
    queryMatrixCaption:
      '같은 query matrix 예측에 strict gold와 고정 contract review registry를 각각 적용한 F1입니다.',
    robustTitle: 'Robust 품질',
    robustCaption:
      '같은 500개 실제 오류 문장의 F1입니다. Contract review가 없으므로 raw와 contract-adjusted 결과가 같습니다.',
    chartDescription: '제품별 raw와 contract-adjusted F1 막대 비교',
    rawLabel: 'Raw',
    adjustedLabel: 'Contract-adjusted',
    metricLabel: 'F1',
    confusionTitle: 'Canonical confusion matrix',
    backend: '제품',
    rawCounts: 'Raw TP / FP / TN / FN',
    adjustedCounts: 'TPᶜ / FPᶜ / TNᶜ / FNᶜ',
    performanceTitle: '성능 측정 단위',
    performanceParagraph:
      '형태 품질 workload는 fresh process에서 warm-up 1회 뒤 5회 측정합니다. 다음 표의 값은 중앙값이며, 품질 지표와 하나의 점수로 합치지 않습니다.',
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
      'A dataset without contract reviews uses its raw expectation unchanged. The report still includes the identical adjusted result and a reviewed-case count of zero. Raw evidence is never replaced by the adjusted view.',
    ],
    fnExample:
      'The full-POS query matrix has four raw false negatives and zero contract-adjusted false negatives. The execution result is identical in both views. The four raw misses target strict gold spans, while the registry classifies their indistinguishable homographs and source-aligned components as positive under the product contract. FNᶜ = 0 therefore means that the four misses are outside the product false-negative objective; it does not mean that the implementation fixed four errors.',
    canonicalTitle: 'Canonical quality',
    canonicalCaption:
      'F1 on the same 1,000 explicit-POS cases. Rows without contract reviews have identical raw and adjusted values and a review count of zero.',
    queryMatrixTitle: 'Query-matrix quality',
    queryMatrixCaption:
      'F1 from the same predictions evaluated with strict gold and the fixed contract-review registry.',
    robustTitle: 'Robust quality',
    robustCaption:
      'F1 on the same 500 natural noisy sentences. No contract review is applied, so raw and adjusted results are identical.',
    chartDescription: 'Raw and contract-adjusted F1 bars for each product',
    rawLabel: 'Raw',
    adjustedLabel: 'Contract-adjusted',
    metricLabel: 'F1',
    confusionTitle: 'Canonical confusion matrix',
    backend: 'Product',
    rawCounts: 'Raw TP / FP / TN / FN',
    adjustedCounts: 'TPᶜ / FPᶜ / TNᶜ / FNᶜ',
    performanceTitle: 'Performance units',
    performanceParagraph:
      'Morphology workloads run in fresh processes with one warm-up followed by five measurements. The table contains medians and does not combine execution cost with quality.',
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

export default function BenchmarksPage(): React.JSX.Element {
  const locale = useDocumentLocale();
  const text = copy[locale];
  const queryMatrixQuality =
    benchmarkSnapshot.query_matrix.explicit_pos.quality;
  const queryMatrixBackends = Object.keys(queryMatrixQuality);

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
        <QualityChart
          adjustedLabel={text.adjustedLabel}
          caption={text.canonicalCaption}
          description={text.chartDescription}
          metricLabel={text.metricLabel}
          rawLabel={text.rawLabel}
          rows={chartRows(
            benchmarkSnapshot.backends,
            benchmarkSnapshot.quality,
          )}
          title={text.canonicalTitle}
        />
        <h3>{text.confusionTitle}</h3>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">{text.backend}</th>
                <th scope="col">{text.rawCounts}</th>
                <th scope="col">{text.adjustedCounts}</th>
              </tr>
            </thead>
            <tbody>
              {benchmarkSnapshot.backends.map((backend) => {
                const result = requiredEntry(
                  benchmarkSnapshot.quality,
                  backend,
                  'canonical quality',
                );
                const contract = adjusted(result);

                return (
                  <tr key={backend}>
                    <th scope="row">{backendLabels[backend] ?? backend}</th>
                    <td>
                      {result.overall.tp} / {result.overall.fp} /{' '}
                      {result.overall.tn} / {result.overall.fn}
                    </td>
                    <td>
                      {contract.contract_tp} / {contract.contract_fp} /{' '}
                      {contract.contract_tn} / {contract.contract_fn}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </DocumentSection>

      <DocumentSection id="query-matrix-quality" title={text.queryMatrixTitle}>
        <QualityChart
          adjustedLabel={text.adjustedLabel}
          caption={text.queryMatrixCaption}
          description={text.chartDescription}
          metricLabel={text.metricLabel}
          rawLabel={text.rawLabel}
          rows={chartRows(queryMatrixBackends, queryMatrixQuality)}
          title={text.queryMatrixTitle}
        />
      </DocumentSection>

      <DocumentSection id="robust-quality" title={text.robustTitle}>
        <QualityChart
          adjustedLabel={text.adjustedLabel}
          caption={text.robustCaption}
          description={text.chartDescription}
          metricLabel={text.metricLabel}
          rawLabel={text.rawLabel}
          rows={chartRows(
            benchmarkSnapshot.robustness.explicit_pos.backends,
            benchmarkSnapshot.robustness.explicit_pos.quality,
          )}
          title={text.robustTitle}
        />
      </DocumentSection>

      <DocumentSection id="performance-units" title={text.performanceTitle}>
        <p>{text.performanceParagraph}</p>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">{text.backend}</th>
                <th scope="col">{text.initialization}</th>
                <th scope="col">{text.throughput}</th>
                <th scope="col">{text.latency}</th>
                <th scope="col">{text.memory}</th>
              </tr>
            </thead>
            <tbody>
              {Object.keys(benchmarkSnapshot.performance).map((backend) => {
                const result = requiredEntry(
                  benchmarkSnapshot.performance,
                  backend,
                  'performance',
                );

                return (
                  <tr key={backend}>
                    <th scope="row">{backendLabels[backend] ?? backend}</th>
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
