use crate::hangul::{
    JONG_BIEUP, JONG_DIGEUT, JONG_HIEUH, JONG_NIEUN, JONG_NONE, JONG_RIEUL, JONG_SIOT, JUNG_A,
    JUNG_AE, JUNG_E, JUNG_EO, JUNG_EU, JUNG_I, JUNG_O, JUNG_OE, JUNG_U, JUNG_WA, JUNG_WAE,
    JUNG_WEO, JUNG_YA, JUNG_YAE, JUNG_YE, JUNG_YEO, add_final, decompose_syllable, drop_last_final,
    has_final, has_rieul_final, replace_last_final, replace_last_vowel,
};
use crate::{LexicalAlternation, PredicateEntry, RuleId};

use super::{DerivedSurface, GenerateError};

const CHOSEONG_RIEUL: u8 = 5;

pub(super) fn aeo_surfaces(
    entry: &PredicateEntry,
    stem: &str,
) -> Result<Vec<DerivedSurface>, GenerateError> {
    let lexical_rule = alternation_rule(entry.alternation)
        .into_iter()
        .collect::<Vec<_>>();
    match entry.alternation {
        LexicalAlternation::Regular => regular_aeo(entry, stem, lexical_rule),
        LexicalAlternation::DToL => {
            let altered = require_and_replace_final(entry, stem, JONG_DIGEUT, JONG_RIEUL)?;
            regular_aeo(entry, &altered, lexical_rule)
        }
        LexicalAlternation::DropS => {
            let altered = require_and_drop_final(entry, stem, JONG_SIOT)?;
            let suffix = if is_light_vowel(last_vowel(&altered)?) {
                "아"
            } else {
                "어"
            };
            let core_len = altered.len();
            Ok(vec![derived(
                format!("{altered}{suffix}"),
                core_len,
                lexical_rule,
            )])
        }
        LexicalAlternation::BToWa | LexicalAlternation::BToWo => {
            let base = b_vowel_stem(entry, stem)?;
            let vowel = if entry.alternation == LexicalAlternation::BToWa {
                JUNG_WA
            } else {
                JUNG_WEO
            };
            let surface = replace_last_vowel(&base, vowel).ok_or_else(|| mismatch(entry))?;
            Ok(vec![DerivedSurface {
                core_len: surface.len(),
                surface,
                rules: lexical_rule,
            }])
        }
        LexicalAlternation::DropH => {
            let without_h = require_and_drop_final(entry, stem, JONG_HIEUH)?;
            let last = decompose_syllable(without_h.chars().next_back().expect("altered stem"))
                .ok_or_else(|| mismatch(entry))?;
            let contracted_vowel = match last.jungseong {
                JUNG_YA => JUNG_YAE,
                JUNG_YEO => JUNG_YE,
                _ => JUNG_AE,
            };
            let surface =
                replace_last_vowel(&without_h, contracted_vowel).ok_or_else(|| mismatch(entry))?;
            Ok(vec![DerivedSurface {
                core_len: surface.len(),
                surface,
                rules: vec![rule("lexical.drop-h"), rule("contraction.h-irregular")],
            }])
        }
        LexicalAlternation::ReuDoubleL => {
            let without_reu = require_reu_stem(entry, stem)?;
            let last = without_reu.chars().next_back().expect("stem before 르");
            let doubled = if has_rieul_final(last) {
                without_reu.to_owned()
            } else {
                add_final(without_reu, JONG_RIEUL).ok_or_else(|| mismatch(entry))?
            };
            let suffix = if is_light_vowel(last_vowel(without_reu)?) {
                "라"
            } else {
                "러"
            };
            let surface = format!("{doubled}{suffix}");
            Ok(vec![DerivedSurface {
                core_len: surface.len(),
                surface,
                rules: lexical_rule,
            }])
        }
        LexicalAlternation::Reo => {
            require_reu_stem(entry, stem)?;
            let surface = format!("{stem}러");
            Ok(vec![DerivedSurface {
                core_len: stem.len(),
                surface,
                rules: lexical_rule,
            }])
        }
        LexicalAlternation::Ha => {
            if !stem.ends_with('하') {
                return Err(mismatch(entry));
            }
            let contracted = replace_last_vowel(stem, JUNG_AE).ok_or_else(|| mismatch(entry))?;
            Ok(vec![
                DerivedSurface {
                    surface: format!("{stem}여"),
                    core_len: stem.len(),
                    rules: lexical_rule.clone(),
                },
                DerivedSurface {
                    core_len: contracted.len(),
                    surface: contracted,
                    rules: with_rule(lexical_rule, "contraction.ha-yeo"),
                },
            ])
        }
        LexicalAlternation::UToEo => {
            let last = decompose_syllable(stem.chars().next_back().expect("stem"))
                .ok_or_else(|| mismatch(entry))?;
            if last.jungseong != JUNG_U || last.jongseong != JONG_NONE {
                return Err(mismatch(entry));
            }
            let surface = replace_last_vowel(stem, JUNG_EO).ok_or_else(|| mismatch(entry))?;
            Ok(vec![DerivedSurface {
                core_len: surface.len(),
                surface,
                rules: lexical_rule,
            }])
        }
        LexicalAlternation::Copula | LexicalAlternation::Suppletive => Ok(Vec::new()),
    }
}

