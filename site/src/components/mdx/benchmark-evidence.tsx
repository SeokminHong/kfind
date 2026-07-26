import type { DurationChartRow, QualityChartRow } from '../quality-chart';

import { DocumentLocale, useDocumentLocale } from '../../app/i18n';
import benchmarkSnapshotJson from '../../generated-benchmark/site-morphology.json';
import searchBaselineSnapshotJson from '../../generated-benchmark/site-search-baseline.json';
import { DurationChart, QualityChart } from '../quality-chart';

type MorphologyWorkload = 'canonical' | 'query_matrix' | 'robustness';

interface RawQuality {
  readonly f1_percent: number;
  readonly fn: number;
  readonly fp: number;
  readonly tn: number;
  readonly tp: number;
}

interface ContractQuality {
  readonly contract_f1_percent: number;
  readonly contract_fn: number;
  readonly contract_fp: number;
  readonly contract_tn: number;
  readonly contract_tp: number;
}

interface QualityResult {
  readonly contract_adjusted?: { readonly overall: ContractQuality };
  readonly overall: RawQuality;
}

interface PerformanceResult {
  readonly cases_per_second: number;
  readonly initialization_seconds: number;
  readonly latency_p95_ms: number;
  readonly peak_rss_kib: number;
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

interface SearchMetric {
  readonly f1_percent: number;
  readonly fn: number;
  readonly fp: number;
  readonly tn: number;
  readonly tp: number;
}

interface SearchQuality {
  readonly contract_adjusted: SearchMetric;
  readonly id: string;
  readonly raw: SearchMetric;
}

interface SearchPerformance {
  readonly effective_mib_per_second: number;
  readonly id: string;
  readonly max_ms: number;
  readonly median_ms: number;
  readonly min_ms: number;
  readonly p95_ms: number;
}

interface SearchBaselineSnapshot {
  readonly performance: {
    readonly methods: readonly SearchPerformance[];
  };
  readonly quality: readonly SearchQuality[];
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
  komoran: 'KOMORAN',
  lindera: 'Lindera',
  'mecab-ko': 'MeCab-ko',
};

const searchLabels: Readonly<
  Record<DocumentLocale, Readonly<Record<string, string>>>
> = {
  en: {
    grep_enumerated: 'grep · enumerated surfaces',
    grep_stem: 'grep · short stems',
    kfind_any: 'kfind full POS · any',
    kfind_smart: 'kfind full POS · smart',
    regex_enumerated: 'Enumerated-surface regex',
    regex_stem: 'Short-stem regex',
    rg_enumerated: 'rg · enumerated surfaces',
    rg_stem: 'rg · short stems',
  },
  ko: {
    grep_enumerated: 'grep · 활용형 열거',
    grep_stem: 'grep · 짧은 어간',
    kfind_any: 'kfind full POS · any',
    kfind_smart: 'kfind full POS · smart',
    regex_enumerated: '활용형 열거 정규식',
    regex_stem: '짧은 어간 정규식',
    rg_enumerated: 'rg · 활용형 열거',
    rg_stem: 'rg · 짧은 어간',
  },
};

const copy = {
  en: {
    adjusted: 'Contract-adjusted',
    adjustedCounts: 'TPᶜ / TNᶜ / FPᶜ / FNᶜ',
    backend: 'Profile or product',
    captions: {
      canonical:
        'F1 on the same 1,000 explicit-POS cases for four kfind profiles and fixed external-analyzer settings.',
      query_matrix:
        'F1 from strict gold and the fixed contract-review registry on the same query matrix.',
      robustness:
        'F1 on the same 500 natural noisy sentences. Raw and adjusted results are identical where no review applies.',
    },
    chartDescription: 'Raw and contract-adjusted F1 for every profile',
    confusion: 'Confusion matrix',
    effectiveThroughput: 'Effective throughput',
    initialization: 'Initialization',
    latency: 'p95',
    maximum: 'Maximum',
    median: 'Median',
    memory: 'Peak RSS',
    metric: 'F1',
    minimum: 'Minimum',
    performance: 'Same-workload performance',
    performanceDescription:
      'Fresh processes run one warm-up followed by five measurements. Quality and cost remain separate.',
    raw: 'Raw',
    rawCounts: 'Raw TP / TN / FP / FN',
    searchCaption:
      'F1 from strict gold and fixed contract expectations on the same 112 cases.',
    searchChartDescription:
      'Raw and contract-adjusted F1 for kfind and regex search strategies',
    searchDurationCaption:
      'Median seven-query fresh-process batch wall time; lower is shorter.',
    searchDurationDescription:
      'Median seven-query batch time for kfind, rg, and grep strategies',
    searchPerformance: 'Seven-query batch time',
    searchStrategy: 'Search strategy',
    throughput: 'cases/s',
    titles: {
      canonical: 'Canonical quality',
      query_matrix: 'Query-matrix quality',
      robustness: 'Robust quality',
    },
  },
  ko: {
    adjusted: 'Contract-adjusted',
    adjustedCounts: 'TPᶜ / TNᶜ / FPᶜ / FNᶜ',
    backend: '프로필·제품',
    captions: {
      canonical:
        '같은 1,000개 explicit-POS 사례에서 kfind profile 4종과 외부 분석기 고정 설정의 F1입니다.',
      query_matrix:
        '같은 query matrix에 strict gold와 고정 contract review registry를 적용한 F1입니다.',
      robustness:
        '같은 500개 실제 오류 문장의 F1입니다. Review가 없는 결과는 raw와 adjusted가 같습니다.',
    },
    chartDescription: '프로필별 raw와 contract-adjusted F1',
    confusion: 'Confusion matrix',
    effectiveThroughput: '유효 처리량',
    initialization: '초기화',
    latency: 'p95',
    maximum: '최댓값',
    median: '중앙값',
    memory: 'peak RSS',
    metric: 'F1',
    minimum: '최솟값',
    performance: '동일 workload 성능',
    performanceDescription:
      'Fresh process에서 warm-up 1회 뒤 5회 측정합니다. 품질과 비용은 하나의 점수로 합치지 않습니다.',
    raw: 'Raw',
    rawCounts: 'Raw TP / TN / FP / FN',
    searchCaption:
      '같은 112개 case에 strict gold와 고정 contract expectation을 적용한 F1입니다.',
    searchChartDescription:
      'kfind와 정규식 검색 전략의 raw 및 contract-adjusted F1',
    searchDurationCaption:
      '7-query fresh-process batch wall time 중앙값이며 낮을수록 짧습니다.',
    searchDurationDescription:
      'kfind, rg, grep 검색 전략별 7-query batch 중앙값',
    searchPerformance: '7-query batch 시간',
    searchStrategy: '검색 전략',
    throughput: 'cases/s',
    titles: {
      canonical: 'Canonical 품질',
      query_matrix: 'Query matrix 품질',
      robustness: 'Robust 품질',
    },
  },
} as const;

export function MorphologyEvidence({
  workload,
}: {
  readonly workload: MorphologyWorkload;
}): React.JSX.Element {
  const locale = useDocumentLocale();
  const text = copy[locale];
  const comparison = benchmarkSnapshot.profile_comparisons[workload];

  return (
    <>
      <QualityChart
        adjustedLabel={text.adjusted}
        caption={text.captions[workload]}
        description={text.chartDescription}
        metricLabel={text.metric}
        rawLabel={text.raw}
        rows={comparison.profiles.map((profile) => {
          const result = requiredEntry(comparison.quality, profile);
          return {
            adjusted: adjusted(result).contract_f1_percent,
            label: backendLabels[profile] ?? profile,
            raw: result.overall.f1_percent,
          };
        })}
        title={text.titles[workload]}
      />
      <h3>{text.confusion}</h3>
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
            {comparison.profiles.map((profile) => {
              const result = requiredEntry(comparison.quality, profile);
              const contract = adjusted(result);
              return (
                <tr key={profile}>
                  <th scope="row">{backendLabels[profile] ?? profile}</th>
                  <td>{counts(result.overall)}</td>
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
      <h3>{text.performance}</h3>
      <p>{text.performanceDescription}</p>
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
            {comparison.profiles.map((profile) => {
              const result = requiredEntry(comparison.performance, profile);
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

export function SearchBaselineEvidence(): React.JSX.Element {
  const locale = useDocumentLocale();
  const text = copy[locale];
  const qualityRows: readonly QualityChartRow[] =
    searchBaselineSnapshot.quality.map((result) => ({
      adjusted: result.contract_adjusted.f1_percent,
      label: searchLabels[locale][result.id] ?? result.id,
      raw: result.raw.f1_percent,
    }));
  const durationRows: readonly DurationChartRow[] =
    searchBaselineSnapshot.performance.methods.map((result) => ({
      label: searchLabels[locale][result.id] ?? result.id,
      milliseconds: result.median_ms,
    }));

  return (
    <>
      <QualityChart
        adjustedLabel={text.adjusted}
        caption={text.searchCaption}
        description={text.searchChartDescription}
        metricLabel={text.metric}
        rawLabel={text.raw}
        rows={qualityRows}
        title={text.searchStrategy}
      />
      <h3>{text.confusion}</h3>
      <div className="table-scroll">
        <table>
          <thead>
            <tr>
              <th scope="col">{text.searchStrategy}</th>
              <th scope="col">{text.rawCounts}</th>
              <th scope="col">{text.adjustedCounts}</th>
            </tr>
          </thead>
          <tbody>
            {searchBaselineSnapshot.quality.map((result) => (
              <tr key={result.id}>
                <th scope="row">
                  {searchLabels[locale][result.id] ?? result.id}
                </th>
                <td>{counts(result.raw)}</td>
                <td>{counts(result.contract_adjusted)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <h3>{text.searchPerformance}</h3>
      <DurationChart
        caption={text.searchDurationCaption}
        description={text.searchDurationDescription}
        rows={durationRows}
        title={text.searchPerformance}
      />
      <div className="table-scroll">
        <table>
          <thead>
            <tr>
              <th scope="col">{text.searchStrategy}</th>
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
                  {searchLabels[locale][result.id] ?? result.id}
                </th>
                <td>{result.median_ms.toFixed(2)} ms</td>
                <td>{result.min_ms.toFixed(2)} ms</td>
                <td>{result.max_ms.toFixed(2)} ms</td>
                <td>{result.p95_ms.toFixed(2)} ms</td>
                <td>
                  {result.effective_mib_per_second.toLocaleString(locale)} MiB/s
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <SourceIdentifier
        report="https://github.com/SeokminHong/kfind/blob/main/docs/benchmarks/2026-07-24-search-strategy-baseline.md"
        revision={searchBaselineSnapshot.source_report.revision}
        sha256={searchBaselineSnapshot.source_report.sha256}
      />
    </>
  );
}

export function BenchmarkSourceEvidence(): React.JSX.Element {
  return (
    <SourceIdentifier
      report="https://github.com/SeokminHong/kfind/tree/main/docs/benchmarks"
      revision={benchmarkSnapshot.source_report.revision}
      sha256={benchmarkSnapshot.source_report.sha256}
    />
  );
}

function SourceIdentifier({
  report,
  revision,
  sha256,
}: {
  readonly report: string;
  readonly revision: string;
  readonly sha256: string;
}): React.JSX.Element {
  const locale = useDocumentLocale();
  return (
    <>
      <p className="source-identifiers">
        <code>{revision}</code> · <code>{sha256}</code>
      </p>
      <p className="reference-link">
        <a href={report}>
          {locale === DocumentLocale.Korean
            ? '벤치마크 계약과 보고서'
            : 'Benchmark contract and reports'}
        </a>
      </p>
    </>
  );
}

function adjusted(result: QualityResult): ContractQuality {
  return (
    result.contract_adjusted?.overall ?? {
      contract_f1_percent: result.overall.f1_percent,
      contract_fn: result.overall.fn,
      contract_fp: result.overall.fp,
      contract_tn: result.overall.tn,
      contract_tp: result.overall.tp,
    }
  );
}

function counts(result: RawQuality | SearchMetric): string {
  return `${result.tp} / ${result.tn} / ${result.fp} / ${result.fn}`;
}

function requiredEntry<Value>(
  values: Readonly<Record<string, Value>>,
  key: string,
): Value {
  const value = values[key];
  if (value === undefined) {
    throw new Error(`benchmark result is unavailable for ${key}`);
  }
  return value;
}
