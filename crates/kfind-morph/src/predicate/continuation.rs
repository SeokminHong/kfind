use crate::{ContinuationState, PredicatePos, RuleId, has_final, has_rieul_final};

mod auxiliary;
mod hao;
mod humble;
mod report;
mod trial;
pub(super) use hao::{HAO_ENDINGS, HONORIFIC_HAO_AFTER_ACTION, HONORIFIC_HAO_ENDINGS, SOSEO};

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
const COPULA_RA_SUFFIXES: &[Suffix] = &[
    suffix("", &["ending.copula-ra-connective"]),
    suffix("고", &["ending.copula-quotative-go"]),
    suffix("고요", &["ending.copula-quotative-go", "ending.polite-yo"]),
    suffix("는", &["ending.copula-quotative-adnominal"]),
    suffix(
        "는군요",
        &[
            "ending.copula-quotative-adnominal",
            "ending.final-neun-exclamative",
            "ending.polite-yo",
        ],
    ),
    suffix("나", &["ending.copula-ra-final"]),
    suffix("네", &["ending.copula-ra-final"]),
    suffix("느니", &["ending.copula-ra-connective"]),
    suffix("도", &["ending.copula-ra-connective"]),
    suffix("니", &["ending.copula-ra-final"]),
    suffix("니까", &["ending.copula-ra-final"]),
    suffix("며", &["ending.copula-ra-connective"]),
    suffix("면", &["ending.copula-ra-connective"]),
    suffix("면서", &["ending.copula-ra-connective"]),
    suffix("서", &["ending.copula-ra-connective"]),
    suffix("손", &["ending.copula-ra-connective"]),
    suffix("야", &["ending.copula-ra-connective"]),
    suffix("야만", &["ending.copula-ra-connective"]),
    suffix("오", &["ending.copula-ra-final"]),
    suffix("지", &["ending.copula-ra-final"]),
];

const COPULA_REPORTING_AND_RO_ENDINGS: &[Suffix] = &[
    suffix("란", &["ending.copula-quotative-adnominal"]),
    suffix("란다", &["ending.copula-reporting-final"]),
    suffix("람", &["ending.copula-reporting-final"]),
    suffix("랍니까", &["ending.copula-reporting-final"]),
    suffix("랍니다", &["ending.copula-reporting-final"]),
    suffix("랍시고", &["ending.copula-reporting-connective"]),
    suffix("래", &["ending.copula-reporting-final"]),
    suffix(
        "래요",
        &["ending.copula-reporting-final", "ending.polite-yo"],
    ),
    suffix("로고", &["ending.copula-ro-final"]),
    suffix("로구나", &["ending.copula-ro-final"]),
    suffix("로구려", &["ending.copula-ro-final"]),
    suffix("로구먼", &["ending.copula-ro-final"]),
    suffix("로군", &["ending.copula-ro-final"]),
    suffix("로군요", &["ending.copula-ro-final", "ending.polite-yo"]),
    suffix("로다", &["ending.copula-ro-final"]),
    suffix("로되", &["ending.copula-ro-connective"]),
    suffix("로라", &["ending.copula-ro-final"]),
    suffix("로서니", &["ending.copula-ro-connective"]),
    suffix("로세", &["ending.copula-ro-final"]),
];

const GO_YO: Suffix = suffix("고요", &["ending.connective-go"]);
const DAM: Suffix = suffix("담", &["ending.self-question-dam"]);
const NAM: Suffix = suffix("남", &["ending.self-question-nam"]);
const NEUNDAM: Suffix = suffix("는담", &["ending.self-question-dam"]);
pub(super) const HONORIFIC_DAM: Suffix =
    suffix("시담", &["ending.honorific", "ending.self-question-dam"]);
const HONORIFIC_NDAM: Suffix = suffix("신담", &["ending.honorific", "ending.self-question-dam"]);
const PURPOSE_REO: Suffix = suffix("러", &["ending.purpose-reo"]);
const PROMISE_MA: Suffix = suffix("마", &["ending.promise-ma"]);
const HONORIFIC_PURPOSE_REO: Suffix = suffix("시러", &["ending.honorific", "ending.purpose-reo"]);

const COPULA_COMMON_ENDINGS: &[Suffix] = &[
    suffix("랴", &["ending.rya"]),
    suffix("렷다", &["ending.ryeotda"]),
    suffix("게", &["ending.copula-ge"]),
    suffix("고말고", &["ending.copula-common-final"]),
    suffix("고요", &["ending.connective-go", "ending.polite-yo"]),
    suffix("구나", &["ending.copula-common-final"]),
    suffix("구려", &["ending.copula-common-final"]),
    suffix("구먼", &["ending.copula-common-final"]),
    suffix("군", &["ending.copula-common-final"]),
    suffix("군요", &["ending.copula-common-final", "ending.polite-yo"]),
    suffix("길래", &["ending.copula-common-connective"]),
    suffix("네", &["ending.copula-common-final"]),
    suffix("네요", &["ending.copula-common-final", "ending.polite-yo"]),
    suffix("니", &["ending.copula-common-connective"]),
    suffix("니까", &["ending.copula-common-connective"]),
    suffix("니까는", &["ending.copula-common-connective"]),
    suffix("니깐", &["ending.copula-common-connective"]),
    suffix("니라", &["ending.copula-common-final"]),
    suffix("면", &["ending.copula-common-connective"]),
    suffix("면서", &["ending.copula-common-connective"]),
    suffix("세요", &["ending.honorific", "ending.copula-polite"]),
    suffix("야", &["ending.copula-ya"]),
    suffix("어요", &["ending.copula-polite"]),
    suffix("에요", &["ending.copula-polite"]),
    suffix("자", &["ending.copula-common-connective"]),
    suffix("죠", &["ending.copula-common-final"]),
    suffix("지요", &["ending.copula-common-final"]),
];

fn copula_common_omission(surface: &str) -> bool {
    matches!(
        surface,
        "게" | "랴"
            | "렷다"
            | "구나"
            | "구려"
            | "구먼"
            | "군"
            | "군요"
            | "더군"
            | "더군요"
            | "네"
            | "네요"
            | "니"
            | "니까"
            | "니까는"
            | "니깐"
            | "냐"
            | "면"
            | "면서"
            | "죠"
            | "지요"
            | "예요"
            | "여요"
    )
}

pub(super) fn is_copula_omission_ending(surface: &str) -> bool {
    surface.strip_prefix('라').is_some_and(|tail| {
        COPULA_RA_SUFFIXES
            .iter()
            .any(|ending| ending.surface == tail)
    }) || copula_common_omission(surface)
        || COPULA_REPORTING_AND_RO_ENDINGS
            .iter()
            .any(|ending| ending.surface == surface)
}
const HONORIFIC_RO_ENDINGS: &[Suffix] = &[
    suffix("시로구나", &["ending.honorific", "ending.copula-ro-final"]),
    suffix("시로구려", &["ending.honorific", "ending.copula-ro-final"]),
    suffix("시로구먼", &["ending.honorific", "ending.copula-ro-final"]),
    suffix("시로군", &["ending.honorific", "ending.copula-ro-final"]),
    suffix(
        "시로군요",
        &[
            "ending.honorific",
            "ending.copula-ro-final",
            "ending.polite-yo",
        ],
    ),
    suffix(
        "시로되",
        &["ending.honorific", "ending.copula-ro-connective"],
    ),
];
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
    suffix("가고", &["ending.auxiliary-gada", "ending.connective-go"]),
    suffix("가야", &["ending.auxiliary-gada", "ending.connective-ya"]),
    suffix(
        "가면서",
        &["ending.auxiliary-gada", "ending.simultaneous-myeonseo"],
    ),
    suffix("도", &["ending.connective-do"]),
    suffix("야", &["ending.connective-ya"]),
    suffix("요", &["ending.polite-yo"]),
    suffix("라", &["ending.imperative-ra"]),
];

const A_OR_EO_SEO_SUFFIXES: &[Suffix] = &[
    suffix("서도", &["ending.aoeo-seo", "particle.additive"]),
    suffix("서", &["ending.aoeo-seo"]),
    suffix("서야", &["ending.aoeo-seoya"]),
];

const GUN: Suffix = suffix("군", &["ending.final-gun"]);
const GUN_YO: Suffix = suffix("군요", &["ending.final-gun", "ending.polite-yo"]);
const NEUN_GUN: Suffix = suffix("군", &["ending.final-neun-exclamative"]);
const NEUN_GUN_YO: Suffix = suffix(
    "군요",
    &["ending.final-neun-exclamative", "ending.polite-yo"],
);
const NEUN_GUNA: Suffix = suffix("구나", &["ending.final-neun-exclamative"]);
const HONORIFIC_NEUN_GUN: Suffix = suffix(
    "시는군",
    &["ending.honorific", "ending.final-neun-exclamative"],
);
const HONORIFIC_NEUN_GUN_YO: Suffix = suffix(
    "시는군요",
    &[
        "ending.honorific",
        "ending.final-neun-exclamative",
        "ending.polite-yo",
    ],
);
const HONORIFIC_GUN_ENDINGS: &[Suffix] = &[
    suffix("시군", &["ending.honorific", "ending.final-gun"]),
    suffix(
        "셨군",
        &["ending.honorific", "ending.past", "ending.final-gun"],
    ),
    suffix(
        "시겠군",
        &["ending.honorific", "ending.future", "ending.final-gun"],
    ),
    suffix(
        "시군요",
        &["ending.honorific", "ending.final-gun", "ending.polite-yo"],
    ),
    suffix(
        "셨군요",
        &[
            "ending.honorific",
            "ending.past",
            "ending.final-gun",
            "ending.polite-yo",
        ],
    ),
    suffix(
        "시겠군요",
        &[
            "ending.honorific",
            "ending.future",
            "ending.final-gun",
            "ending.polite-yo",
        ],
    ),
];