fn regular_aeo(
    entry: &PredicateEntry,
    stem: &str,
    base_rules: Vec<RuleId>,
) -> Result<Vec<DerivedSurface>, GenerateError> {
    let last = decompose_syllable(stem.chars().next_back().expect("stem"))
        .ok_or_else(|| GenerateError::InvalidLemma(stem.into()))?;
    if last.jongseong != JONG_NONE {
        let suffix = if is_light_vowel(last.jungseong) {
            "아"
        } else {
            "어"
        };
        return Ok(vec![DerivedSurface {
            surface: format!("{stem}{suffix}"),
            core_len: stem.len(),
            rules: base_rules,
        }]);
    }

    let mut surfaces = Vec::new();
    match last.jungseong {
        JUNG_A | JUNG_EO => surfaces.push(derived(stem.to_owned(), stem.len(), base_rules)),
        JUNG_O => {
            surfaces.push(derived(format!("{stem}아"), stem.len(), base_rules.clone()));
            surfaces.push(derived(
                replace_last_vowel(stem, JUNG_WA).expect("valid vowel replacement"),
                stem.len(),
                with_rule(base_rules, "contraction.o-a"),
            ));
        }
        JUNG_U => {
            surfaces.push(derived(format!("{stem}어"), stem.len(), base_rules.clone()));
            surfaces.push(derived(
                replace_last_vowel(stem, JUNG_WEO).expect("valid vowel replacement"),
                stem.len(),
                with_rule(base_rules, "contraction.u-eo"),
            ));
        }
        JUNG_OE => {
            surfaces.push(derived(format!("{stem}어"), stem.len(), base_rules.clone()));
            surfaces.push(derived(
                replace_last_vowel(stem, JUNG_WAE).expect("valid vowel replacement"),
                stem.len(),
                with_rule(base_rules, "contraction.oe-eo"),
            ));
        }
        JUNG_I => {
            surfaces.push(derived(format!("{stem}어"), stem.len(), base_rules.clone()));
            if !entry
                .flags
                .contains(crate::PredicateFlags::NO_I_EO_CONTRACTION)
            {
                surfaces.push(derived(
                    replace_last_vowel(stem, JUNG_YEO).expect("valid vowel replacement"),
                    stem.len(),
                    with_rule(base_rules, "contraction.i-eo"),
                ));
            }
        }
        JUNG_EU => {
            let harmony_vowel = preceding_harmony_vowel(stem).unwrap_or(JUNG_EO);
            let target = if is_light_vowel(harmony_vowel) {
                JUNG_A
            } else {
                JUNG_EO
            };
            let surface = replace_last_vowel(stem, target).expect("valid vowel replacement");
            let surface_len = surface.len();
            surfaces.push(derived(
                surface,
                surface_len,
                with_rule(base_rules, "contraction.eu-drop"),
            ));
        }
        JUNG_AE | JUNG_E => {
            surfaces.push(derived(format!("{stem}어"), stem.len(), base_rules.clone()));
            surfaces.push(derived(
                stem.to_owned(),
                stem.len(),
                with_rule(base_rules, "contraction.identical-vowel"),
            ));
        }
        JUNG_YEO => {
            surfaces.push(derived(format!("{stem}어"), stem.len(), base_rules.clone()));
            surfaces.push(derived(
                stem.to_owned(),
                stem.len(),
                with_rule(base_rules, "contraction.yeo-eo"),
            ));
        }
        _ => {
            let suffix = if is_light_vowel(last.jungseong) {
                "아"
            } else {
                "어"
            };
            surfaces.push(derived(format!("{stem}{suffix}"), stem.len(), base_rules));
        }
    }
    Ok(surfaces)
}

