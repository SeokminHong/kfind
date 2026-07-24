import { DocumentLocale } from '../app/i18n';

import { getMorphemeGlossaryEntries } from './glossary-morphemes';

export enum GlossaryCategory {
  Search = 'search',
  Grammar = 'grammar',
  Morpheme = 'morpheme',
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
  readonly example?: string;
  readonly aliases: readonly string[];
}

interface GlossaryContent {
  readonly categoryLabels: Readonly<Record<GlossaryCategory, string>>;
  readonly eyebrow: string;
  readonly overview: string;
  readonly title: string;
  readonly terms: readonly GlossaryTerm[];
}

const koreanTerms: readonly GlossaryTerm[] = [
  {
    id: 'lemma',
    category: GlossaryCategory.Search,
    name: '표제어',
    notation: 'lemma',
    definition:
      '활용이나 조사가 붙지 않은 사전의 기본형입니다. kfind는 표제어를 검색 가능한 표면형 집합으로 컴파일합니다.',
    example: '걷다의 표면형에는 걷고, 걸어, 걸었다가 있습니다.',
    aliases: ['표제어', 'lemma'],
  },
  {
    id: 'part-of-speech',
    category: GlossaryCategory.Search,
    name: '품사',
    notation: 'POS · part of speech',
    definition:
      '명사, 동사, 형용사처럼 단어의 문법적 역할과 결합 규칙을 나타내는 분류입니다.',
    example: '먹다는 동사, 맛있다는 형용사, 음식은 명사입니다.',
    aliases: ['part of speech', '품사', 'POS'],
  },
  {
    id: 'query',
    category: GlossaryCategory.Search,
    name: '검색 질의',
    notation: 'query',
    definition:
      '표제어, 선택적 품사 태그, 구 구조와 옵션을 묶어 kfind에 전달하는 검색 표현입니다.',
    example:
      'n:사용자 v:검증하다는 명사와 동사를 순서대로 찾는 검색 질의입니다.',
    aliases: ['query', '검색 질의'],
  },
  {
    id: 'atom',
    category: GlossaryCategory.Search,
    name: '형태 검색 단위',
    notation: 'atom',
    definition:
      '한 표제어와 선택적 품사 태그로 이루어진 검색 질의의 최소 단위입니다.',
    example: 'n:사용자와 v:검증하다는 각각 하나의 atom입니다.',
    aliases: ['atom', '형태 검색 단위'],
  },
  {
    id: 'phrase',
    category: GlossaryCategory.Search,
    name: '구 검색',
    notation: 'phrase',
    definition:
      '둘 이상의 atom을 순서와 최대 간격 조건으로 결합한 검색 질의입니다.',
    example:
      'n:사용자 v:검증하다는 사용자와 검증하다의 활용형을 정해진 순서로 찾습니다.',
    aliases: ['phrase', '구 검색'],
  },
  {
    id: 'inflection',
    category: GlossaryCategory.Search,
    name: '활용',
    notation: 'inflection',
    definition:
      '품사를 바꾸지 않고 조사나 어미를 결합하거나 어간을 교체해 표면형을 만드는 과정입니다.',
    example: '걷다 → 걷고, 걸어, 걸었다',
    aliases: ['inflection', '활용'],
  },
  {
    id: 'derivation',
    category: GlossaryCategory.Search,
    name: '파생',
    notation: 'derivation',
    definition:
      '접사를 결합해 새 표제어나 다른 품사의 단어를 만드는 과정입니다.',
    example: '검증 → 검증하다, 문화 → 문화적',
    aliases: ['derivation', '파생'],
  },
  {
    id: 'particle',
    category: GlossaryCategory.Grammar,
    name: '조사',
    notation: 'J* · particle',
    definition:
      '체언 뒤에 붙어 문장 안의 관계나 의미를 나타내는 문법 요소입니다. 주격, 목적격, 보조사 등이 있습니다.',
    example: '사람이의 이/JKS, 책을의 을/JKO, 나도에서 도/JX',
    aliases: ['particle', '조사'],
  },
  {
    id: 'ending',
    category: GlossaryCategory.Grammar,
    name: '어미',
    notation: 'E* · ending',
    definition:
      '용언의 어간 뒤에 붙어 시제, 양태, 문장 종결, 다른 절과의 연결을 나타내는 문법 요소입니다.',
    example: '먹었다 = 먹/VV + 었/EP + 다/EF',
    aliases: ['ending', '어미'],
  },
  {
    id: 'prefinal-ending',
    category: GlossaryCategory.Grammar,
    name: '선어말어미',
    notation: 'EP · prefinal ending',
    definition:
      '어간과 종결·연결·전성 어미 사이에 놓여 높임, 시제, 추측 같은 문법 의미를 더하는 어미입니다. 둘 이상 이어질 수 있습니다.',
    example: '먹었겠지만 = 먹/VV + 었/EP + 겠/EP + 지만/EC',
    aliases: ['prefinal ending', '선어말어미'],
  },
  {
    id: 'final-ending',
    category: GlossaryCategory.Grammar,
    name: '종결어미',
    notation: 'EF · final ending',
    definition:
      '문장을 끝내면서 서술, 질문, 명령 같은 문장 유형을 나타내는 어미입니다.',
    example: '먹습니다의 습니다/EF, 먹니의 니/EF',
    aliases: ['final ending', '종결어미'],
  },
  {
    id: 'connective-ending',
    category: GlossaryCategory.Grammar,
    name: '연결어미',
    notation: 'EC · connective ending',
    definition:
      '용언과 뒤 절을 연결해 나열, 원인, 대조, 조건 같은 관계를 나타내는 어미입니다.',
    example: '먹고의 고/EC, 먹지만의 지만/EC, 먹으면의 으면/EC',
    aliases: ['connective ending', '연결어미'],
  },
  {
    id: 'adnominal-ending',
    category: GlossaryCategory.Grammar,
    name: '관형사형 전성어미',
    notation: 'ETM · adnominal ending',
    definition: '용언이 뒤의 체언을 꾸미도록 관형어 기능을 만드는 어미입니다.',
    example: '먹는 사람의 는/ETM, 먹을 음식의 을/ETM',
    aliases: ['adnominal ending', '관형사형 전성어미'],
  },
  {
    id: 'nominal-ending',
    category: GlossaryCategory.Grammar,
    name: '명사형 전성어미',
    notation: 'ETN · nominal ending',
    definition: '용언이 문장에서 명사처럼 쓰이도록 명사형을 만드는 어미입니다.',
    example: '먹기의 기/ETN, 믿음의 음/ETN',
    aliases: ['nominal ending', '명사형 전성어미'],
  },
  {
    id: 'allomorph',
    category: GlossaryCategory.Grammar,
    name: '이형태',
    notation: 'allomorph',
    definition:
      '같은 문법 기능을 가지지만 앞말의 음운 조건에 따라 다른 형태로 나타나는 요소입니다.',
    example: '받침 뒤의 은·이·을과 모음 뒤의 는·가·를',
    aliases: ['allomorph', '이형태'],
  },
  {
    id: 'irregular-conjugation',
    category: GlossaryCategory.Grammar,
    name: '불규칙 활용',
    notation: 'irregular conjugation',
    definition:
      '어미가 결합할 때 어간이나 어미가 일반 규칙과 다른 모양으로 바뀌는 활용입니다.',
    example: '걷다 → 걸어(ㄷ 불규칙), 돕다 → 도와(ㅂ 불규칙)',
    aliases: ['irregular conjugation', '불규칙 활용'],
  },
  {
    id: 'query-plan',
    category: GlossaryCategory.Execution,
    name: '검색 계획',
    notation: 'query plan',
    definition:
      '검색 질의를 원문에서 실행할 수 있도록 후보 프로그램, anchor와 검증 조건을 묶은 컴파일 결과입니다.',
    aliases: ['query plan', '검색 계획'],
  },
  {
    id: 'candidate-program',
    category: GlossaryCategory.Execution,
    name: '후보 프로그램',
    notation: 'CandidateProgram',
    definition:
      '고정 anchor, core 투영, suffix 소비 상태, 판정 제약과 생성 근거를 보존하는 실행 단위입니다.',
    aliases: ['CandidateProgram', '후보 프로그램'],
  },
  {
    id: 'anchor',
    category: GlossaryCategory.Execution,
    name: '고정 검색 문자열',
    notation: 'anchor',
    definition:
      '원문에서 형태 후보의 위치를 빠르게 찾기 위해 먼저 검색하는 고정 byte 문자열입니다.',
    aliases: ['anchor', '고정 검색 문자열'],
  },
  {
    id: 'boundary',
    category: GlossaryCategory.Execution,
    name: '경계 정책',
    notation: 'boundary',
    definition:
      '후보가 독립된 token이어야 하는지, 형태 구조로 확인된 부분이어야 하는지, 부분 문자열도 허용하는지 정하는 조건입니다.',
    aliases: ['boundary', '경계 정책'],
  },
  {
    id: 'provenance',
    category: GlossaryCategory.Execution,
    name: '생성 근거',
    notation: 'provenance',
    definition:
      '일치가 어떤 품사 분석과 활용·파생 규칙에서 만들어졌는지 나타내는 설명 정보입니다.',
    aliases: ['provenance', '생성 근거'],
  },
  {
    id: 'lexicon',
    category: GlossaryCategory.Resource,
    name: '형태 사전',
    notation: 'lexicon',
    definition:
      '표제어의 품사, 활용 유형과 형태 정보를 검색 질의 컴파일에 제공하는 리소스입니다.',
    aliases: ['lexicon', '형태 사전'],
  },
  {
    id: 'component-resource',
    category: GlossaryCategory.Resource,
    name: '구성 요소 리소스',
    notation: 'component resource',
    definition:
      '후보 token의 세부 품사와 정렬된 형태소 span을 제공해 국소 구조 판정에 사용하는 리소스입니다.',
    aliases: ['component resource', '구성 요소 리소스'],
  },
  {
    id: 'true-positive',
    category: GlossaryCategory.Quality,
    name: '참양성',
    notation: 'TP · true positive',
    definition: '정답인 항목을 검색 결과로 반환한 수입니다.',
    aliases: ['true positive', '참양성', 'TP'],
  },
  {
    id: 'true-negative',
    category: GlossaryCategory.Quality,
    name: '참음성',
    notation: 'TN · true negative',
    definition: '정답이 아닌 항목을 검색 결과에서 제외한 수입니다.',
    aliases: ['true negative', '참음성', 'TN'],
  },
  {
    id: 'false-positive',
    category: GlossaryCategory.Quality,
    name: '거짓양성',
    notation: 'FP · false positive',
    definition: '정답이 아닌 항목을 검색 결과로 반환한 수입니다.',
    aliases: ['false positive', '거짓양성', '오탐', 'FP'],
  },
  {
    id: 'false-negative',
    category: GlossaryCategory.Quality,
    name: '거짓음성',
    notation: 'FN · false negative',
    definition: '정답인 항목을 검색 결과에서 누락한 수입니다.',
    aliases: ['false negative', '거짓음성', 'FN'],
  },
  {
    id: 'precision',
    category: GlossaryCategory.Quality,
    name: '정밀도',
    notation: 'Precision = TP / (TP + FP)',
    definition:
      '검색 결과 중 실제 정답의 비율입니다. 결과를 얼마나 정확하게 골랐는지 나타냅니다. 분모가 0이면 이 문서의 벤치마크에서는 0으로 기록합니다.',
    example: 'TP 8, FP 2이면 정밀도는 8 / 10 = 0.8입니다.',
    aliases: ['precision', '정밀도'],
  },
  {
    id: 'recall',
    category: GlossaryCategory.Quality,
    name: '재현율',
    notation: 'Recall = TP / (TP + FN)',
    definition:
      '찾아야 할 정답 중 실제로 반환한 비율입니다. 정답을 얼마나 빠짐없이 찾았는지 나타냅니다. 분모가 0이면 이 문서의 벤치마크에서는 0으로 기록합니다.',
    example: 'TP 8, FN 2이면 재현율은 8 / 10 = 0.8입니다.',
    aliases: ['recall', '재현율'],
  },
  {
    id: 'f1-score',
    category: GlossaryCategory.Quality,
    name: 'F1 점수',
    notation: 'F1 = 2PR / (P + R)',
    definition:
      '정밀도와 재현율의 조화 평균입니다. 한쪽이 낮으면 함께 낮아지므로 두 지표의 균형을 확인하는 데 사용합니다. 두 값의 합이 0이면 0으로 기록합니다.',
    example: '정밀도 0.8, 재현율 0.8이면 F1은 0.8입니다.',
    aliases: ['F1 score', 'F1 점수', 'F1'],
  },
  {
    id: 'raw-metric',
    category: GlossaryCategory.Quality,
    name: '원시 지표',
    notation: 'raw metric',
    definition:
      'fixture가 선언한 모든 기대값을 제품 계약과 관계없이 그대로 TP, TN, FP, FN에 반영한 품질 지표입니다.',
    example: 'FN 4에는 제품 목표 밖 사례도 포함될 수 있습니다.',
    aliases: ['raw metric', '원시 지표'],
  },
  {
    id: 'contract-adjusted-metric',
    category: GlossaryCategory.Quality,
    name: '계약 조정 지표',
    notation: 'contract-adjusted metric · TPᶜ/TNᶜ/FPᶜ/FNᶜ',
    definition:
      '사람이 검토해 제품 계약 안으로 판정한 사례만 TPᶜ, TNᶜ, FPᶜ, FNᶜ에 반영한 품질 지표입니다. 검토되지 않은 fixture에서는 원시 지표와 같은 confusion matrix를 사용하며 reviewed 수는 0입니다.',
    example:
      'raw FN 4, FNᶜ 0이면 네 누락은 관측되었지만 모두 제품 목표 밖이며 계약 안의 누락은 없습니다.',
    aliases: [
      'contract-adjusted metric',
      '계약 조정 지표',
      'contract-adjusted',
      'TPᶜ',
      'TNᶜ',
      'FPᶜ',
      'FNᶜ',
      'TPc',
      'TNc',
      'FPc',
      'FNc',
    ],
  },
  {
    id: 'latency',
    category: GlossaryCategory.Quality,
    name: '지연 시간',
    notation: 'latency · p50/p95',
    definition:
      '한 작업을 끝내는 데 걸린 시간입니다. p50은 중앙값, p95는 관측값의 95%가 그 이하인 값을 뜻합니다.',
    aliases: ['latency', '지연 시간', 'p50', 'p95'],
  },
  {
    id: 'throughput',
    category: GlossaryCategory.Quality,
    name: '처리량',
    notation: 'throughput · cases/s · MiB/s',
    definition:
      '단위 시간에 처리한 작업량입니다. fixture 품질 경로는 cases/s, corpus 검색은 MiB/s처럼 workload에 맞는 단위를 사용합니다.',
    aliases: ['throughput', '처리량', 'cases/s', 'MiB/s'],
  },
];