const DAGA: Suffix = suffix("다가", &["ending.connective-daga"]);

pub(super) const STEM_FIXED_ENDINGS: &[Suffix] = &[
    suffix("고", &["ending.connective-go"]),
    suffix("지", &["ending.connective-ji"]),
    suffix("게", &["ending.adverbial-ge"]),
    suffix("던", &["ending.retrospective-adnominal"]),
    suffix("더니", &["ending.retrospective-connective"]),
    suffix("더라도", &["ending.concessive-deorado"]),
    suffix("도록", &["ending.purpose-dorok"]),
    DAGA,
];

macro_rules! daganeun_endings {
    ($prefix:literal $(, $prior:literal)*) => {
        &[
            suffix(concat!($prefix, "다가는"), &[$($prior,)* "ending.connective-daganeun"]),
            suffix(concat!($prefix, "다간"), &[$($prior,)* "ending.connective-daganeun"]),
            suffix(concat!($prefix, "단"), &[$($prior,)* "ending.connective-daganeun"]),
        ]
    };
}

const DAGANEUN_ENDINGS: &[Suffix] = daganeun_endings!("");
const SI_DAGANEUN_ENDINGS: &[Suffix] = daganeun_endings!("시", "ending.honorific");
const SYEOT_DAGANEUN_ENDINGS: &[Suffix] =
    daganeun_endings!("셨", "ending.honorific", "ending.past");
const SIGET_DAGANEUN_ENDINGS: &[Suffix] =
    daganeun_endings!("시겠", "ending.honorific", "ending.future");

fn starts_with_daganeun(following: &str) -> bool {
    following.starts_with("다간") || following.starts_with("다가는") || following.starts_with('단')
}
const DEON_RECOLLECTION: &[Suffix] = &[
    suffix("던가", &["ending.retrospective-question-deon"]),
    suffix("던감", &["ending.retrospective-question-deon"]),
    suffix("던들", &["ending.retrospective-connective-deon"]),
    suffix("던지", &["ending.retrospective-connective-deon"]),
];

macro_rules! retrospective_endings {
    ($prefix:literal $(, $prior:literal)*) => {
        &[
            suffix(concat!($prefix, "더라"), &[$($prior,)* "ending.retrospective-deora"]),
            suffix(concat!($prefix, "더라오"), &[$($prior,)* "ending.retrospective-deora", "ending.hao-reporting-dao"]),
            suffix(concat!($prefix, "더라고"), &[$($prior,)* "ending.retrospective-deora", "ending.quotative-go"]),
            suffix(concat!($prefix, "더라고요"), &[$($prior,)* "ending.retrospective-deora", "ending.quotative-go", "ending.polite-yo"]),
            suffix(concat!($prefix, "더구나"), &[$($prior,)* "ending.retrospective-exclamatory"]),
            suffix(concat!($prefix, "더구려"), &[$($prior,)* "ending.retrospective-exclamatory"]),
            suffix(concat!($prefix, "더구먼"), &[$($prior,)* "ending.retrospective-exclamatory"]),
            suffix(concat!($prefix, "더군"), &[$($prior,)* "ending.final-deogun"]),
            suffix(concat!($prefix, "더냐"), &[$($prior,)* "ending.retrospective-direct-question"]),
            suffix(concat!($prefix, "더뇨"), &[$($prior,)* "ending.retrospective-direct-question"]),
            suffix(concat!($prefix, "더니이까"), &[$($prior,)* "ending.retrospective-direct-question"]),
            suffix(concat!($prefix, "더이까"), &[$($prior,)* "ending.retrospective-direct-question"]),
            suffix(concat!($prefix, "더니라"), &[$($prior,)* "ending.retrospective-direct-final"]),
            suffix(concat!($prefix, "더니이다"), &[$($prior,)* "ending.retrospective-direct-final"]),
            suffix(concat!($prefix, "더이다"), &[$($prior,)* "ending.retrospective-direct-final"]),
            suffix(concat!($prefix, "더람"), &[$($prior,)* "ending.retrospective-direct-final"]),
            suffix(concat!($prefix, "더니"), &[$($prior,)* "ending.retrospective-connective"]),
            suffix(concat!($prefix, "더니마는"), &[$($prior,)* "ending.retrospective-contrast"]),
            suffix(concat!($prefix, "더니만"), &[$($prior,)* "ending.retrospective-contrast"]),
            suffix(concat!($prefix, "더라나"), &[$($prior,)* "ending.retrospective-deora-report"]),
            suffix(concat!($prefix, "더라니"), &[$($prior,)* "ending.retrospective-deora-report"]),
            suffix(concat!($prefix, "더라니까"), &[$($prior,)* "ending.retrospective-deora-report"]),
            suffix(concat!($prefix, "더라며"), &[$($prior,)* "ending.retrospective-deora-report"]),
            suffix(concat!($prefix, "더라면서"), &[$($prior,)* "ending.retrospective-deora-report"]),
            suffix(concat!($prefix, "더라지"), &[$($prior,)* "ending.retrospective-deora-report"]),
            suffix(concat!($prefix, "더라면"), &[$($prior,)* "ending.retrospective-deora-condition"]),
            suffix(concat!($prefix, "더라손"), &[$($prior,)* "ending.retrospective-deora-condition"]),
            suffix(concat!($prefix, "더라도"), &[$($prior,)* "ending.concessive-deorado"]),
            suffix(concat!($prefix, "더군요"), &[$($prior,)* "ending.final-deogun", "ending.polite-yo"]),
            suffix(concat!($prefix, "더라니까요"), &[$($prior,)* "ending.retrospective-deora-report", "ending.polite-yo"]),
        ]
    };
}

const RETROSPECTIVE: &[Suffix] = retrospective_endings!("");
const GET_RETROSPECTIVE: &[Suffix] = retrospective_endings!("겠", "ending.future");
const SI_RETROSPECTIVE: &[Suffix] = retrospective_endings!("시", "ending.honorific");
const SYEOT_RETROSPECTIVE: &[Suffix] =
    retrospective_endings!("셨", "ending.honorific", "ending.past");
const SIGET_RETROSPECTIVE: &[Suffix] =
    retrospective_endings!("시겠", "ending.honorific", "ending.future");
const RYEO_CONJECTURE: &[Suffix] = &[
    suffix("려나", &["ending.conjecture-ryeo"]),
    suffix("려니", &["ending.conjecture-ryeo"]),
    suffix("려니와", &["ending.concessive-ryeoniwa"]),
];

pub(super) const FAMILIAR_QUESTION: Suffix = suffix("니", &["ending.interrogative-ni"]);
pub(super) const NIRA: Suffix = suffix("니라", &["ending.archaic-nira"]);
pub(super) const NEUNIRA: Suffix = suffix("느니라", &["ending.archaic-nira"]);
pub(super) const HONORIFIC_NIRA: &[Suffix] = &[
    suffix("시니라", &["ending.honorific", "ending.archaic-nira"]),
    suffix("시느니라", &["ending.honorific", "ending.archaic-nira"]),
];
const JIMAN: Suffix = suffix("지만", &["ending.connective-jiman"]);
const GEONA: Suffix = suffix("거나", &["ending.connective-geona"]);
const IMMEDIATE: Suffix = suffix("자마자", &["ending.connective-jamaja"]);
const HONORIFIC_IMMEDIATE: Suffix = suffix(
    "시자마자",
    &["ending.honorific", "ending.connective-jamaja"],
);
const NEGATIVE_TOPIC: Suffix = suffix("진", &["ending.connective-ji", "particle.topic"]);
const HONORIFIC_NEGATIVE_TOPIC: Suffix = suffix(
    "시진",
    &["ending.honorific", "ending.connective-ji", "particle.topic"],
);
const CONCESSIVE_NA: Suffix = suffix("나", &["ending.connective-na"]);
const INTERROGATIVE_NA_YO: Suffix = suffix("나요", &["ending.interrogative-na"]);
const HONORIFIC_INTERROGATIVE_NA_YO: Suffix =
    suffix("시나요", &["ending.honorific", "ending.interrogative-na"]);
const CONCESSIVE_EUNA: Suffix = suffix("으나", &["ending.connective-na"]);
const HONORIFIC_COORDINATING_ENDINGS: &[Suffix] = &[
    suffix("시지만", &["ending.honorific", "ending.connective-jiman"]),
    suffix("시거나", &["ending.honorific", "ending.connective-geona"]),
    suffix("시나", &["ending.honorific", "ending.connective-na"]),
];
const HONORIFIC_FAMILIAR_QUESTION: Suffix =
    suffix("시니", &["ending.honorific", "ending.interrogative-ni"]);

