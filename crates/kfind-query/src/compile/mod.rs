use std::collections::HashSet;
use std::sync::{Arc, OnceLock};

use crate::lexicons::{data_fine_pos, predicate_from_derivation};
use crate::{
    Analysis, AnalysisSource, AtomPlan, CandidateConsumption, CandidateLeftContext,
    CandidateProgram, CompileError, CompileErrorKind, CompileOptions, CoreMapping, ExpandMode,
    LexiconQueryAnalyzer, Morphology, Origin, QueryAnalyzer, QueryAtom, QueryDiagnostic, QueryPlan,
    parse_query,
};
use kfind_data::{
    DICTIONARY_ADVERBIAL_I_RULE_ID, DICTIONARY_CONJUGATION_RULE_ID,
    DICTIONARY_RELATED_ADVERB_RULE_ID, DICTIONARY_VOICE_DERIVATION_RULE_ID, DerivationRule,
};
use kfind_morph::{
    CoarsePos, ComponentCapability, ParticleAllomorph, PredicatePosSet, RuleId,
    generate_predicate_branches, generate_predicate_fallback_stems,
};

mod normalization;

use normalization::{DraftBranch, DraftDecision, normalize_and_merge, normalize_atom};

const PROGRAM_OVERHEAD_BYTES: usize = 64;
const NIKL_ENDING_CATALOG: &str = include_str!("../../../../data/rules/nikl-modern-endings.tsv");
const CONNECTIVE_JI_RULE_ID: &str = "ending.connective-ji";
const NOMINALIZER_RULE_IDS: &[&str] = &["ending.nominalizer", "ending.nominalizer-gi"];
const INTERNAL_PROVENANCE_IDS: &[&str] = &[
    "contraction.eu-drop",
    "contraction.h-irregular",
    "contraction.identical-vowel",
    DICTIONARY_ADVERBIAL_I_RULE_ID,
    DICTIONARY_CONJUGATION_RULE_ID,
    DICTIONARY_VOICE_DERIVATION_RULE_ID,
    DICTIONARY_RELATED_ADVERB_RULE_ID,
    "structural.ending-path",
];
const PREDICATE_CONSUMPTION_RULE_IDS: &[&str] = &[
    "ending.aoeo-seo",
    "particle.additive",
    "ending.connective-do",
    "ending.connective-ya",
    "ending.polite-yo",
    "ending.imperative-ra",
    "ending.polite-declarative",
    "ending.connective-eudoe",
    "ending.conditional",
    "ending.connective-jiman",
    "ending.connective-neunde",
    "ending.quotative-go",
    "ending.quotative-adnominal",
    "ending.quotative-retrospective",
    "ending.quotative-ni",
    "ending.quotative-myeo",
    "ending.quotative-myeonseo",
    "ending.quotative-neunde",
    "ending.quotative-ji",
    "ending.final-da",
    "ending.connective-go",
    "ending.honorific",
    "ending.polite-imperative",
    "ending.past",
    "ending.past-adnominal",
    "ending.future-adnominal",
    "ending.coordinate-myeo",
];
const NOMINAL_CONSUMPTION_RULE_IDS: &[&str] = &[
    "particle.plural",
    "particle.source.egeseo",
    "particle.source.hanteseo",
    "particle.direction",
    "particle.capacity.roseo",
    "particle.instrument.rosseo",
    "particle.dative",
    "particle.source",
    "particle.locative",
    "particle.genitive",
    "particle.subject",
    "particle.object",
    "particle.comitative",
    "particle.connector-myeon",
    "particle.topic",
    "particle.additive",
    "particle.only",
    "particle.limit.ggaji",
    "particle.from",
    "particle.even.jocha",
    "particle.even.majeo",
];
const NON_REPLACING_NOMINAL_OVERRIDE_RULE_IDS: &[&str] = &["particle.genitive"];

