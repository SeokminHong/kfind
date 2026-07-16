export enum GlossaryCategory {
  Search = 'search',
  Execution = 'execution',
  Resource = 'resource',
  Quality = 'quality',
}

export interface GlossaryTerm {
  readonly id: string;
  readonly category: GlossaryCategory;
  readonly name: string;
  readonly notation?: string;
  readonly definition: string;
  readonly aliases: readonly string[];
}

export const glossaryCategoryLabels: Readonly<
  Record<GlossaryCategory, string>
> = {
  [GlossaryCategory.Search]: '검색 입력',
  [GlossaryCategory.Execution]: '컴파일과 실행',
  [GlossaryCategory.Resource]: '데이터와 리소스',
  [GlossaryCategory.Quality]: '품질과 성능',
};

export const glossaryTerms: readonly GlossaryTerm[] = [
  {
    id: 'lemma',
    category: GlossaryCategory.Search,
    name: '표제어',
    notation: 'lemma',
    definition:
      '활용이나 조사를 제거해 사전에서 기본형으로 삼는 단어입니다. kfind 검색의 출발점입니다.',
    aliases: ['표제어', 'lemma'],
  },
  {
    id: 'part-of-speech',
    category: GlossaryCategory.Search,
    name: '품사',
    notation: 'POS · part of speech',
    definition: '명사, 동사처럼 단어의 문법적 역할을 나타내는 분류입니다.',
    aliases: ['part of speech', '품사', 'POS'],
  },
  {
    id: 'query',
    category: GlossaryCategory.Search,
    name: 'query',
    notation: '검색 질의',
    definition:
      '찾을 표제어, 품사 태그와 옵션을 포함해 kfind에 전달하는 검색 표현입니다.',
    aliases: ['query', '쿼리'],
  },
  {
    id: 'atom',
    category: GlossaryCategory.Search,
    name: 'atom',
    notation: '형태 검색 단위',
    definition:
      '한 표제어와 선택적 품사 태그로 이루어진 query의 최소 형태 검색 단위입니다.',
    aliases: ['atom'],
  },
  {
    id: 'phrase',
    category: GlossaryCategory.Search,
    name: 'phrase',
    notation: '구(句) 검색',
    definition:
      '둘 이상의 atom을 순서와 최대 간격 조건으로 결합한 query입니다.',
    aliases: ['phrase', '구(句)'],
  },
  {
    id: 'inflection',
    category: GlossaryCategory.Search,
    name: 'inflection',
    notation: '활용',
    definition:
      '품사를 바꾸지 않고 조사·어미 결합이나 불규칙 교체로 표면형을 만드는 확장입니다.',
    aliases: ['inflection', '활용'],
  },
  {
    id: 'derivation',
    category: GlossaryCategory.Search,
    name: 'derivation',
    notation: '파생',
    definition: '접미사를 붙여 새 품사의 표면형까지 넓히는 확장입니다.',
    aliases: ['derivation', '파생'],
  },
  {
    id: 'literal',
    category: GlossaryCategory.Search,
    name: 'literal',
    notation: '문자 그대로 검색',
    definition:
      '형태 확장이나 품사 해석 없이 입력 문자열 자체를 찾는 검색 모드입니다.',
    aliases: ['literal'],
  },
  {
    id: 'normalization',
    category: GlossaryCategory.Search,
    name: 'normalization',
    notation: 'Unicode 정규화',
    definition:
      '같은 문자로 보이는 서로 다른 Unicode 표현을 비교 가능한 형식으로 맞추는 과정입니다.',
    aliases: ['normalization', '정규화'],
  },
  {
    id: 'query-plan',
    category: GlossaryCategory.Execution,
    name: 'query plan',
    notation: '검색 계획',
    definition:
      'query를 실행할 수 있도록 candidate program, anchor와 제한 정보를 묶은 컴파일 결과입니다.',
    aliases: ['query plan', '검색 계획'],
  },
  {
    id: 'candidate-program',
    category: GlossaryCategory.Execution,
    name: 'CandidateProgram',
    notation: '검색 후보 프로그램',
    definition:
      '하나 이상의 분석에서 만든 실행 후보로, anchor, core 투영, 후보 범위, consumption, 판정 제약과 provenance를 보존합니다.',
    aliases: ['CandidateProgram', 'program'],
  },
  {
    id: 'anchor',
    category: GlossaryCategory.Execution,
    name: 'anchor',
    notation: '고정 검색 문자열',
    definition:
      '원문에서 후보 위치를 빠르게 찾기 위해 먼저 검색하는 고정 byte 문자열입니다.',
    aliases: ['anchor'],
  },
  {
    id: 'structural-constraint',
    category: GlossaryCategory.Execution,
    name: 'StructuralConstraint',
    notation: '국소 구조 제약',
    definition:
      'anchor 주변의 component와 인접 token 배치로 품사 구조를 판정하는 query 소유 제약입니다.',
    aliases: ['StructuralConstraint', '구조 제약'],
  },
  {
    id: 'boundary',
    category: GlossaryCategory.Execution,
    name: 'boundary',
    notation: '경계 정책',
    definition:
      '후보가 token 전체여야 하는지 또는 부분 문자열도 허용하는지 정하는 좌우 경계 정책입니다.',
    aliases: ['boundary', '경계'],
  },
  {
    id: 'span',
    category: GlossaryCategory.Execution,
    name: 'span',
    notation: '원문 위치 범위',
    definition: '원문에서 match가 차지하는 시작과 끝 위치의 범위입니다.',
    aliases: ['span'],
  },
  {
    id: 'provenance',
    category: GlossaryCategory.Execution,
    name: 'provenance',
    notation: '생성 근거',
    definition:
      '어떤 분석과 활용·파생 규칙으로 match가 만들어졌는지 나타내는 생성 근거입니다.',
    aliases: ['provenance', '생성 근거'],
  },
  {
    id: 'corpus',
    category: GlossaryCategory.Resource,
    name: 'corpus',
    notation: '검색 말뭉치',
    definition:
      '검색 대상이 되는 파일이나 메모리 text 전체를 가리키는 입력 집합입니다.',
    aliases: ['corpus', '코퍼스'],
  },
  {
    id: 'lexicon',
    category: GlossaryCategory.Resource,
    name: 'lexicon',
    notation: '사전 리소스',
    definition: '표제어의 품사와 형태 정보를 조회하는 사전 resource입니다.',
    aliases: ['lexicon'],
  },
  {
    id: 'component-resource',
    category: GlossaryCategory.Resource,
    name: 'component resource',
    notation: '구조 판정 리소스',
    definition:
      '후보 token의 source POS와 정렬된 component span을 제공하는 선택적 compact resource입니다.',
    aliases: ['component resource', 'component'],
  },
  {
    id: 'precision',
    category: GlossaryCategory.Quality,
    name: 'precision',
    notation: '정밀도',
    definition: '반환한 결과 중 실제 정답이 차지하는 비율입니다.',
    aliases: ['precision'],
  },
  {
    id: 'recall',
    category: GlossaryCategory.Quality,
    name: 'recall',
    notation: '재현율',
    definition: '찾아야 할 정답 중 실제로 반환한 비율입니다.',
    aliases: ['recall'],
  },
  {
    id: 'false-positive',
    category: GlossaryCategory.Quality,
    name: 'false positive',
    notation: 'FP · 오탐',
    definition: '정답이 아닌데 검색 결과로 반환된 항목이며 FP로 줄여 씁니다.',
    aliases: ['false positive', 'FP'],
  },
  {
    id: 'false-negative',
    category: GlossaryCategory.Quality,
    name: 'false negative',
    notation: 'FN · 누락',
    definition:
      '찾아야 하지만 검색 결과에서 누락된 항목이며 FN으로 줄여 씁니다.',
    aliases: ['false negative', 'FN'],
  },
  {
    id: 'workload',
    category: GlossaryCategory.Quality,
    name: 'workload',
    notation: '측정 작업 조건',
    definition:
      '성능이나 품질을 측정할 때 입력, 옵션과 실행 경로를 고정한 작업 조건입니다.',
    aliases: ['workload'],
  },
];
