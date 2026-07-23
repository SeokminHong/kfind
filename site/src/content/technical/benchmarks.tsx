import type { TechnicalDocuments } from './types';

import { DocumentLocale } from '../../app/i18n';
import { RoutePath } from '../../app/navigation';

import { section } from './section';

export const benchmarkDocuments: TechnicalDocuments = {
  [RoutePath.BenchmarkMethodology]: {
    [DocumentLocale.Korean]: {
      eyebrow: '벤치마크 · 품질',
      title: '평가 방법',
      summary:
        '품질 평가는 고정 fixture, gold label과 동일 backend task에서 confusion matrix를 계산합니다.',
      sections: [
        section('fixture', [
          'Canonical은 사람이 표준 맞춤법을 확인한 양성 500·음성 500 사례입니다. Robust는 실제 오류 문장의 양성 250·음성 250 사례이며 표준문 결과에 합산하지 않습니다.',
          'Query matrix는 한 문장에 여러 positive·negative query를 적용해 문법 조합별 후보를 측정합니다.',
        ]),
        section('gold 판정', [
          'Positive는 표제어·품사와 목표 span을 함께 선언합니다. Negative는 같은 문장 안의 형태 또는 경계 경쟁자를 포함해 false positive를 관찰합니다.',
          '제품 실행 전 versioned contract registry를 고정합니다. 실행 결과를 본 뒤 편의상 gold를 바꾸지 않습니다.',
        ]),
        section('집계', [
          '모든 backend는 같은 fixture와 query를 실행하고 TP, FP, TN, FN을 case 단위로 계산합니다. Precision, recall과 F1은 confusion matrix에서 직접 산출합니다.',
          '외부 제품도 raw와 contract-adjusted matrix를 모두 기록합니다. Review가 없는 dataset은 두 결과가 같고 review 0건임을 명시합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'BENCHMARKS · QUALITY',
      title: 'Methodology',
      summary:
        'Quality evaluation computes confusion matrices from fixed fixtures, gold labels, and identical backend tasks.',
      sections: [
        section('Fixtures', [
          'Canonical has 500 positive and 500 negative cases manually checked for standard spelling. Robust has 250 positive and 250 negative natural noisy cases and is not merged with canonical results.',
          'The query matrix applies several positive and negative queries to each sentence to measure grammar combinations.',
        ]),
        section('Gold labels', [
          'A positive declares lemma, POS, and target span. Negatives include morphology and boundary competitors in the same sentence to expose false positives.',
          'A versioned contract registry is fixed before product execution. Gold is never changed for convenience after seeing predictions.',
        ]),
        section('Aggregation', [
          'Every backend runs the same fixture and query; TP, FP, TN, and FN are counted per case. Precision, recall, and F1 derive directly from the matrix.',
          'External products also report raw and adjusted matrices. A dataset without review explicitly records identical results and zero reviewed cases.',
        ]),
      ],
    },
  },
  [RoutePath.BenchmarkContract]: {
    [DocumentLocale.Korean]: {
      eyebrow: '벤치마크 · 계약',
      title: '품질 계약',
      summary:
        'Raw는 corpus gold를 보존하고 contract-adjusted는 같은 예측을 제품 목표의 고정 registry로 다시 판정합니다.',
      sections: [
        section('raw matrix', [
          'Raw TP·FP·TN·FN은 원본 fixture의 strict gold와 backend 예측을 그대로 비교합니다. 데이터 품질이나 제품 범위에 대한 사후 해석을 섞지 않습니다.',
          'Raw 지표는 corpus task 자체의 성능을 보여 주며 adjusted 결과가 있어도 생략하지 않습니다.',
        ]),
        section('contract-adjusted matrix', [
          'Contract-adjusted는 실행 전에 고정한 disposition을 같은 예측에 적용합니다. 의미로 구분할 수 없는 동형이의, source가 지지하는 내부 성분과 gold span 오류는 재분류할 수 있습니다.',
          'Contract-adjusted confusion matrix는 raw 약어 오른쪽 위에 c를 붙인 TPᶜ·FPᶜ·TNᶜ·FNᶜ로 표기합니다. FNᶜ는 제품 계약 안의 false negative입니다.',
          '비표준 입력이 명시된 제품 입력 계약 밖이면 제외할 수 있습니다. 구현이 어렵거나 아직 지원하지 않는 문법은 제외 사유가 아닙니다.',
        ]),
        section('disposition', [
          'Raw FN 4와 FNᶜ 0은 실행 결과가 달라졌다는 뜻이 아닙니다. FN 4가 strict gold의 실패인 반면 registry가 네 case를 제품 목표 밖 또는 제품 계약상 positive로 판정했기 때문에 FNᶜ가 0입니다.',
          '모든 조정은 case ID, 이유와 review 상태를 ledger에 남깁니다. 미분류 case는 raw 판정을 유지해 adjusted 지표가 누락을 숨기지 않게 합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'BENCHMARKS · CONTRACT',
      title: 'Quality contract',
      summary:
        'Raw preserves corpus gold; contract-adjusted reevaluates the same predictions through a fixed product registry.',
      sections: [
        section('Raw matrix', [
          'Raw TP, FP, TN, and FN compare backend predictions directly with strict fixture gold. They contain no post-hoc interpretation of data quality or product scope.',
          'Raw metrics show the corpus task and remain visible even when an adjusted view exists.',
        ]),
        section('Contract-adjusted matrix', [
          'Contract-adjusted applies dispositions fixed before execution to the same predictions. Indistinguishable homographs, source-supported components, and gold-span errors may be reclassified.',
          'The contract-adjusted confusion matrix adds a superscript c to each raw abbreviation: TPᶜ, FPᶜ, TNᶜ, and FNᶜ. FNᶜ means an in-contract false negative.',
          'A nonstandard input may be excluded only when it is explicitly outside the product input contract. Difficult or unsupported grammar remains in the denominator.',
        ]),
        section('Dispositions', [
          'Four raw FNs and FNᶜ = 0 do not indicate a changed execution result. The four remain strict-gold failures, while the registry places them outside the product objective or recognizes them as positive under the product contract.',
          'Every adjustment records case ID, reason, and review status in a ledger. Unclassified cases retain raw labels so adjusted metrics cannot hide omissions.',
        ]),
      ],
    },
  },
  [RoutePath.BenchmarkCanonical]: {
    [DocumentLocale.Korean]: {
      eyebrow: '벤치마크 · 품질',
      title: '표준문 품질',
      summary:
        'Canonical은 표준 맞춤법과 explicit POS에서 기본 형태 coverage와 hard negative precision을 측정합니다.',
      sections: [
        section('dataset', [
          '양성 500개는 명사, 대명사, 수사, 동사, 형용사, 관형사와 부사의 목표 span을 가집니다. 음성 500개는 같은 표면이 다른 형태 기능으로 쓰이는 hard negative를 포함합니다.',
          '문장은 수동 검토한 표준문만 사용하고 오류 문장은 Robust로 분리합니다.',
        ]),
        section('지표', [
          'Backend별 raw confusion matrix와 precision, recall, F1을 기록합니다. Contract review가 없으므로 adjusted matrix는 raw와 같고 reviewed cases는 0입니다.',
          'D3 차트는 두 series를 모두 표시해 동일함을 숨기지 않습니다.',
        ]),
        section('해석 한계', [
          'Canonical F1은 표준문 lemma search 품질이며 파일 scan 처리량이나 오류 문장 robustness를 포함하지 않습니다.',
          '품사별 분모가 다르므로 작은 범주의 percent를 전체 결과와 단순 평균하지 않습니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'BENCHMARKS · QUALITY',
      title: 'Canonical quality',
      summary:
        'Canonical measures core morphology coverage and hard-negative precision on standard spelling with explicit POS.',
      sections: [
        section('Dataset', [
          'Five hundred positives cover noun, pronoun, numeral, verb, adjective, determiner, and adverb target spans. Five hundred negatives include identical surfaces serving other morphological functions.',
          'Only manually reviewed standard sentences are included; noisy sentences remain in Robust.',
        ]),
        section('Metrics', [
          'Each backend reports a raw confusion matrix plus precision, recall, and F1. No contract review applies, so adjusted matrices equal raw and reviewed cases equal zero.',
          'The D3 chart renders both series rather than hiding their equality.',
        ]),
        section('Interpretation limits', [
          'Canonical F1 measures lemma search on standard sentences. It includes neither file-scan throughput nor noisy-sentence robustness.',
          'POS strata have different denominators, so their percentages are not averaged directly into the overall result.',
        ]),
      ],
    },
  },
  [RoutePath.BenchmarkQueryMatrix]: {
    [DocumentLocale.Korean]: {
      eyebrow: '벤치마크 · 품질',
      title: 'query matrix',
      summary:
        'Query matrix는 한 source 문장의 여러 positive query와 같은 품사의 paired negative query를 case 단위로 추적하는 진단 fixture입니다.',
      sections: [
        section('조합 차원', [
          '한 source sentence에서 최대 세 개의 “있어야 하는” 표제어·품사·span 질의를 고르고, 각 질의마다 같은 품사의 “없어야 하는” 질의를 짝지어 여러 query case를 만듭니다.',
          '체언·용언 class, 조사·어미, 불규칙, compound, explicit POS와 boundary policy를 교차합니다.',
          '각 case는 expected match, expected no-match와 선택적 contract disposition을 가집니다.',
        ]),
        section('coverage', [
          'Evaluator는 backend가 반환한 candidate를 case에 정렬하고 partial span, crossing match와 extra match를 구분합니다. Candidate coverage가 100%가 아니면 matrix를 승인하지 않습니다.',
          'Aggregate F1과 함께 문법 차원별 TP·FP·FN을 남겨 특정 규칙의 누락을 찾습니다.',
          'Query matrix는 진단 workload이며 Canonical 회귀선과 합치거나 대체하지 않습니다.',
        ]),
        section('disposition ledger', [
          'Raw와 adjusted matrix는 같은 prediction set에서 계산합니다. Registry는 case ID별 confirmed, reclassified, excluded 이유를 version control에 보존합니다.',
          'FNᶜ 0은 분류 가능한 제품 목표 안 FN이 없다는 뜻입니다. Raw FN과 unclassified count는 별도로 계속 표시합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'BENCHMARKS · QUALITY',
      title: 'Query matrix',
      summary:
        'The query matrix is a diagnostic fixture that tracks multiple positive queries and same-POS paired negatives in one source sentence.',
      sections: [
        section('Dimensions', [
          'Up to three lemma-POS-span queries that should match are selected from one source sentence, and each is paired with a same-POS query that should not match.',
          'Nominal and predicate classes, particles, endings, irregulars, compounds, explicit POS, and boundary policies are crossed.',
          'Each case declares expected match, expected no match, and an optional contract disposition.',
        ]),
        section('Coverage', [
          'The evaluator aligns backend candidates to cases and distinguishes partial spans, crossings, and extra matches. Candidate coverage below 100% blocks approval.',
          'Aggregate F1 is accompanied by per-dimension TP, FP, and FN so a missing rule remains identifiable.',
          'The query matrix is a diagnostic workload and neither replaces nor merges with the Canonical regression baseline.',
        ]),
        section('Disposition ledger', [
          'Raw and adjusted matrices use the same predictions. The registry stores confirmed, reclassified, and excluded reasons by case ID in version control.',
          'FNᶜ = 0 means no classified in-contract false negative. Raw FN and unclassified counts remain visible.',
        ]),
      ],
    },
  },
  [RoutePath.BenchmarkRobustness]: {
    [DocumentLocale.Korean]: {
      eyebrow: '벤치마크 · 품질',
      title: '오류 문장 품질',
      summary:
        'Robust는 오류 위치가 목표 span인지 주변 문맥인지 분리해 비표준 입력에서의 검색 품질을 측정합니다.',
      sections: [
        section('오류 dataset', [
          '전체 500 case는 positive 250·negative 250입니다. 한글 typo, 띄어쓰기 분리, 비표준 통사와 외국어 text typo를 원문 그대로 보존합니다.',
          '표준문 gold와 별도 fixture이며 Canonical 점수에 합산하지 않습니다.',
        ]),
        section('오류 class', [
          'Positive 가운데 100건은 오류 표식이 gold token에 직접 걸린 target-span이고 150건은 다른 token에만 있는 context-only입니다. Negative 250건은 context-only 분모에 포함합니다.',
          '두 scope의 recall을 따로 보면 형태 core 손상과 주변 noise 내성을 구분할 수 있습니다.',
        ]),
        section('분리 보고', [
          '모든 backend에 raw와 adjusted confusion matrix를 기록합니다. Contract review가 없으므로 두 값은 같고 review 0건입니다.',
          'Robust 설정은 full error correction이 아닙니다. 각 backend가 실제로 받은 입력과 option을 source report에 보존합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'BENCHMARKS · QUALITY',
      title: 'Robustness quality',
      summary:
        'Robust separates errors on target spans from errors in surrounding context.',
      sections: [
        section('Noisy dataset', [
          'The 500 cases contain 250 positives and 250 negatives, preserving Hangul typos, spacing splits, nonstandard syntax, and foreign-text typos.',
          'It is a separate fixture and never contributes to Canonical scores.',
        ]),
        section('Error classes', [
          'Among positives, 100 target-span cases place the error on the gold token and 150 context-only cases place it elsewhere. The 250 negatives join the context-only denominator.',
          'Separate recall reveals damage to the morphology core versus tolerance of surrounding noise.',
        ]),
        section('Separate reporting', [
          'Every backend reports raw and adjusted matrices. With no contract review, the two are equal and review count is zero.',
          'Robust settings are not full error correction. Source reports preserve exact input and options for every backend.',
        ]),
      ],
    },
  },
  [RoutePath.BenchmarkComparisons]: {
    [DocumentLocale.Korean]: {
      eyebrow: '벤치마크 · 비교',
      title: '외부 제품 비교',
      summary:
        '외부 분석기는 형태소열을 kfind의 lemma-span task에 projection한 뒤 동일 matrix로 비교합니다.',
      sections: [
        section('출력 정렬', [
          'Kiwi, Lindera, MeCab-ko와 KOMORAN의 token·morpheme output을 source span과 lemma·POS 조건으로 정렬합니다. Offset 좌표계와 normalization 차이를 backend adapter에서 변환합니다.',
          '정렬 실패나 backend 오류를 no-match로 숨기지 않고 별도 실행 오류로 기록합니다.',
        ]),
        section('품질 비교', [
          '모든 제품 행은 raw와 contract-adjusted TP·FP·TN·FN, precision, recall과 F1을 포함합니다. Contract registry는 제품별로 다르게 적용하지 않습니다.',
          '외부 제품의 목적이 문장 분석이라는 점과 projection의 한계를 설명하되 불리한 결과를 생략하지 않습니다.',
        ]),
        section('비용 비교', [
          '초기화, case 처리량, p95와 peak RSS를 같은 process model에서 측정합니다. Python·JVM backend의 runtime 시작 비용도 실제 workflow 비용에 포함합니다.',
          '품질과 성능을 하나의 종합 점수로 합치지 않습니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'BENCHMARKS · COMPARISON',
      title: 'External comparisons',
      summary:
        'External morphology output is projected into the same lemma-span task before comparison.',
      sections: [
        section('Output alignment', [
          'Token and morpheme output from Kiwi, Lindera, MeCab-ko, and KOMORAN is aligned by source span, lemma, and POS. Backend adapters convert offset systems and normalization.',
          'Alignment failure and backend errors remain execution errors instead of becoming no-match predictions.',
        ]),
        section('Quality comparison', [
          'Every product row includes raw and adjusted TP, FP, TN, FN, precision, recall, and F1. The contract registry is not customized per product.',
          'Reports explain that external products target sentence analysis and disclose projection limits without omitting unfavorable results.',
        ]),
        section('Cost comparison', [
          'Initialization, cases per second, p95, and peak RSS use the same process model. Python and JVM runtime startup remains part of workflow cost.',
          'Quality and performance are never combined into one composite score.',
        ]),
      ],
    },
  },
  [RoutePath.BenchmarkPerformance]: {
    [DocumentLocale.Korean]: {
      eyebrow: '벤치마크 · 성능',
      title: '성능 측정',
      summary:
        '서로 다른 단위의 workload를 분리하고 baseline과 candidate를 같은 환경에서 비교합니다.',
      sections: [
        section('workload', [
          'Morphology는 fresh process initialization, cases/s, p95와 RSS를 측정합니다. Query compile, matcher scan, 1 GiB file scan, npm startup과 TUI index는 별도 entrypoint를 사용합니다.',
          'Lock 대기와 build 시간은 workload 시간에서 제외하지만 실행 환경과 binary revision에는 포함해 기록합니다.',
        ]),
        section('통계', [
          'Warm-up 1회 뒤 5회 측정하고 median, min, max를 보고합니다. 단발 실행이나 smoke success는 성능 근거가 아닙니다.',
          'Throughput, latency, seconds와 bytes를 같은 percent score로 합치지 않습니다.',
        ]),
        section('회귀 판정', [
          'Baseline과 candidate는 같은 toolchain, resource, input checksum과 option을 사용합니다. 품질 projection을 함께 실행해 빠르지만 잘못된 candidate를 채택하지 않습니다.',
          '불리한 증감과 noise 범위를 모두 보고하고 원인을 모르면 추정이라고 표시합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'BENCHMARKS · PERFORMANCE',
      title: 'Performance measurement',
      summary:
        'Separate workloads with different units and compare baseline and candidate in one environment.',
      sections: [
        section('Workloads', [
          'Morphology measures fresh-process initialization, cases per second, p95, and RSS. Query compile, matcher scan, 1 GiB file scan, npm startup, and TUI indexing have separate entrypoints.',
          'Lock wait and build time are excluded from workload time but remain part of recorded environment and revision.',
        ]),
        section('Statistics', [
          'One warm-up precedes five measurements; reports include median, minimum, and maximum. A single run or successful smoke test is not performance evidence.',
          'Throughput, latency, seconds, and bytes are not collapsed into one percentage score.',
        ]),
        section('Regression decision', [
          'Baseline and candidate use identical toolchains, resources, input checksums, and options. A quality projection prevents adoption of a faster but incorrect candidate.',
          'Unfavorable changes and noise ranges remain visible, and unknown causes are labeled as inference.',
        ]),
      ],
    },
  },
  [RoutePath.BenchmarkReproducibility]: {
    [DocumentLocale.Korean]: {
      eyebrow: '벤치마크 · 근거',
      title: '재현 방법',
      summary:
        'Source report는 같은 결과를 다시 만들 수 있는 revision, 입력과 실행 환경을 완결된 단위로 기록합니다.',
      sections: [
        section('provenance', [
          'Baseline·candidate 전체 Git revision, dirty state, OS·architecture, CPU, memory, Rust·Node·Python과 backend version을 기록합니다.',
          'Resource와 fixture는 path뿐 아니라 SHA-256과 logical case count를 포함합니다.',
        ]),
        section('실행 명령', [
          '공식 wrapper와 모든 environment override를 그대로 기록합니다. Morphology Python test는 `tools/morph-compare`에서 두 import root를 포함하는 discovery 명령을 사용합니다.',
          '직접 Docker나 cargo command로 lock·fixture 계약을 우회한 결과는 승인 report가 아닙니다.',
        ]),
        section('산출물 검증', [
          'JSON report, Markdown table과 site snapshot의 수치가 일치하는지 schema validator로 검사합니다. Chart는 snapshot field만 소비합니다.',
          'Source report revision과 SHA-256이 바뀌면 snapshot을 같은 변경에서 다시 생성합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'BENCHMARKS · EVIDENCE',
      title: 'Reproducibility',
      summary:
        'A source report records revisions, inputs, and environment as a complete recipe for recreating results.',
      sections: [
        section('Provenance', [
          'Record full baseline and candidate revisions, dirty state, OS, architecture, CPU, memory, and Rust, Node, Python, and backend versions.',
          'Resources and fixtures include SHA-256 and logical case counts, not just paths.',
        ]),
        section('Commands', [
          'Preserve official wrappers and every environment override. Morphology Python tests run from `tools/morph-compare` with discovery that includes both import roots.',
          'Results from direct Docker or cargo commands that bypass lock and fixture contracts are not approved reports.',
        ]),
        section('Artifact verification', [
          'Schema validation checks that JSON reports, Markdown tables, and site snapshots contain identical values. Charts consume only snapshot fields.',
          'A changed source-report revision or SHA-256 requires regenerating the snapshot in the same change.',
        ]),
      ],
    },
  },
  [RoutePath.BenchmarkReports]: {
    [DocumentLocale.Korean]: {
      eyebrow: '벤치마크 · 기록',
      title: '역사 보고서',
      summary:
        '날짜별 report는 실험과 변화량을 보존하고 현재 제품 문서는 승인된 결론만 가리킵니다.',
      sections: [
        section('보고서 구조', [
          '`docs/benchmarks`의 날짜별 Markdown은 목적, revision, 환경, 입력, 명령, raw result와 해석을 담습니다. Baseline·candidate 증감은 이 위치에만 기록합니다.',
          '품질 report는 case-level disposition과 미분류 수를 포함합니다.',
        ]),
        section('site snapshot', [
          'Site는 승인된 source report에서 chart에 필요한 field만 JSON snapshot으로 export합니다. Snapshot은 source revision과 report SHA-256을 보존합니다.',
          '현재 결과 route는 snapshot을 읽고 날짜별 report 목록을 본문으로 복사하지 않습니다.',
        ]),
        section('보존 계약', [
          '불리한 결과, 채택하지 않은 실험과 측정 noise도 report에 남깁니다. 제품 code에 실험 장치를 남길 필요는 없습니다.',
          '현재 README와 기술 문서는 이전 상태나 개선 서사를 포함하지 않고 현재 contract만 설명합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'BENCHMARKS · RECORDS',
      title: 'Historical reports',
      summary:
        'Dated reports preserve experiments and deltas; current product documentation points only to approved conclusions.',
      sections: [
        section('Report structure', [
          'Dated Markdown under `docs/benchmarks` contains purpose, revisions, environment, inputs, commands, raw results, and interpretation. Baseline-candidate deltas live only there.',
          'Quality reports include case-level dispositions and unclassified counts.',
        ]),
        section('Site snapshot', [
          'The site exports only chart-consumed fields from an approved source report. The snapshot retains source revision and report SHA-256.',
          'The current-results route reads that snapshot and does not copy dated report lists into the main narrative.',
        ]),
        section('Retention contract', [
          'Unfavorable results, rejected experiments, and measurement noise remain in reports. Experimental instrumentation need not stay in product code.',
          'Current README and technical documentation describe only the present contract, without prior-state or improvement narratives.',
        ]),
      ],
    },
  },
};