pub fn compile_query(
    source: &str,
    options: &CompileOptions,
    analyzer: &LexiconQueryAnalyzer,
) -> Result<QueryPlan, CompileError> {
    let ast = parse_query(source, options)
        .map_err(|error| CompileError::new(None, CompileErrorKind::Query(error)))?;
    let rule_depth = usize::from(analyzer.lexicons().rules().max_continuation_depth);
    if rule_depth > options.limits.max_continuation_depth {
        return Err(CompileError::new(
            None,
            CompileErrorKind::ContinuationDepthExceeded {
                actual: rule_depth,
                limit: options.limits.max_continuation_depth,
            },
        ));
    }

    let particle_rules = analyzer.particle_rules();
    let known_rule_ids = particle_rules.known.as_ref();
    let allowed_predicate_rules = Arc::clone(&particle_rules.predicate);
    let allowed_particle_rules = Arc::clone(&particle_rules.particle);
    let particle_allomorphs = Arc::clone(&particle_rules.allomorphs);
    let particle_transitions = Arc::clone(&particle_rules.transitions);
    let allowed_auxiliary_particle_rules = Arc::clone(&particle_rules.auxiliary);
    let allowed_adverb_initial_particle_rules = Arc::clone(&particle_rules.adverb_initial);
    let allowed_predicate_ending_initial_particle_rules =
        Arc::clone(&particle_rules.predicate_ending_initial);
    let mut diagnostics = Vec::new();
    if options.requires_full_pos_lexicon() && !analyzer.lexicons().full_pos_loaded() {
        diagnostics.push(QueryDiagnostic::FullPosLexiconUnavailable);
    }
    let mut excluded_rules = Vec::new();

    let mut atom_plans = Vec::with_capacity(ast.atoms.len());
    let mut total_programs = 0;
    let mut estimated_matcher_bytes = shared_rule_bytes(&[
        &allowed_predicate_rules,
        &allowed_particle_rules,
        &allowed_auxiliary_particle_rules,
        &allowed_adverb_initial_particle_rules,
        &allowed_predicate_ending_initial_particle_rules,
    ]) + particle_allomorphs
        .iter()
        .map(|form| std::mem::size_of::<ParticleAllomorph>() + form.surface.len())
        .sum::<usize>();
    let mut uses_predicate_consumption = false;
    let mut uses_nominal_consumption = false;
    for (atom_index, atom) in ast.atoms.iter().enumerate() {
        let one_scalar_atom = atom.raw.chars().count() == 1;
        let normalized = normalize_atom(atom, options.normalization);
        let mut effective = normalized.clone();
        effective.forced_pos = effective.forced_pos.or(options.global_pos);
        let analyses = analyzer.analyze(&effective).map_err(|error| {
            CompileError::new(Some(atom_index), CompileErrorKind::Analyze(error))
        })?;
        let analysis_limit = options
            .limits
            .max_analyses_per_atom
            .min(usize::from(u16::MAX));
        if analyses.len() > analysis_limit {
            return Err(CompileError::new(
                Some(atom_index),
                CompileErrorKind::TooManyAnalyses {
                    actual: analyses.len(),
                    limit: analysis_limit,
                },
            ));
        }
        if is_unregistered_da_literal(&effective, &analyses) {
            diagnostics.push(QueryDiagnostic::UnregisteredDaLiteralOnly {
                atom_index,
                lemma: effective.raw.clone(),
            });
        }

        let mut drafts = Vec::<DraftBranch>::new();
        let mut analysis_draft_ranges = Vec::<(usize, usize)>::with_capacity(analyses.len());
        for (analysis_index, analysis) in analyses.iter().enumerate() {
            let draft_range = if let Some((prior_index, source_pos)) =
                reusable_predicate_analysis(&analyses, analysis_index)
            {
                let (prior_start, prior_end) = analysis_draft_ranges[prior_index];
                for draft in &mut drafts[prior_start..prior_end] {
                    draft.consumption.add_source_position(source_pos);
                    let mut origins = draft
                        .origins
                        .iter()
                        .filter(|origin| origin.analysis_index == prior_index as u16)
                        .cloned()
                        .collect::<Vec<_>>();
                    for origin in &mut origins {
                        origin.analysis_index = analysis_index as u16;
                    }
                    draft.origins.extend(origins);
                }
                (prior_start, prior_end)
            } else {
                let start = drafts.len();
                compile_analysis(
                    &effective.raw,
                    analysis_index as u16,
                    analysis,
                    options,
                    analyzer,
                    &allowed_predicate_rules,
                    &allowed_particle_rules,
                    &allowed_auxiliary_particle_rules,
                    &allowed_adverb_initial_particle_rules,
                    known_rule_ids,
                    &mut excluded_rules,
                    &mut drafts,
                )?;
                (start, drafts.len())
            };
            analysis_draft_ranges.push(draft_range);
        }
        let programs = normalize_and_merge(
            drafts,
            &analyses,
            options.normalization,
            options.boundary,
            one_scalar_atom,
            atom_index,
        )?;
        if programs.is_empty() {
            return Err(CompileError::new(
                Some(atom_index),
                CompileErrorKind::NoSearchablePrograms,
            ));
        }
        uses_predicate_consumption |= programs.iter().any(|program| {
            matches!(
                &program.consumption,
                CandidateConsumption::PredicateContinuation { .. }
            )
        });
        uses_nominal_consumption |= programs.iter().any(|program| {
            matches!(
                &program.consumption,
                CandidateConsumption::NominalParticleChain { .. }
                    | CandidateConsumption::NominalCopulaEndingChain { .. }
            )
        });
        total_programs += programs.len();
        if total_programs > options.limits.max_programs {
            return Err(CompileError::new(
                Some(atom_index),
                CompileErrorKind::TooManyPrograms {
                    actual: total_programs,
                    limit: options.limits.max_programs,
                },
            ));
        }
        estimated_matcher_bytes += programs.iter().map(estimate_program_bytes).sum::<usize>();
        if estimated_matcher_bytes > options.limits.max_matcher_bytes {
            return Err(CompileError::new(
                Some(atom_index),
                CompileErrorKind::MatcherMemoryExceeded {
                    estimated: estimated_matcher_bytes,
                    limit: options.limits.max_matcher_bytes,
                },
            ));
        }
        atom_plans.push(AtomPlan {
            analyses,
            programs,
            boundary: options.boundary,
        });
    }

    if uses_predicate_consumption {
        excluded_rules.extend(missing_rules(
            PREDICATE_CONSUMPTION_RULE_IDS,
            known_rule_ids,
        ));
    }
    if uses_nominal_consumption {
        excluded_rules.extend(missing_rules(NOMINAL_CONSUMPTION_RULE_IDS, known_rule_ids));
    }
    excluded_rules.sort();
    excluded_rules.dedup();
    if !excluded_rules.is_empty() {
        diagnostics.push(QueryDiagnostic::RuleVocabularyRestricted {
            excluded_rule_ids: excluded_rules.into_boxed_slice(),
        });
    }
    Ok(QueryPlan {
        raw_query: source.into(),
        atoms: atom_plans,
        phrase_policy: ast.phrase,
        normalization: options.normalization,
        limits: options.limits,
        diagnostics,
        particle_allomorphs,
        particle_transitions,
        auxiliary_particle_rules: allowed_auxiliary_particle_rules,
        predicate_ending_initial_particle_rules: allowed_predicate_ending_initial_particle_rules,
        estimated_matcher_bytes,
    })
}

