//! Predicate stem alternation and ending branch generation.

use std::error::Error;
use std::fmt;
use std::sync::OnceLock;

use crate::hangul::{
    JONG_NONE, JONG_RIEUL, JONG_SSANGSIOT, JUNG_YEO, add_final, decompose_syllable,
    drop_last_final, has_rieul_final, replace_last_vowel,
};
use crate::{
    ContinuationState, LexicalAlternation, PredicateEntry, PredicateFlags, PredicatePos,
    PredicateStemClass, RuleId, SurfaceBranchSpec,
};

mod alternation;
mod continuation;

pub use continuation::{PredicateContinuationMatch, verify_predicate_continuation};

use alternation::{
    aeo_surfaces, conditional_surface, coordinate_surface, ending_base, eu_anchor,
    future_adnominal, honorific_anchor, intentive_adnominal_surface, intentive_surface,
    nominalizer_surface, past_adnominal, polite_declarative, present_adnominal,
    present_declarative, propositive_surface,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerateError {
    InvalidLemma(Box<str>),
    AlternationMismatch {
        lemma: Box<str>,
        alternation: LexicalAlternation,
    },
    InvalidOverride {
        lemma: Box<str>,
        surface: Box<str>,
    },
}

impl fmt::Display for GenerateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLemma(lemma) => {
                write!(
                    formatter,
                    "predicate lemma must be a non-empty -다 form: {lemma}"
                )
            }
            Self::AlternationMismatch { lemma, alternation } => write!(
                formatter,
                "predicate stem does not satisfy {alternation:?}: {lemma}"
            ),
            Self::InvalidOverride { lemma, surface } => write!(
                formatter,
                "override core length is invalid for {lemma}: {surface}"
            ),
        }
    }
}

impl Error for GenerateError {}

#[derive(Debug, Clone)]
struct DerivedSurface {
    surface: String,
    core_len: usize,
    rules: Vec<RuleId>,
}

/// Compiles a predicate entry into fixed anchors and suffix-verifier start states.
///
/// The result intentionally stops at productive continuation states such as
/// `Past` and `Eu`; it does not enumerate complete ending chains.
pub fn generate_predicate_branches(
    entry: &PredicateEntry,
) -> Result<Vec<SurfaceBranchSpec>, GenerateError> {
    let stem = entry
        .lemma
        .strip_suffix('다')
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| GenerateError::InvalidLemma(entry.lemma.clone()))?;
    if decompose_syllable(stem.chars().next_back().expect("non-empty stem")).is_none() {
        return Err(GenerateError::InvalidLemma(entry.lemma.clone()));
    }

    let mut branches = Vec::new();
    let final_da_continuation = if matches!(
        entry.pos,
        PredicatePos::Adjective | PredicatePos::AuxiliaryAdjective
    ) && !entry
        .flags
        .contains(PredicateFlags::NO_DECLARATIVE_CONTINUATION)
    {
        ContinuationState::Declarative
    } else {
        ContinuationState::Terminal
    };
    push_branch(
        &mut branches,
        entry,
        entry.lemma.to_string(),
        stem.len(),
        final_da_continuation,
        vec![rule("ending.final-da")],
    );

    if !matches!(
        entry.alternation,
        LexicalAlternation::Suppletive | LexicalAlternation::SurfaceOnly
    ) {
        push_branch(
            &mut branches,
            entry,
            format!("{stem}기"),
            stem.len(),
            ContinuationState::Terminal,
            vec![rule("ending.nominalizer-gi")],
        );
        push_derived(
            &mut branches,
            entry,
            nominalizer_surface(entry, stem)?,
            ContinuationState::Terminal,
        );
    }

    if entry.alternation == LexicalAlternation::Copula {
        compile_copula(entry, stem, &mut branches)?;
    } else if !matches!(
        entry.alternation,
        LexicalAlternation::Suppletive | LexicalAlternation::SurfaceOnly
    ) {
        compile_productive(entry, stem, &mut branches)?;
    }

    for override_form in &entry.overrides {
        if override_form.core_len > override_form.surface.len()
            || !override_form
                .surface
                .is_char_boundary(override_form.core_len)
        {
            return Err(GenerateError::InvalidOverride {
                lemma: entry.lemma.clone(),
                surface: override_form.surface.clone(),
            });
        }
        push_branch(
            &mut branches,
            entry,
            override_form.surface.to_string(),
            override_form.core_len,
            override_form.continuation,
            vec![override_form.rule_id.clone()],
        );
    }

    Ok(branches)
}

