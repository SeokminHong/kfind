import type { TechnicalDocuments } from './types';

import { DocumentLocale } from '../../app/i18n';
import { RoutePath } from '../../app/navigation';

import { section } from './section';

export const morphologyDocuments: TechnicalDocuments = {
  [RoutePath.MorphPartsOfSpeech]: {
    [DocumentLocale.Korean]: {
      eyebrow: '한국어 형태 · 문법',
      title: '품사 체계',
      sections: [
        section('세부 태그', [
          '체언에는 NNG·NNP·NNB·NNBC·NP·NR, 용언에는 VV·VA·VX·VCP·VCN이 있습니다. J 계열은 조사, EP·EF·EC·ETN·ETM은 어미 기능을 구분합니다.',
          'XPN·XSN·XSV·XSA와 XR은 파생 구조를 표현합니다. Tag는 surface 문자열이 아니라 source analysis의 node와 edge에 붙습니다.',
        ]),
        section('coarse mapping', [
          '`noun`은 NNG·NNP·NNB·NNBC를, `verb`는 VV를, `adjective`는 VA를 포함합니다. VCP·VCN은 지정사 규칙에서 별도 처리하고 query 분석이 요구하는 범주로 projection합니다.',
          'Coarse POS가 세부 태그를 지우지는 않습니다. Match origin에는 실제 query analysis와 source analysis index가 남습니다.',
        ]),
        section('중의 분석', [
          '`나는`은 `나/NP+는/JX`와 `날다/VV+는/ETM` 같은 여러 분석을 가질 수 있습니다. Query POS가 pronoun이면 첫 경로만, verb이면 둘째 경로만 program이 됩니다.',
          'Source에서도 여러 분석이 구조를 만족하면 후보를 유지합니다. 의미 문맥으로 하나를 선택하지 않습니다.',
          '따라서 `pro:나`와 `v:날다`는 source `나는 새를 본다`의 `나는`에 각각 match합니다. 반면 `n:나`는 대명사 NP를 일반명사 NNG로 바꾸지 않으므로 이 분석에 match하지 않습니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'KOREAN MORPHOLOGY · GRAMMAR',
      title: 'Part-of-speech system',
      sections: [
        section('Detailed tags', [
          'Nominals include NNG, NNP, NNB, NNBC, NP, and NR; predicates include VV, VA, VX, VCP, and VCN. J tags separate particles, while EP, EF, EC, ETN, and ETM distinguish ending functions.',
          'XPN, XSN, XSV, XSA, and XR represent derivation. Tags belong to nodes and edges in source analyses, not merely to surface strings.',
        ]),
        section('Coarse mapping', [
          '`noun` includes NNG, NNP, NNB, and NNBC; `verb` includes VV; `adjective` includes VA. Copulas VCP and VCN follow copular rules and project into the category required by query analysis.',
          'Coarse POS does not erase detailed tags. Match origins retain actual query analyses and source analysis indices.',
        ]),
        section('Ambiguous analyses', [
          '`나는` may be `나/NP+는/JX` or `날다/VV+는/ETM`. A pronoun query builds the first path; a verb query builds the second.',
          'A source candidate remains when multiple analyses satisfy structure. No semantic context is used to choose one.',
          'Accordingly, both `pro:나` and `v:날다` match `나는` in `나는 새를 본다` through their respective analyses. `n:나` does not reclassify pronoun NP as common-noun NNG and does not match this analysis.',
        ]),
      ],
    },
  },
  [RoutePath.Nominals]: {
    [DocumentLocale.Korean]: {
      eyebrow: '한국어 형태 · 체언',
      title: '체언',
      sections: [
        section('체언 분류', [
          '일반명사 NNG와 고유명사 NNP는 noun, 대명사 NP는 pronoun, 수사 NR은 numeral query에 대응합니다. 의존명사 NNB·단위성 의존명사 NNBC는 앞 성분 조건을 요구할 수 있습니다.',
          '체언 program은 core 뒤에서 조사 automaton을 시작하고 어미 automaton을 사용하지 않습니다.',
        ]),
        section('체언 suffix', [
          '조사 앞의 `들`, `적`, `성` 같은 요소는 모두 같은 방식이 아닙니다. 사전 entry가 명사 파생 또는 복수 표지를 선언할 때만 typed suffix로 소비합니다.',
          '`책임하에서`의 `하`처럼 명사 suffix로 지지되는 surface는 해당 span과 후속 조사가 완성될 때만 noun path를 만듭니다.',
        ]),
        section('수사와 단위', [
          'ASCII 숫자 뒤의 NR 하나 이상과 NNB·NNBC 단위가 정렬된 span으로 이어질 때 수사 구조를 만듭니다. `2026년에는`은 숫자+단위+조사 경로입니다.',
          'NR 없이 시작하는 단위, 끝의 일반명사, unknown node와 불완전한 조사 연쇄는 이 경로에서 허용하지 않습니다.',
          '`num:2026`은 정렬된 수사·단위 분석이 있는 `2026년에는`에 match하지만, 단위 앞 NR 경로가 없는 `2026학교에는`에는 match하지 않습니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'KOREAN MORPHOLOGY · NOMINALS',
      title: 'Nominals',
      sections: [
        section('Nominal classes', [
          'Common NNG and proper NNP map to noun, NP to pronoun, and NR to numeral queries. Bound NNB and unit-like NNBC may require preceding-component conditions.',
          'A nominal program starts the particle automaton after its core and never uses the ending automaton.',
        ]),
        section('Nominal suffixes', [
          'Elements such as `들`, `적`, and `성` before particles do not share one rule. They are consumed as typed suffixes only when lexicon entries declare plural or nominal derivation.',
          'A supported nominal suffix such as `하` in `책임하에서` forms a noun path only when its span and following particle complete.',
        ]),
        section('Numerals and units', [
          'A numeric structure requires ASCII digits followed by one or more NR nodes and an aligned NNB or NNBC unit span. `2026년에는` follows digit, unit, and particle nodes.',
          'Units without NR, trailing common nouns, unknown nodes, and incomplete particle chains are rejected.',
          '`num:2026` matches `2026년에는` when its numeral and unit analyses align, but not `2026학교에는`, which has no NR-to-unit path.',
        ]),
      ],
    },
  },
  [RoutePath.Particles]: {
    [DocumentLocale.Korean]: {
      eyebrow: '한국어 형태 · 체언',
      title: '조사',
      sections: [
        section('조사 분류', [
          '격조사 JKS·JKC·JKG·JKO·JKB·JKV·JKQ, 보조사 JX와 접속조사 JC를 기능별 전이로 표현합니다. Query가 조사 자체면 particle POS를 사용하고 체언 검색에서는 host 뒤 suffix로 소비합니다.',
          '`학교에서는`은 `학교/NNG+에서/JKB+는/JX`로 두 전이가 완성됩니다.',
        ]),
        section('이형태', [
          '주격 `이/가`, 목적격 `을/를`, 보조사 `은/는`은 host 받침 유무를 검사합니다. 방향격 `으로/로`는 받침이 없거나 ㄹ 받침이면 `로`, 그 밖의 받침이면 `으로`입니다.',
          '`길로`는 허용되지만 `집로`는 같은 경로에서 거부됩니다. 받침은 정규화된 한글 음절을 분해해 판정합니다.',
        ]),
        section('조사 연쇄', [
          '복수 표지 뒤에는 격조사나 보조사가 올 수 있고, 격조사 뒤에는 `은/는`, `도`, `만`, `까지`, `조차`, `마저`처럼 해당 규칙의 `next`에 선언된 보조사만 올 수 있습니다. `만` 뒤의 `은/는·도`, `까지` 뒤의 `은/는·도·만·조차·마저`도 각각 명시된 전이입니다. `도`, `이면/면`, `이나/나`처럼 `next`가 빈 규칙은 연쇄를 끝냅니다.',
          '예를 들어 `n:학교`는 `--boundary smart`에서 `학교에서는`의 `에서/JKB+는/JX`에 match합니다. 같은 query는 `학교에서가`에 match하지 않습니다. `에서` 다음에 주격 `가/JKS`가 오는 전이는 없고 남은 suffix 때문에 token을 닫을 수 없기 때문입니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'KOREAN MORPHOLOGY · NOMINALS',
      title: 'Particles',
      sections: [
        section('Particle classes', [
          'Case particles JKS, JKC, JKG, JKO, JKB, JKV, and JKQ, auxiliary JX, and conjunctive JC form functional transitions. A particle query uses particle POS; a nominal query consumes them after the host.',
          '`학교에서는` completes `학교/NNG+에서/JKB+는/JX`.',
        ]),
        section('Allomorphs', [
          'Subject `이/가`, object `을/를`, and topic `은/는` inspect the host coda. Directional `으로/로` selects `로` after a vowel or final ㄹ and `으로` after other consonants.',
          '`길로` is valid, while `집로` is rejected on the same path. Coda checks decompose normalized Hangul syllables.',
        ]),
        section('Particle chains', [
          'A plural marker may continue to a case or auxiliary particle. A case particle may continue only to auxiliaries declared by its `next` field, such as topic, additive, exclusive, limit, or even. `만` may continue to topic or additive, while `까지` has its own topic, additive, exclusive, and even continuations. Rules with an empty `next`, including `도`, `이면/면`, and `이나/나`, terminate the chain.',
          'For example, `n:학교` under `--boundary smart` matches `학교에서는` through `에서/JKB+는/JX`. It does not match `학교에서가`: no transition permits subject `가/JKS` after `에서`, so the remaining suffix prevents token closure.',
        ]),
      ],
    },
  },
  [RoutePath.Predicates]: {
    [DocumentLocale.Korean]: {
      eyebrow: '한국어 형태 · 용언',
      title: '용언',
      sections: [
        section('어간', [
          '동사 VV와 형용사 VA의 사전형 `-다`를 제거해 기본 어간을 얻습니다. Lexicon entry는 규칙형 또는 불규칙 교체 class와 모음 조화 정보를 제공합니다.',
          '어간 surface는 어미 환경에 따라 달라질 수 있으므로 하나의 고정 prefix로 취급하지 않습니다.',
        ]),
        section('활용 class', [
          '규칙 활용은 종성, 마지막 모음과 어미 시작을 조합합니다. 불규칙 entry는 공통 ending transition 앞에서 제한된 stem substitution을 적용합니다.',
          '같은 표제어가 규칙형과 불규칙형을 모두 허용하면 두 program을 보존합니다.',
          '`v:먹다`는 `먹는다`, `먹었지만`, `먹기`에 match합니다. 명사 suffix가 붙은 `먹거리`는 용언 어미 전이가 아니므로 같은 inflection query에 match하지 않습니다.',
        ]),
        section('지정사', [
          '긍정 지정사 VCP `이다`와 부정 지정사 VCN `아니다`는 명사 host와 어미를 잇습니다. `학생이었다`는 `학생/NNG+이/VCP+었/EP+다/EF` 구조입니다.',
          '표면 `이`가 주격 조사인지 지정사인지 source 구조가 구분할 수 없으면 해당 query에 맞는 분석을 각각 유지합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'KOREAN MORPHOLOGY · PREDICATES',
      title: 'Predicates',
      sections: [
        section('Stems', [
          'Dictionary-form `-다` is removed from VV verbs and VA adjectives to obtain a base stem. Lexicon entries provide regular or irregular substitution classes and vowel-harmony information.',
          'A stem surface is not a fixed prefix because it can change with the ending environment.',
        ]),
        section('Inflection classes', [
          'Regular inflection combines final consonant, last vowel, and ending onset. Irregular entries apply bounded stem substitutions before shared ending transitions.',
          'When a lemma permits both regular and irregular forms, both programs remain.',
          '`v:먹다` matches `먹는다`, `먹었지만`, and `먹기`. It does not match the derived noun `먹거리`, which is not a predicate-ending transition.',
        ]),
        section('Copulas', [
          'Positive VCP `이다` and negative VCN `아니다` connect nominal hosts to endings. `학생이었다` has `학생/NNG+이/VCP+었/EP+다/EF`.',
          'If source structure cannot distinguish copular `이` from the subject particle, analyses compatible with the query remain.',
        ]),
      ],
    },
  },
  [RoutePath.Endings]: {
    [DocumentLocale.Korean]: {
      eyebrow: '한국어 형태 · 용언',
      title: '어미',
      sections: [
        section('선어말어미', [
          'EP는 종결·연결 어미 앞에서 높임 `시`, 과거 `었/았`, 회상 `더`, 추측 `겠` 등을 나타냅니다. `먹었겠지만`은 `먹/VV+었/EP+겠/EP+지만/EC`입니다.',
          '허용 순서는 전이표로 제한합니다. EP가 반복될 수 있다는 이유로 임의 순열을 만들지 않습니다.',
          '`v:먹다`는 과거 뒤 추측이 이어진 `먹었겠지만`에 match하지만, 전이 순서를 뒤집은 `먹겠었지만`은 생성하지 않으므로 match하지 않습니다.',
        ]),
        section('어말어미', [
          'EF는 종결, EC는 연결, ETN은 명사형, ETM은 관형형 기능을 닫습니다. `먹는다/먹으면/먹기/먹는`은 각각 다른 final state입니다.',
          '어말어미가 닫힌 뒤 허용된 조사나 punctuation boundary만 소비합니다. 다른 어미를 무제한 이어 붙이지 않습니다.',
        ]),
        section('어미 연쇄', [
          'Generator는 어간 alternation과 ending transition을 분리한 뒤 실제 surface를 조립합니다. 같은 surface가 여러 문법 경로에서 나오면 origin을 합칩니다.',
          'Rule data에 없는 연쇄는 문법적으로 가능해 보여도 생성하지 않습니다. Coverage 변화는 fixture와 rule ID를 함께 추가합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'KOREAN MORPHOLOGY · PREDICATES',
      title: 'Endings',
      sections: [
        section('Prefinal endings', [
          'EP marks honorific `시`, past `었/았`, retrospective `더`, and conjectural `겠` before final endings. `먹었겠지만` is `먹/VV+었/EP+겠/EP+지만/EC`.',
          'A transition table bounds their order. The possibility of repeated EP does not license arbitrary permutations.',
          '`v:먹다` matches `먹었겠지만`, where past precedes conjectural, but not the reversed `먹겠었지만`, which has no generated transition path.',
        ]),
        section('Final endings', [
          'EF terminates, EC connects, ETN nominalizes, and ETM adnominalizes. `먹는다`, `먹으면`, `먹기`, and `먹는` close in different final states.',
          'After final closure, only permitted particles or punctuation boundaries may follow. Ending concatenation is not unbounded.',
        ]),
        section('Ending chains', [
          'The generator separates stem alternation from ending transitions and then assembles surfaces. Identical surfaces from several grammar paths merge origins.',
          'A plausible chain absent from rule data is not generated. Coverage additions include both fixtures and rule IDs.',
        ]),
      ],
    },
  },
  [RoutePath.Irregulars]: {
    [DocumentLocale.Korean]: {
      eyebrow: '한국어 형태 · 용언',
      title: '불규칙 활용',
      sections: [
        section('어휘 class', [
          'ㄷ `걷다→걸어`, ㅂ `돕다→도와`, 르 `모르다→몰라`, ㅅ `낫다→나아`, ㅎ `파랗다→파란`, 우 `푸다→퍼`를 별도 class로 둡니다.',
          '같은 받침을 가진 모든 용언에 규칙을 일반화하지 않습니다. Lexicon이 class를 선언해야 합니다.',
        ]),
        section('음운 조건', [
          '교체는 모음 시작 어미, 아/어 계열, 관형형 등 rule별 환경에서만 실행됩니다. `걷고`는 ㄷ을 유지하고 `걸어`는 ㄹ 교체를 사용합니다.',
          '한글 초성·중성·종성을 분해해 받침 탈락과 모음 결합을 계산한 뒤 완성 음절로 재조합합니다.',
          '`v:걷다`는 `걸어`와 `걷고`에 모두 match하지만, 자음 시작 어미 앞에서도 ㄹ로 바꾼 `걸고`에는 match하지 않습니다.',
        ]),
        section('규칙 근거', [
          'Match origin의 `lexical.d-to-l`, `ending.aoeo`처럼 lexical 교체와 ending rule을 순서대로 보존합니다.',
          'Suppletive entry와 생산 규칙이 같은 surface를 만들면 두 analysis origin을 합치며 임의의 대표 하나만 남기지 않습니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'KOREAN MORPHOLOGY · PREDICATES',
      title: 'Irregular inflection',
      sections: [
        section('Lexical classes', [
          'Classes include ㄷ `걷다→걸어`, ㅂ `돕다→도와`, 르 `모르다→몰라`, ㅅ `낫다→나아`, ㅎ `파랗다→파란`, and 우 `푸다→퍼`.',
          'A shared coda does not generalize the rule to every predicate. The lexicon must declare the class.',
        ]),
        section('Phonological conditions', [
          'Substitution is limited to rule-specific environments such as vowel-initial, 아/어, or adnominal endings. `걷고` retains ㄷ while `걸어` substitutes ㄹ.',
          'Hangul onset, nucleus, and coda are decomposed for deletion and vowel combination, then recomposed into syllables.',
          '`v:걷다` matches both `걸어` and `걷고`, but not `걸고`, which incorrectly applies ㄷ-to-ㄹ substitution before a consonant-initial ending.',
        ]),
        section('Rule provenance', [
          'Origins preserve lexical and ending steps in order, such as `lexical.d-to-l` followed by `ending.aoeo`.',
          'When a suppletive entry and productive rule yield the same surface, their origins merge instead of selecting one representative.',
        ]),
      ],
    },
  },
  [RoutePath.Derivation]: {
    [DocumentLocale.Korean]: {
      eyebrow: '한국어 형태 · 파생',
      title: '파생',
      sections: [
        section('파생 mode', [
          '`inflection`은 원래 품사를 유지하고 `derivation`은 등록된 새 표제어와 그 활용을 포함합니다. Query가 literal이면 파생은 비활성화됩니다.',
          '파생 결과도 일반 program으로 compile되어 같은 anchor·boundary와 provenance 계약을 따릅니다.',
          '`n:안정`은 `--expand derivation --boundary smart`에서 `안정되었다`의 명사 component에 match합니다. `--expand inflection`에서는 noun을 파생 용언 내부로 확장하지 않으므로 같은 source에 match하지 않습니다.',
        ]),
        section('파생 접미사', [
          '명사+`하다`, `되다`, `시키다`와 지정된 XSV·XSA·XSN 경로를 지원합니다. `안정하다`, `안정되었다`는 명사 component와 파생 용언 어간을 각각 증명해야 합니다.',
          '접두사 XPN과 어근 XR은 lexicon이 선언한 결합에서만 사용합니다.',
        ]),
        section('생성 제약', [
          'Source component span, 품사 순서와 후속 어미가 완성돼야 내부 query core를 승인합니다. 표면 substring만 맞는 파생은 거부합니다.',
          '파생을 추가할 때 canonical positive, hard negative와 query-matrix disposition을 함께 갱신합니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'KOREAN MORPHOLOGY · DERIVATION',
      title: 'Derivation',
      sections: [
        section('Derivation mode', [
          '`inflection` preserves the original POS; `derivation` includes registered derived lemmas and their inflection. Literal queries disable derivation.',
          'A derived result compiles into the same program, anchor, boundary, and provenance contract.',
          '`n:안정` with `--expand derivation --boundary smart` matches the nominal component in `안정되었다`. With `--expand inflection`, the noun is not expanded into a derived predicate and does not match that source.',
        ]),
        section('Derivational suffixes', [
          'Supported paths include noun plus `하다`, `되다`, `시키다`, and selected XSV, XSA, and XSN sequences. `안정하다` and `안정되었다` must prove both nominal components and derived predicate stems.',
          'XPN prefixes and XR roots are used only in lexicon-declared combinations.',
        ]),
        section('Generation constraints', [
          'Source component spans, POS order, and following endings must complete before an internal query core is accepted. A matching substring alone is insufficient.',
          'A new derivation updates canonical positives, hard negatives, and query-matrix dispositions together.',
        ]),
      ],
    },
  },
  [RoutePath.Compounds]: {
    [DocumentLocale.Korean]: {
      eyebrow: '한국어 형태 · 복합 구조',
      title: '합성어와 보조용언',
      sections: [
        section('합성명사', [
          '`물줄기는`의 source가 `물/NNG+줄기/NNG+는/JX`를 전체 host 분석으로 선언하면 `물` component를 찾습니다. Exact host span과 조사 소비가 모두 맞아야 합니다.',
          '`산길`에 독립 전체 분석과 대안 분해가 경쟁하면 대안에만 있는 `길`은 기본적으로 승인하지 않습니다.',
          '`n:물`은 component resource를 사용하는 `--boundary smart`에서 `물줄기는`에 match합니다. 같은 설정의 `n:길`은 독립 `산길/NNG` 분석과 경쟁하는 대안 분해만으로는 `산길을`에 match하지 않습니다.',
        ]),
        section('합성용언', [
          '`들어가다`처럼 선행 용언+연결 어미+후행 용언이 하나의 token을 이루면 내부 용언 core와 continuation을 구조 constraint로 표현합니다.',
          'Source가 단일 용언으로만 분석한 surface를 임의로 두 용언으로 분해하지 않습니다.',
        ]),
        section('보조용언', [
          '`빼놓을`, `생겨났던`, `극심해지겠지만`은 용언+EC+VX+E 경로 또는 결과 변화 `-아/어지다` 경로가 완성될 때 승인됩니다.',
          '연결 어미가 core 밖에 있거나 짧은 중의 surface만으로는 보조용언 경로를 만들지 않습니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'KOREAN MORPHOLOGY · COMPOUNDS',
      title: 'Compounds and auxiliaries',
      sections: [
        section('Nominal compounds', [
          'If source declares `물/NNG+줄기/NNG+는/JX` as the whole host analysis of `물줄기는`, the `물` component is searchable. Exact host spans and particle consumption must agree.',
          'When an independent whole analysis competes with an alternative split of `산길`, `길` found only in the alternative is not accepted by default.',
          '`n:물` under `--boundary smart` with the component resource matches `물줄기는`. Under the same settings, `n:길` does not match `산길을` solely from an alternative split that competes with the independent `산길/NNG` analysis.',
        ]),
        section('Predicate compounds', [
          'When a preceding predicate, connective ending, and following predicate form one token as in `들어가다`, internal cores and continuations become structural constraints.',
          'A surface analyzed only as one predicate is not arbitrarily split into two predicates.',
        ]),
        section('Auxiliaries', [
          'Forms such as `빼놓을`, `생겨났던`, and `극심해지겠지만` require a completed predicate+EC+VX+E path or the result-change `-아/어지다` family.',
          'An out-of-core connective or a short ambiguous surface alone cannot establish an auxiliary path.',
        ]),
      ],
    },
  },
  [RoutePath.Contractions]: {
    [DocumentLocale.Korean]: {
      eyebrow: '한국어 형태 · 표면형',
      title: '축약과 영형태',
      sections: [
        section('축약', [
          '`보아→봐`, `주어→줘`, `되어→돼`는 어간 끝 모음과 아/어 계열을 결합합니다. Surface는 짧아져도 stem과 ending rule provenance를 유지합니다.',
          '허용된 모음 결합표 밖의 음절을 유사 발음만으로 만들지 않습니다.',
          '`v:보다`는 기본 inflection에서 `봐`에 match합니다. `lit:보다`는 형태 조립을 하지 않으므로 같은 source `봐`에 match하지 않습니다.',
        ]),
        section('영형태', [
          '일부 source 분석은 문법 기능을 나타내지만 독립 surface span이 없는 node를 가질 수 있습니다. 영형태는 인접 component 관계를 설명하며 검색 span을 새로 만들지 않습니다.',
          'Query core가 영형태만으로 충족되지는 않습니다. 관찰 가능한 anchor와 non-empty core span이 필요합니다.',
        ]),
        section('span 보존', [
          '축약 surface의 core span은 실제 원문 음절을 가리키고 origin은 조립 전 morpheme 경로를 가리킵니다. Atom token span은 뒤에 소비한 어미까지 포함할 수 있습니다.',
          'NFD source에서도 normalized match를 원래 code unit 범위로 되돌립니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'KOREAN MORPHOLOGY · SURFACES',
      title: 'Contractions and zero surfaces',
      sections: [
        section('Contractions', [
          '`보아→봐`, `주어→줘`, and `되어→돼` combine a stem-final vowel with the 아/어 family. The shorter surface retains stem and ending provenance.',
          'No syllable is generated from phonetic similarity outside the permitted combination table.',
          '`v:보다` matches `봐` under inflection. `lit:보다` performs no morphological assembly and therefore does not match the same source `봐`.',
        ]),
        section('Zero surfaces', [
          'Some source analyses contain grammatical nodes without independent surface spans. A zero surface explains adjacent component relations and creates no new search span.',
          'A query core cannot be satisfied only by zero surfaces. It requires an observable anchor and non-empty core.',
        ]),
        section('Span preservation', [
          'A contracted core span points to actual source syllables while origins point to pre-assembly morpheme paths. An atom token span may include consumed following endings.',
          'Normalized matches in NFD source are mapped back to original code-unit ranges.',
        ]),
      ],
    },
  },
  [RoutePath.Ambiguity]: {
    [DocumentLocale.Korean]: {
      eyebrow: '한국어 형태 · 판정',
      title: '중의성과 경계',
      sections: [
        section('분석 집합', [
          '같은 표제어·surface에 여러 품사와 규칙 경로가 있으면 각각 origin으로 보존합니다. 실행 조건이 같을 때만 program body를 공유합니다.',
          'Auto POS는 빈도 순 하나를 고르지 않습니다. Explicit POS가 사용자의 의도 범위를 정합니다.',
        ]),
        section('구조 거부', [
          'Source 전체 token의 component span과 POS path가 query constraint와 맞지 않으면 substring 후보를 거부합니다. 조사 `가`를 동사 `가다`로 보지 않는 판정이 예입니다.',
          '독립 분석과 compound 분석이 경쟁하면 exact whole-token path와 query가 요구한 component 조건을 함께 비교합니다.',
          '`v:가다`는 `--boundary smart`에서 `집에 간다`의 `간다`에 match하지만, `친구가 왔다`의 조사 `가/JKS`에는 match하지 않습니다.',
        ]),
        section('의미 한계', [
          '형태 구조가 동일한 동음이의어는 결과에 남습니다. 주제, 감정, 행위자 같은 의미 feature를 추정하지 않습니다.',
          '사용자는 phrase atom, path와 주변 문맥으로 후보를 좁힙니다. 의미 판정은 contract-adjusted benchmark의 제품 범위 밖 disposition이 될 수 있습니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'KOREAN MORPHOLOGY · DECISIONS',
      title: 'Ambiguity and boundaries',
      sections: [
        section('Analysis sets', [
          'Several POS or rule paths for one lemma and surface remain as separate origins. Only analyses with identical execution conditions share a program body.',
          'Auto POS does not select one frequency-ranked answer. Explicit POS defines user intent.',
        ]),
        section('Structural rejection', [
          'A substring candidate is rejected when whole-token component spans and POS paths do not satisfy the query constraint. This prevents particle `가` from matching verb `가다`.',
          'Competing independent and compound analyses are compared by exact whole-token paths and query-required component conditions.',
          '`v:가다` under `--boundary smart` matches `간다` in `집에 간다`, but not the particle `가/JKS` in `친구가 왔다`.',
        ]),
        section('Semantic limits', [
          'Homonyms with identical morphology remain in the result. No topic, sentiment, or agent role is inferred.',
          'Users narrow candidates with phrase atoms, paths, and surrounding context. Semantic decisions can be out-of-contract dispositions in adjusted benchmarks.',
        ]),
      ],
    },
  },
  [RoutePath.Coverage]: {
    [DocumentLocale.Korean]: {
      eyebrow: '한국어 형태 · 범위',
      title: '문법 범위',
      sections: [
        section('지원 범위', [
          '체언의 주요 조사·이형태, 용언의 대표 종결·연결·관형·명사형 어미, 선어말어미, 주요 불규칙과 등록 파생을 지원합니다.',
          '각 규칙은 stable rule ID와 canonical positive, crossing·hard negative를 가집니다.',
        ]),
        section('제한 규칙', [
          '조사·어미 연쇄, 파생 접사와 compound continuation은 닫힌 전이표입니다. 사전에 없는 표제어나 observed source component가 없는 경로는 자동 일반화하지 않습니다.',
          'Full POS와 enriched resource는 coverage를 넓히지만 core 문법 engine의 전이 의미를 바꾸지 않습니다.',
          '`v:걷다`는 등록된 표준 활용 `걸어`에 match합니다. 철자 오류 `거러`나 규칙표에 없는 임의 활용은 기본 `robustness=off` 검색에서 match하지 않습니다.',
        ]),
        section('비목표', [
          '모든 신조어 추론, 띄어쓰기 교정, 의미 disambiguation과 문장 전체 형태 분석은 범위 밖입니다.',
          'Raw FN이 있어도 review가 제품 목표 밖으로 분류한 case는 contract FNᶜ에서 제외합니다. 해당 disposition은 case ledger에 근거가 있어야 하며 미분류 FN을 숨기지 않습니다.',
        ]),
      ],
    },
    [DocumentLocale.English]: {
      eyebrow: 'KOREAN MORPHOLOGY · SCOPE',
      title: 'Grammar coverage',
      sections: [
        section('Supported scope', [
          'Coverage includes major nominal particles and allomorphs; common terminal, connective, adnominal, and nominalizing endings; prefinal endings; major irregulars; and registered derivation.',
          'Each rule has a stable ID plus canonical positives and crossing or hard negatives.',
        ]),
        section('Bounded rules', [
          'Particle and ending chains, derivational affixes, and compound continuations use closed transition tables. Unknown lemmas or paths lacking observed source components are not generalized automatically.',
          'Full-POS and enriched resources broaden coverage without changing transition semantics in the core grammar engine.',
          '`v:걷다` matches the registered standard inflection `걸어`. The misspelling `거러` and arbitrary forms absent from rule data do not match under the default `robustness=off` behavior.',
        ]),
        section('Non-goals', [
          'Universal neologism inference, spacing correction, semantic disambiguation, and whole-sentence morphology are outside scope.',
          'A reviewed out-of-contract raw FN is excluded from contract FNᶜ. Every disposition needs case-ledger evidence, and unclassified FNs cannot be hidden.',
        ]),
      ],
    },
  },
};