const NE: Suffix = suffix("네", &["ending.final-ne"]);
const NE_YO: Suffix = suffix("네요", &["ending.final-ne", "ending.polite-yo"]);
pub(super) const HONORIFIC_NE_ENDINGS: [Suffix; 2] = [
    suffix("시네", &["ending.honorific", "ending.final-ne"]),
    suffix(
        "시네요",
        &["ending.honorific", "ending.final-ne", "ending.polite-yo"],
    ),
];
const HONORIFIC_PREFINAL_NE_ENDINGS: &[Suffix] = &[
    suffix(
        "셨네",
        &["ending.honorific", "ending.past", "ending.final-ne"],
    ),
    suffix(
        "셨네요",
        &[
            "ending.honorific",
            "ending.past",
            "ending.final-ne",
            "ending.polite-yo",
        ],
    ),
    suffix(
        "시겠네",
        &["ending.honorific", "ending.future", "ending.final-ne"],
    ),
    suffix(
        "시겠네요",
        &[
            "ending.honorific",
            "ending.future",
            "ending.final-ne",
            "ending.polite-yo",
        ],
    ),
];

const PAST_SUFFIXES: &[Suffix] = &[
    PREFINAL_DAMYEON,
    PREFINAL_DAMYEONSEO,
    PREFINAL_EUNI,
    PREFINAL_EUNIKKA,
    PREFINAL_EUNIKKANEUN,
    PREFINAL_EUNIKKAN,
    DAGA,
    suffix("자", &["ending.concessive-eotja"]),
    NAM,
    NOMINALIZER_EUM,
    NOMINALIZER_GI,
    NEGATIVE_TOPIC,
    GEONA,
    CONCESSIVE_EUNA,
    NE,
    NE_YO,
    PLAIN_QUOTATIVE_ADNOMINAL,
    GUN,
    GUN_YO,
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
    suffix("으면", &["ending.conditional"]),
    suffix("으며", &["ending.coordinate-myeo"]),
    JIMAN,
    suffix("는데", &["ending.connective-neunde"]),
    suffix("다고", &["ending.quotative-go"]),
    suffix("던", &["ending.retrospective-adnominal"]),
    suffix("을", &["ending.future-adnominal"]),
    suffix("다", &["ending.final-da"]),
    suffix("고", &["ending.connective-go"]),
    suffix("어", &["ending.prefinal-eo"]),
    suffix("어도", &["ending.aoeo", "ending.connective-do"]),
    suffix("어야", &["ending.aoeo", "ending.connective-ya"]),
    suffix("어요", &["ending.past-polite-yo"]),
    suffix("게", &["ending.final-ge"]),
];

const GEOTDA: &[Suffix] = &[suffix("것다", &["ending.geotda"])];
const GOSEO: &[Suffix] = &[
    suffix("고서", &["ending.connective-goseo"]),
    suffix("고서야", &["ending.connective-goseo"]),
];
const HONORIFIC_GOSEO: &[Suffix] = &[
    suffix("시고서", &["ending.honorific", "ending.connective-goseo"]),
    suffix("시고서야", &["ending.honorific", "ending.connective-goseo"]),
];

const MAE: Suffix = suffix("매", &["ending.connective-mae"]);
const EUMAE: Suffix = suffix("으매", &["ending.connective-mae"]);
const HONORIFIC_MAE: Suffix = suffix("시매", &["ending.honorific", "ending.connective-mae"]);
const MEURO: &[Suffix] = &[suffix("므로", &["ending.connective-meuro"])];
const EUMEURO: &[Suffix] = &[suffix("으므로", &["ending.connective-meuro"])];
const HONORIFIC_MEURO_ENDINGS: &[Suffix] = &[
    suffix("시므로", &["ending.honorific", "ending.connective-meuro"]),
    suffix(
        "셨으므로",
        &["ending.honorific", "ending.past", "ending.connective-meuro"],
    ),
    suffix(
        "시겠으므로",
        &[
            "ending.honorific",
            "ending.future",
            "ending.connective-meuro",
        ],
    ),
];

const NOMINALIZER_EUM: Suffix = suffix("음", &["ending.nominalizer"]);
const NOMINALIZER_GI: Suffix = suffix("기", &["ending.nominalizer-gi"]);
const NOMINALIZER_GIL: Suffix = suffix("길", &["ending.nominalizer-gi", "particle.object"]);
const HONORIFIC_NOMINALIZER_GIL: Suffix = suffix(
    "시길",
    &[
        "ending.honorific",
        "ending.nominalizer-gi",
        "particle.object",
    ],
);

const FUTURE_SUFFIXES: &[Suffix] = &[
    PREFINAL_DAMYEON,
    PREFINAL_DAMYEONSEO,
    PREFINAL_EUNI,
    PREFINAL_EUNIKKA,
    PREFINAL_EUNIKKANEUN,
    PREFINAL_EUNIKKAN,
    NAM,
    suffix("지", &["ending.confirmation-jiyo"]),
    NOMINALIZER_EUM,
    NOMINALIZER_GI,
    NEGATIVE_TOPIC,
    GEONA,
    CONCESSIVE_EUNA,
    NE,
    NE_YO,
    PLAIN_QUOTATIVE_ADNOMINAL,
    GUN,
    GUN_YO,
    suffix("습니까", &["ending.formal-interrogative"]),
    suffix("습디까", &["ending.retrospective-interrogative"]),
    suffix("습디다", &["ending.retrospective-declarative"]),
    suffix("습니다", &["ending.polite-declarative"]),
    suffix("으되", &["ending.connective-eudoe"]),
    JIMAN,
    suffix("는데", &["ending.connective-neunde"]),
    suffix("다", &["ending.final-da"]),
    suffix("고", &["ending.connective-go"]),
    suffix("어", &["ending.prefinal-eo"]),
    suffix("어요", &["ending.aoeo", "ending.polite-yo"]),
];

const PREFINAL_EUNI: Suffix = suffix("으니", &["ending.connective-ni"]);
const PREFINAL_EUNIKKA: Suffix = suffix("으니까", &["ending.connective-ni"]);
const PREFINAL_EUNIKKANEUN: Suffix = suffix("으니까는", &["ending.connective-ni"]);
const PREFINAL_EUNIKKAN: Suffix = suffix("으니깐", &["ending.connective-ni"]);
const DECLARATIVE_MYEONSEO: Suffix = suffix("면서", &["ending.quotative-myeonseo"]);
const DECLARATIVE_NEUNDE: Suffix = suffix("는데", &["ending.quotative-neunde"]);
const DECLARATIVE_GO: Suffix = suffix("고", &["ending.quotative-go"]);
const DECLARATIVE_ADNOMINAL: Suffix = suffix("는", &["ending.quotative-adnominal"]);
const PLAIN_QUOTATIVE_ADNOMINAL: Suffix = suffix("다는", &["ending.quotative-adnominal"]);
const DECLARATIVE_NEUN_GUN_YO: Suffix = suffix(
    "는군요",
    &[
        "ending.quotative-adnominal",
        "ending.final-neun-exclamative",
        "ending.polite-yo",
    ],
);
const DECLARATIVE_RETROSPECTIVE: Suffix = suffix("던", &["ending.quotative-retrospective"]);
const DECLARATIVE_DEONI: Suffix = suffix("더니", &["ending.retrospective-connective"]);
const DECLARATIVE_DEONIMAN: Suffix = suffix("더니만", &["ending.retrospective-contrast"]);
const DECLARATIVE_DEONIMANEUN: Suffix = suffix("더니마는", &["ending.retrospective-contrast"]);
const DECLARATIVE_CONDITIONAL: Suffix = suffix("면", &["ending.conditional"]);
const PREFINAL_DAMYEON: Suffix = suffix("다면", &["ending.conditional"]);
const PREFINAL_DAMYEONSEO: Suffix = suffix("다면서", &["ending.quotative-myeonseo"]);
const DECLARATIVE_NI: Suffix = suffix("니", &["ending.quotative-ni"]);
const DECLARATIVE_MYEO: Suffix = suffix("며", &["ending.quotative-myeo"]);
const DECLARATIVE_JI: Suffix = suffix("지", &["ending.quotative-ji"]);
const DECLARATIVE_NE: Suffix = suffix("네", &["ending.final-ne"]);
const DECLARATIVE_NE_YO: Suffix = suffix("네요", &["ending.final-ne", "ending.polite-yo"]);
const DECLARATIVE_O: Suffix = suffix("오", &["ending.hao-reporting-dao"]);
const DAO: &[Suffix] = &[suffix("다오", &["ending.hao-reporting-dao"])];
const HONORIFIC_DAO: &[Suffix] = &[
    suffix(
        "신다오",
        &[
            "ending.honorific",
            "ending.declarative",
            "ending.hao-reporting-dao",
        ],
    ),
    suffix("시다오", &["ending.honorific", "ending.hao-reporting-dao"]),
    suffix(
        "셨다오",
        &[
            "ending.honorific",
            "ending.past",
            "ending.hao-reporting-dao",
        ],
    ),
    suffix(
        "시겠다오",
        &[
            "ending.honorific",
            "ending.future",
            "ending.hao-reporting-dao",
        ],
    ),
];
const HONORIFIC_NIEUN_DAO: &[Suffix] = &[suffix(
    "다오",
    &["ending.declarative", "ending.hao-reporting-dao"],
)];
const NEGATIVE_RAO: &[Suffix] = &[suffix("라오", &["ending.copula-ra-final"])];