fn reusable_predicate_analysis(
    analyses: &[Analysis],
    current_index: usize,
) -> Option<(usize, kfind_morph::PredicatePos)> {
    let current = analyses.get(current_index)?;
    let Morphology::Predicate(current_predicate) = &current.morphology else {
        return None;
    };
    analyses[..current_index]
        .iter()
        .enumerate()
        .find_map(|(index, previous)| {
            let Morphology::Predicate(previous_predicate) = &previous.morphology else {
                return None;
            };
            (current.lemma == previous.lemma
                && current.coarse_pos == previous.coarse_pos
                && current.source == previous.source
                && current_predicate.pos.execution() == previous_predicate.pos.execution()
                && current_predicate.alternation == previous_predicate.alternation
                && current_predicate.flags == previous_predicate.flags
                && current_predicate.overrides == previous_predicate.overrides
                && current_predicate.derivations == previous_predicate.derivations)
                .then_some((index, current_predicate.pos))
        })
}

#[allow(clippy::too_many_arguments)]
fn compile_analysis(
    atom_surface: &str,
    analysis_index: u16,
    analysis: &Analysis,
    options: &CompileOptions,
    analyzer: &LexiconQueryAnalyzer,
    predicate_rules: &Arc<[RuleId]>,
    particle_rules: &Arc<[RuleId]>,
    auxiliary_particle_rules: &Arc<[RuleId]>,
    adverb_initial_particle_rules: &Arc<[RuleId]>,
    known_rule_ids: &HashSet<Box<str>>,
    excluded_rules: &mut Vec<RuleId>,
    output: &mut Vec<DraftBranch>,
) -> Result<(), CompileError> {
    if options.expand == ExpandMode::Literal {
        output.push(exact_branch(atom_surface, analysis_index, Vec::new(), true));
        return Ok(());
    }

    if matches!(analysis.morphology, Morphology::Exact) {
        let decision = exact_candidate_decision(analysis);
        if analysis.coarse_pos == CoarsePos::Adverb {
            output.push(DraftBranch {
                anchor: atom_surface.to_owned(),
                consumption: CandidateConsumption::NominalParticleChain {
                    initial_allowed_rule_ids: Arc::clone(adverb_initial_particle_rules),
                    allowed_rule_ids: Arc::clone(auxiliary_particle_rules),
                    blocked_rule_ids: Arc::from([]),
                },
                core_mapping: CoreMapping::WholeAnchor,
                origins: vec![Origin {
                    analysis_index,
                    rule_path: Vec::new(),
                }],
                smart_left: true,
                decision,
            });
        } else {
            output.push(exact_branch_with_decision(
                atom_surface,
                analysis_index,
                Vec::new(),
                true,
                decision,
            ));
        }
        return Ok(());
    }

    match &analysis.morphology {
        Morphology::Predicate(_) => compile_predicate(
            analysis,
            analysis_index,
            Vec::new(),
            options.expand,
            analyzer.lexicons().full_pos_loaded(),
            options.boundary == crate::BoundaryPolicy::Smart
                && analyzer.lexicons().full_pos_loaded(),
            predicate_rules,
            known_rule_ids,
            excluded_rules,
            output,
        )?,
        Morphology::Nominal(nominal) => {
            let blocked_rule_ids = blocked_override_rules(nominal);
            output.push(DraftBranch {
                anchor: analysis.lemma.to_string(),
                consumption: CandidateConsumption::NominalParticleChain {
                    initial_allowed_rule_ids: Arc::clone(particle_rules),
                    allowed_rule_ids: Arc::clone(particle_rules),
                    blocked_rule_ids,
                },
                core_mapping: CoreMapping::WholeAnchor,
                origins: vec![Origin {
                    analysis_index,
                    rule_path: Vec::new(),
                }],
                smart_left: true,
                decision: DraftDecision::Structural(ComponentCapability::SourceAndRuntime),
            });
            for override_form in &nominal.overrides {
                output.push(exact_branch(
                    &override_form.surface,
                    analysis_index,
                    vec![override_form.rule_id.clone()],
                    true,
                ));
            }
            compile_nominal_contractions(
                analysis,
                analysis_index,
                analyzer,
                particle_rules,
                output,
            );
            if options.expand == ExpandMode::Derivation {
                compile_derivations(
                    analysis,
                    analysis_index,
                    analyzer,
                    predicate_rules,
                    particle_rules,
                    known_rule_ids,
                    excluded_rules,
                    output,
                )?;
            }
        }
        Morphology::Particle(particle) => {
            let expand_allomorphs = options.boundary != crate::BoundaryPolicy::Smart
                || analysis.source == AnalysisSource::Forced;
            for variant in particle
                .variants
                .iter()
                .filter(|variant| expand_allomorphs || variant.as_ref() == atom_surface)
            {
                if let Some(rule_id) = &particle.rule_id {
                    output.push(DraftBranch {
                        anchor: variant.to_string(),
                        consumption: CandidateConsumption::DirectParticleHost {
                            rule_id: rule_id.clone(),
                        },
                        core_mapping: CoreMapping::WholeAnchor,
                        origins: vec![Origin {
                            analysis_index,
                            rule_path: vec![rule_id.clone()],
                        }],
                        smart_left: false,
                        decision: DraftDecision::Boundary,
                    });
                } else {
                    output.push(exact_branch(variant, analysis_index, Vec::new(), true));
                }
            }
        }
        Morphology::Exact => unreachable!("exact morphology returned above"),
    }
    Ok(())
}

