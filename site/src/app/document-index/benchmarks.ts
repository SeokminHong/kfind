import type { DocumentGroupIndex } from './types';

import { RoutePath } from '../route-path';

import { localized, page } from './types';

export const benchmarksGroup: DocumentGroupIndex = {
  labelKey: 'navigation.primary.benchmarks',
  categories: [
    {
      label: localized('결과', 'RESULTS'),
      pages: [
        page(
          RoutePath.Benchmarks,
          '평가 개요',
          'Evaluation overview',
          '품질 계약, 최신 결과, 성능 단위와 source report의 관계를 설명합니다.',
          'Connect the quality contract, current results, performance units, and source reports.',
          [
            ['evaluation-scope', '평가 범위', 'Evaluation scope'],
            ['quality-contract', '품질 계약', 'Quality contract'],
            ['canonical-quality', '표준문 품질', 'Canonical quality'],
            [
              'query-matrix-quality',
              'query matrix 품질',
              'Query-matrix quality',
            ],
            ['robust-quality', '오류 문장 품질', 'Robust quality'],
            ['performance-units', '성능 단위', 'Performance units'],
            ['source-evidence', 'source 근거', 'Source evidence'],
          ],
        ),
        page(
          RoutePath.BenchmarkCurrent,
          '최신 결과',
          'Current results',
          '승인 snapshot의 품질·성능 결과와 적용 범위를 한곳에 표시합니다.',
          'Present the approved quality and performance snapshot with its scope.',
          [
            ['evaluation-scope', '평가 범위', 'Evaluation scope'],
            ['quality-contract', '품질 계약', 'Quality contract'],
            ['canonical-quality', '표준문 품질', 'Canonical quality'],
            [
              'query-matrix-quality',
              'query matrix 품질',
              'Query-matrix quality',
            ],
            ['robust-quality', '오류 문장 품질', 'Robust quality'],
            ['performance-units', '성능 단위', 'Performance units'],
            ['source-evidence', 'source 근거', 'Source evidence'],
          ],
        ),
      ],
    },
    {
      label: localized('품질', 'QUALITY'),
      pages: [
        page(
          RoutePath.BenchmarkMethodology,
          '평가 방법',
          'Methodology',
          'fixture, gold, backend와 confusion matrix 산출 절차를 정의합니다.',
          'Define fixtures, gold labels, backends, and confusion-matrix computation.',
          [
            ['fixtures', 'fixture', 'Fixtures'],
            ['gold', 'gold 판정', 'Gold labels'],
            ['aggregation', '집계', 'Aggregation'],
          ],
        ),
        page(
          RoutePath.BenchmarkContract,
          '품질 계약',
          'Quality contract',
          'raw와 contract-adjusted 지표의 분모와 disposition 절차를 정의합니다.',
          'Define raw and contract-adjusted denominators and the disposition process.',
          [
            ['raw', 'raw matrix', 'Raw matrix'],
            [
              'contract-adjusted',
              'contract-adjusted matrix',
              'Contract-adjusted matrix',
            ],
            ['dispositions', 'disposition', 'Dispositions'],
          ],
        ),
        page(
          RoutePath.BenchmarkCanonical,
          '표준문 품질',
          'Canonical quality',
          '표준 맞춤법 fixture의 positive·negative 구성과 비교 기준을 설명합니다.',
          'Describe positive and negative cases in the canonical-spelling fixture.',
          [
            ['dataset', 'dataset', 'Dataset'],
            ['metrics', '지표', 'Metrics'],
            ['limits', '해석 한계', 'Interpretation limits'],
          ],
        ),
        page(
          RoutePath.BenchmarkQueryMatrix,
          'query matrix',
          'Query matrix',
          '문법 요소와 옵션 조합별 coverage 및 case disposition을 설명합니다.',
          'Measure grammar and option combinations with case-level dispositions.',
          [
            ['dimensions', '조합 차원', 'Dimensions'],
            ['coverage', 'coverage', 'Coverage'],
            ['disposition-ledger', 'disposition ledger', 'Disposition ledger'],
          ],
        ),
        page(
          RoutePath.BenchmarkRobustness,
          '오류 문장 품질',
          'Robustness quality',
          'target-span과 context-only 오류를 분리한 500-case 평가를 설명합니다.',
          'Evaluate 500 noisy cases while separating target-span from context-only errors.',
          [
            ['dataset', '오류 dataset', 'Noisy dataset'],
            ['classes', '오류 class', 'Error classes'],
            ['reporting', '분리 보고', 'Separate reporting'],
          ],
        ),
        page(
          RoutePath.BenchmarkComparisons,
          '외부 제품 비교',
          'External comparisons',
          '동일 gold에서 kfind와 외부 분석기의 raw·contract-adjusted 지표를 비교합니다.',
          'Compare kfind and external analyzers on identical raw and adjusted gold.',
          [
            ['alignment', '출력 정렬', 'Output alignment'],
            ['quality', '품질 비교', 'Quality comparison'],
            ['cost', '비용 비교', 'Cost comparison'],
          ],
        ),
      ],
    },
    {
      label: localized('성능과 근거', 'PERFORMANCE AND EVIDENCE'),
      pages: [
        page(
          RoutePath.BenchmarkPerformance,
          '성능 측정',
          'Performance measurement',
          'compile, matcher, startup, throughput와 RSS를 workload별로 측정합니다.',
          'Measure compile, matcher, startup, throughput, and RSS as separate workloads.',
          [
            ['workloads', 'workload', 'Workloads'],
            ['statistics', '통계', 'Statistics'],
            ['regression', '회귀 판정', 'Regression decision'],
          ],
        ),
        page(
          RoutePath.BenchmarkReproducibility,
          '재현 방법',
          'Reproducibility',
          'revision, input checksum, 환경, warm-up과 실행 명령을 고정합니다.',
          'Pin revisions, input checksums, environments, warm-ups, and commands.',
          [
            ['provenance', 'provenance', 'Provenance'],
            ['commands', '실행 명령', 'Commands'],
            ['verification', '산출물 검증', 'Artifact verification'],
          ],
        ),
        page(
          RoutePath.BenchmarkReports,
          '역사 보고서',
          'Historical reports',
          '날짜별 측정과 baseline·candidate 변화의 source report 위치를 안내합니다.',
          'Locate dated source reports for measurements and baseline-candidate changes.',
          [
            ['reports', '보고서 구조', 'Report structure'],
            ['snapshots', 'site snapshot', 'Site snapshot'],
            ['retention', '보존 계약', 'Retention contract'],
          ],
        ),
      ],
    },
  ],
};