pub(super) fn eu_anchor(
    entry: &PredicateEntry,
    stem: &str,
) -> Result<Option<DerivedSurface>, GenerateError> {
    let (altered, rules) = match entry.alternation {
        LexicalAlternation::DToL => (
            require_and_replace_final(entry, stem, JONG_DIGEUT, JONG_RIEUL)?,
            vec![rule("lexical.d-to-l")],
        ),
        LexicalAlternation::DropS => (
            require_and_drop_final(entry, stem, JONG_SIOT)?,
            vec![rule("lexical.drop-s")],
        ),
        LexicalAlternation::Regular => {
            let last = stem.chars().next_back().expect("stem");
            if !has_final(last) || has_rieul_final(last) {
                return Ok(None);
            }
            (stem.to_owned(), Vec::new())
        }
        _ => return Ok(None),
    };
    let core_len = altered.len();
    Ok(Some(DerivedSurface {
        surface: format!("{altered}으"),
        core_len,
        rules,
    }))
}

pub(super) fn conditional_surface(
    entry: &PredicateEntry,
    stem: &str,
) -> Result<Option<DerivedSurface>, GenerateError> {
    let Some((base, mut rules)) = conditional_base(entry, stem)? else {
        return Ok(None);
    };
    let core_len = base.len();
    rules.push(rule("ending.conditional"));
    Ok(Some(derived(format!("{base}면"), core_len, rules)))
}

pub(super) fn coordinate_surface(
    entry: &PredicateEntry,
    stem: &str,
) -> Result<Option<DerivedSurface>, GenerateError> {
    let Some((base, mut rules)) = conditional_base(entry, stem)? else {
        return Ok(None);
    };
    let core_len = base.len();
    rules.push(rule("ending.coordinate-myeo"));
    Ok(Some(derived(format!("{base}며"), core_len, rules)))
}

pub(super) fn intentive_surface(
    entry: &PredicateEntry,
    stem: &str,
) -> Result<Option<DerivedSurface>, GenerateError> {
    if let Some(mut base) = eu_anchor(entry, stem)? {
        base.surface.push_str("려고");
        base.rules.push(rule("ending.intentive-ryeogo"));
        return Ok(Some(base));
    }
    let Some((base, mut rules)) = conditional_base(entry, stem)? else {
        return Ok(None);
    };
    let core_len = base.len();
    rules.push(rule("ending.intentive-ryeogo"));
    Ok(Some(derived(format!("{base}려고"), core_len, rules)))
}

pub(super) fn honorific_anchor(
    entry: &PredicateEntry,
    stem: &str,
) -> Result<Option<DerivedSurface>, GenerateError> {
    if entry.alternation == LexicalAlternation::Regular
        && has_rieul_final(stem.chars().next_back().expect("stem"))
    {
        return Ok(None);
    }
    let Some((base, rules)) = conditional_base(entry, stem)? else {
        return Ok(None);
    };
    let core_len = base.len();
    Ok(Some(derived(base, core_len, rules)))
}

fn conditional_base(
    entry: &PredicateEntry,
    stem: &str,
) -> Result<Option<(String, Vec<RuleId>)>, GenerateError> {
    let base = match entry.alternation {
        LexicalAlternation::BToWa | LexicalAlternation::BToWo => {
            let lexical_rule = alternation_rule(entry.alternation)
                .expect("B irregular alternation has a provenance rule");
            (b_vowel_stem(entry, stem)?, vec![lexical_rule])
        }
        LexicalAlternation::DropH => (
            require_and_drop_final(entry, stem, JONG_HIEUH)?,
            vec![rule("lexical.drop-h")],
        ),
        LexicalAlternation::Suppletive => return Ok(None),
        _ => (stem.to_owned(), Vec::new()),
    };
    Ok(Some(base))
}

