use crate::{ContinuationState, PredicatePos, RuleId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateContinuationMatch {
    /// Byte offset of the verified token end in `anchor + following`.
    pub token_end: usize,
    /// Number of bytes consumed from `following`.
    pub consumed_bytes: usize,
    pub rule_path: Vec<RuleId>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Suffix {
    pub(super) surface: &'static str,
    pub(super) rules: &'static [&'static str],
}

const YAJI: Suffix = suffix("야지", &["ending.necessity-yaji"]);
const YAJI_POLITE: Suffix = suffix("야지요", &["ending.necessity-yaji-polite"]);
const YAJI_CONTRACTED: Suffix = suffix("야죠", &["ending.necessity-yaji-polite"]);
pub(super) const YAJI_ENDINGS: &[Suffix] = &[YAJI, YAJI_POLITE, YAJI_CONTRACTED];
const HONORIFIC_YAJI: Suffix = suffix(
    "셔야지",
    &["ending.honorific", "ending.aoeo", "ending.necessity-yaji"],
);
const HONORIFIC_YAJI_POLITE: Suffix = suffix(
    "셔야지요",
    &[
        "ending.honorific",
        "ending.aoeo",
        "ending.necessity-yaji-polite",
    ],
);
const HONORIFIC_YAJI_CONTRACTED: Suffix = suffix(
    "셔야죠",
    &[
        "ending.honorific",
        "ending.aoeo",
        "ending.necessity-yaji-polite",
    ],
);
pub(super) const HONORIFIC_YAJI_ENDINGS: &[Suffix] = &[
    HONORIFIC_YAJI,
    HONORIFIC_YAJI_POLITE,
    HONORIFIC_YAJI_CONTRACTED,
];

const A_OR_EO_SUFFIXES: &[Suffix] = &[
    YAJI,
    YAJI_POLITE,
    YAJI_CONTRACTED,
    suffix("지거들랑", &["ending.auxiliary-jida", "ending.geodeun"]),
    suffix("지건", &["ending.auxiliary-jida", "ending.geodeun-short"]),
    suffix("지거든", &["ending.auxiliary-jida", "ending.geodeun"]),
    suffix(
        "지거든요",
        &["ending.auxiliary-jida", "ending.geodeun-polite"],
    ),
    suffix("가고", &["ending.auxiliary-gada", "ending.connective-go"]),
    suffix("가야", &["ending.auxiliary-gada", "ending.connective-ya"]),
    suffix(
        "졌습니다",
        &[
            "ending.auxiliary-jida",
            "ending.past",
            "ending.polite-declarative",
        ],
    ),
    suffix("서도", &["ending.aoeo-seo", "particle.additive"]),
    suffix(
        "졌다",
        &["ending.auxiliary-jida", "ending.past", "ending.final-da"],
    ),
    suffix("지면", &["ending.auxiliary-jida", "ending.conditional"]),
    suffix("지고", &["ending.auxiliary-jida", "ending.connective-go"]),
    suffix("진", &["ending.auxiliary-jida", "ending.past-adnominal"]),
    suffix("질", &["ending.auxiliary-jida", "ending.future-adnominal"]),
    suffix("지다", &["ending.auxiliary-jida", "ending.final-da"]),
    suffix("서", &["ending.aoeo-seo"]),
    suffix("도", &["ending.connective-do"]),
    suffix("야", &["ending.connective-ya"]),
    suffix("요", &["ending.polite-yo"]),
    suffix("라", &["ending.imperative-ra"]),
];

const PAST_SUFFIXES: &[Suffix] = &[
    suffix("습니까", &["ending.formal-interrogative"]),
    suffix("습디까", &["ending.retrospective-interrogative"]),
    suffix("습디다", &["ending.retrospective-declarative"]),
    suffix("어야지", &["ending.necessity-yaji"]),
    suffix("어야지요", &["ending.necessity-yaji-polite"]),
    suffix("어야죠", &["ending.necessity-yaji-polite"]),
    GEON,
    GEODEUN,
    GEODEUN_POLITE,
    GEODEULLANG,
    suffix("습니다", &["ending.polite-declarative"]),
    suffix("으되", &["ending.connective-eudoe"]),
    suffix("느냐는", &["ending.interrogative-neunya", "particle.topic"]),
    suffix("으면", &["ending.conditional"]),
    suffix("으며", &["ending.coordinate-myeo"]),
    suffix("지만", &["ending.connective-jiman"]),
    suffix("는데", &["ending.connective-neunde"]),
    suffix("다고", &["ending.quotative-go"]),
    suffix("느냐", &["ending.interrogative-neunya"]),
    suffix("던", &["ending.retrospective-adnominal"]),
    suffix("을", &["ending.future-adnominal"]),
    suffix("다", &["ending.final-da"]),
    suffix("고", &["ending.connective-go"]),
    suffix("어요", &["ending.past-polite-yo"]),
];

const FUTURE_SUFFIXES: &[Suffix] = &[
    suffix("습니까", &["ending.formal-interrogative"]),
    suffix("습디까", &["ending.retrospective-interrogative"]),
    suffix("습디다", &["ending.retrospective-declarative"]),
    suffix("습니다", &["ending.polite-declarative"]),
    suffix("으되", &["ending.connective-eudoe"]),
    suffix("지만", &["ending.connective-jiman"]),
    suffix("는데", &["ending.connective-neunde"]),
    suffix("다", &["ending.final-da"]),
    suffix("고", &["ending.connective-go"]),
];

const DECLARATIVE_MYEONSEO: Suffix = suffix("면서", &["ending.quotative-myeonseo"]);
const DECLARATIVE_NEUNDE: Suffix = suffix("는데", &["ending.quotative-neunde"]);
const DECLARATIVE_GO: Suffix = suffix("고", &["ending.quotative-go"]);
const DECLARATIVE_ADNOMINAL: Suffix = suffix("는", &["ending.quotative-adnominal"]);
const DECLARATIVE_RETROSPECTIVE: Suffix = suffix("던", &["ending.quotative-retrospective"]);
const DECLARATIVE_CONDITIONAL: Suffix = suffix("면", &["ending.conditional"]);
const DECLARATIVE_NI: Suffix = suffix("니", &["ending.quotative-ni"]);
const DECLARATIVE_MYEO: Suffix = suffix("며", &["ending.quotative-myeo"]);
const DECLARATIVE_JI: Suffix = suffix("지", &["ending.quotative-ji"]);

const HONORIFIC_FORMAL: Suffix =
    suffix("십시오", &["ending.honorific", "ending.imperative-formal"]);
const HONORIFIC_FORMAL_SHORT: Suffix = suffix(
    "십쇼",
    &["ending.honorific", "ending.imperative-formal-short"],
);
const HONORIFIC_PROPOSITIVE: Suffix = suffix("십시다", &["ending.honorific", "ending.propositive"]);
pub(super) const INTENTIVE: Suffix = suffix("려", &["ending.intentive-ryeo"]);
pub(super) const CONDITIONAL: Suffix = suffix("면", &["ending.conditional"]);
pub(super) const COORDINATE: Suffix = suffix("며", &["ending.coordinate-myeo"]);
pub(super) const INTENTIVE_CONDITIONAL: Suffix = suffix("려면", &["ending.intentive-ryeomyeon"]);
pub(super) const HONORIFIC_INTENTIVE_CONDITIONAL: Suffix = suffix(
    "시려면",
    &["ending.honorific", "ending.intentive-ryeomyeon"],
);

pub(super) const HONORIFIC_BIEUP_ENDINGS: &[Suffix] = &[
    HONORIFIC_FORMAL,
    HONORIFIC_FORMAL_SHORT,
    HONORIFIC_PROPOSITIVE,
];
const GEODEUN: Suffix = suffix("거든", &["ending.geodeun"]);
const GEODEUN_POLITE: Suffix = suffix("거든요", &["ending.geodeun-polite"]);
const GEODEULLANG: Suffix = suffix("거들랑", &["ending.geodeun"]);
const GEON: Suffix = suffix("건", &["ending.geodeun-short"]);
const HONORIFIC_GEODEUN: Suffix = suffix("시거든", &["ending.honorific", "ending.geodeun"]);
const HONORIFIC_GEODEUN_POLITE: Suffix =
    suffix("시거든요", &["ending.honorific", "ending.geodeun-polite"]);
const HONORIFIC_GEODEULLANG: Suffix = suffix("시거들랑", &["ending.honorific", "ending.geodeun"]);
const HONORIFIC_GEON: Suffix = suffix("시건", &["ending.honorific", "ending.geodeun-short"]);

pub(super) const GEODEUN_ENDINGS: &[Suffix] = &[GEODEUN, GEODEUN_POLITE, GEODEULLANG, GEON];
pub(super) const HONORIFIC_GEODEUN_ENDINGS: &[Suffix] = &[
    HONORIFIC_GEODEUN,
    HONORIFIC_GEODEUN_POLITE,
    HONORIFIC_GEODEULLANG,
    HONORIFIC_GEON,
];

const EU_SUFFIXES: &[Suffix] = &[
    suffix(
        "십니까",
        &["ending.honorific", "ending.formal-interrogative"],
    ),
    suffix(
        "십디까",
        &["ending.honorific", "ending.retrospective-interrogative"],
    ),
    suffix(
        "십디다",
        &["ending.honorific", "ending.retrospective-declarative"],
    ),
    suffix(
        "셨습니까",
        &[
            "ending.honorific",
            "ending.past",
            "ending.formal-interrogative",
        ],
    ),
    suffix(
        "셨습디까",
        &[
            "ending.honorific",
            "ending.past",
            "ending.retrospective-interrogative",
        ],
    ),
    suffix(
        "셨습디다",
        &[
            "ending.honorific",
            "ending.past",
            "ending.retrospective-declarative",
        ],
    ),
    suffix(
        "시겠습니까",
        &[
            "ending.honorific",
            "ending.future",
            "ending.formal-interrogative",
        ],
    ),
    suffix(
        "시겠습디까",
        &[
            "ending.honorific",
            "ending.future",
            "ending.retrospective-interrogative",
        ],
    ),
    suffix(
        "시겠습디다",
        &[
            "ending.honorific",
            "ending.future",
            "ending.retrospective-declarative",
        ],
    ),
    HONORIFIC_YAJI,
    HONORIFIC_YAJI_POLITE,
    HONORIFIC_YAJI_CONTRACTED,
    HONORIFIC_GEODEUN,
    HONORIFIC_GEODEUN_POLITE,
    HONORIFIC_GEODEULLANG,
    HONORIFIC_GEON,
    suffix(
        "셨건",
        &["ending.honorific", "ending.past", "ending.geodeun-short"],
    ),
    suffix(
        "셨거든",
        &["ending.honorific", "ending.past", "ending.geodeun"],
    ),
    suffix(
        "셨거든요",
        &["ending.honorific", "ending.past", "ending.geodeun-polite"],
    ),
    suffix(
        "셨거들랑",
        &["ending.honorific", "ending.past", "ending.geodeun"],
    ),
    HONORIFIC_FORMAL,
    HONORIFIC_FORMAL_SHORT,
    HONORIFIC_PROPOSITIVE,
    INTENTIVE,
    INTENTIVE_CONDITIONAL,
    HONORIFIC_INTENTIVE_CONDITIONAL,
    suffix("면서", &["ending.simultaneous-myeonseo"]),
    suffix("려던", &["ending.intentive-adnominal"]),
    suffix("러", &["ending.purpose-reo"]),
    suffix("나", &["ending.connective-na"]),
    suffix("시오", &["ending.imperative-sio"]),
    suffix("쇼", &["ending.imperative-sio-short"]),
    suffix(
        "시겠습니다",
        &[
            "ending.honorific",
            "ending.future",
            "ending.polite-declarative",
        ],
    ),
    suffix(
        "셨습니다",
        &[
            "ending.honorific",
            "ending.past",
            "ending.polite-declarative",
        ],
    ),
    suffix(
        "셨다",
        &["ending.honorific", "ending.past", "ending.final-da"],
    ),
    suffix(
        "셨던",
        &[
            "ending.honorific",
            "ending.past",
            "ending.retrospective-adnominal",
        ],
    ),
    suffix(
        "셨고",
        &["ending.honorific", "ending.past", "ending.connective-go"],
    ),
    suffix("세요", &["ending.honorific", "ending.polite-imperative"]),
    suffix("십니다", &["ending.honorific", "ending.polite-declarative"]),
    suffix("시다", &["ending.honorific", "ending.final-da"]),
    suffix("시는", &["ending.honorific", "ending.present-adnominal"]),
    suffix("시고", &["ending.honorific", "ending.connective-go"]),
    suffix("시면", &["ending.honorific", "ending.conditional"]),
    suffix("신", &["ending.honorific", "ending.past-adnominal"]),
    suffix("실", &["ending.honorific", "ending.future-adnominal"]),
    CONDITIONAL,
    COORDINATE,
    suffix("니까는", &["ending.connective-ni"]),
    suffix("니까", &["ending.connective-ni"]),
    suffix("니깐", &["ending.connective-ni"]),
    suffix("니", &["ending.connective-ni"]),
];

const RIEUL_FINAL: &[Suffix] = &[
    suffix("세", &["ending.final-rieulse"]),
    suffix("지", &["ending.doubt-rieulji"]),
    suffix("지요", &["ending.doubt-rieulji"]),
    suffix("는지", &["ending.doubt-rieulneunji"]),
    suffix("는지요", &["ending.doubt-rieulneunji"]),
    suffix("걸", &["ending.conjecture-rieulgeol"]),
    suffix("걸요", &["ending.conjecture-rieulgeol"]),
];

const PAST_RIEUL_FINAL: &[Suffix] = &[
    suffix("을지", &["ending.doubt-rieulji"]),
    suffix("을지요", &["ending.doubt-rieulji"]),
    suffix("을는지", &["ending.doubt-rieulneunji"]),
    suffix("을는지요", &["ending.doubt-rieulneunji"]),
    suffix("을걸", &["ending.conjecture-rieulgeol"]),
    suffix("을걸요", &["ending.conjecture-rieulgeol"]),
];

const EU_RIEUL_FINAL: &[Suffix] = &[
    suffix("실지", &["ending.honorific", "ending.doubt-rieulji"]),
    suffix("실지요", &["ending.honorific", "ending.doubt-rieulji"]),
    suffix("실는지", &["ending.honorific", "ending.doubt-rieulneunji"]),
    suffix(
        "실는지요",
        &["ending.honorific", "ending.doubt-rieulneunji"],
    ),
    suffix("실걸", &["ending.honorific", "ending.conjecture-rieulgeol"]),
    suffix(
        "실걸요",
        &["ending.honorific", "ending.conjecture-rieulgeol"],
    ),
    suffix(
        "셨을지",
        &["ending.honorific", "ending.past", "ending.doubt-rieulji"],
    ),
    suffix(
        "셨을지요",
        &["ending.honorific", "ending.past", "ending.doubt-rieulji"],
    ),
    suffix(
        "셨을는지",
        &[
            "ending.honorific",
            "ending.past",
            "ending.doubt-rieulneunji",
        ],
    ),
    suffix(
        "셨을는지요",
        &[
            "ending.honorific",
            "ending.past",
            "ending.doubt-rieulneunji",
        ],
    ),
    suffix(
        "셨을걸",
        &[
            "ending.honorific",
            "ending.past",
            "ending.conjecture-rieulgeol",
        ],
    ),
    suffix(
        "셨을걸요",
        &[
            "ending.honorific",
            "ending.past",
            "ending.conjecture-rieulgeol",
        ],
    ),
];

const BIEUP_FINAL: &[Suffix] = &[
    suffix("니다", &["ending.polite-declarative"]),
    suffix("니까", &["ending.formal-interrogative"]),
    suffix("디까", &["ending.retrospective-interrogative"]),
    suffix("디다", &["ending.retrospective-declarative"]),
];

macro_rules! prospective_suffixes {
    ($prefix:literal $(, $rule:literal)*) => {
        &[
            suffix(concat!($prefix, "리"), &[$($rule,)* "ending.prospective-final"]),
            suffix(concat!($prefix, "리라"), &[$($rule,)* "ending.prospective-final"]),
            suffix(concat!($prefix, "리라고"), &[$($rule,)* "ending.prospective-quotative"]),
            suffix(concat!($prefix, "리다"), &[$($rule,)* "ending.prospective-final"]),
            suffix(concat!($prefix, "리오"), &[$($rule,)* "ending.prospective-final"]),
            suffix(concat!($prefix, "리까"), &[$($rule,)* "ending.prospective-final"]),
            suffix(concat!($prefix, "리니라"), &[$($rule,)* "ending.prospective-final"]),
            suffix(concat!($prefix, "리로다"), &[$($rule,)* "ending.prospective-final"]),
            suffix(concat!($prefix, "리니"), &[$($rule,)* "ending.prospective-connective"]),
            suffix(concat!($prefix, "리만치"), &[$($rule,)* "ending.prospective-connective"]),
            suffix(concat!($prefix, "리만큼"), &[$($rule,)* "ending.prospective-connective"]),
        ]
    };
}

const PROSPECTIVE: &[Suffix] = prospective_suffixes!("");
const PREFINAL_PROSPECTIVE: &[Suffix] = prospective_suffixes!("으");
const HONORIFIC_PROSPECTIVE: &[Suffix] = prospective_suffixes!("시", "ending.honorific");
const HONORIFIC_PAST_PROSPECTIVE: &[Suffix] =
    prospective_suffixes!("셨으", "ending.honorific", "ending.past");

macro_rules! question {
    ($surface:literal, $rule:literal $(, $prior:literal)*) => {
        &[
            suffix($surface, &[$($prior,)* $rule]),
            suffix(concat!($surface, "고"), &[$($prior,)* $rule, "ending.quotative-go"]),
            suffix(concat!($surface, "고요"), &[$($prior,)* $rule, "ending.quotative-go", "ending.polite-yo"]),
        ]
    };
}

macro_rules! background {
    ($surface:literal, $rule:literal $(, $prior:literal)*) => {
        &[
            suffix($surface, &[$($prior,)* $rule]),
            suffix(concat!($surface, "도"), &[$($prior,)* $rule, "particle.additive"]),
            suffix(concat!($surface, "요"), &[$($prior,)* $rule, "ending.polite-yo"]),
        ]
    };
}

const NYA: &[Suffix] = question!("냐", "ending.interrogative-nya");
const NEUNYA: &[Suffix] = question!("느냐", "ending.interrogative-neunya");
const SI_NYA: &[Suffix] = question!("시냐", "ending.interrogative-nya", "ending.honorific");
const SI_NEUNYA: &[Suffix] = question!("시느냐", "ending.interrogative-neunya", "ending.honorific");
const DEONDE: &[Suffix] = background!("던데", "ending.connective-deonde");
const NEUNDE: &[Suffix] = background!("는데", "ending.connective-neunde");
const SINDE: &[Suffix] = background!("신데", "ending.connective-neunde", "ending.honorific");
const SINEUNDE: &[Suffix] = background!("시는데", "ending.connective-neunde", "ending.honorific");
const SYEOTNEUNDE: &[Suffix] = background!(
    "셨는데",
    "ending.connective-neunde",
    "ending.honorific",
    "ending.past"
);
const SIGETNEUNDE: &[Suffix] = background!(
    "시겠는데",
    "ending.connective-neunde",
    "ending.honorific",
    "ending.future"
);
const BACKGROUND: &[Suffix] = background!("데", "ending.connective-neunde");
const SYEOT_NYA: &[Suffix] = question!(
    "셨냐",
    "ending.interrogative-nya",
    "ending.honorific",
    "ending.past"
);
const SYEOT_NEUNYA: &[Suffix] = question!(
    "셨느냐",
    "ending.interrogative-neunya",
    "ending.honorific",
    "ending.past"
);
const SIGET_NYA: &[Suffix] = question!(
    "시겠냐",
    "ending.interrogative-nya",
    "ending.honorific",
    "ending.future"
);
const SIGET_NEUNYA: &[Suffix] = question!(
    "시겠느냐",
    "ending.interrogative-neunya",
    "ending.honorific",
    "ending.future"
);

const INTERROGATIVE: &[Suffix] = &[
    suffix("고", &["ending.quotative-go"]),
    suffix("고요", &["ending.quotative-go", "ending.polite-yo"]),
];

macro_rules! nieun_extensions {
    ($prefix:literal $(, $prior:literal)*) => {
        &[
            suffix(concat!($prefix, "고"), &[$($prior,)* "ending.interrogative-ngo"]),
            suffix(concat!($prefix, "걸"), &[$($prior,)* "ending.explanatory-ngeol"]),
            suffix(concat!($prefix, "걸요"), &[$($prior,)* "ending.explanatory-ngeol", "ending.polite-yo"]),
            suffix(concat!($prefix, "바"), &[$($prior,)* "ending.connective-nba"]),
        ]
    };
}

const NIEUN_EXTENSIONS: &[Suffix] = nieun_extensions!("");
const NEUN_EXTENSIONS: &[Suffix] = nieun_extensions!("는");
const DEON_EXTENSIONS: &[Suffix] = nieun_extensions!("던");
const SIDEON_EXTENSIONS: &[Suffix] = nieun_extensions!("시던", "ending.honorific");
const SIN_EXTENSIONS: &[Suffix] = nieun_extensions!("신", "ending.honorific");
const SINEUN_EXTENSIONS: &[Suffix] = nieun_extensions!("시는", "ending.honorific");
const SYEOTNEUN_EXTENSIONS: &[Suffix] =
    nieun_extensions!("셨는", "ending.honorific", "ending.past");
const SIGETNEUN_EXTENSIONS: &[Suffix] =
    nieun_extensions!("시겠는", "ending.honorific", "ending.future");
const COMPLETED_NIEUN: &[Suffix] = &[
    suffix("걸", &["ending.explanatory-ngeol"]),
    suffix("걸요", &["ending.explanatory-ngeol", "ending.polite-yo"]),
    suffix("바", &["ending.connective-nba"]),
];

fn nieun_tail(following: &str, prefix: &str) -> bool {
    following
        .strip_prefix(prefix)
        .is_some_and(|tail| tail.starts_with(['고', '걸', '바']))
}

/// Consumes the longest suffix accepted by a predicate branch's verifier state.
#[must_use]
pub fn verify_predicate_continuation(
    state: ContinuationState,
    pos: PredicatePos,
    anchor: &str,
    following: &str,
) -> Option<PredicateContinuationMatch> {
    let candidates = match state {
        ContinuationState::Terminal => {
            return Some(matched(anchor.len(), None));
        }
        ContinuationState::AOrEo => A_OR_EO_SUFFIXES,
        ContinuationState::Past => PAST_SUFFIXES,
        ContinuationState::Future => FUTURE_SUFFIXES,
        ContinuationState::Declarative => {
            return Some(matched(anchor.len(), declarative_suffix(following)));
        }
        ContinuationState::Eu => EU_SUFFIXES,
        ContinuationState::StemEnding => GEODEUN_ENDINGS,
        ContinuationState::NieunEnding => BACKGROUND,
        ContinuationState::CompletedNieunEnding => COMPLETED_NIEUN,
        ContinuationState::Interrogative => INTERROGATIVE,
        ContinuationState::RieulFinal => RIEUL_FINAL,
        ContinuationState::BieupFinal => BIEUP_FINAL,
        ContinuationState::Prospective => PROSPECTIVE,
    };
    let additional_endings = match state {
        ContinuationState::NieunEnding => NIEUN_EXTENSIONS,
        ContinuationState::Past | ContinuationState::Future if nieun_tail(following, "는") => {
            NEUN_EXTENSIONS
        }
        ContinuationState::StemEnding | ContinuationState::Past if nieun_tail(following, "던") => {
            DEON_EXTENSIONS
        }
        ContinuationState::Eu if nieun_tail(following, "신") => SIN_EXTENSIONS,
        ContinuationState::Eu if nieun_tail(following, "시던") => SIDEON_EXTENSIONS,
        ContinuationState::Eu if nieun_tail(following, "시는") => SINEUN_EXTENSIONS,
        ContinuationState::Eu if nieun_tail(following, "셨는") => SYEOTNEUN_EXTENSIONS,
        ContinuationState::Eu if nieun_tail(following, "시겠는") => SIGETNEUN_EXTENSIONS,
        ContinuationState::StemEnding | ContinuationState::Past | ContinuationState::Future
            if following.starts_with('냐') =>
        {
            NYA
        }
        ContinuationState::Past | ContinuationState::Future if following.starts_with("느냐") => {
            NEUNYA
        }
        ContinuationState::StemEnding | ContinuationState::Past
            if following.starts_with("던데") =>
        {
            DEONDE
        }
        ContinuationState::Past | ContinuationState::Future if following.starts_with("는데") => {
            NEUNDE
        }
        ContinuationState::Eu if following.starts_with("시냐") => SI_NYA,
        ContinuationState::Eu if following.starts_with("시느냐") => SI_NEUNYA,
        ContinuationState::Eu if following.starts_with("셨냐") => SYEOT_NYA,
        ContinuationState::Eu if following.starts_with("셨느냐") => SYEOT_NEUNYA,
        ContinuationState::Eu if following.starts_with("시겠냐") => SIGET_NYA,
        ContinuationState::Eu if following.starts_with("시겠느냐") => SIGET_NEUNYA,
        ContinuationState::Eu if following.starts_with("신데") => SINDE,
        ContinuationState::Eu if following.starts_with("시는데") => SINEUNDE,
        ContinuationState::Eu if following.starts_with("셨는데") => SYEOTNEUNDE,
        ContinuationState::Eu if following.starts_with("시겠는데") => SIGETNEUNDE,
        ContinuationState::Past | ContinuationState::Future if following.starts_with("으리") => {
            PREFINAL_PROSPECTIVE
        }
        ContinuationState::Eu if following.starts_with('리') => PROSPECTIVE,
        ContinuationState::Eu if following.starts_with("시리") => HONORIFIC_PROSPECTIVE,
        ContinuationState::Eu if following.starts_with("셨으리") => HONORIFIC_PAST_PROSPECTIVE,
        ContinuationState::Past => PAST_RIEUL_FINAL,
        ContinuationState::Eu => EU_RIEUL_FINAL,
        _ => &[],
    };

    let suffix = candidates
        .iter()
        .chain(additional_endings)
        .filter(|suffix| {
            following.starts_with(suffix.surface)
                && (pos.is_action()
                    || !suffix.rules.iter().any(|rule| {
                        matches!(
                            *rule,
                            "ending.imperative-ra"
                                | "ending.imperative-sio"
                                | "ending.imperative-sio-short"
                                | "ending.imperative-formal"
                                | "ending.imperative-formal-short"
                                | "ending.propositive"
                                | "ending.intentive-ryeo"
                                | "ending.intentive-adnominal"
                                | "ending.purpose-reo"
                                | "ending.present-adnominal"
                        )
                    }))
        })
        .max_by_key(|suffix| suffix.surface.len());
    if suffix.is_none()
        && matches!(
            state,
            ContinuationState::Eu
                | ContinuationState::StemEnding
                | ContinuationState::NieunEnding
                | ContinuationState::CompletedNieunEnding
                | ContinuationState::RieulFinal
                | ContinuationState::BieupFinal
                | ContinuationState::Prospective
        )
    {
        return None;
    }
    Some(matched(anchor.len(), suffix))
}

fn declarative_suffix(following: &str) -> Option<&'static Suffix> {
    match following.chars().next()? {
        '면' if following.starts_with(DECLARATIVE_MYEONSEO.surface) => Some(&DECLARATIVE_MYEONSEO),
        '면' => Some(&DECLARATIVE_CONDITIONAL),
        '는' if following.starts_with(DECLARATIVE_NEUNDE.surface) => Some(&DECLARATIVE_NEUNDE),
        '는' => Some(&DECLARATIVE_ADNOMINAL),
        '고' => Some(&DECLARATIVE_GO),
        '던' => Some(&DECLARATIVE_RETROSPECTIVE),
        '니' => Some(&DECLARATIVE_NI),
        '며' => Some(&DECLARATIVE_MYEO),
        '지' => Some(&DECLARATIVE_JI),
        _ => None,
    }
}

