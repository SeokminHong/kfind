//! Predicate stem alternation and ending branch generation.

use std::error::Error;
use std::fmt;
use std::sync::OnceLock;

use crate::hangul::{
    JONG_IEUNG, JONG_MIEUM, JONG_NIEUN, JONG_NONE, JONG_RIEUL, JONG_SSANGSIOT, JUNG_U, JUNG_WEO,
    JUNG_YEO, add_final, decompose_syllable, drop_last_final, has_rieul_final, replace_last_vowel,
};
use crate::{
    ContinuationState, LexicalAlternation, PredicateDerivation, PredicateEntry, PredicateFlags,
    PredicatePos, PredicateStemClass, RuleId, SurfaceBranchSpec,
};

mod alternation;
mod continuation;

pub use continuation::{PredicateContinuationMatch, verify_predicate_continuation};

use alternation::{
    aeo_surfaces, bieup_ending_base, ending_base, eu_anchor, formal_bieup_anchor, future_adnominal,
    intentive_adnominal_surface, intentive_surface, nominalizer_surface, past_adnominal,
    present_adnominal, present_declarative,
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

impl PredicateDerivation {
    pub fn generated_branches(&self) -> Result<&[SurfaceBranchSpec], GenerateError> {
        self.branches
            .get_or_init(|| {
                generate_predicate_branches(&PredicateEntry::new(
                    self.target_lemma.clone(),
                    self.target_pos,
                    LexicalAlternation::Regular,
                ))
                .map(Vec::into_boxed_slice)
            })
            .as_deref()
            .map_err(Clone::clone)
    }

    pub fn generated_fallback_stems(
        &self,
    ) -> Result<&[(SurfaceBranchSpec, PredicateStemClass)], GenerateError> {
        self.fallback_stems
            .get_or_init(|| {
                generate_predicate_fallback_stems(&PredicateEntry::new(
                    self.target_lemma.clone(),
                    self.target_pos,
                    LexicalAlternation::Regular,
                ))
                .map(Vec::into_boxed_slice)
            })
            .as_deref()
            .map_err(Clone::clone)
    }
}

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
    compile_stem(entry, stem, &mut branches)?;
    if let Some(contracted) = contract_eoji_stem(entry, stem) {
        let first_alias = branches.len();
        compile_stem(entry, &contracted, &mut branches)?;
        for branch in &mut branches[first_alias..] {
            branch.rule_path.insert(0, rule("contraction.u-eo"));
        }
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

fn contract_eoji_stem(entry: &PredicateEntry, stem: &str) -> Option<String> {
    if !entry.pos.is_action() || entry.alternation != LexicalAlternation::Regular {
        return None;
    }
    let base = stem.strip_suffix("어지")?;
    let last = decompose_syllable(base.chars().next_back()?)?;
    if last.jungseong != JUNG_U || last.jongseong != JONG_NONE {
        return None;
    }
    let mut contracted = replace_last_vowel(base, JUNG_WEO)?;
    contracted.push('지');
    Some(contracted)
}

fn compile_stem(
    entry: &PredicateEntry,
    stem: &str,
    branches: &mut Vec<SurfaceBranchSpec>,
) -> Result<(), GenerateError> {
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
        branches,
        entry,
        format!("{stem}다"),
        stem.len(),
        final_da_continuation,
        vec![rule("ending.final-da")],
    );

    if !matches!(
        entry.alternation,
        LexicalAlternation::Suppletive | LexicalAlternation::SurfaceOnly
    ) {
        push_branch(
            branches,
            entry,
            format!("{stem}기"),
            stem.len(),
            ContinuationState::Terminal,
            vec![rule("ending.nominalizer-gi")],
        );
        push_derived(
            branches,
            entry,
            nominalizer_surface(entry, stem)?,
            ContinuationState::Terminal,
        );
    }

    if entry.alternation == LexicalAlternation::Copula {
        compile_copula(entry, stem, branches)?;
    } else if !matches!(
        entry.alternation,
        LexicalAlternation::Suppletive | LexicalAlternation::SurfaceOnly
    ) {
        compile_productive(entry, stem, branches)?;
    }

    Ok(())
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
                            || !matched.rule_path.is_empty()
                            || matched.consumed_bytes > 0)
                })
        })
}