const JIYO: Suffix = suffix("지요", &["ending.confirmation-jiyo"]);
const JYO: Suffix = suffix("죠", &["ending.confirmation-jiyo"]);
const CONFIRMATION_JIYO: &[Suffix] = &[JIYO, JYO];
const HONORIFIC_JIYO: &[Suffix] = &[
    suffix("시지요", &["ending.honorific", "ending.confirmation-jiyo"]),
    suffix("시죠", &["ending.honorific", "ending.confirmation-jiyo"]),
    suffix(
        "셨지요",
        &[
            "ending.honorific",
            "ending.past",
            "ending.confirmation-jiyo",
        ],
    ),
    suffix(
        "셨죠",
        &[
            "ending.honorific",
            "ending.past",
            "ending.confirmation-jiyo",
        ],
    ),
    suffix(
        "시겠지요",
        &[
            "ending.honorific",
            "ending.future",
            "ending.confirmation-jiyo",
        ],
    ),
    suffix(
        "시겠죠",
        &[
            "ending.honorific",
            "ending.future",
            "ending.confirmation-jiyo",
        ],
    ),
    suffix(
        "셨겠지요",
        &[
            "ending.honorific",
            "ending.past",
            "ending.future",
            "ending.confirmation-jiyo",
        ],
    ),
    suffix(
        "셨겠죠",
        &[
            "ending.honorific",
            "ending.past",
            "ending.future",
            "ending.confirmation-jiyo",
        ],
    ),
];
const PAST_FUTURE_JIYO: &[Suffix] = &[
    suffix("겠지요", &["ending.future", "ending.confirmation-jiyo"]),
    suffix("겠죠", &["ending.future", "ending.confirmation-jiyo"]),
];

const HONORIFIC_FORMAL: Suffix =
    suffix("십시오", &["ending.honorific", "ending.imperative-formal"]);
const HONORIFIC_FORMAL_SHORT: Suffix = suffix(
    "십쇼",
    &["ending.honorific", "ending.imperative-formal-short"],
);
const HONORIFIC_PROPOSITIVE: Suffix = suffix("십시다", &["ending.honorific", "ending.propositive"]);
const RYEOM_IMPERATIVES: &[Suffix] = &[
    suffix("렴", &["ending.imperative-ryeom"]),
    suffix("려무나", &["ending.imperative-ryeom"]),
];
const HONORIFIC_EARNEST_REQUEST: Suffix = suffix(
    "십시사",
    &["ending.honorific", "ending.earnest-request-sisa"],
);
pub(super) const INTENTIVE: Suffix = suffix("려", &["ending.intentive-ryeo"]);
const INTENTIVE_RYEOGO: Suffix = suffix("려고", &["ending.intentive-ryeogo"]);
const HONORIFIC_INTENTIVE_RYEOGO: Suffix =
    suffix("시려고", &["ending.honorific", "ending.intentive-ryeogo"]);
pub(super) const CONDITIONAL: Suffix = suffix("면", &["ending.conditional"]);
pub(super) const COORDINATE: Suffix = suffix("며", &["ending.coordinate-myeo"]);
pub(super) const INTENTIVE_CONDITIONAL: Suffix = suffix("려면", &["ending.intentive-ryeomyeon"]);
pub(super) const INTENTIVE_RYEODEUN: Suffix = suffix("려든", &["ending.intentive-ryeodeun"]);
pub(super) const HONORIFIC_INTENTIVE_CONDITIONAL: Suffix = suffix(
    "시려면",
    &["ending.honorific", "ending.intentive-ryeomyeon"],
);