fn compile_nominal_contractions(
    analysis: &Analysis,
    analysis_index: u16,
    analyzer: &LexiconQueryAnalyzer,
    particle_rules: &Arc<[RuleId]>,
    output: &mut Vec<DraftBranch>,
) {
    if analysis.coarse_pos != CoarsePos::Pronoun {
        return;
    }
    for rule in &analyzer.lexicons().rules().contractions {
        match rule.kind.as_str() {
            "nominal-particle-compose" => {
                let Some(prefix) = analysis.lemma.strip_suffix(&rule.left) else {
                    continue;
                };
                output.push(exact_branch(
                    &format!("{prefix}{}", rule.result),
                    analysis_index,
                    vec![RuleId::from(rule.id.clone())],
                    true,
                ));
            }
            "nominal-copula-ending-compose" if analysis.lemma.as_ref() == rule.left => {
                output.push(DraftBranch {
                    anchor: rule.result.clone(),
                    consumption: CandidateConsumption::NominalCopulaEndingChain {
                        initial_allowed_rule_ids: Arc::clone(particle_rules),
                        allowed_rule_ids: Arc::clone(particle_rules),
                        blocked_rule_ids: Arc::from([]),
                    },
                    core_mapping: CoreMapping::WholeAnchor,
                    origins: vec![Origin {
                        analysis_index,
                        rule_path: vec![RuleId::from(rule.id.clone())],
                    }],
                    smart_left: true,
                    decision: DraftDecision::Structural(ComponentCapability::Source),
                });
            }
            _ => {}
        }
    }
}

