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