pub(super) const HONORIFIC_BIEUP_ENDINGS: &[Suffix] = &[
    HONORIFIC_FORMAL,
    HONORIFIC_FORMAL_SHORT,
    HONORIFIC_PROPOSITIVE,
    HONORIFIC_EARNEST_REQUEST,
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

pub(super) const POLITE_HONORIFIC_ENDINGS: &[Suffix] = &[
    suffix("시어요", &["ending.honorific", "ending.polite-imperative"]),
    suffix("셔요", &["ending.honorific", "ending.polite-imperative"]),
    suffix("세요", &["ending.honorific", "ending.polite-imperative"]),
];

pub(super) const HONORIFIC_VOWEL_CONNECTIVES: &[Suffix] = &[
    suffix("시어서", &["ending.honorific", "ending.aoeo-seo"]),
    suffix(
        "셔서",
        &["ending.honorific", "contraction.i-eo", "ending.aoeo-seo"],
    ),
    suffix(
        "시어도",
        &["ending.honorific", "ending.aoeo", "ending.connective-do"],
    ),
    suffix(
        "셔도",
        &[
            "ending.honorific",
            "contraction.i-eo",
            "ending.aoeo",
            "ending.connective-do",
        ],
    ),
];

const EU_SUFFIXES: &[Suffix] = &[
    suffix("시남", &["ending.honorific", "ending.self-question-nam"]),
    suffix("시것다", &["ending.honorific", "ending.geotda"]),
    suffix(
        "셨것다",
        &["ending.honorific", "ending.past", "ending.geotda"],
    ),
    suffix("시게", &["ending.honorific", "ending.adverbial-ge"]),
    suffix(
        "셨게",
        &["ending.honorific", "ending.past", "ending.final-ge"],
    ),
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
    HONORIFIC_EARNEST_REQUEST,
    INTENTIVE,
    INTENTIVE_RYEOGO,
    HONORIFIC_INTENTIVE_RYEOGO,
    INTENTIVE_CONDITIONAL,
    INTENTIVE_RYEODEUN,
    HONORIFIC_INTENTIVE_CONDITIONAL,
    suffix("면서", &["ending.simultaneous-myeonseo"]),
    suffix("려던", &["ending.intentive-adnominal"]),
    PURPOSE_REO,
    HONORIFIC_PURPOSE_REO,
    CONCESSIVE_NA,
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
        "셨어",
        &["ending.honorific", "ending.past", "ending.prefinal-eo"],
    ),
    suffix(
        "셨어요",
        &["ending.honorific", "ending.past", "ending.past-polite-yo"],
    ),
    suffix(
        "시겠어",
        &["ending.honorific", "ending.future", "ending.prefinal-eo"],
    ),
    suffix(
        "시겠어요",
        &[
            "ending.honorific",
            "ending.future",
            "ending.aoeo",
            "ending.polite-yo",
        ],
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
    suffix("십니다", &["ending.honorific", "ending.polite-declarative"]),
    suffix("시다", &["ending.honorific", "ending.final-da"]),
    suffix("시는", &["ending.honorific", "ending.present-adnominal"]),
    suffix("시고", &["ending.honorific", "ending.connective-go"]),
    suffix("시면", &["ending.honorific", "ending.conditional"]),
    suffix(
        "시면서",
        &["ending.honorific", "ending.simultaneous-myeonseo"],
    ),
    suffix("신", &["ending.honorific", "ending.past-adnominal"]),
    suffix("실", &["ending.honorific", "ending.future-adnominal"]),
    CONDITIONAL,
    COORDINATE,
    suffix("니까는", &["ending.connective-ni"]),
    suffix("니까", &["ending.connective-ni"]),
    suffix("니깐", &["ending.connective-ni"]),
    suffix("니", &["ending.connective-ni"]),
];

macro_rules! rieul_endings {
    ($prefix:literal, [$($prior:literal),*] $(, $extension:expr)* $(,)?) => {
        &[
            $($extension,)*
            suffix(concat!($prefix, "지"), &[$($prior,)* "ending.doubt-rieulji"]),
            suffix(concat!($prefix, "지요"), &[$($prior,)* "ending.doubt-rieulji"]),
            suffix(concat!($prefix, "는지"), &[$($prior,)* "ending.doubt-rieulneunji"]),
            suffix(concat!($prefix, "는지요"), &[$($prior,)* "ending.doubt-rieulneunji"]),
            suffix(concat!($prefix, "걸"), &[$($prior,)* "ending.conjecture-rieulgeol"]),
            suffix(concat!($prefix, "걸요"), &[$($prior,)* "ending.conjecture-rieulgeol"]),
            suffix(concat!($prefix, "까"), &[$($prior,)* "ending.interrogative-rieulkka"]),
            suffix(concat!($prefix, "꼬"), &[$($prior,)* "ending.interrogative-rieulkko"]),
            suffix(concat!($prefix, "런가"), &[$($prior,)* "ending.interrogative-rieulreonga"]),
            suffix(concat!($prefix, "라"), &[$($prior,)* "ending.concern-rieulra"]),
            suffix(concat!($prefix, "세라"), &[$($prior,)* "ending.concern-rieulsera"]),
            suffix(concat!($prefix, "수록"), &[$($prior,)* "ending.connective-rieulsurok"]),
            suffix(concat!($prefix, "망정"), &[$($prior,)* "ending.concessive-rieul"]),
            suffix(concat!($prefix, "지라도"), &[$($prior,)* "ending.concessive-rieul"]),
            suffix(concat!($prefix, "지언정"), &[$($prior,)* "ending.concessive-rieul"]),
            suffix(concat!($prefix, "밖에"), &[$($prior,)* "ending.final-rieulbakke"]),
            suffix(concat!($prefix, "쏘냐"), &[$($prior,)* "ending.interrogative-rieulssonya"]),
            suffix(concat!($prefix, "지니"), &[$($prior,)* "ending.connective-rieuljini"]),
            suffix(concat!($prefix, "진대"), &[$($prior,)* "ending.conditional-rieuljindae"]),
            suffix(concat!($prefix, "진저"), &[$($prior,)* "ending.final-rieuljinjeo"]),
            suffix(concat!($prefix, "까요"), &[$($prior,)* "ending.interrogative-rieulkka", "ending.polite-yo"]),
        ]
    };
}

macro_rules! present_rieul_endings {
    ($prefix:literal $(, $prior:literal)*) => {
        rieul_endings!($prefix, [$($prior),*],
            suffix(concat!($prefix, "세"), &[$($prior,)* "ending.final-rieulse"]),
            suffix(concat!($prefix, "래"), &[$($prior,)* "ending.intent-rieulrae"]),
            suffix(concat!($prefix, "래요"), &[$($prior,)* "ending.intent-rieulrae", "ending.polite-yo"]),
            suffix(concat!($prefix, "라치면"), &[$($prior,)* "ending.conditional-rieulrachimyeon"]),
        )
    };
}

const RIEUL_FINAL: &[Suffix] = present_rieul_endings!("");
const PAST_RIEUL_FINAL: &[Suffix] = rieul_endings!("을", []);
const HONORIFIC_RIEUL_FINAL: &[Suffix] = present_rieul_endings!("실", "ending.honorific");
const HONORIFIC_PAST_RIEUL_FINAL: &[Suffix] =
    rieul_endings!("셨을", ["ending.honorific", "ending.past"]);

const BIEUP_FINAL: &[Suffix] = &[
    suffix("니다", &["ending.polite-declarative"]),
    suffix("니까", &["ending.formal-interrogative"]),
    suffix("디까", &["ending.retrospective-interrogative"]),
    suffix("디다", &["ending.retrospective-declarative"]),
];

macro_rules! formal_bieup_extensions {
    ($prefix:literal $(, $prior:literal)*) => {
        &[
            suffix(concat!($prefix, "네"), &[$($prior,)* "ending.formal-boast-ne"]),
            suffix(concat!($prefix, "늰다"), &[$($prior,)* "ending.formal-declarative-neunda"]),
            suffix(concat!($prefix, "딘다"), &[$($prior,)* "ending.formal-retrospective-dinda"]),
            suffix(concat!($prefix, "지요"), &[$($prior,)* "ending.formal-confirmation-jiyo"]),
            suffix(concat!($prefix, "죠"), &[$($prior,)* "ending.formal-confirmation-jiyo"]),
        ]
    };
}

const BIEUP_EXTENSIONS: &[Suffix] = formal_bieup_extensions!("");
const SEUP_EXTENSIONS: &[Suffix] = formal_bieup_extensions!("습");
const GET_SEUP_EXTENSIONS: &[Suffix] = formal_bieup_extensions!("겠습", "ending.future");
const SIP_EXTENSIONS: &[Suffix] = formal_bieup_extensions!("십", "ending.honorific");
const SYEOT_SEUP_EXTENSIONS: &[Suffix] =
    formal_bieup_extensions!("셨습", "ending.honorific", "ending.past");
const SIGET_SEUP_EXTENSIONS: &[Suffix] =
    formal_bieup_extensions!("시겠습", "ending.honorific", "ending.future");

macro_rules! directive_endings {
    ($prefix:literal, $base:expr, $quote:expr; $($tail:literal),*) => {
        &[
            suffix($prefix, $base),
            $(
                suffix(concat!($prefix, $tail), $quote),
            )*
        ]
    };
}

macro_rules! all_directive_endings {
    ($prefix:literal $(, $prior:literal)*) => {
        directive_endings!(
            $prefix,
            &[$($prior,)* "ending.directive-ra"],
            &[$($prior,)* "ending.directive-quotative"];
            "거나", "거든", "거든요", "거들랑", "고", "고요", "기에", "나", "는", "는구나", "는군", "는군요", "는데", "는데요", "느니", "니", "니까", "니까는", "니까요", "니깐", "니요", "더군", "더군요", "더니", "더라", "던", "던가", "던데", "던데요", "데", "든지", "며", "면", "면서", "손", "오", "지", "죠"
        )
    };
}

const DIRECTIVE_ENDINGS: &[Suffix] = all_directive_endings!("라");
const HONORIFIC_DIRECTIVE_ENDINGS: &[Suffix] = all_directive_endings!("시라", "ending.honorific");

macro_rules! prospective_suffixes {
    ($prefix:literal $(, $rule:literal)*) => {
        &[
            suffix(concat!($prefix, "랴"), &[$($rule,)* "ending.rya"]),
            suffix(concat!($prefix, "렷다"), &[$($rule,)* "ending.ryeotda"]),
            suffix(concat!($prefix, "리"), &[$($rule,)* "ending.prospective-final"]),
            suffix(concat!($prefix, "리라"), &[$($rule,)* "ending.prospective-final"]),
            suffix(concat!($prefix, "리라오"), &[$($rule,)* "ending.prospective-final", "ending.hao-reporting-dao"]),
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
const HONORIFIC_FUTURE_PROSPECTIVE: &[Suffix] =
    prospective_suffixes!("시겠으", "ending.honorific", "ending.future");

fn starts_prospective(following: &str, prefix: &str) -> bool {
    following
        .strip_prefix(prefix)
        .is_some_and(|tail| tail.starts_with(['리', '랴', '렷']))
}

macro_rules! question {
    ($surface:literal, $rule:literal $(, $prior:literal)*) => {
        &[
            suffix($surface, &[$($prior,)* $rule]),
            suffix(concat!($surface, "고"), &[$($prior,)* $rule, "ending.quotative-go"]),
            suffix(concat!($surface, "고요"), &[$($prior,)* $rule, "ending.quotative-go", "ending.polite-yo"]),
            suffix(concat!($surface, "는"), &[$($prior,)* $rule, "particle.topic"]),
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

macro_rules! nieun_questions {
    ($prefix:literal $(, $prior:literal)*) => {
        &[
            suffix(concat!($prefix, "가"), &[$($prior,)* "ending.interrogative-nga"]),
            suffix(concat!($prefix, "가요"), &[$($prior,)* "ending.interrogative-nga", "ending.polite-yo"]),
            suffix(concat!($prefix, "지"), &[$($prior,)* "ending.interrogative-nji"]),
            suffix(concat!($prefix, "지요"), &[$($prior,)* "ending.interrogative-nji", "ending.polite-yo"]),
        ]
    };
}

const NIEUN_QUESTIONS: &[Suffix] = nieun_questions!("");
const NEUN_QUESTIONS: &[Suffix] = nieun_questions!("는");
const SIN_QUESTIONS: &[Suffix] = nieun_questions!("신", "ending.honorific");
const SINEUN_QUESTIONS: &[Suffix] = nieun_questions!("시는", "ending.honorific");
const SYEOTNEUN_QUESTIONS: &[Suffix] = nieun_questions!("셨는", "ending.honorific", "ending.past");
const SIGETNEUN_QUESTIONS: &[Suffix] =
    nieun_questions!("시겠는", "ending.honorific", "ending.future");

fn nieun_question_tail(following: &str, prefix: &str) -> bool {
    following
        .strip_prefix(prefix)
        .is_some_and(|tail| tail.starts_with(['가', '지']))
}

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
    verify_predicate_continuation_with_prefinal_context(state, pos, anchor, following, || true)
}

/// The caller may require source evidence before composing prefinals inside a token.
#[must_use]
pub fn verify_predicate_continuation_with_prefinal_context(
    state: ContinuationState,
    pos: PredicatePos,
    anchor: &str,
    following: &str,
    allows_prefinal_chain: impl FnOnce() -> bool,
) -> Option<PredicateContinuationMatch> {
    if state == ContinuationState::AOrEo
        && pos != PredicatePos::Copula
        && let Some(copula_tail) = following
            .strip_prefix('서')
            .filter(|tail| tail.starts_with(['이', '인', '일', '입', '임', '였', '여']))
        && let copula = copula_tail
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or_default()
        && super::verify_complete_copula_surface(copula)
    {
        let consumed_bytes = '서'.len_utf8() + copula.len();
        return Some(PredicateContinuationMatch {
            token_end: anchor.len() + consumed_bytes,
            consumed_bytes,
            rule_path: vec![
                RuleId::from("ending.aoeo-seo"),
                RuleId::from("ending.aoeo-seo-copula"),
            ],
        });
    }
    if state == ContinuationState::AOrEo
        && let Some(auxiliary) = match following.chars().next() {
            Some('보' | '봐' | '봤' | '본' | '볼' | '봅' | '봄') if pos.is_action() => {
                trial::verify(anchor.len(), following)
            }
            _ => auxiliary::verify_resultative(anchor.len(), following),
        }
    {
        return Some(with_connective_politeness(auxiliary, following));
    }
    let reported = following
        .starts_with(['대', '댔', '래', '랬', '재', '쟀', '냬', '시', '신', '셨'])
        || following.starts_with("는대")
        || following.starts_with("는댔")
        || following.starts_with("느냬");
    let ordinary = reported
        .then(|| report::verify(state, pos, anchor, following))
        .flatten()
        .or_else(|| verify_simple_continuation(state, pos, anchor, following));
    let honorific_declarative = if state == ContinuationState::Eu {
        let declarative = if pos.is_action() { "신다" } else { "시다" };
        following.strip_prefix(declarative).and_then(|tail| {
            let suffix = declarative_suffix(tail)?;
            let base_rule = if pos.is_action() {
                "ending.declarative"
            } else {
                "ending.final-da"
            };
            let consumed_bytes = declarative.len() + suffix.surface.len();
            Some(PredicateContinuationMatch {
                token_end: anchor.len() + consumed_bytes,
                consumed_bytes,
                rule_path: ["ending.honorific", base_rule]
                    .into_iter()
                    .chain(suffix.rules.iter().copied())
                    .map(RuleId::from)
                    .collect(),
            })
        })
    } else {
        None
    };
    let ordinary = ordinary
        .into_iter()
        .chain(honorific_declarative)
        .max_by_key(|matched| matched.consumed_bytes);
    if let Some(stacked) = verify_prefinal_chain(state, pos, anchor.len(), following)
        && ordinary
            .as_ref()
            .is_none_or(|matched| stacked.consumed_bytes > matched.consumed_bytes)
        && allows_prefinal_chain()
    {
        return Some(with_connective_politeness(stacked, following));
    }
    ordinary.map(|matched| with_connective_politeness(matched, following))
}

fn with_connective_politeness(
    mut matched: PredicateContinuationMatch,
    following: &str,
) -> PredicateContinuationMatch {
    if following
        .get(matched.consumed_bytes..)
        .is_some_and(|tail| tail.starts_with('요'))
        && matched.rule_path.last().is_some_and(|rule| {
            matches!(
                rule.as_str(),
                "ending.connective-go" | "ending.quotative-go" | "ending.intentive-ryeogo"
            )
        })
    {
        let polite_len = '요'.len_utf8();
        matched.consumed_bytes += polite_len;
        matched.token_end += polite_len;
    }
    matched
}

fn verify_prefinal_chain(
    state: ContinuationState,
    pos: PredicatePos,
    anchor_len: usize,
    following: &str,
) -> Option<PredicateContinuationMatch> {
    use ContinuationState::{Eu, Future, Past};

    let mut tail = following;
    let mut state = state;
    let mut honorific = false;
    if state == Eu {
        let (prefix, next_state) = if following.starts_with("시겠") {
            ("시겠", Future)
        } else if following.starts_with("시었") {
            ("시었", Past)
        } else if following.starts_with('셨') {
            ("셨", Past)
        } else {
            return None;
        };
        tail = &tail[prefix.len()..];
        state = next_state;
        honorific = true;
    } else if state != Past {
        return None;
    }
    let repeated_past = state == Past && tail.starts_with('었');
    if repeated_past {
        tail = &tail['었'.len_utf8()..];
    }
    let past_with_future = state == Past && tail.starts_with('겠');
    if past_with_future {
        tail = &tail['겠'.len_utf8()..];
        state = Future;
    }
    let prefix_len = following.len() - tail.len();
    if prefix_len == 0 {
        return None;
    }
    let prefix = &following[..prefix_len];
    let matched = verify_simple_continuation(state, pos, prefix, tail)?;
    if matched.consumed_bytes == 0 {
        return None;
    }
    let mut rule_path = Vec::new();
    if honorific {
        rule_path.push(RuleId::from("ending.honorific"));
        rule_path.push(RuleId::from(if prefix.starts_with("시겠") {
            "ending.future"
        } else {
            "ending.past"
        }));
    }
    if repeated_past {
        rule_path.push(RuleId::from("ending.past"));
    }
    if past_with_future {
        rule_path.push(RuleId::from("ending.future"));
    }
    rule_path.extend(matched.rule_path);
    Some(PredicateContinuationMatch {
        token_end: anchor_len + matched.token_end,
        consumed_bytes: prefix_len + matched.consumed_bytes,
        rule_path,
    })
}

fn verify_simple_continuation(
    state: ContinuationState,
    pos: PredicatePos,
    anchor: &str,
    following: &str,
) -> Option<PredicateContinuationMatch> {
    let necessity_prefix = match state {
        ContinuationState::AOrEo => "야겠",
        ContinuationState::Eu => "셔야겠",
        _ => "",
    };
    if !necessity_prefix.is_empty()
        && let Some(tail) = following.strip_prefix(necessity_prefix)
        && let Some(ending) = verify_simple_continuation(ContinuationState::Future, pos, "겠", tail)
        && ending.consumed_bytes != 0
    {
        let consumed_bytes = necessity_prefix.len() + ending.consumed_bytes;
        return Some(PredicateContinuationMatch {
            token_end: anchor.len() + consumed_bytes,
            consumed_bytes,
            rule_path: (state == ContinuationState::Eu)
                .then_some("ending.honorific")
                .into_iter()
                .chain(["ending.connective-ya", "ending.future"])
                .map(RuleId::from)
                .chain(ending.rule_path)
                .collect(),
        });
    }
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
        ContinuationState::StemEnding if following.starts_with("고요") => &[GO_YO],
        ContinuationState::StemEnding if pos.is_action() && following.starts_with('남') => &[NAM],
        ContinuationState::StemEnding => GEODEUN_ENDINGS,
        ContinuationState::NieunEnding => BACKGROUND,
        ContinuationState::CompletedNieunEnding => COMPLETED_NIEUN,
        ContinuationState::Interrogative => INTERROGATIVE,
        ContinuationState::CopulaRa => COPULA_RA_SUFFIXES,
        ContinuationState::RieulFinal => RIEUL_FINAL,
        ContinuationState::BieupFinal => BIEUP_FINAL,
        ContinuationState::Prospective => PROSPECTIVE,
        ContinuationState::ReportEnding
            if pos.is_action()
                && following.starts_with('담')
                && anchor.chars().next_back().is_some_and(has_final) =>
        {
            &[DAM]
        }
        ContinuationState::ReportEnding => return None,
    };
    if let Some(continuation) = humble::verify(state, pos, anchor, following) {
        return Some(continuation);
    }
    let additional_endings: &[Suffix] = match state {
        ContinuationState::StemEnding | ContinuationState::Past | ContinuationState::Future
            if starts_with_daganeun(following) =>
        {
            DAGANEUN_ENDINGS
        }
        ContinuationState::Eu
            if following
                .strip_prefix('시')
                .is_some_and(starts_with_daganeun) =>
        {
            SI_DAGANEUN_ENDINGS
        }
        ContinuationState::Eu
            if following
                .strip_prefix('셨')
                .is_some_and(starts_with_daganeun) =>
        {
            SYEOT_DAGANEUN_ENDINGS
        }
        ContinuationState::Eu
            if following
                .strip_prefix("시겠")
                .is_some_and(starts_with_daganeun) =>
        {
            SIGET_DAGANEUN_ENDINGS
        }
        ContinuationState::Eu if pos.is_action() && following.starts_with('마') => &[PROMISE_MA],
        ContinuationState::StemEnding
            if pos.is_action()
                && following.starts_with('마')
                && anchor.chars().next_back().is_some_and(has_rieul_final) =>
        {
            &[PROMISE_MA]
        }
        ContinuationState::StemEnding
            if pos.is_action()
                && following.starts_with("는담")
                && anchor
                    .chars()
                    .next_back()
                    .is_some_and(|last| has_final(last) && !has_rieul_final(last)) =>
        {
            &[NEUNDAM]
        }
        ContinuationState::StemEnding
            if following.starts_with('담') && !pos.is_action() && pos != PredicatePos::Copula =>
        {
            &[DAM]
        }
        ContinuationState::Past | ContinuationState::Future if following.starts_with('담') => {
            &[DAM]
        }
        ContinuationState::NieunEnding
            if pos.is_action() && anchor.ends_with('신') && following.starts_with('담') =>
        {
            &[DAM]
        }
        ContinuationState::Eu if pos.is_action() && following.starts_with("신담") => {
            &[HONORIFIC_NDAM]
        }
        ContinuationState::Eu if following.starts_with("시담") => &[HONORIFIC_DAM],
        ContinuationState::StemEnding | ContinuationState::Past | ContinuationState::Future
            if pos != PredicatePos::Copula
                && following.starts_with("나요")
                && (state != ContinuationState::StemEnding
                    || !anchor.chars().next_back().is_some_and(has_rieul_final)) =>
        {
            &[INTERROGATIVE_NA_YO]
        }
        ContinuationState::Eu if pos != PredicatePos::Copula && following.starts_with("시나요") => {
            &[HONORIFIC_INTERROGATIVE_NA_YO]
        }
        ContinuationState::StemEnding | ContinuationState::Past | ContinuationState::Future
            if pos != PredicatePos::Copula && following.starts_with('길') =>
        {
            &[NOMINALIZER_GIL]
        }
        ContinuationState::Eu if following.starts_with("시길") => &[HONORIFIC_NOMINALIZER_GIL],
        ContinuationState::StemEnding if pos.is_action() && following.starts_with("자마자") => {
            &[IMMEDIATE]
        }
        ContinuationState::Eu if pos.is_action() && following.starts_with("시자마자") => {
            &[HONORIFIC_IMMEDIATE]
        }
        ContinuationState::Eu if following.starts_with("시어") || following.starts_with('셔') => {
            HONORIFIC_VOWEL_CONNECTIVES
        }
        ContinuationState::StemEnding
            if following.starts_with("려고")
                && anchor
                    .chars()
                    .next_back()
                    .is_some_and(|last| last == '시' || has_rieul_final(last)) =>
        {
            &[INTENTIVE_RYEOGO]
        }
        ContinuationState::StemEnding if following.starts_with('진') => &[NEGATIVE_TOPIC],
        ContinuationState::Eu if following.starts_with("시진") => &[HONORIFIC_NEGATIVE_TOPIC],
        ContinuationState::StemEnding if following.starts_with("지만") => &[JIMAN],
        ContinuationState::StemEnding if following.starts_with("거나") => &[GEONA],
        ContinuationState::StemEnding if anchor.ends_with('시') && following.starts_with('나') => {
            &[CONCESSIVE_NA]
        }
        ContinuationState::Eu
            if following.starts_with("시지만")
                || following.starts_with("시거나")
                || following.starts_with("시나") =>
        {
            HONORIFIC_COORDINATING_ENDINGS
        }
        ContinuationState::Eu if following.starts_with("시네") => &HONORIFIC_NE_ENDINGS,
        ContinuationState::Eu
            if following.starts_with("셨네") || following.starts_with("시겠네") =>
        {
            HONORIFIC_PREFINAL_NE_ENDINGS
        }
        ContinuationState::NieunEnding
            if (following.starts_with('군') || following.starts_with("구나"))
                && anchor.ends_with('는')
                && (pos.is_action() || anchor.ends_with("있는") || anchor.ends_with("없는")) =>
        {
            &[NEUN_GUN, NEUN_GUN_YO, NEUN_GUNA]
        }
        ContinuationState::StemEnding
            if following.starts_with('군')
                && !pos.is_action()
                && pos != PredicatePos::Copula
                && anchor != "아니" =>
        {
            &[GUN, GUN_YO]
        }
        ContinuationState::Prospective if following.starts_with('군') && anchor.ends_with('시') => {
            &[GUN, GUN_YO]
        }
        ContinuationState::Eu
            if following.starts_with("시는군")
                && (pos.is_action() || anchor.ends_with(['있', '없'])) =>
        {
            &[HONORIFIC_NEUN_GUN, HONORIFIC_NEUN_GUN_YO]
        }
        ContinuationState::Eu
            if following.starts_with("시군")
                || following.starts_with("셨군")
                || following.starts_with("시겠군") =>
        {
            HONORIFIC_GUN_ENDINGS
        }
        ContinuationState::AOrEo if following.starts_with('서') => A_OR_EO_SEO_SUFFIXES,
        ContinuationState::StemEnding
            if following.starts_with("고서") && (pos.is_action() || anchor == "아니") =>
        {
            GOSEO
        }
        ContinuationState::Eu if following.starts_with("시고서") => HONORIFIC_GOSEO,
        ContinuationState::StemEnding if anchor.ends_with('시') && following.starts_with('러') => {
            &[PURPOSE_REO]
        }
        ContinuationState::Past | ContinuationState::Future if following.starts_with("다오") => {
            DAO
        }
        ContinuationState::StemEnding
            if anchor.ends_with('시') && following.starts_with("다오") =>
        {
            DAO
        }
        ContinuationState::NieunEnding
            if pos.is_action() && anchor.ends_with('신') && following.starts_with("다오") =>
        {
            HONORIFIC_NIEUN_DAO
        }
        ContinuationState::Eu
            if following.starts_with("시다오")
                || following.starts_with("셨다오")
                || following.starts_with("시겠다오")
                || (pos.is_action() && following.starts_with("신다오")) =>
        {
            HONORIFIC_DAO
        }
        ContinuationState::StemEnding if anchor == "아니" && following.starts_with("라오") => {
            NEGATIVE_RAO
        }
        ContinuationState::Eu | ContinuationState::Prospective if following.starts_with('매') => {
            &[MAE]
        }
        ContinuationState::Past | ContinuationState::Future if following.starts_with("으매") => {
            &[EUMAE]
        }
        ContinuationState::Eu if following.starts_with("시매") => &[HONORIFIC_MAE],
        ContinuationState::Eu | ContinuationState::Prospective
            if pos != PredicatePos::Copula && following.starts_with('므') =>
        {
            MEURO
        }
        ContinuationState::StemEnding
            if pos == PredicatePos::Copula && anchor == "이" && following.starts_with('므') =>
        {
            MEURO
        }
        ContinuationState::Past | ContinuationState::Future if following.starts_with("으므") => {
            EUMEURO
        }
        ContinuationState::Eu | ContinuationState::StemEnding
            if (state == ContinuationState::Eu
                || (pos == PredicatePos::Copula && anchor == "이"))
                && (following.starts_with("시므")
                    || following.starts_with("셨으므")
                    || following.starts_with("시겠으므")) =>
        {
            HONORIFIC_MEURO_ENDINGS
        }
        ContinuationState::Eu
            if pos.is_action()
                && (following.starts_with('렴') || following.starts_with("려무나")) =>
        {
            RYEOM_IMPERATIVES
        }
        ContinuationState::Eu | ContinuationState::Prospective
            if following.starts_with("려나") || following.starts_with("려니") =>
        {
            RYEO_CONJECTURE
        }
        ContinuationState::Eu | ContinuationState::Prospective
            if pos != PredicatePos::Copula && following.starts_with('라') =>
        {
            DIRECTIVE_ENDINGS
        }
        ContinuationState::Eu if following.starts_with("시라") => HONORIFIC_DIRECTIVE_ENDINGS,
        ContinuationState::BieupFinal => BIEUP_EXTENSIONS,
        ContinuationState::Past | ContinuationState::Future if following.starts_with('습') => {
            SEUP_EXTENSIONS
        }
        ContinuationState::Past if following.starts_with("겠습") => GET_SEUP_EXTENSIONS,
        ContinuationState::Eu if following.starts_with('십') => SIP_EXTENSIONS,
        ContinuationState::Eu if following.starts_with("셨습") => SYEOT_SEUP_EXTENSIONS,
        ContinuationState::Eu if following.starts_with("시겠습") => SIGET_SEUP_EXTENSIONS,
        ContinuationState::StemEnding | ContinuationState::Past
            if following.starts_with("것다") =>
        {
            GEOTDA
        }
        ContinuationState::StemEnding
            if pos == PredicatePos::Copula
                && matches!(anchor, "이" | "이시")
                && following.starts_with("니라") =>
        {
            &[NIRA]
        }
        ContinuationState::StemEnding
            if pos == PredicatePos::Copula
                && anchor == "이시"
                && following.starts_with("느니라") =>
        {
            &[NEUNIRA]
        }
        ContinuationState::Eu if !pos.is_action() && following.starts_with("니라") => &[NIRA],
        ContinuationState::StemEnding
            if (pos.is_action() || anchor.ends_with(['있', '없']) || anchor == "계시")
                && !anchor.chars().next_back().is_some_and(has_rieul_final)
                && following.starts_with("느니라") =>
        {
            &[NEUNIRA]
        }
        ContinuationState::Past | ContinuationState::Future if following.starts_with("느니라") => {
            &[NEUNIRA]
        }
        ContinuationState::Eu
            if following.starts_with("시니라") || following.starts_with("시느니라") =>
        {
            HONORIFIC_NIRA
        }
        ContinuationState::StemEnding
            if (pos == PredicatePos::Copula || anchor == "아니")
                && following.starts_with(['란', '람', '랍', '래', '로']) =>
        {
            COPULA_REPORTING_AND_RO_ENDINGS
        }
        ContinuationState::StemEnding
            if (pos == PredicatePos::Copula || anchor == "아니")
                && following.starts_with([
                    '게', '고', '구', '군', '길', '네', '니', '면', '세', '야', '어', '에', '자',
                    '죠', '지', '랴', '렷',
                ]) =>
        {
            COPULA_COMMON_ENDINGS
        }
        ContinuationState::StemEnding
            if following.starts_with('니')
                && !anchor.chars().next_back().is_some_and(has_rieul_final) =>
        {
            &[FAMILIAR_QUESTION]
        }
        ContinuationState::Past | ContinuationState::Future if following.starts_with('니') => {
            &[FAMILIAR_QUESTION]
        }
        ContinuationState::Eu if following.starts_with("시니") => &[HONORIFIC_FAMILIAR_QUESTION],
        ContinuationState::StemEnding | ContinuationState::Past | ContinuationState::Future
            if following.starts_with('죠') || following.starts_with("지요") =>
        {
            CONFIRMATION_JIYO
        }
        ContinuationState::Past
            if following.starts_with("겠죠") || following.starts_with("겠지요") =>
        {
            PAST_FUTURE_JIYO
        }
        ContinuationState::Eu
            if ["시", "셨", "시겠", "셨겠"].iter().any(|prefix| {
                following
                    .strip_prefix(*prefix)
                    .is_some_and(|tail| tail.starts_with('죠') || tail.starts_with("지요"))
            }) =>
        {
            HONORIFIC_JIYO
        }
        ContinuationState::NieunEnding if nieun_question_tail(following, "") => NIEUN_QUESTIONS,
        ContinuationState::Past | ContinuationState::Future
            if nieun_question_tail(following, "는") =>
        {
            NEUN_QUESTIONS
        }
        ContinuationState::Eu if nieun_question_tail(following, "신") => SIN_QUESTIONS,
        ContinuationState::Eu if nieun_question_tail(following, "시는") => SINEUN_QUESTIONS,
        ContinuationState::Eu if nieun_question_tail(following, "셨는") => SYEOTNEUN_QUESTIONS,
        ContinuationState::Eu if nieun_question_tail(following, "시겠는") => SIGETNEUN_QUESTIONS,
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
        ContinuationState::StemEnding | ContinuationState::Past | ContinuationState::Future
            if following.starts_with('던') =>
        {
            DEON_RECOLLECTION
        }
        ContinuationState::StemEnding | ContinuationState::Past | ContinuationState::Future
            if following.starts_with('더') =>
        {
            RETROSPECTIVE
        }
        ContinuationState::Past if following.starts_with("겠더") => GET_RETROSPECTIVE,
        ContinuationState::Eu if following.starts_with("시더") => SI_RETROSPECTIVE,
        ContinuationState::Eu if following.starts_with("셨더") => SYEOT_RETROSPECTIVE,
        ContinuationState::Eu if following.starts_with("시겠더") => SIGET_RETROSPECTIVE,
        ContinuationState::Past | ContinuationState::Future if following.starts_with("는데") => {
            NEUNDE
        }
        ContinuationState::Eu if following.starts_with("시냐") => SI_NYA,
        ContinuationState::Eu if following.starts_with("시로") => HONORIFIC_RO_ENDINGS,
        ContinuationState::Eu if following.starts_with("시느냐") => SI_NEUNYA,
        ContinuationState::Eu if following.starts_with("셨냐") => SYEOT_NYA,
        ContinuationState::Eu if following.starts_with("셨느냐") => SYEOT_NEUNYA,
        ContinuationState::Eu if following.starts_with("시겠냐") => SIGET_NYA,
        ContinuationState::Eu if following.starts_with("시겠느냐") => SIGET_NEUNYA,
        ContinuationState::Eu if following.starts_with("신데") => SINDE,
        ContinuationState::Eu if following.starts_with("시는데") => SINEUNDE,
        ContinuationState::Eu if following.starts_with("셨는데") => SYEOTNEUNDE,
        ContinuationState::Eu if following.starts_with("시겠는데") => SIGETNEUNDE,
        ContinuationState::Past | ContinuationState::Future
            if starts_prospective(following, "으") =>
        {
            PREFINAL_PROSPECTIVE
        }
        ContinuationState::Eu if starts_prospective(following, "") => PROSPECTIVE,
        ContinuationState::Eu if starts_prospective(following, "시") => HONORIFIC_PROSPECTIVE,
        ContinuationState::Eu if starts_prospective(following, "셨으") => {
            HONORIFIC_PAST_PROSPECTIVE
        }
        ContinuationState::Eu if starts_prospective(following, "시겠으") => {
            HONORIFIC_FUTURE_PROSPECTIVE
        }
        ContinuationState::Past if following.starts_with('을') => PAST_RIEUL_FINAL,
        ContinuationState::Eu if following.starts_with('실') => HONORIFIC_RIEUL_FINAL,
        ContinuationState::Eu if following.starts_with("셨을") => HONORIFIC_PAST_RIEUL_FINAL,
        _ => hao::additional_endings(state, pos, anchor, following),
    };

    let suffix = candidates
        .iter()
        .chain(additional_endings)
        .chain(
            if state == ContinuationState::Eu
                && (following.starts_with("시어") || following.starts_with(['셔', '세']))
            {
                POLITE_HONORIFIC_ENDINGS
            } else {
                &[]
            },
        )
        .filter(|suffix| {
            following.starts_with(suffix.surface)
                && ((suffix.rules.last() != Some(&"ending.connective-daganeun")
                    && (suffix.rules != ["ending.connective-ni"]
                        || !suffix.surface.starts_with('으')))
                    || !following[suffix.surface.len()..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_alphanumeric() || next == '_'))
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
                                | "ending.earnest-request-sisa"
                                | "ending.intentive-ryeo"
                                | "ending.intentive-ryeogo"
                                | "ending.intentive-ryeodeun"
                                | "ending.intentive-adnominal"
                                | "ending.purpose-reo"
                                | "ending.present-adnominal"
                                | "ending.intent-rieulrae"
                                | "ending.conditional-rieulrachimyeon"
                        )
                    }))
        })
        .max_by_key(|suffix| suffix.surface.len())
        .filter(|suffix| {
            !(suffix.rules.last() == Some(&"ending.connective-na")
                && following[suffix.surface.len()..].starts_with('요'))
        });
    if state == ContinuationState::CopulaRa
        && suffix.is_some_and(|suffix| suffix.surface.is_empty())
        && following
            .chars()
            .next()
            .is_some_and(|next| next.is_alphanumeric() || next == '_')
    {
        return None;
    }
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
                | ContinuationState::ReportEnding
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
        '는' if following.starts_with(DECLARATIVE_NEUN_GUN_YO.surface) => {
            Some(&DECLARATIVE_NEUN_GUN_YO)
        }
        '는' => Some(&DECLARATIVE_ADNOMINAL),
        '고' => Some(&DECLARATIVE_GO),
        '네' if following.starts_with("네요") => Some(&DECLARATIVE_NE_YO),
        '네' => Some(&DECLARATIVE_NE),
        '던' => Some(&DECLARATIVE_RETROSPECTIVE),
        '더' if following.starts_with(DECLARATIVE_DEONIMANEUN.surface) => {
            Some(&DECLARATIVE_DEONIMANEUN)
        }
        '더' if following.starts_with(DECLARATIVE_DEONIMAN.surface) => Some(&DECLARATIVE_DEONIMAN),
        '더' if following.starts_with(DECLARATIVE_DEONI.surface) => Some(&DECLARATIVE_DEONI),
        '니' => Some(&DECLARATIVE_NI),
        '며' => Some(&DECLARATIVE_MYEO),
        '지' => Some(&DECLARATIVE_JI),
        '오' => Some(&DECLARATIVE_O),
        '우' => Some(&hao::HAO_U),
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
    fn daganeun_continuation_requires_the_end_of_the_token() {
        for ending in ["다가는", "다간", "단"] {
            let text = format!("{ending}.");
            let found = verify_predicate_continuation(
                ContinuationState::StemEnding,
                PredicatePos::Verb,
                "읽",
                &text,
            )
            .expect("connective ending before punctuation");
            assert_eq!(found.consumed_bytes, ending.len());
            assert_eq!(found.rule_path[0].as_str(), "ending.connective-daganeun");

            let text = format!("{ending}다");
            let found = verify_predicate_continuation(
                ContinuationState::StemEnding,
                PredicatePos::Verb,
                "읽",
                &text,
            );
            assert!(
                found.is_none_or(|matched| !matched
                    .rule_path
                    .iter()
                    .any(|rule| rule.as_str() == "ending.connective-daganeun")),
                "accepted an internal connective ending in {text}",
            );
        }
    }

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
    fn causal_mae_does_not_consume_a_bare_ending_after_a_consonant() {
        assert!(
            verify_predicate_continuation(
                ContinuationState::StemEnding,
                PredicatePos::Verb,
                "먹",
                "매 기쁘다."
            )
            .is_none()
        );
    }

    #[test]
    fn promise_ma_requires_eu_or_retained_rieul_on_action_verbs() {
        for (state, pos, anchor) in [
            (ContinuationState::StemEnding, PredicatePos::Verb, "갚"),
            (ContinuationState::Eu, PredicatePos::Adjective, "좋으"),
        ] {
            let result = verify_predicate_continuation(state, pos, anchor, "마.");
            assert_eq!(result.map_or(0, |matched| matched.consumed_bytes), 0);
        }
    }

    #[test]
    fn self_question_dam_preserves_stem_and_position_constraints() {
        for (state, pos, anchor, tail) in [
            (
                ContinuationState::StemEnding,
                PredicatePos::Verb,
                "마시",
                "담?",
            ),
            (
                ContinuationState::StemEnding,
                PredicatePos::Verb,
                "읽",
                "담?",
            ),
            (
                ContinuationState::StemEnding,
                PredicatePos::Verb,
                "살",
                "는담?",
            ),
            (
                ContinuationState::ReportEnding,
                PredicatePos::Adjective,
                "좋은",
                "담?",
            ),
        ] {
            assert!(verify_predicate_continuation(state, pos, anchor, tail).is_none());
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
    fn resultative_uses_the_auxiliary_verbs_ending_and_provenance() {
        for (pos, anchor, following, rules) in [
            (
                PredicatePos::Verb,
                "다뤄",
                "진다",
                &["ending.declarative"][..],
            ),
            (
                PredicatePos::Verb,
                "바꿔",
                "진다고",
                &["ending.declarative", "ending.quotative-go"],
            ),
            (
                PredicatePos::Adjective,
                "흐려",
                "지더니",
                &["ending.retrospective-connective"],
            ),
            (
                PredicatePos::Verb,
                "버려",
                "졌던가",
                &["ending.past", "ending.retrospective-question-deon"],
            ),
            (
                PredicatePos::Adjective,
                "넓어",
                "지겠다",
                &["ending.future", "ending.final-da"],
            ),
        ] {
            let matched =
                verify_predicate_continuation(ContinuationState::AOrEo, pos, anchor, following)
                    .expect("resultative continuation");
            assert_eq!(matched.token_end, anchor.len() + following.len());
            assert!(
                matched
                    .rule_path
                    .iter()
                    .map(RuleId::as_str)
                    .eq(std::iter::once("ending.auxiliary-jida").chain(rules.iter().copied()))
            );
        }
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