fn exact_candidate_decision(analysis: &Analysis) -> DraftDecision {
    match analysis.coarse_pos {
        CoarsePos::Determiner => DraftDecision::Structural(ComponentCapability::SourceAndRuntime),
        CoarsePos::Adverb => DraftDecision::Structural(ComponentCapability::SourceAndRuntime),
        _ => DraftDecision::Boundary,
    }
}

#[allow(clippy::too_many_arguments)]
fn compile_predicate(
    analysis: &Analysis,
    analysis_index: u16,
    prefix_rules: Vec<RuleId>,
    expand: ExpandMode,
    exact_component: bool,
    structural_fallback: bool,
    allowed_rules: &Arc<[RuleId]>,
    known_rule_ids: &HashSet<Box<str>>,
    excluded_rules: &mut Vec<RuleId>,
    output: &mut Vec<DraftBranch>,
) -> Result<(), CompileError> {
    let Morphology::Predicate(predicate) = &analysis.morphology else {
        unreachable!("predicate compile received non-predicate analysis")
    };
    for derivation in &predicate.derivations {
        let derived_predicate = kfind_morph::PredicateEntry::new(
            derivation.target_lemma.clone(),
            predicate.pos,
            kfind_morph::LexicalAlternation::Regular,
        );
        let derived_analysis = Analysis {
            lemma: derivation.target_lemma.clone(),
            coarse_pos: analysis.coarse_pos,
            fine_pos: analysis.fine_pos,
            morphology: Morphology::Predicate(derived_predicate),
            source: AnalysisSource::EnrichedLexicon,
        };
        let mut derivation_path = prefix_rules.clone();
        derivation_path.push(derivation.rule_id.clone());
        compile_predicate(
            &derived_analysis,
            analysis_index,
            derivation_path,
            expand,
            exact_component,
            structural_fallback,
            allowed_rules,
            known_rule_ids,
            excluded_rules,
            output,
        )?;
    }
    let branches = generate_predicate_branches(predicate)
        .map_err(|error| CompileError::new(None, CompileErrorKind::Generate(error)))?;
    for branch in &branches {
        let environment = predicate_environment(predicate, branch);
        let mut rule_path = prefix_rules.clone();
        rule_path.extend(branch.rule_path.iter().cloned());
        if expand != ExpandMode::Derivation
            && rule_path
                .iter()
                .any(|rule| rule.as_str() == DICTIONARY_RELATED_ADVERB_RULE_ID)
        {
            continue;
        }
        let unsupported = rule_path
            .iter()
            .filter(|rule| !is_known_or_internal(rule, known_rule_ids))
            .cloned()
            .collect::<Vec<_>>();
        if !unsupported.is_empty() {
            excluded_rules.extend(unsupported);
            continue;
        }
        let nominal_particle_transition = rule_path
            .last()
            .is_some_and(|rule| NOMINALIZER_RULE_IDS.contains(&rule.as_str()));
        let dictionary_adverbial = rule_path
            .iter()
            .any(|rule| rule.as_str() == DICTIONARY_ADVERBIAL_I_RULE_ID);
        let smart_left = predicate.alternation != kfind_morph::LexicalAlternation::Copula
            && !(analysis.source == AnalysisSource::Forced
                && rule_path
                    .last()
                    .is_some_and(|rule| rule.as_str() == CONNECTIVE_JI_RULE_ID));
        let structural_future_adnominal = structural_fallback
            && rule_path
                .iter()
                .any(|rule| rule.as_str() == "ending.future-adnominal");
        output.push(DraftBranch {
            anchor: branch.anchor.to_string(),
            consumption: if dictionary_adverbial {
                CandidateConsumption::Anchor
            } else {
                CandidateConsumption::PredicateContinuation {
                    continuation: branch.continuation,
                    pos: predicate.pos.execution(),
                    source_positions: PredicatePosSet::one(predicate.pos),
                    allowed_rule_ids: Arc::clone(allowed_rules),
                    nominal_particle_transition,
                    left_context: environment,
                }
            },
            core_mapping: CoreMapping::PrefixBytes(branch.core_len),
            origins: vec![Origin {
                analysis_index,
                rule_path,
            }],
            smart_left,
            decision: if dictionary_adverbial
                || predicate.alternation == kfind_morph::LexicalAlternation::Copula
                || exact_component
                || structural_future_adnominal
            {
                DraftDecision::Structural(ComponentCapability::SourceAndRuntime)
            } else {
                DraftDecision::Boundary
            },
        });
    }
    if structural_fallback {
        let mut stems = generate_predicate_fallback_stems(predicate)
            .map_err(|error| CompileError::new(None, CompileErrorKind::Generate(error)))?
            .into_iter()
            .map(|(stem, class)| (stem, class, kfind_morph::ContinuationState::Terminal, true))
            .collect::<Vec<_>>();
        stems.extend(
            branches
                .iter()
                .filter(|branch| {
                    branch.continuation != kfind_morph::ContinuationState::Terminal
                        || branch.rule_path.last().is_some_and(|rule| {
                            matches!(
                                rule.as_str(),
                                "ending.future-adnominal" | "ending.connective-go"
                            )
                        })
                })
                .filter_map(|branch| {
                    structural_stem_class(&branch.anchor)
                        .map(|class| (branch.clone(), class, branch.continuation, false))
                }),
        );
        stems.sort_by(|left, right| left.0.anchor.cmp(&right.0.anchor));
        stems.dedup_by(|left, right| {
            left.0.anchor == right.0.anchor && left.0.rule_path == right.0.rule_path
        });
        for (stem, stem_class, base_state, validate_anchor) in stems {
            let mut rule_path = prefix_rules.clone();
            rule_path.extend(stem.rule_path);
            let unsupported = rule_path
                .iter()
                .filter(|rule| !is_known_or_internal(rule, known_rule_ids))
                .cloned()
                .collect::<Vec<_>>();
            if !unsupported.is_empty() {
                excluded_rules.extend(unsupported);
                continue;
            }
            output.push(DraftBranch {
                anchor: stem.anchor.into(),
                consumption: CandidateConsumption::StructuralPredicateEnding {
                    pos: predicate.pos.execution(),
                    source_positions: PredicatePosSet::one(predicate.pos),
                    flags: predicate.flags,
                    base_state,
                    validate_anchor,
                    stem_class,
                    allowed_suffixes: modern_ending_surfaces(),
                },
                core_mapping: CoreMapping::PrefixBytes(stem.core_len),
                origins: vec![Origin {
                    analysis_index,
                    rule_path,
                }],
                smart_left: true,
                decision: DraftDecision::Structural(ComponentCapability::SourceAndRuntime),
            });
        }
    }
    Ok(())
}