/// Returns whether the whole surface is a generated inflection of the copula `이다`.
///
/// This is intentionally stricter than prefix verification: terminal branches must consume the
/// whole input, and productive branches must also consume their complete continuation.
#[must_use]
fn verify_complete_copula_surface(surface: &str) -> bool {
    static BRANCHES: OnceLock<Box<[SurfaceBranchSpec]>> = OnceLock::new();

    BRANCHES
        .get_or_init(|| {
            generate_predicate_branches(&PredicateEntry::new(
                "이다",
                PredicatePos::Copula,
                LexicalAlternation::Copula,
            ))
            .unwrap_or_default()
            .into_boxed_slice()
        })
        .iter()
        .any(|branch| {
            surface
                .strip_prefix(branch.anchor.as_ref())
                .and_then(|following| {
                    verify_predicate_continuation(
                        branch.continuation,
                        branch.pos,
                        branch.anchor.as_ref(),
                        following,
                    )
                })
                .is_some_and(|matched| {
                    matched.token_end == surface.len()
                        && (branch.continuation == ContinuationState::Terminal
                            || matched.consumed_bytes > 0)
                })
        })
}

/// Verifies an explicit or phonologically contracted copula after a nominal surface.
#[must_use]
pub fn verify_copula_surface_after_nominal(preceding: char, surface: &str) -> bool {
    if verify_complete_copula_surface(surface) {
        return !surface.starts_with('여') || preceding_allows_copula_contraction(preceding);
    }
    if !preceding_allows_copula_contraction(preceding) {
        return false;
    }
    let expanded = if surface == "다" {
        Some("이다".to_owned())
    } else {
        surface
            .strip_prefix("였")
            .map(|following| format!("이었{following}"))
    };
    expanded.is_some_and(|surface| verify_complete_copula_surface(&surface))
}

fn preceding_allows_copula_contraction(preceding: char) -> bool {
    decompose_syllable(preceding).is_some_and(|syllable| syllable.jongseong == JONG_NONE)
}

pub fn generate_predicate_fallback_stems(
    entry: &PredicateEntry,
) -> Result<Vec<(SurfaceBranchSpec, PredicateStemClass)>, GenerateError> {
    let stem = entry
        .lemma
        .strip_suffix('다')
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| GenerateError::InvalidLemma(entry.lemma.clone()))?;
    let mut stems = Vec::new();
    push_fallback_stem(&mut stems, entry, stem.to_owned(), stem.len(), Vec::new())?;
    if let Some(eu) = eu_anchor(entry, stem)? {
        push_fallback_stem(&mut stems, entry, eu.surface, eu.core_len, eu.rules)?;
    }
    if let Some(base) = ending_base(entry, stem)?
        && base.surface != stem
    {
        push_fallback_stem(&mut stems, entry, base.surface, base.core_len, base.rules)?;
    }
    stems.sort_by(|left, right| left.0.anchor.cmp(&right.0.anchor));
    stems.dedup_by(|left, right| {
        left.0.anchor == right.0.anchor && left.0.rule_path == right.0.rule_path
    });
    Ok(stems)
}

fn push_fallback_stem(
    output: &mut Vec<(SurfaceBranchSpec, PredicateStemClass)>,
    entry: &PredicateEntry,
    anchor: String,
    core_len: usize,
    rule_path: Vec<RuleId>,
) -> Result<(), GenerateError> {
    let final_syllable = anchor
        .chars()
        .next_back()
        .and_then(decompose_syllable)
        .ok_or_else(|| GenerateError::InvalidLemma(entry.lemma.clone()))?;
    let class = match final_syllable.jongseong {
        JONG_NONE => PredicateStemClass::Vowel,
        JONG_RIEUL => PredicateStemClass::Rieul,
        _ => PredicateStemClass::Consonant,
    };
    output.push((
        SurfaceBranchSpec {
            anchor: anchor.into_boxed_str(),
            core_len,
            continuation: ContinuationState::Terminal,
            rule_path,
            pos: entry.pos,
            alternation: entry.alternation,
        },
        class,
    ));
    Ok(())
}

