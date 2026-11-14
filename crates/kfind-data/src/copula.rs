#[derive(Clone, Copy, Debug)]
pub struct CopulaStemEnding {
    pub suffix: EndingSurface,
    pub omission: CopulaOmission,
}

#[derive(Clone, Copy, Debug)]
pub struct EndingSurface {
    pub surface: &'static str,
    pub rules: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopulaOmission {
    Never,
    Bare,
    BareOrHonorific,
}

use CopulaOmission::{Bare, BareOrHonorific, Never};

const fn ending(
    surface: &'static str,
    rules: &'static [&'static str],
    omission: CopulaOmission,
) -> CopulaStemEnding {
    CopulaStemEnding {
        suffix: EndingSurface { surface, rules },
        omission,
    }
}

include!("../../../data/rules/copula-stem-endings.rs");

const fn suffix(surface: &'static str, rules: &'static [&'static str]) -> EndingSurface {
    EndingSurface { surface, rules }
}

pub const COPULA_RA_SUFFIXES: &[EndingSurface] = &[
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

pub const COPULA_REPORTING_AND_RO_ENDINGS: &[EndingSurface] = &[
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