const localizedKoreanTerms: readonly GlossaryTerm[] = [
  ...koreanTerms,
  ...getMorphemeGlossaryEntries(DocumentLocale.Korean).map((term) => ({
    ...term,
    category: GlossaryCategory.Morpheme,
  })),
];

const englishTerms: readonly GlossaryTerm[] = [
  ...koreanTerms,
  ...getMorphemeGlossaryEntries(DocumentLocale.English).map((term) => ({
    ...term,
    category: GlossaryCategory.Morpheme,
  })),
];

const englishOverrides: Readonly<Record<string, Partial<GlossaryTerm>>> = {
  lemma: {
    name: 'Lemma',
    definition:
      'The dictionary form without inflectional endings or particles. kfind compiles a lemma into searchable surface forms.',
    example: '걷다 has surfaces such as 걷고, 걸어, and 걸었다.',
  },
  'part-of-speech': {
    name: 'Part of speech',
    definition:
      'A grammatical class, such as noun, verb, or adjective, that determines a word’s role and combination rules.',
    example: '먹다 is a verb, 맛있다 is an adjective, and 음식 is a noun.',
  },
  query: {
    name: 'Query',
    definition:
      'A search expression containing lemmas, optional POS tags, phrase structure, and options.',
  },
  atom: {
    name: 'Atom',
    definition: 'The smallest query unit: one lemma and an optional POS tag.',
  },
  phrase: {
    name: 'Phrase',
    definition:
      'A query that joins two or more atoms under ordering and maximum-gap constraints.',
  },
  inflection: {
    name: 'Inflection',
    definition:
      'The formation of surface forms through particles, endings, or stem alternations without changing the part of speech.',
  },
  derivation: {
    name: 'Derivation',
    definition:
      'The formation of a new lemma or part of speech by attaching a derivational affix.',
  },
  particle: {
    name: 'Particle',
    definition:
      'A grammatical element attached to a nominal to mark relations or meaning, including case particles and auxiliary particles.',
  },
  ending: {
    name: 'Ending',
    definition:
      'A grammatical element attached to a predicate stem to express tense, modality, clause linkage, or sentence ending.',
  },
  'prefinal-ending': {
    name: 'Prefinal ending',
    definition:
      'An ending between the stem and a final, connective, or transformative ending. It adds honorific, tense, or modal meaning, and multiple prefinal endings may be chained.',
  },
  'final-ending': {
    name: 'Final ending',
    definition:
      'An ending that closes a sentence and marks a statement, question, command, or another sentence type.',
  },
  'connective-ending': {
    name: 'Connective ending',
    definition:
      'An ending that links a predicate to another clause and marks relations such as sequence, cause, contrast, or condition.',
  },
  'adnominal-ending': {
    name: 'Adnominal ending',
    definition:
      'An ending that turns a predicate into a modifier of a following nominal.',
  },
  'nominal-ending': {
    name: 'Nominal ending',
    definition:
      'An ending that lets a predicate function as a nominal in a sentence.',
  },
  allomorph: {
    name: 'Allomorph',
    definition:
      'One of multiple forms with the same grammatical function, selected by the phonological context.',
  },
  'irregular-conjugation': {
    name: 'Irregular conjugation',
    definition:
      'An inflection in which the stem or ending changes outside the regular combination pattern.',
  },
  'query-plan': {
    name: 'Query plan',
    definition:
      'The compiled representation that combines candidate programs, anchors, and verification conditions.',
  },
  'candidate-program': {
    name: 'Candidate program',
    definition:
      'An execution unit preserving a fixed anchor, core projection, suffix-consumption state, decision constraints, and provenance.',
  },
  anchor: {
    name: 'Anchor',
    definition:
      'A fixed byte string searched first to locate possible morphological matches in the corpus.',
  },
  boundary: {
    name: 'Boundary policy',
    definition:
      'A condition deciding whether a candidate must be a token, a structurally verified component, or an unrestricted substring.',
  },
  provenance: {
    name: 'Provenance',
    definition:
      'Explanation data identifying the POS analysis and inflectional or derivational rules that produced a match.',
  },
  lexicon: {
    name: 'Morphological lexicon',
    definition:
      'A resource supplying lemma POS, conjugation class, and morphological information during query compilation.',
  },
  'component-resource': {
    name: 'Component resource',
    definition:
      'A resource supplying fine POS and aligned morpheme spans for local structural verification of a candidate token.',
  },
  'true-positive': {
    name: 'True positive',
    definition: 'A correct item returned by the search.',
  },
  'true-negative': {
    name: 'True negative',
    definition: 'An incorrect item excluded from the search results.',
  },
  'false-positive': {
    name: 'False positive',
    definition: 'An incorrect item returned by the search.',
  },
  'false-negative': {
    name: 'False negative',
    definition: 'A correct item omitted from the search results.',
  },
  precision: {
    name: 'Precision',
    definition:
      'The fraction of returned results that are correct. The benchmark records zero when the denominator is zero.',
    example: 'With TP 8 and FP 2, precision is 8 / 10 = 0.8.',
  },
  recall: {
    name: 'Recall',
    definition:
      'The fraction of expected results that were returned. The benchmark records zero when the denominator is zero.',
    example: 'With TP 8 and FN 2, recall is 8 / 10 = 0.8.',
  },
  'f1-score': {
    name: 'F1 score',
    definition:
      'The harmonic mean of precision and recall. It falls when either input falls, and is recorded as zero when their sum is zero.',
  },
  'raw-metric': {
    name: 'Raw metric',
    definition:
      'A quality metric that counts every fixture expectation in TP, TN, FP, and FN, regardless of the product contract.',
    example: 'Raw FN 4 may include cases outside the product contract.',
  },
  'contract-adjusted-metric': {
    name: 'Contract-adjusted metric',
    definition:
      'A quality metric that counts only human-reviewed in-contract cases in TPᶜ, TNᶜ, FPᶜ, and FNᶜ. A fixture without review uses the raw confusion matrix and reports reviewed as zero.',
    example:
      'Raw FN 4 with FNᶜ 0 means four misses were observed, all outside the contract, and no in-contract item was missed.',
  },
  latency: {
    name: 'Latency',
    definition:
      'Time to complete one operation. p50 is the median, while p95 is the value at or below which 95% of observations fall.',
  },
  throughput: {
    name: 'Throughput',
    definition:
      'Work completed per unit time. Units remain workload-specific, such as cases/s for fixture evaluation and MiB/s for corpus scanning.',
  },
};