fn compile_productive(
    entry: &PredicateEntry,
    stem: &str,
    branches: &mut Vec<SurfaceBranchSpec>,
) -> Result<(), GenerateError> {
    for (suffix, ending_rule) in [
        ("고", "ending.connective-go"),
        ("지", "ending.connective-ji"),
        ("게", "ending.adverbial-ge"),
        ("던", "ending.retrospective-adnominal"),
        ("더니", "ending.retrospective-connective"),
        ("더라도", "ending.concessive-deorado"),
        ("도록", "ending.purpose-dorok"),
    ] {
        push_branch(
            branches,
            entry,
            format!("{stem}{suffix}"),
            stem.len(),
            ContinuationState::Terminal,
            vec![rule(ending_rule)],
        );
    }
    push_branch(
        branches,
        entry,
        format!("{stem}겠"),
        stem.len(),
        ContinuationState::Future,
        vec![rule("ending.future")],
    );

    for aeo in aeo_surfaces(entry, stem)? {
        let mut aeo_rules = aeo.rules.clone();
        aeo_rules.push(rule("ending.aoeo"));
        push_branch(
            branches,
            entry,
            aeo.surface.clone(),
            aeo.core_len,
            ContinuationState::AOrEo,
            aeo_rules,
        );

        let past = add_final(&aeo.surface, JONG_SSANGSIOT).ok_or_else(|| {
            GenerateError::AlternationMismatch {
                lemma: entry.lemma.clone(),
                alternation: entry.alternation,
            }
        })?;
        let mut past_rules = aeo.rules;
        past_rules.push(rule("ending.past"));
        push_branch(
            branches,
            entry,
            past,
            aeo.core_len,
            ContinuationState::Past,
            past_rules,
        );
    }

    if let Some(eu) = eu_anchor(entry, stem)? {
        push_branch(
            branches,
            entry,
            eu.surface,
            eu.core_len,
            ContinuationState::Eu,
            eu.rules,
        );
    } else if let Some(conditional) = conditional_surface(entry, stem)? {
        push_derived(branches, entry, conditional, ContinuationState::Terminal);
        if let Some(coordinate) = coordinate_surface(entry, stem)? {
            push_derived(branches, entry, coordinate, ContinuationState::Terminal);
        }
        if entry.alternation == LexicalAlternation::Regular
            && has_rieul_final(stem.chars().next_back().expect("stem"))
        {
            compile_rieul_eu_endings(entry, stem, branches);
            compile_rieul_honorific(entry, stem, branches);
        } else if let Some(honorific) = honorific_anchor(entry, stem)? {
            push_derived(branches, entry, honorific, ContinuationState::Eu);
        }
    }

    if entry.pos.is_action() {
        for (suffix, rules) in [
            ("자", vec![rule("ending.propositive-ja")]),
            (
                "자고",
                vec![rule("ending.propositive-ja"), rule("ending.quotative-go")],
            ),
            ("느냐", vec![rule("ending.interrogative-neunya")]),
            (
                "곤",
                vec![rule("ending.connective-go"), rule("particle.topic")],
            ),
        ] {
            push_branch(
                branches,
                entry,
                format!("{stem}{suffix}"),
                stem.len(),
                ContinuationState::Terminal,
                rules,
            );
        }
        push_branch(
            branches,
            entry,
            format!("{stem}거라"),
            stem.len(),
            ContinuationState::Terminal,
            vec![rule("ending.imperative-geora")],
        );
        if stem.ends_with('오') {
            push_branch(
                branches,
                entry,
                format!("{stem}너라"),
                stem.len(),
                ContinuationState::Terminal,
                vec![rule("ending.imperative-neora")],
            );
        }
        if let Some(intentive) = intentive_surface(entry, stem)? {
            push_derived(branches, entry, intentive, ContinuationState::Terminal);
        }
        if let Some(intentive) = intentive_adnominal_surface(entry, stem)? {
            push_derived(branches, entry, intentive, ContinuationState::Terminal);
        }
        if let Some(propositive) = propositive_surface(entry, stem)? {
            push_derived(branches, entry, propositive, ContinuationState::Terminal);
        }
        push_derived(
            branches,
            entry,
            present_adnominal(stem)?,
            ContinuationState::Terminal,
        );
        push_derived(
            branches,
            entry,
            present_declarative(stem)?,
            ContinuationState::Declarative,
        );
    }
    push_derived(
        branches,
        entry,
        past_adnominal(entry, stem)?,
        ContinuationState::Terminal,
    );
    push_derived(
        branches,
        entry,
        future_adnominal(entry, stem)?,
        ContinuationState::Terminal,
    );
    let mut rieulse = future_adnominal(entry, stem)?;
    rieulse.surface.push('세');
    let ending = rieulse
        .rules
        .last_mut()
        .expect("future adnominal surface has an ending rule");
    *ending = rule("ending.final-rieulse");
    push_derived(branches, entry, rieulse, ContinuationState::Terminal);
    push_derived(
        branches,
        entry,
        polite_declarative(stem)?,
        ContinuationState::Terminal,
    );

    Ok(())
}

fn compile_rieul_eu_endings(
    entry: &PredicateEntry,
    stem: &str,
    branches: &mut Vec<SurfaceBranchSpec>,
) {
    let dropped = drop_last_final(stem).expect("rieul-final stem");
    for ending in ["니", "니까", "니까는", "니깐"] {
        push_branch(
            branches,
            entry,
            format!("{dropped}{ending}"),
            dropped.len(),
            ContinuationState::Terminal,
            vec![rule("ending.connective-ni")],
        );
    }
    push_branch(
        branches,
        entry,
        format!("{stem}리라"),
        stem.len(),
        ContinuationState::Terminal,
        vec![rule("ending.prospective-final")],
    );
    push_branch(
        branches,
        entry,
        format!("{stem}리라고"),
        stem.len(),
        ContinuationState::Terminal,
        vec![rule("ending.prospective-quotative")],
    );
}