/// Verifies an explicit or phonologically contracted copula after a nominal surface.
#[must_use]
pub fn verify_copula_surface_after_nominal(preceding: char, surface: &str) -> bool {
    if is_omitted_copula_ending(surface) {
        return preceding_allows_copula_contraction(preceding);
    }
    if !verify_complete_copula_surface(surface) {
        return false;
    }
    !is_contracted_copula_surface(surface) || preceding_allows_copula_contraction(preceding)
}

/// Tests the ending after a vowel-final nominal when `이-` is omitted.
#[must_use]
pub fn is_omitted_copula_ending(surface: &str) -> bool {
    continuation::is_copula_omission_ending(surface.strip_prefix('시').unwrap_or(surface))
}

/// Tests whether a syllable can begin a registered ending with an omitted copula stem.
#[must_use]
pub fn starts_omitted_copula_ending(character: char) -> bool {
    matches!(
        character,
        '라' | '란'
            | '게'
            | '람'
            | '랍'
            | '래'
            | '로'
            | '구'
            | '군'
            | '네'
            | '니'
            | '면'
            | '죠'
            | '지'
            | '예'
            | '여'
    )
}

fn preceding_allows_copula_contraction(preceding: char) -> bool {
    decompose_syllable(preceding).is_some_and(|syllable| syllable.jongseong == JONG_NONE)
}

