import type { DocumentGroupIndex } from './types';

import { RoutePath } from '../route-path';

import { localized, page } from './types';

const executionPages = [
  page(
    RoutePath.Architecture,
    '아키텍처',
    'Architecture',
    'query compile, anchor scan, 국소 검증과 출력의 구성 요소 경계를 설명합니다.',
    'Trace the boundaries between query compilation, anchor scanning, local verification, and output.',
    [
      [
        'query-and-corpus-lanes',
        'query와 corpus 경로',
        'Query and corpus lanes',
      ],
      ['candidate-programs', '후보 program', 'Candidate programs'],
      ['local-verification', '국소 검증', 'Local verification'],
      ['phrase-spans', '구 span', 'Phrase spans'],
      ['parallel-output', '병렬 출력', 'Parallel output'],
      ['execution-surfaces', '실행 surface', 'Execution surfaces'],
    ],
  ),
  page(
    RoutePath.Pipeline,
    '실행 pipeline',
    'Execution pipeline',
    '질의 정규화부터 provenance 출력까지의 데이터 흐름을 단계별로 정의합니다.',
    'Follow data from query normalization through provenance-bearing output.',
    [
      ['compile', 'compile 단계', 'Compilation'],
      ['scan', 'scan 단계', 'Scanning'],
      ['verify', 'verify 단계', 'Verification'],
      ['emit', '출력 단계', 'Emission'],
    ],
  ),
  page(
    RoutePath.QueryCompiler,
    'query compiler',
    'Query compiler',
    '형태 분석을 제한된 CandidateProgram과 anchor 집합으로 낮추는 방식을 설명합니다.',
    'Lower morphological analyses into bounded CandidatePrograms and anchor sets.',
    [
      ['analysis', '분석 후보', 'Analysis candidates'],
      ['program-ir', 'program IR', 'Program IR'],
      ['deduplication', '정렬과 중복 제거', 'Ordering and deduplication'],
    ],
  ),
  page(
    RoutePath.Matcher,
    'matcher',
    'Matcher',
    'Aho-Corasick anchor scan과 program별 suffix·경계 검증을 설명합니다.',
    'Combine Aho-Corasick anchor scanning with per-program suffix and boundary verification.',
    [
      ['anchors', 'anchor index', 'Anchor index'],
      ['candidate-window', '후보 window', 'Candidate window'],
      ['match-selection', 'match 선택', 'Match selection'],
    ],
  ),
  page(
    RoutePath.StructuralVerification,
    '구조 판정',
    'Structural verification',
    'compact component graph가 source 품사열과 span 제약을 증명하는 방식을 설명합니다.',
    'Use the compact component graph to prove source POS sequences and span constraints.',
    [
      ['constraints', '구조 제약', 'Structural constraints'],
      ['graph-preparation', 'graph 준비', 'Graph preparation'],
      ['failure', '판정 실패', 'Verification failure'],
    ],
  ),
  page(
    RoutePath.InternalResources,
    'resource 구조',
    'Resource architecture',
    '사전과 compact artifact의 생성, schema, digest와 초기화 수명주기를 설명합니다.',
    'Describe generation, schema, digest validation, and initialization lifecycle for resources.',
    [
      ['resource-layers', 'resource 계층', 'Resource layers'],
      ['container', 'container 형식', 'Container format'],
      ['lifecycle', '초기화 수명주기', 'Initialization lifecycle'],
    ],
  ),
  page(
    RoutePath.UnicodeSpans,
    'Unicode와 span',
    'Unicode and spans',
    'NFC·NFD 검색과 UTF-8·UTF-16 좌표 변환의 불변식을 설명합니다.',
    'Preserve invariants across NFC/NFD matching and UTF-8/UTF-16 coordinate conversion.',
    [
      ['normalization', '정규화', 'Normalization'],
      ['coordinates', '좌표계', 'Coordinate systems'],
      ['source-spans', '원문 span', 'Source spans'],
    ],
  ),
  page(
    RoutePath.Optimization,
    '성능 구조',
    'Performance architecture',
    'compile, initialization과 scan 비용을 분리하고 workload별 최적화를 설명합니다.',
    'Separate compile, initialization, and scan costs before applying workload-specific optimization.',
    [
      ['cost-separation', '비용 분리', 'Cost separation'],
      ['anchors-and-matchers', 'anchor와 matcher', 'Anchors and matchers'],
      ['plan-limits', '계획 상한', 'Plan limits'],
      ['resource-initialization', 'resource 초기화', 'Resource initialization'],
      ['scan-path', 'scan 경로', 'Scan path'],
      ['metric-boundaries', '지표 경계', 'Metric boundaries'],
    ],
  ),
];