fn compile_rieul_honorific(
    entry: &PredicateEntry,
    stem: &str,
    branches: &mut Vec<SurfaceBranchSpec>,
) {
    let base = drop_last_final(stem).expect("rieul-final stem");
    let core_len = base.len();
    let past_base = replace_last_vowel(&format!("{base}시"), JUNG_YEO)
        .expect("honorific syllable accepts a vowel replacement");
    let past =
        add_final(&past_base, JONG_SSANGSIOT).expect("honorific syllable accepts a past final");
    push_branch(
        branches,
        entry,
        past,
        core_len,
        ContinuationState::Past,
        vec![
            rule("ending.honorific"),
            rule("contraction.si-past"),
            rule("ending.past"),
        ],
    );
    for (surface, rules) in [
        (
            format!("{base}시다"),
            vec![rule("ending.honorific"), rule("ending.final-da")],
        ),
        (
            format!("{base}십니다"),
            vec![rule("ending.honorific"), rule("ending.polite-declarative")],
        ),
        (
            format!("{base}시면"),
            vec![rule("ending.honorific"), rule("ending.conditional")],
        ),
        (
            format!("{base}신"),
            vec![rule("ending.honorific"), rule("ending.past-adnominal")],
        ),
        (
            format!("{base}실"),
            vec![rule("ending.honorific"), rule("ending.future-adnominal")],
        ),
    ] {
        push_branch(
            branches,
            entry,
            surface,
            core_len,
            ContinuationState::Terminal,
            rules,
        );
    }
}

fn compile_copula(
    entry: &PredicateEntry,
    stem: &str,
    branches: &mut Vec<SurfaceBranchSpec>,
) -> Result<(), GenerateError> {
    if stem != "이" {
        return Err(GenerateError::AlternationMismatch {
            lemma: entry.lemma.clone(),
            alternation: entry.alternation,
        });
    }
    for (surface, continuation, ending_rule) in [
        (
            format!("{stem}고"),
            ContinuationState::Terminal,
            "ending.connective-go",
        ),
        (format!("{stem}어"), ContinuationState::AOrEo, "ending.aoeo"),
        (
            "여서".to_owned(),
            ContinuationState::Terminal,
            "ending.aoeo-seo",
        ),
        (
            "인".to_owned(),
            ContinuationState::Terminal,
            "ending.past-adnominal",
        ),
        (
            "일".to_owned(),
            ContinuationState::Terminal,
            "ending.future-adnominal",
        ),
        (
            format!("{stem}라고"),
            ContinuationState::Terminal,
            "ending.copula-quotative-go",
        ),
        (
            format!("{stem}라는"),
            ContinuationState::Terminal,
            "ending.copula-quotative-adnominal",
        ),
        (
            format!("{stem}지"),
            ContinuationState::Terminal,
            "ending.connective-ji",
        ),
        (
            format!("{stem}며"),
            ContinuationState::Terminal,
            "ending.coordinate-myeo",
        ),
        (
            format!("{stem}므로"),
            ContinuationState::Terminal,
            "ending.connective-meuro",
        ),
    ] {
        push_branch(
            branches,
            entry,
            surface,
            stem.len(),
            continuation,
            vec![rule("lexical.copula"), rule(ending_rule)],
        );
    }

    let mut polite = polite_declarative(stem)?;
    polite.rules.insert(0, rule("lexical.copula"));
    push_derived(branches, entry, polite, ContinuationState::Terminal);

    let past = add_final(&format!("{stem}어"), JONG_SSANGSIOT)
        .expect("copula vowel ending accepts a past final");
    push_branch(
        branches,
        entry,
        past,
        stem.len(),
        ContinuationState::Past,
        vec![rule("lexical.copula"), rule("ending.past")],
    );
    Ok(())
}

fn push_derived(
    branches: &mut Vec<SurfaceBranchSpec>,
    entry: &PredicateEntry,
    derived: DerivedSurface,
    continuation: ContinuationState,
) {
    push_branch(
        branches,
        entry,
        derived.surface,
        derived.core_len,
        continuation,
        derived.rules,
    );
}

fn push_branch(
    branches: &mut Vec<SurfaceBranchSpec>,
    entry: &PredicateEntry,
    anchor: String,
    core_len: usize,
    continuation: ContinuationState,
    rule_path: Vec<RuleId>,
) {
    branches.push(SurfaceBranchSpec {
        anchor: anchor.into_boxed_str(),
        core_len,
        continuation,
        rule_path,
        pos: entry.pos,
        alternation: entry.alternation,
    });
}

fn rule(id: &str) -> RuleId {
    RuleId::from(id)
}

#[cfg(test)]
#[path = "predicate/tests.rs"]
mod tests;