const localizedEnglishTerms = englishTerms.map((term) => ({
  ...term,
  ...englishOverrides[term.id],
}));

export const glossaryContent: Readonly<
  Record<DocumentLocale, GlossaryContent>
> = {
  [DocumentLocale.Korean]: {
    eyebrow: '참조 · 단어장',
    title: '검색·문법·측정 용어',
    overview:
      'kfind 문서와 벤치마크에서 사용하는 검색 구조, 한국어 문법, 품질 지표와 성능 단위를 정의합니다.',
    categoryLabels: {
      [GlossaryCategory.Search]: '검색 입력',
      [GlossaryCategory.Grammar]: '한국어 문법',
      [GlossaryCategory.Morpheme]: '형태소 레이블',
      [GlossaryCategory.Execution]: '컴파일과 실행',
      [GlossaryCategory.Resource]: '데이터와 리소스',
      [GlossaryCategory.Quality]: '품질과 성능',
    },
    terms: localizedKoreanTerms,
  },
  [DocumentLocale.English]: {
    eyebrow: 'REFERENCE · GLOSSARY',
    title: 'Search, grammar, and measurement terms',
    overview:
      'Definitions for the search structures, Korean grammar, quality metrics, and performance units used throughout the kfind documentation.',
    categoryLabels: {
      [GlossaryCategory.Search]: 'Search input',
      [GlossaryCategory.Grammar]: 'Korean grammar',
      [GlossaryCategory.Morpheme]: 'Morpheme labels',
      [GlossaryCategory.Execution]: 'Compilation and execution',
      [GlossaryCategory.Resource]: 'Data and resources',
      [GlossaryCategory.Quality]: 'Quality and performance',
    },
    terms: localizedEnglishTerms,
  },
};

export function getGlossaryContent(locale: DocumentLocale): GlossaryContent {
  return glossaryContent[locale];
}