fn structural_stem_class(surface: &str) -> Option<kfind_morph::PredicateStemClass> {
    let final_syllable = surface
        .chars()
        .next_back()
        .and_then(kfind_morph::decompose_syllable)?;
    Some(match final_syllable.jongseong {
        kfind_morph::hangul::JONG_NONE => kfind_morph::PredicateStemClass::Vowel,
        kfind_morph::hangul::JONG_RIEUL => kfind_morph::PredicateStemClass::Rieul,
        _ => kfind_morph::PredicateStemClass::Consonant,
    })
}

#[allow(clippy::too_many_arguments)]
fn compile_derivations(
    analysis: &Analysis,
    analysis_index: u16,
    analyzer: &LexiconQueryAnalyzer,
    predicate_rules: &Arc<[RuleId]>,
    particle_rules: &Arc<[RuleId]>,
    known_rule_ids: &HashSet<Box<str>>,
    excluded_rules: &mut Vec<RuleId>,
    output: &mut Vec<DraftBranch>,
) -> Result<(), CompileError> {
    for rule in &analyzer.lexicons().rules().derivations {
        if !derivation_accepts(rule, analysis) {
            continue;
        }
        let derived_lemma = format!("{}{}", analysis.lemma, rule.suffix);
        let derivation_path = vec![RuleId::from(rule.id.clone())];
        if let Some(derived) =
            predicate_from_derivation(&derived_lemma, rule, AnalysisSource::ProductiveSuffix)
        {
            compile_predicate(
                &derived,
                analysis_index,
                derivation_path,
                ExpandMode::Derivation,
                analyzer.lexicons().full_pos_loaded(),
                false,
                predicate_rules,
                known_rule_ids,
                excluded_rules,
                output,
            )?;
        } else if rule.result_pos.is_nominal() {
            output.push(DraftBranch {
                anchor: derived_lemma,
                consumption: CandidateConsumption::NominalParticleChain {
                    initial_allowed_rule_ids: Arc::clone(particle_rules),
                    allowed_rule_ids: Arc::clone(particle_rules),
                    blocked_rule_ids: Arc::from([]),
                },
                core_mapping: CoreMapping::WholeAnchor,
                origins: vec![Origin {
                    analysis_index,
                    rule_path: derivation_path,
                }],
                smart_left: true,
                decision: DraftDecision::Structural(ComponentCapability::SourceAndRuntime),
            });
        } else {
            output.push(exact_branch(
                &derived_lemma,
                analysis_index,
                derivation_path,
                true,
            ));
        }
    }
    Ok(())
}

