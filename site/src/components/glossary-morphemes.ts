import { DocumentLocale } from '../app/i18n';

export interface MorphemeGlossaryEntry {
  readonly aliases: readonly string[];
  readonly definition: string;
  readonly example?: string;
  readonly id: string;
  readonly name: string;
  readonly notation: string;
}

interface MorphemeLabel {
  readonly example: string;
  readonly label: string;
  readonly name: Readonly<Record<DocumentLocale, string>>;
  readonly definition: Readonly<Record<DocumentLocale, string>>;
}

const morphemeLabels: readonly MorphemeLabel[] = [
  {
    label: 'NNG',
    name: { ko: '일반 명사', en: 'General noun' },
    definition: {
      ko: '사람, 사물, 개념의 일반적인 이름을 나타내는 체언입니다.',
      en: 'A nominal that names a general person, object, or concept.',
    },
    example: '학교/NNG',
  },
  {
    label: 'NNP',
    name: { ko: '고유 명사', en: 'Proper noun' },
    definition: {
      ko: '특정 인물, 장소, 기관과 같은 고유한 대상을 가리키는 체언입니다.',
      en: 'A nominal naming a specific person, place, organization, or other unique entity.',
    },
    example: '서울/NNP',
  },
  {
    label: 'NNB',
    name: { ko: '의존 명사', en: 'Dependent noun' },
    definition: {
      ko: '앞말의 수식이 있어야 자연스럽게 쓰이는 명사입니다.',
      en: 'A noun that normally requires a preceding modifier.',
    },
    example: '할 수의 수/NNB',
  },
  {
    label: 'NNBC',
    name: { ko: '단위성 의존 명사', en: 'Counter noun' },
    definition: {
      ko: '수량 표현 뒤에서 단위나 횟수를 나타내는 의존 명사입니다.',
      en: 'A dependent noun used as a counter or measurement unit after a quantity.',
    },
    example: '세 명의 명/NNBC',
  },
  {
    label: 'NP',
    name: { ko: '대명사', en: 'Pronoun' },
    definition: {
      ko: '사람이나 사물의 이름을 대신하는 체언입니다.',
      en: 'A nominal that stands in place of a person or thing.',
    },
    example: '나/NP',
  },
  {
    label: 'NR',
    name: { ko: '수사', en: 'Numeral' },
    definition: {
      ko: '수량이나 순서를 나타내는 체언입니다.',
      en: 'A nominal expressing quantity or order.',
    },
    example: '셋/NR',
  },
  {
    label: 'VV',
    name: { ko: '동사', en: 'Verb' },
    definition: {
      ko: '동작이나 변화를 나타내며 어미와 결합해 활용하는 용언입니다.',
      en: 'A predicate expressing an action or change and inflecting with endings.',
    },
    example: '먹/VV',
  },
  {
    label: 'VA',
    name: { ko: '형용사', en: 'Adjective' },
    definition: {
      ko: '상태나 성질을 나타내며 어미와 결합해 활용하는 용언입니다.',
      en: 'A predicate expressing a state or property and inflecting with endings.',
    },
    example: '맑/VA',
  },
  {
    label: 'VX',
    name: { ko: '보조 용언', en: 'Auxiliary predicate' },
    definition: {
      ko: '앞 용언 뒤에서 양태나 진행, 결과 같은 의미를 더하는 보조 동사·형용사입니다.',
      en: 'An auxiliary verb or adjective adding aspect, modality, or result meaning after another predicate.',
    },
    example: '먹어 보았다의 보/VX',
  },
  {
    label: 'VCP',
    name: { ko: '긍정 지정사', en: 'Positive copula' },
    definition: {
      ko: '체언을 서술어로 만드는 긍정 지정사 이다의 형태입니다.',
      en: 'A form of the positive copula 이다 that turns a nominal into a predicate.',
    },
    example: '학생이었다의 이/VCP',
  },
  {
    label: 'VCN',
    name: { ko: '부정 지정사', en: 'Negative copula' },
    definition: {
      ko: '체언을 부정하는 지정사 아니다의 형태입니다.',
      en: 'A form of the negative copula 아니다.',
    },
    example: '학생이 아니다의 아니/VCN',
  },
  {
    label: 'JKS',
    name: { ko: '주격 조사', en: 'Subject case particle' },
    definition: {
      ko: '체언이 문장의 주어임을 나타내는 격조사입니다.',
      en: 'A case particle marking a nominal as the subject.',
    },
    example: '사람이의 이/JKS',
  },
  {
    label: 'JKC',
    name: { ko: '보격 조사', en: 'Complement case particle' },
    definition: {
      ko: '되다·아니다 앞의 체언이 보어임을 나타내는 격조사입니다.',
      en: 'A case particle marking a complement before predicates such as 되다 or 아니다.',
    },
    example: '학생이 되다의 이/JKC',
  },
  {
    label: 'JKG',
    name: { ko: '관형격 조사', en: 'Adnominal case particle' },
    definition: {
      ko: '앞 체언이 뒤 체언을 꾸미는 관계임을 나타내는 격조사입니다.',
      en: 'A case particle marking an adnominal relation to a following nominal.',
    },
    example: '우리의 집의 의/JKG',
  },
  {
    label: 'JKO',
    name: { ko: '목적격 조사', en: 'Object case particle' },
    definition: {
      ko: '체언이 서술어의 목적어임을 나타내는 격조사입니다.',
      en: 'A case particle marking a nominal as the object.',
    },
    example: '책을의 을/JKO',
  },
  {
    label: 'JKB',
    name: { ko: '부사격 조사', en: 'Adverbial case particle' },
    definition: {
      ko: '장소, 방향, 수단, 대상 같은 부사적 관계를 나타내는 격조사입니다.',
      en: 'A case particle marking an adverbial relation such as place, direction, means, or recipient.',
    },
    example: '학교에서의 에서/JKB',
  },
  {
    label: 'JKV',
    name: { ko: '호격 조사', en: 'Vocative case particle' },
    definition: {
      ko: '부르는 대상을 나타내는 격조사입니다.',
      en: 'A case particle marking the addressee of a call.',
    },
    example: '철수야의 야/JKV',
  },
  {
    label: 'JKQ',
    name: { ko: '인용격 조사', en: 'Quotative case particle' },
    definition: {
      ko: '앞말이 인용된 내용임을 나타내는 격조사입니다.',
      en: 'A case particle marking quoted content.',
    },
    example: '좋다고의 고/JKQ',
  },
  {
    label: 'JX',
    name: { ko: '보조사', en: 'Auxiliary particle' },
    definition: {
      ko: '주제, 대조, 한정, 추가 같은 의미를 더하는 조사입니다.',
      en: 'A particle adding meanings such as topic, contrast, limitation, or addition.',
    },
    example: '나도에서 도/JX',
  },
  {
    label: 'JC',
    name: { ko: '접속 조사', en: 'Conjunctive particle' },
    definition: {
      ko: '둘 이상의 체언을 같은 자격으로 이어 주는 조사입니다.',
      en: 'A particle coordinating two or more nominals.',
    },
    example: '사과와 배의 와/JC',
  },
  {
    label: 'EP',
    name: { ko: '선어말어미', en: 'Prefinal ending' },
    definition: {
      ko: '어간과 어말어미 사이에서 높임, 시제, 추측 같은 의미를 더하는 어미입니다.',
      en: 'An ending between a stem and final ending that adds honorific, tense, or modal meaning.',
    },
    example: '먹었다의 었/EP',
  },
  {
    label: 'EF',
    name: { ko: '종결어미', en: 'Final ending' },
    definition: {
      ko: '문장을 끝내고 서술, 질문, 명령 같은 문장 유형을 나타내는 어미입니다.',
      en: 'An ending that closes a sentence and marks its sentence type.',
    },
    example: '먹었다의 다/EF',
  },
  {
    label: 'EC',
    name: { ko: '연결어미', en: 'Connective ending' },
    definition: {
      ko: '용언을 뒤 절이나 용언에 연결하는 어미입니다.',
      en: 'An ending connecting a predicate to a following clause or predicate.',
    },
    example: '먹고의 고/EC',
  },
  {
    label: 'ETN',
    name: { ko: '명사형 전성어미', en: 'Nominal ending' },
    definition: {
      ko: '용언이 문장에서 명사처럼 쓰이도록 만드는 전성어미입니다.',
      en: 'An ending that lets a predicate function as a nominal.',
    },
    example: '먹기의 기/ETN',
  },
  {
    label: 'ETM',
    name: { ko: '관형사형 전성어미', en: 'Adnominal ending' },
    definition: {
      ko: '용언이 뒤 체언을 꾸미도록 만드는 전성어미입니다.',
      en: 'An ending that turns a predicate into a modifier of a following nominal.',
    },
    example: '먹는 사람의 는/ETM',
  },
  {
    label: 'XPN',
    name: { ko: '체언 접두사', en: 'Nominal prefix' },
    definition: {
      ko: '체언이나 어근 앞에 붙어 새 체언을 만드는 접두사입니다.',
      en: 'A prefix attached before a nominal or root to form a nominal.',
    },
    example: '풋사과의 풋/XPN',
  },
  {
    label: 'XSN',
    name: { ko: '명사 파생 접미사', en: 'Noun-forming suffix' },
    definition: {
      ko: '어근이나 단어 뒤에 붙어 명사를 만드는 파생 접미사입니다.',
      en: 'A derivational suffix that forms a noun.',
    },
    example: '문화적의 적/XSN',
  },
  {
    label: 'XSV',
    name: { ko: '동사 파생 접미사', en: 'Verb-forming suffix' },
    definition: {
      ko: '명사나 어근 뒤에 붙어 동사를 만드는 파생 접미사입니다.',
      en: 'A derivational suffix that forms a verb.',
    },
    example: '공부하다의 하/XSV',
  },
  {
    label: 'XSA',
    name: { ko: '형용사 파생 접미사', en: 'Adjective-forming suffix' },
    definition: {
      ko: '명사나 어근 뒤에 붙어 형용사를 만드는 파생 접미사입니다.',
      en: 'A derivational suffix that forms an adjective.',
    },
    example: '자연스럽다의 스럽/XSA',
  },
  {
    label: 'XR',
    name: { ko: '어근', en: 'Root' },
    definition: {
      ko: '파생 접사가 붙어 단어를 만드는 중심 의미 요소입니다.',
      en: 'A lexical root that combines with derivational affixes to form a word.',
    },
    example: '아름답다의 아름/XR',
  },
];

export function getMorphemeGlossaryEntries(
  locale: DocumentLocale,
): readonly MorphemeGlossaryEntry[] {
  return morphemeLabels.map((entry) => ({
    id: `morpheme-${entry.label.toLowerCase()}`,
    name: entry.name[locale],
    notation: entry.label,
    definition: entry.definition[locale],
    example: entry.example,
    aliases: [entry.label],
  }));
}