const morphologyPages = [
  page(
    RoutePath.Analysis,
    '형태 처리 개요',
    'Morphology overview',
    '표제어 정방향 생성과 source 국소 판정을 결합한 형태 처리 원리를 설명합니다.',
    'Combine forward lemma generation with local structural verification of source text.',
    [
      ['analysis-direction', '분석 방향', 'Analysis direction'],
      ['lexicon-layers', '사전 계층', 'Lexicon layers'],
      ['particles-and-allomorphs', '조사와 이형태', 'Particles and allomorphs'],
      ['endings', '어미와 선어말어미', 'Endings and prefinal endings'],
      [
        'irregulars-and-contractions',
        '불규칙과 축약',
        'Irregulars and contractions',
      ],
      [
        'derivation-and-compounds',
        '파생과 복합 구조',
        'Derivation and compounds',
      ],
      [
        'structural-verification',
        '국소 구조 판정',
        'Local structural verification',
      ],
    ],
  ),
  page(
    RoutePath.MorphPartsOfSpeech,
    '품사 체계',
    'Part-of-speech system',
    '세부 품사 태그와 공개 coarse POS의 포함 관계를 설명합니다.',
    'Map detailed POS tags into the public coarse-POS categories.',
    [
      ['tag-system', '세부 태그', 'Detailed tags'],
      ['coarse-mapping', 'coarse mapping', 'Coarse mapping'],
      ['ambiguity', '중의 분석', 'Ambiguous analyses'],
    ],
  ),
  page(
    RoutePath.Nominals,
    '체언',
    'Nominals',
    '명사·대명사·수사와 의존 명사의 검색 program을 설명합니다.',
    'Describe search programs for nouns, pronouns, numerals, and bound nouns.',
    [
      ['classes', '체언 분류', 'Nominal classes'],
      ['suffixes', '체언 suffix', 'Nominal suffixes'],
      ['numbers', '수사와 단위', 'Numerals and units'],
    ],
  ),
  page(
    RoutePath.Particles,
    '조사',
    'Particles',
    '격조사·보조사·접속조사의 연쇄와 받침 이형태를 설명합니다.',
    'Model particle chains and final-consonant allomorphy.',
    [
      ['classes', '조사 분류', 'Particle classes'],
      ['allomorphs', '이형태', 'Allomorphs'],
      ['chains', '조사 연쇄', 'Particle chains'],
    ],
  ),
  page(
    RoutePath.Predicates,
    '용언',
    'Predicates',
    '동사·형용사·지정사의 어간과 활용 class를 설명합니다.',
    'Represent stems and inflection classes for verbs, adjectives, and copulas.',
    [
      ['stems', '어간', 'Stems'],
      ['classes', '활용 class', 'Inflection classes'],
      ['copulas', '지정사', 'Copulas'],
    ],
  ),
  page(
    RoutePath.Endings,
    '어미',
    'Endings',
    '선어말어미와 어말어미의 순서·종결 조건을 예제로 설명합니다.',
    'Explain the order and closure rules of prefinal and final endings.',
    [
      ['prefinal', '선어말어미', 'Prefinal endings'],
      ['final', '어말어미', 'Final endings'],
      ['chains', '어미 연쇄', 'Ending chains'],
    ],
  ),
  page(
    RoutePath.Irregulars,
    '불규칙 활용',
    'Irregular inflection',
    'ㄷ·ㅂ·르·ㅅ·ㅎ·우 불규칙과 규칙형 공존을 설명합니다.',
    'Cover ㄷ, ㅂ, 르, ㅅ, ㅎ, and 우 irregulars plus regular alternatives.',
    [
      ['lexical-classes', '어휘 class', 'Lexical classes'],
      ['phonology', '음운 조건', 'Phonological conditions'],
      ['provenance', '규칙 근거', 'Rule provenance'],
    ],
  ),
  page(
    RoutePath.Derivation,
    '파생',
    'Derivation',
    '접두사·접미사와 명사-용언 전환의 제한된 생성 범위를 설명합니다.',
    'Bound productive prefix, suffix, and noun-to-predicate derivation.',
    [
      ['modes', '파생 mode', 'Derivation mode'],
      ['suffixes', '파생 접미사', 'Derivational suffixes'],
      ['constraints', '생성 제약', 'Generation constraints'],
    ],
  ),
  page(
    RoutePath.Compounds,
    '합성어와 보조용언',
    'Compounds and auxiliaries',
    'source component와 연결 어미를 이용한 내부 성분 검색을 설명합니다.',
    'Search internal components using source analyses and connective endings.',
    [
      ['nominal-compounds', '합성명사', 'Nominal compounds'],
      ['predicate-compounds', '합성용언', 'Predicate compounds'],
      ['auxiliaries', '보조용언', 'Auxiliaries'],
    ],
  ),
  page(
    RoutePath.Contractions,
    '축약과 영형태',
    'Contractions and zero surfaces',
    '한글 음절 재조합, 축약 surface와 표면이 없는 문법 성분을 설명합니다.',
    'Handle Hangul recomposition, contracted surfaces, and grammatical zero surfaces.',
    [
      ['contractions', '축약', 'Contractions'],
      ['zero-surfaces', '영형태', 'Zero surfaces'],
      ['spans', 'span 보존', 'Span preservation'],
    ],
  ),
  page(
    RoutePath.Ambiguity,
    '중의성과 경계',
    'Ambiguity and boundaries',
    '여러 형태 분석을 보존하면서 구조로 거부 가능한 후보를 분리합니다.',
    'Preserve genuine ambiguity while rejecting candidates disproved by structure.',
    [
      ['analysis-sets', '분석 집합', 'Analysis sets'],
      ['structural-rejection', '구조 거부', 'Structural rejection'],
      ['semantic-limits', '의미 한계', 'Semantic limits'],
    ],
  ),
  page(
    RoutePath.Coverage,
    '문법 범위',
    'Grammar coverage',
    '현재 규칙 data가 보장하는 문법 요소와 제품 범위 밖 사례를 정의합니다.',
    'Define grammar guaranteed by rule data and cases outside the product contract.',
    [
      ['supported', '지원 범위', 'Supported scope'],
      ['bounded-rules', '제한 규칙', 'Bounded rules'],
      ['non-goals', '비목표', 'Non-goals'],
    ],
  ),
];

export const internalsGroup: DocumentGroupIndex = {
  labelKey: 'navigation.primary.internals',
  categories: [
    { label: localized('실행 구조', 'EXECUTION'), pages: executionPages },
    {
      label: localized('한국어 형태', 'KOREAN MORPHOLOGY'),
      pages: morphologyPages,
    },
  ],
};
