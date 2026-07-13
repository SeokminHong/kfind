//! Predicate stem alternation and ending branch generation.

use std::error::Error;
use std::fmt;

use crate::hangul::{
    JONG_SSANGSIOT, JUNG_YEO, add_final, decompose_syllable, drop_last_final, has_rieul_final,
    replace_last_vowel,
};
use crate::{ContinuationState, LexicalAlternation, PredicateEntry, RuleId, SurfaceBranchSpec};

mod alternation;
mod continuation;

pub use continuation::{PredicateContinuationMatch, verify_predicate_continuation};

use alternation::{
    aeo_surfaces, conditional_surface, coordinate_surface, eu_anchor, future_adnominal,
    honorific_anchor, intentive_surface, past_adnominal, polite_declarative, present_adnominal,
    present_declarative,
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
    push_branch(
        &mut branches,
        entry,
        entry.lemma.to_string(),
        stem.len(),
        ContinuationState::Terminal,
        vec![rule("ending.final-da")],
    );

    if entry.alternation != LexicalAlternation::Suppletive {
        push_branch(
            &mut branches,
            entry,
            format!("{stem}기"),
            stem.len(),
            ContinuationState::Terminal,
            vec![rule("ending.nominalizer-gi")],
        );
    }

    if entry.alternation == LexicalAlternation::Copula {
        compile_copula(entry, stem, &mut branches)?;
    } else if entry.alternation != LexicalAlternation::Suppletive {
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

fn compile_productive(
    entry: &PredicateEntry,
    stem: &str,
    branches: &mut Vec<SurfaceBranchSpec>,
) -> Result<(), GenerateError> {
    for (suffix, ending_rule) in [
        ("고", "ending.connective-go"),
        ("지", "ending.connective-ji"),
        ("게", "ending.adverbial-ge"),
        ("더라도", "ending.concessive-deorado"),
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
        if let Some(intentive) = intentive_surface(entry, stem)? {
            push_derived(branches, entry, intentive, ContinuationState::Terminal);
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
            ContinuationState::Terminal,
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
    push_branch(
        branches,
        entry,
        format!("{dropped}니"),
        dropped.len(),
        ContinuationState::Terminal,
        vec![rule("ending.connective-ni")],
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