fn blocked_override_rules(nominal: &crate::NominalMorphology) -> Arc<[RuleId]> {
    let mut rules = nominal
        .overrides
        .iter()
        .filter(|override_form| {
            !NON_REPLACING_NOMINAL_OVERRIDE_RULE_IDS.contains(&override_form.rule_id.as_str())
        })
        .map(|override_form| override_form.rule_id.clone())
        .collect::<Vec<_>>();
    rules.sort();
    rules.dedup();
    rules.into()
}

fn predicate_environment(
    predicate: &kfind_morph::PredicateEntry,
    branch: &kfind_morph::SurfaceBranchSpec,
) -> CandidateLeftContext {
    let is_contracted_copula = predicate.alternation == kfind_morph::LexicalAlternation::Copula
        && matches!(branch.anchor.as_ref(), "다" | "였" | "여서");
    if !is_contracted_copula {
        return CandidateLeftContext::Any;
    }

    let Some(stem) = predicate.lemma.strip_suffix('다') else {
        return CandidateLeftContext::Any;
    };
    if branch.anchor.starts_with(stem) {
        CandidateLeftContext::Any
    } else {
        CandidateLeftContext::ContractedAfterVowel {
            uncontracted_prefix: stem.into(),
        }
    }
}

fn derivation_accepts(rule: &DerivationRule, analysis: &Analysis) -> bool {
    rule.source_pos
        .iter()
        .any(|pos| data_fine_pos(*pos) == analysis.fine_pos)
}