pub(super) fn present_adnominal(stem: &str) -> Result<DerivedSurface, GenerateError> {
    let last = stem.chars().next_back().expect("stem");
    let base = if has_rieul_final(last) {
        drop_last_final(stem).expect("rieul-final stem")
    } else {
        stem.to_owned()
    };
    let core_len = base.len();
    Ok(derived(
        format!("{base}는"),
        core_len,
        vec![rule("ending.present-adnominal")],
    ))
}

pub(super) fn present_declarative(stem: &str) -> Result<DerivedSurface, GenerateError> {
    let last = stem.chars().next_back().expect("stem");
    let syllable =
        decompose_syllable(last).ok_or_else(|| GenerateError::InvalidLemma(stem.into()))?;
    let (base, surface) = if syllable.jongseong == JONG_NONE {
        let base = add_final(stem, JONG_NIEUN).expect("vowel-final stem");
        let surface = format!("{base}다");
        (base, surface)
    } else if syllable.jongseong == JONG_RIEUL {
        let base = replace_last_final(stem, JONG_NIEUN).expect("rieul-final stem");
        let surface = format!("{base}다");
        (base, surface)
    } else {
        (stem.to_owned(), format!("{stem}는다"))
    };
    Ok(derived(
        surface,
        base.len(),
        vec![rule("ending.declarative")],
    ))
}

pub(super) fn past_adnominal(
    entry: &PredicateEntry,
    stem: &str,
) -> Result<DerivedSurface, GenerateError> {
    let (base, force_eun, mut rules) = lexical_vowel_stem(entry, stem)?;
    let last = base.chars().next_back().expect("base");
    let syllable = decompose_syllable(last).ok_or_else(|| mismatch(entry))?;
    let surface = if force_eun || (syllable.jongseong != JONG_NONE && !has_rieul_final(last)) {
        format!("{base}은")
    } else if has_rieul_final(last) {
        replace_last_final(&base, JONG_NIEUN).expect("rieul-final stem")
    } else {
        add_final(&base, JONG_NIEUN).expect("vowel-final stem")
    };
    let core_len = if surface.len() == base.len() {
        surface.len()
    } else {
        base.len()
    };
    rules.push(rule("ending.past-adnominal"));
    Ok(derived(surface, core_len, rules))
}

pub(super) fn future_adnominal(
    entry: &PredicateEntry,
    stem: &str,
) -> Result<DerivedSurface, GenerateError> {
    let (base, force_eul, mut rules) = lexical_vowel_stem(entry, stem)?;
    let last = base.chars().next_back().expect("base");
    let syllable = decompose_syllable(last).ok_or_else(|| mismatch(entry))?;
    let surface = if force_eul || (syllable.jongseong != JONG_NONE && !has_rieul_final(last)) {
        format!("{base}을")
    } else if has_rieul_final(last) {
        base.clone()
    } else {
        add_final(&base, JONG_RIEUL).expect("vowel-final stem")
    };
    let core_len = if surface.len() == base.len() {
        surface.len()
    } else {
        base.len()
    };
    rules.push(rule("ending.future-adnominal"));
    Ok(derived(surface, core_len, rules))
}

pub(super) fn polite_declarative(stem: &str) -> Result<DerivedSurface, GenerateError> {
    let last = stem.chars().next_back().expect("stem");
    let syllable =
        decompose_syllable(last).ok_or_else(|| GenerateError::InvalidLemma(stem.into()))?;
    let (base, surface) = if syllable.jongseong == JONG_NONE {
        let base = add_final(stem, JONG_BIEUP).expect("vowel-final stem");
        let surface = format!("{base}니다");
        (base, surface)
    } else if syllable.jongseong == JONG_RIEUL {
        let base = replace_last_final(stem, JONG_BIEUP).expect("rieul-final stem");
        let surface = format!("{base}니다");
        (base, surface)
    } else {
        (stem.to_owned(), format!("{stem}습니다"))
    };
    Ok(derived(
        surface,
        base.len(),
        vec![rule("ending.polite-declarative")],
    ))
}

