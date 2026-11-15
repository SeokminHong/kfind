// 한국어기초사전의 관형형과 보조 용언 결합. 각 열은 미래, 현재·과거 허용 여부다.
const ADNOMINAL_AUXILIARIES: &[(&str, PredicatePos, LexicalAlternation, bool, bool)] = &[
    (
        "만하다",
        PredicatePos::AuxiliaryAdjective,
        LexicalAlternation::Ha,
        true,
        false,
    ),
    (
        "법하다",
        PredicatePos::AuxiliaryAdjective,
        LexicalAlternation::Ha,
        true,
        false,
    ),
    (
        "뻔하다",
        PredicatePos::AuxiliaryAdjective,
        LexicalAlternation::Ha,
        true,
        false,
    ),
    (
        "척하다",
        PredicatePos::AuxiliaryVerb,
        LexicalAlternation::Ha,
        false,
        true,
    ),
    (
        "체하다",
        PredicatePos::AuxiliaryVerb,
        LexicalAlternation::Ha,
        false,
        true,
    ),
    (
        "양하다",
        PredicatePos::AuxiliaryVerb,
        LexicalAlternation::Ha,
        false,
        true,
    ),
    (
        "듯하다",
        PredicatePos::AuxiliaryAdjective,
        LexicalAlternation::Ha,
        true,
        true,
    ),
    (
        "듯싶다",
        PredicatePos::AuxiliaryAdjective,
        LexicalAlternation::Regular,
        true,
        true,
    ),
    (
        "성하다",
        PredicatePos::AuxiliaryAdjective,
        LexicalAlternation::Ha,
        true,
        true,
    ),
    (
        "성싶다",
        PredicatePos::AuxiliaryAdjective,
        LexicalAlternation::Regular,
        true,
        true,
    ),
    (
        "성부르다",
        PredicatePos::AuxiliaryAdjective,
        LexicalAlternation::ReuDoubleL,
        true,
        true,
    ),
];

// 한국어기초사전에서 -(으)ㄴ가를 받는 추측 보조 용언.
const INTERROGATIVE_AUXILIARIES: &[&str] = &["보다", "싶다"];

// 한국어기초사전에서 동사의 '-고'를 받는 진행·희망 보조 용언.
const PROGRESSIVE_OR_DESIDERATIVE_AUXILIARIES: &[(&str, PredicatePos)] = &[
    ("있다", PredicatePos::AuxiliaryVerb),
    ("계시다", PredicatePos::AuxiliaryVerb),
    ("싶다", PredicatePos::AuxiliaryAdjective),
];
// 한국어기초사전의 '-어' 결합 주석 중 활용 전체가 허용되는 구성.
// 일부 선행 용언 또는 특정 후행 어미로 범위가 제한된 뜻풀이는 포함하지 않는다.
const ATTACHED_RESULTATIVE_LEMMA: &str = "지다";
const VOWEL_AUXILIARIES: &[(&str, LexicalAlternation, VowelAuxiliaryHost)] = &[
    ("가다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("계시다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("나가다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("나다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("내다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("놓다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("대다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("두다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("드리다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("마지아니하다", LexicalAlternation::Ha, VowelAuxiliaryHost::Verb),
    ("마지않다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("버릇하다", LexicalAlternation::Ha, VowelAuxiliaryHost::Verb),
    ("버리다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("보다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("쌓다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("오다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("있다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("재끼다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("젖히다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("주다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("죽다", LexicalAlternation::Regular, VowelAuxiliaryHost::Adjective),
    (ATTACHED_RESULTATIVE_LEMMA, LexicalAlternation::Regular, VowelAuxiliaryHost::VerbOrAdjective),
    ("치우다", LexicalAlternation::Regular, VowelAuxiliaryHost::Verb),
    ("하다", LexicalAlternation::Ha, VowelAuxiliaryHost::Adjective),
];