fn exact_branch(
    surface: &str,
    analysis_index: u16,
    rule_path: Vec<RuleId>,
    smart_left: bool,
) -> DraftBranch {
    exact_branch_with_decision(
        surface,
        analysis_index,
        rule_path,
        smart_left,
        DraftDecision::Boundary,
    )
}

fn exact_branch_with_decision(
    surface: &str,
    analysis_index: u16,
    rule_path: Vec<RuleId>,
    smart_left: bool,
    decision: DraftDecision,
) -> DraftBranch {
    DraftBranch {
        anchor: surface.to_owned(),
        consumption: CandidateConsumption::Anchor,
        core_mapping: CoreMapping::WholeAnchor,
        origins: vec![Origin {
            analysis_index,
            rule_path,
        }],
        smart_left,
        decision,
    }
}

fn modern_ending_surfaces() -> Arc<[Box<str>]> {
    static SURFACES: OnceLock<Arc<[Box<str>]>> = OnceLock::new();
    Arc::clone(SURFACES.get_or_init(|| {
        let mut surfaces = NIKL_ENDING_CATALOG
            .lines()
            .skip(1)
            .filter_map(|line| line.split_once('\t').map(|(surface, _)| surface))
            .filter(|surface| !surface.is_empty())
            .map(Box::<str>::from)
            .collect::<Vec<_>>();
        surfaces.sort();
        surfaces.dedup();
        surfaces.into()
    }))
}

fn missing_rules(ids: &[&str], known: &HashSet<Box<str>>) -> impl Iterator<Item = RuleId> {
    ids.iter()
        .copied()
        .filter(|id| !known.contains(*id))
        .map(RuleId::from)
}

fn is_known_or_internal(rule: &RuleId, known: &HashSet<Box<str>>) -> bool {
    known.contains(rule.as_str()) || INTERNAL_PROVENANCE_IDS.contains(&rule.as_str())
}

fn is_unregistered_da_literal(atom: &QueryAtom, analyses: &[Analysis]) -> bool {
    atom.forced_pos.is_none()
        && atom.raw.ends_with('다')
        && analyses.len() == 1
        && analyses[0].source == AnalysisSource::Heuristic
        && analyses[0].coarse_pos == kfind_morph::CoarsePos::Literal
}

fn shared_rule_bytes(rule_sets: &[&[RuleId]]) -> usize {
    rule_sets
        .iter()
        .flat_map(|rules| rules.iter())
        .map(|rule| rule.as_str().len())
        .sum()
}

fn estimate_program_bytes(program: &CandidateProgram) -> usize {
    PROGRAM_OVERHEAD_BYTES
        + program.anchor.len()
        + program
            .origins
            .iter()
            .map(|origin| {
                std::mem::size_of::<Origin>()
                    + origin
                        .rule_path
                        .iter()
                        .map(|rule| rule.as_str().len())
                        .sum::<usize>()
            })
            .sum::<usize>()
        + program
            .structural_patterns()
            .iter()
            .map(|pattern| {
                std::mem::size_of::<kfind_morph::QueryMorphPattern>()
                    + pattern.lexical_form.len()
                    + std::mem::size_of_val(pattern.adjacent.as_ref())
            })
            .sum::<usize>()
}

#[cfg(test)]
mod tests;