fn lexical_vowel_stem(
    entry: &PredicateEntry,
    stem: &str,
) -> Result<(String, bool, Vec<RuleId>), GenerateError> {
    match entry.alternation {
        LexicalAlternation::DToL => Ok((
            require_and_replace_final(entry, stem, JONG_DIGEUT, JONG_RIEUL)?,
            true,
            vec![rule("lexical.d-to-l")],
        )),
        LexicalAlternation::DropS => Ok((
            require_and_drop_final(entry, stem, JONG_SIOT)?,
            true,
            vec![rule("lexical.drop-s")],
        )),
        LexicalAlternation::BToWa | LexicalAlternation::BToWo => Ok((
            b_vowel_stem(entry, stem)?,
            false,
            vec![
                alternation_rule(entry.alternation)
                    .expect("B irregular alternation has a provenance rule"),
            ],
        )),
        LexicalAlternation::DropH => Ok((
            require_and_drop_final(entry, stem, JONG_HIEUH)?,
            false,
            vec![rule("lexical.drop-h")],
        )),
        _ => Ok((stem.to_owned(), false, Vec::new())),
    }
}

fn b_vowel_stem(entry: &PredicateEntry, stem: &str) -> Result<String, GenerateError> {
    let without_b = require_and_drop_final(entry, stem, JONG_BIEUP)?;
    Ok(format!("{without_b}우"))
}

fn require_reu_stem<'a>(entry: &PredicateEntry, stem: &'a str) -> Result<&'a str, GenerateError> {
    let (last_index, last) = stem.char_indices().next_back().expect("stem");
    let syllable = decompose_syllable(last).ok_or_else(|| mismatch(entry))?;
    if syllable.choseong != CHOSEONG_RIEUL
        || syllable.jungseong != JUNG_EU
        || syllable.jongseong != JONG_NONE
        || last_index == 0
    {
        return Err(mismatch(entry));
    }
    Ok(&stem[..last_index])
}

fn require_and_replace_final(
    entry: &PredicateEntry,
    stem: &str,
    expected: u8,
    replacement: u8,
) -> Result<String, GenerateError> {
    let last = decompose_syllable(stem.chars().next_back().expect("stem"))
        .ok_or_else(|| mismatch(entry))?;
    if last.jongseong != expected {
        return Err(mismatch(entry));
    }
    replace_last_final(stem, replacement).ok_or_else(|| mismatch(entry))
}

fn require_and_drop_final(
    entry: &PredicateEntry,
    stem: &str,
    expected: u8,
) -> Result<String, GenerateError> {
    require_and_replace_final(entry, stem, expected, JONG_NONE)
}

fn preceding_harmony_vowel(stem: &str) -> Option<u8> {
    stem.chars()
        .rev()
        .skip(1)
        .find_map(decompose_syllable)
        .map(|syllable| syllable.jungseong)
}

fn last_vowel(stem: &str) -> Result<u8, GenerateError> {
    decompose_syllable(stem.chars().next_back().expect("stem"))
        .map(|syllable| syllable.jungseong)
        .ok_or_else(|| GenerateError::InvalidLemma(stem.into()))
}

const fn is_light_vowel(vowel: u8) -> bool {
    matches!(vowel, JUNG_A | JUNG_YA | JUNG_O | JUNG_WA)
}

fn alternation_rule(alternation: LexicalAlternation) -> Option<RuleId> {
    let id = match alternation {
        LexicalAlternation::Regular => return None,
        LexicalAlternation::DToL => "lexical.d-to-l",
        LexicalAlternation::DropS => "lexical.drop-s",
        LexicalAlternation::BToWa => "lexical.b-to-wa",
        LexicalAlternation::BToWo => "lexical.b-to-wo",
        LexicalAlternation::DropH => "lexical.drop-h",
        LexicalAlternation::ReuDoubleL => "lexical.reu-double-l",
        LexicalAlternation::Reo => "lexical.reo",
        LexicalAlternation::Ha => "lexical.ha",
        LexicalAlternation::UToEo => "lexical.u-to-eo",
        LexicalAlternation::Copula => "lexical.copula",
        LexicalAlternation::Suppletive => "lexical.suppletive",
    };
    Some(rule(id))
}

fn mismatch(entry: &PredicateEntry) -> GenerateError {
    GenerateError::AlternationMismatch {
        lemma: entry.lemma.clone(),
        alternation: entry.alternation,
    }
}

fn with_rule(mut rules: Vec<RuleId>, id: &str) -> Vec<RuleId> {
    rules.push(rule(id));
    rules
}

fn derived(surface: String, core_len: usize, rules: Vec<RuleId>) -> DerivedSurface {
    DerivedSurface {
        surface,
        core_len,
        rules,
    }
}

fn rule(id: &str) -> RuleId {
    RuleId::from(id)
}