fn matched(anchor_len: usize, suffix: Option<&Suffix>) -> PredicateContinuationMatch {
    let consumed_bytes = suffix.map_or(0, |suffix| suffix.surface.len());
    let rule_path = suffix.map_or_else(Vec::new, |suffix| {
        suffix.rules.iter().copied().map(RuleId::from).collect()
    });
    PredicateContinuationMatch {
        token_end: anchor_len + consumed_bytes,
        consumed_bytes,
        rule_path,
    }
}

const fn suffix(surface: &'static str, rules: &'static [&'static str]) -> Suffix {
    Suffix { surface, rules }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn past_state_consumes_a_bounded_suffix() {
        let matched = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Verb,
            "걸었",
            "습니다. 다음",
        )
        .expect("valid continuation");
        assert_eq!(matched.consumed_bytes, "습니다".len());
        assert_eq!(matched.token_end, "걸었습니다".len());
        assert_eq!(matched.rule_path[0].as_str(), "ending.polite-declarative");

        let informal = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Adjective,
            "좋았",
            "어요",
        )
        .expect("past polite continuation");
        assert_eq!(informal.token_end, "좋았어요".len());
        assert_eq!(informal.rule_path[0].as_str(), "ending.past-polite-yo");
    }

    #[test]
    fn prefinal_states_consume_eudoe_connectives() {
        for (state, anchor) in [
            (ContinuationState::Past, "치렀"),
            (ContinuationState::Future, "하겠"),
        ] {
            let matched = verify_predicate_continuation(
                state,
                PredicatePos::Verb,
                anchor,
                "으되 조건은 남는다",
            )
            .expect("eudoe continuation");
            assert_eq!(matched.token_end, format!("{anchor}으되").len());
            assert_eq!(matched.rule_path, [RuleId::from("ending.connective-eudoe")]);
        }
    }

    #[test]
    fn declarative_state_consumes_bounded_continuations() {
        for (following, surface, rule_id) in [
            ("고 말했다", "쓴다고", "ending.quotative-go"),
            ("는 말", "쓴다는", "ending.quotative-adnominal"),
            ("던 말", "쓴다던", "ending.quotative-retrospective"),
            ("면 알겠다", "쓴다면", "ending.conditional"),
            ("니 놀랍다", "쓴다니", "ending.quotative-ni"),
            ("며 나섰다", "쓴다며", "ending.quotative-myeo"),
            ("면서 미뤘다", "쓴다면서", "ending.quotative-myeonseo"),
            ("는데 기다렸다", "쓴다는데", "ending.quotative-neunde"),
            ("지?", "쓴다지", "ending.quotative-ji"),
        ] {
            let matched = verify_predicate_continuation(
                ContinuationState::Declarative,
                PredicatePos::Verb,
                "쓴다",
                following,
            )
            .expect("declarative continuation");
            assert_eq!(matched.token_end, surface.len());
            assert_eq!(matched.rule_path, [RuleId::from(rule_id)]);
        }

        let bare = verify_predicate_continuation(
            ContinuationState::Declarative,
            PredicatePos::Verb,
            "쓴다",
            " 말했다",
        )
        .expect("bare declarative");
        assert_eq!(bare.consumed_bytes, 0);

        let unsupported = verify_predicate_continuation(
            ContinuationState::Declarative,
            PredicatePos::Verb,
            "쓴다",
            "도 말했다",
        )
        .expect("completed declarative remains valid");
        assert_eq!(unsupported.consumed_bytes, 0);
    }

    #[test]
    fn eu_state_requires_a_licensed_suffix() {
        let matched = verify_predicate_continuation(
            ContinuationState::Eu,
            PredicatePos::Verb,
            "걸으",
            "셨다.",
        )
        .expect("valid continuation");
        assert_eq!(matched.token_end, "걸으셨다".len());
        assert!(
            verify_predicate_continuation(ContinuationState::Eu, PredicatePos::Verb, "걸으", "xyz")
                .is_none()
        );

        let connective = verify_predicate_continuation(
            ContinuationState::Eu,
            PredicatePos::Adjective,
            "좋으",
            "니까는 계속한다",
        )
        .expect("reason connective");
        assert_eq!(connective.token_end, "좋으니까는".len());
        assert_eq!(connective.consumed_bytes, "니까는".len());

        let contracted = verify_predicate_continuation(
            ContinuationState::Eu,
            PredicatePos::Adjective,
            "좋으",
            "니깐 계속한다",
        )
        .expect("contracted reason connective");
        assert_eq!(contracted.token_end, "좋으니깐".len());

        let prospective = verify_predicate_continuation(
            ContinuationState::Eu,
            PredicatePos::Verb,
            "얻으",
            "리라고 생각했다",
        )
        .expect("prospective quotative");
        assert_eq!(prospective.token_end, "얻으리라고".len());

        let prospective_final = verify_predicate_continuation(
            ContinuationState::Eu,
            PredicatePos::Verb,
            "않으",
            "리라 생각했다",
        )
        .expect("prospective final");
        assert_eq!(prospective_final.token_end, "않으리라".len());
        assert_eq!(
            prospective_final.rule_path,
            [RuleId::from("ending.prospective-final")]
        );
    }

    #[test]
    fn eu_state_consumes_honorific_imperative_and_past_continuations() {
        for (following, expected, rules) in [
            (
                "세요",
                "걸으세요",
                vec!["ending.honorific", "ending.polite-imperative"],
            ),
            (
                "셨고",
                "걸으셨고",
                vec!["ending.honorific", "ending.past", "ending.connective-go"],
            ),
            (
                "셨던",
                "걸으셨던",
                vec![
                    "ending.honorific",
                    "ending.past",
                    "ending.retrospective-adnominal",
                ],
            ),
        ] {
            let matched = verify_predicate_continuation(
                ContinuationState::Eu,
                PredicatePos::Verb,
                "걸으",
                following,
            )
            .expect("licensed honorific continuation");
            assert_eq!(matched.token_end, expected.len());
            assert_eq!(
                matched.rule_path,
                rules.into_iter().map(RuleId::from).collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn continuation_respects_ending_paths_and_pos_requirements() {
        let future = verify_predicate_continuation(
            ContinuationState::Eu,
            PredicatePos::Verb,
            "걸으",
            "시겠습니다",
        )
        .expect("honorific future path");
        assert_eq!(future.token_end, "걸으시겠습니다".len());

        assert!(
            verify_predicate_continuation(ContinuationState::AOrEo, PredicatePos::Verb, "가", "라")
                .is_some()
        );
        let adjective = verify_predicate_continuation(
            ContinuationState::AOrEo,
            PredicatePos::Adjective,
            "예뻐",
            "라",
        )
        .expect("completed vowel anchor remains valid");
        assert_eq!(adjective.consumed_bytes, 0);
    }

    #[test]
    fn terminal_and_completed_vowel_states_accept_a_boundary() {
        for state in [ContinuationState::Terminal, ContinuationState::AOrEo] {
            let matched = verify_predicate_continuation(state, PredicatePos::Verb, "걸어", " 갔다")
                .expect("boundary is valid");
            assert_eq!(matched.consumed_bytes, 0);
        }
    }

    #[test]
    fn uses_the_longest_vowel_and_future_continuations() {
        let aeo = verify_predicate_continuation(
            ContinuationState::AOrEo,
            PredicatePos::Verb,
            "걸어",
            "서도 좋다",
        )
        .expect("valid continuation");
        assert_eq!(aeo.token_end, "걸어서도".len());
        assert_eq!(aeo.rule_path.len(), 2);

        let future = verify_predicate_continuation(
            ContinuationState::Future,
            PredicatePos::Verb,
            "걷겠",
            "습니다",
        )
        .expect("valid continuation");
        assert_eq!(future.token_end, "걷겠습니다".len());
    }

    #[test]
    fn accepts_gold_retrospective_quotative_and_change_suffixes() {
        let retrospective = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Adjective,
            "예뻤",
            "던",
        )
        .expect("retrospective adnominal");
        assert_eq!(retrospective.consumed_bytes, "던".len());

        let quotative = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Verb,
            "되었",
            "다고",
        )
        .expect("quotative connective");
        assert_eq!(quotative.consumed_bytes, "다고".len());

        let past_adnominal = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Verb,
            "불렀",
            "을 때",
        )
        .expect("past adnominal");
        assert_eq!(past_adnominal.token_end, "불렀을".len());
        assert_eq!(
            past_adnominal.rule_path[0].as_str(),
            "ending.future-adnominal"
        );

        let changed = verify_predicate_continuation(
            ContinuationState::AOrEo,
            PredicatePos::Adjective,
            "빨라",
            "졌다",
        )
        .expect("change auxiliary");
        assert_eq!(changed.consumed_bytes, "졌다".len());
    }

    #[test]
    fn accepts_bounded_progression_auxiliary_suffixes() {
        let coordinate = verify_predicate_continuation(
            ContinuationState::AOrEo,
            PredicatePos::Verb,
            "망해",
            "가고 있다",
        )
        .expect("progression coordinate");
        assert_eq!(coordinate.consumed_bytes, "가고".len());
        assert_eq!(
            coordinate.rule_path,
            ["ending.auxiliary-gada", "ending.connective-go"]
                .into_iter()
                .map(RuleId::from)
                .collect::<Vec<_>>()
        );

        let required = verify_predicate_continuation(
            ContinuationState::AOrEo,
            PredicatePos::Verb,
            "만들어",
            "가야 한다",
        )
        .expect("progression requirement");
        assert_eq!(required.consumed_bytes, "가야".len());

        let unsupported = verify_predicate_continuation(
            ContinuationState::AOrEo,
            PredicatePos::Verb,
            "해",
            "가며 배운다",
        )
        .expect("completed vowel anchor remains valid");
        assert_eq!(unsupported.consumed_bytes, 0);
    }

    #[test]
    fn accepts_bounded_past_interrogative_suffixes() {
        let topicalized = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Verb,
            "했",
            "느냐는 문제",
        )
        .expect("topicalized past interrogative");
        assert_eq!(topicalized.consumed_bytes, "느냐는".len());
        assert_eq!(
            topicalized.rule_path,
            ["ending.interrogative-neunya", "particle.topic"]
                .into_iter()
                .map(RuleId::from)
                .collect::<Vec<_>>()
        );

        let bare = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Verb,
            "먹었",
            "느냐?",
        )
        .expect("bare past interrogative");
        assert_eq!(bare.consumed_bytes, "느냐".len());

        let unsupported = verify_predicate_continuation(
            ContinuationState::Past,
            PredicatePos::Verb,
            "했",
            "느냐도 논점이다",
        )
        .expect("past interrogative remains valid");
        assert_eq!(unsupported.consumed_bytes, "느냐".len());
    }
}