fn is_contracted_copula_surface(surface: &str) -> bool {
    surface == "다" || surface.starts_with(['였', '여'])
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
    if let Some(contracted) = contract_eoji_stem(entry, stem) {
        let core_len = contracted.len();
        push_fallback_stem(
            &mut stems,
            entry,
            contracted,
            core_len,
            vec![rule("contraction.u-eo")],
        )?;
    }
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
    let uses_neun =
        entry.pos.is_action() || stem.ends_with("있") || stem.ends_with("없") || stem == "계시";
    push_branch(
        branches,
        entry,
        stem.to_owned(),
        stem.len(),
        ContinuationState::StemEnding,
        Vec::new(),
    );
    if entry.alternation == LexicalAlternation::Ha
        && let Some(base) = stem.strip_suffix('하')
        && base
            .chars()
            .next_back()
            .and_then(decompose_syllable)
            .is_some_and(|syllable| {
                matches!(
                    syllable.jongseong,
                    JONG_NONE | JONG_RIEUL | JONG_NIEUN | JONG_MIEUM | JONG_IEUNG
                )
            })
    {
        let surface = format!("{base}치");
        let core_len = surface.len();
        push_branch(
            branches,
            entry,
            surface,
            core_len,
            ContinuationState::Terminal,
            vec![rule("contraction.ha-ji"), rule("ending.connective-ji")],
        );
    }
    for ending in continuation::STEM_FIXED_ENDINGS {
        push_branch(
            branches,
            entry,
            format!("{stem}{}", ending.surface),
            stem.len(),
            ContinuationState::Terminal,
            ending.rules.iter().map(|id| rule(id)).collect(),
        );
    }
    let neyo_base = if has_rieul_final(stem.chars().next_back().expect("stem")) {
        drop_last_final(stem).expect("rieul-final stem")
    } else {
        stem.to_owned()
    };
    for (base, suffix, ending_rule) in [
        (neyo_base.as_str(), "네", "ending.final-ne"),
        (stem, "더군", "ending.final-deogun"),
    ] {
        for polite in ["", "요"] {
            let mut rules = vec![rule(ending_rule)];
            if !polite.is_empty() {
                rules.push(rule("ending.polite-yo"));
            }
            push_branch(
                branches,
                entry,
                format!("{base}{suffix}{polite}"),
                base.len(),
                ContinuationState::Terminal,
                rules,
            );
        }
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
        if !uses_neun {
            push_adjective_question(branches, entry, eu.clone());
        }
        push_derived(branches, entry, eu, ContinuationState::Eu);
    } else if let Some(base) = ending_base(entry, stem)? {
        for ending in [
            continuation::CONDITIONAL,
            continuation::COORDINATE,
            continuation::INTENTIVE_CONDITIONAL,
        ]
        .into_iter()
        .chain(entry.pos.is_action().then_some(continuation::INTENTIVE))
        {
            let mut rules = base.rules.clone();
            rules.extend(ending.rules.iter().map(|id| rule(id)));
            push_branch(
                branches,
                entry,
                format!("{}{}", base.surface, ending.surface),
                base.core_len,
                ContinuationState::Terminal,
                rules,
            );
        }
        if entry.alternation == LexicalAlternation::Regular
            && has_rieul_final(stem.chars().next_back().expect("stem"))
        {
            if !uses_neun {
                let dropped = drop_last_final(stem).expect("rieul-final stem");
                let core_len = dropped.len();
                push_adjective_question(
                    branches,
                    entry,
                    DerivedSurface {
                        surface: dropped,
                        core_len,
                        rules: Vec::new(),
                    },
                );
            }
            compile_rieul_eu_endings(entry, stem, branches);
            compile_rieul_honorific(entry, stem, branches);
        } else {
            if !uses_neun && base.surface != stem {
                push_adjective_question(branches, entry, base.clone());
            }
            push_derived(branches, entry, base, ContinuationState::Eu);
        }
    }

    if entry.pos.is_action() {
        for (suffix, rules) in [
            ("자", vec![rule("ending.propositive-ja")]),
            (
                "자고",
                vec![rule("ending.propositive-ja"), rule("ending.quotative-go")],
            ),
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
        if let Some(base) = bieup_ending_base(entry, stem)? {
            for (suffix, ending) in [
                ("시다", "ending.propositive"),
                ("시사", "ending.earnest-request-sisa"),
                ("시오", "ending.imperative-formal"),
                ("쇼", "ending.imperative-formal-short"),
            ] {
                let mut rules = base.rules.clone();
                rules.push(rule(ending));
                push_branch(
                    branches,
                    entry,
                    format!("{}{suffix}", base.surface),
                    base.core_len,
                    ContinuationState::Terminal,
                    rules,
                );
            }
        }
        push_derived(
            branches,
            entry,
            present_declarative(stem)?,
            ContinuationState::Declarative,
        );
    }
    let past = past_adnominal(entry, stem)?;
    if entry.pos.is_action() {
        let mut completed = past.clone();
        completed.rules.pop();
        push_derived(
            branches,
            entry,
            completed,
            ContinuationState::CompletedNieunEnding,
        );
    }
    let mut background = if uses_neun {
        let present = present_adnominal(stem)?;
        let question_stem = present
            .surface
            .strip_suffix('는')
            .expect("present adnominal ends in 는");
        push_branch(
            branches,
            entry,
            format!("{question_stem}느냐"),
            present.core_len,
            ContinuationState::Interrogative,
            vec![rule("ending.interrogative-neunya")],
        );
        if entry.pos.is_action() {
            push_derived(
                branches,
                entry,
                present.clone(),
                ContinuationState::Terminal,
            );
        }
        present
    } else {
        past.clone()
    };
    push_derived(branches, entry, past, ContinuationState::Terminal);
    background.rules.pop();
    push_derived(branches, entry, background, ContinuationState::NieunEnding);
    let mut rieul = future_adnominal(entry, stem)?;
    push_derived(branches, entry, rieul.clone(), ContinuationState::Terminal);
    rieul.rules.pop();
    push_derived(branches, entry, rieul, ContinuationState::RieulFinal);
    push_derived(
        branches,
        entry,
        formal_bieup_anchor(stem)?,
        ContinuationState::BieupFinal,
    );

    Ok(())
}

fn push_adjective_question(
    branches: &mut Vec<SurfaceBranchSpec>,
    entry: &PredicateEntry,
    mut base: DerivedSurface,
) {
    base.surface.push('냐');
    base.rules.push(rule("ending.interrogative-nya"));
    push_derived(branches, entry, base, ContinuationState::Interrogative);
}

fn compile_rieul_eu_endings(
    entry: &PredicateEntry,
    stem: &str,
    branches: &mut Vec<SurfaceBranchSpec>,
) {
    let dropped = drop_last_final(stem).expect("rieul-final stem");
    for (base, suffix, rule_id) in [
        (stem, "면서", "ending.simultaneous-myeonseo"),
        (&dropped, "나", "ending.connective-na"),
    ]
    .into_iter()
    .chain(
        entry
            .pos
            .is_action()
            .then_some([
                (stem, "러", "ending.purpose-reo"),
                (stem, "려던", "ending.intentive-adnominal"),
                (stem, "렴", "ending.imperative-ryeom"),
                (stem, "려무나", "ending.imperative-ryeom"),
                (&dropped, "시오", "ending.imperative-sio"),
                (&dropped, "쇼", "ending.imperative-sio-short"),
            ])
            .into_iter()
            .flatten(),
    ) {
        push_branch(
            branches,
            entry,
            format!("{base}{suffix}"),
            base.len(),
            ContinuationState::Terminal,
            vec![rule(rule_id)],
        );
    }
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
    for suffix in continuation::HAO_ENDINGS
        .iter()
        .chain([&continuation::SOSEO])
    {
        push_branch(
            branches,
            entry,
            format!("{dropped}{}", suffix.surface),
            dropped.len(),
            ContinuationState::Terminal,
            suffix.rules.iter().map(|id| rule(id)).collect(),
        );
    }
    push_branch(
        branches,
        entry,
        stem.to_owned(),
        stem.len(),
        ContinuationState::Prospective,
        Vec::new(),
    );
}

fn compile_rieul_honorific(
    entry: &PredicateEntry,
    stem: &str,
    branches: &mut Vec<SurfaceBranchSpec>,
) {
    let base = drop_last_final(stem).expect("rieul-final stem");
    let core_len = base.len();
    push_branch(
        branches,
        entry,
        format!("{base}실"),
        core_len,
        ContinuationState::RieulFinal,
        vec![rule("ending.honorific")],
    );
    for suffix in continuation::HONORIFIC_GEODEUN_ENDINGS
        .iter()
        .chain(continuation::HONORIFIC_YAJI_ENDINGS)
        .chain(if entry.pos.is_action() {
            continuation::HONORIFIC_HAO_AFTER_ACTION
        } else {
            continuation::HONORIFIC_HAO_ENDINGS
        })
    {
        push_branch(
            branches,
            entry,
            format!("{base}{}", suffix.surface),
            core_len,
            ContinuationState::Terminal,
            suffix.rules.iter().map(|id| rule(id)).collect(),
        );
    }
    let intentive = continuation::HONORIFIC_INTENTIVE_CONDITIONAL;
    push_branch(
        branches,
        entry,
        format!("{base}{}", intentive.surface),
        core_len,
        ContinuationState::Terminal,
        intentive.rules.iter().map(|id| rule(id)).collect(),
    );
    let past_base = replace_last_vowel(&format!("{base}시"), JUNG_YEO)
        .expect("honorific syllable accepts a vowel replacement");
    let past =
        add_final(&past_base, JONG_SSANGSIOT).expect("honorific syllable accepts a past final");
    push_branch(
        branches,
        entry,
        format!("{base}시겠"),
        core_len,
        ContinuationState::Future,
        vec![rule("ending.honorific"), rule("ending.future")],
    );
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
    push_branch(
        branches,
        entry,
        format!("{base}십"),
        core_len,
        ContinuationState::BieupFinal,
        vec![rule("ending.honorific")],
    );
    push_branch(
        branches,
        entry,
        format!("{base}시"),
        core_len,
        ContinuationState::Prospective,
        vec![rule("ending.honorific")],
    );
    push_branch(
        branches,
        entry,
        format!("{base}시"),
        core_len,
        ContinuationState::StemEnding,
        vec![rule("ending.honorific")],
    );
    for anchor in [format!("{base}신"), format!("{base}시는")] {
        push_branch(
            branches,
            entry,
            anchor,
            core_len,
            ContinuationState::NieunEnding,
            vec![rule("ending.honorific")],
        );
    }
    for (suffix, ending) in [
        ("시냐", "ending.interrogative-nya"),
        ("시느냐", "ending.interrogative-neunya"),
    ] {
        push_branch(
            branches,
            entry,
            format!("{base}{suffix}"),
            core_len,
            ContinuationState::Interrogative,
            vec![rule("ending.honorific"), rule(ending)],
        );
    }
    for (surface, rules) in [
        (
            format!("{base}시다"),
            vec![rule("ending.honorific"), rule("ending.final-da")],
        ),
        (
            format!("{base}시고"),
            vec![rule("ending.honorific"), rule("ending.connective-go")],
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
    if entry.pos.is_action() {
        for suffix in continuation::HONORIFIC_BIEUP_ENDINGS {
            push_branch(
                branches,
                entry,
                format!("{base}{}", suffix.surface),
                core_len,
                ContinuationState::Terminal,
                suffix.rules.iter().map(|id| rule(id)).collect(),
            );
        }
        push_branch(
            branches,
            entry,
            format!("{base}시는"),
            core_len,
            ContinuationState::Terminal,
            vec![rule("ending.honorific"), rule("ending.present-adnominal")],
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
    push_branch(
        branches,
        entry,
        "일".to_owned(),
        stem.len(),
        ContinuationState::RieulFinal,
        vec![rule("lexical.copula")],
    );
    push_branch(
        branches,
        entry,
        stem.to_owned(),
        stem.len(),
        ContinuationState::Prospective,
        vec![rule("lexical.copula")],
    );
    for (surface, rules) in [
        ("이", vec![rule("lexical.copula")]),
        (
            "이시",
            vec![rule("lexical.copula"), rule("ending.honorific")],
        ),
    ] {
        push_branch(
            branches,
            entry,
            surface.to_owned(),
            stem.len(),
            ContinuationState::StemEnding,
            rules,
        );
    }
    push_branch(
        branches,
        entry,
        "이겠".to_owned(),
        stem.len(),
        ContinuationState::Future,
        vec![rule("lexical.copula"), rule("ending.future")],
    );
    for surface in ["인", "이신", "이시는"] {
        let mut rules = vec![rule("lexical.copula")];
        if surface != "인" {
            rules.push(rule("ending.honorific"));
        }
        push_branch(
            branches,
            entry,
            surface.to_owned(),
            stem.len(),
            ContinuationState::NieunEnding,
            rules,
        );
    }
    for (surface, ending) in [
        ("이냐", "ending.interrogative-nya"),
        ("이시냐", "ending.interrogative-nya"),
        ("이시느냐", "ending.interrogative-neunya"),
    ] {
        let mut rules = vec![rule("lexical.copula")];
        if surface.starts_with("이시") {
            rules.push(rule("ending.honorific"));
        }
        rules.push(rule(ending));
        push_branch(
            branches,
            entry,
            surface.to_owned(),
            stem.len(),
            ContinuationState::Interrogative,
            rules,
        );
    }
    for suffix in continuation::YAJI_ENDINGS {
        let rules = [rule("lexical.copula"), rule("ending.aoeo")]
            .into_iter()
            .chain(suffix.rules.iter().map(|id| rule(id)))
            .collect();
        push_branch(
            branches,
            entry,
            format!("여{}", suffix.surface),
            stem.len(),
            ContinuationState::Terminal,
            rules,
        );
    }
    for suffix in continuation::GEODEUN_ENDINGS {
        let rules = std::iter::once(rule("lexical.copula"))
            .chain(suffix.rules.iter().map(|id| rule(id)))
            .collect();
        push_branch(
            branches,
            entry,
            format!("{stem}{}", suffix.surface),
            stem.len(),
            ContinuationState::Terminal,
            rules,
        );
    }
    for (surface, continuation, ending_rule) in [
        (
            "다".to_owned(),
            ContinuationState::Terminal,
            "ending.final-da",
        ),
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
        (
            format!("{stem}려면"),
            ContinuationState::Terminal,
            "ending.intentive-ryeomyeon",
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
    for (surface, rules) in [
        ("이라", vec![rule("lexical.copula")]),
        (
            "이시라",
            vec![rule("lexical.copula"), rule("ending.honorific")],
        ),
    ] {
        push_branch(
            branches,
            entry,
            surface.to_owned(),
            stem.len(),
            ContinuationState::CopulaRa,
            rules,
        );
    }

    let mut polite = formal_bieup_anchor(stem)?;
    polite.rules.insert(0, rule("lexical.copula"));
    push_derived(branches, entry, polite, ContinuationState::BieupFinal);

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
    push_branch(
        branches,
        entry,
        "였".to_owned(),
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
