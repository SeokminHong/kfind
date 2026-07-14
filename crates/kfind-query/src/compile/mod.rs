use std::collections::HashSet;
use std::sync::Arc;

use crate::lexicons::{data_fine_pos, predicate_from_derivation};
use crate::{
    Analysis, AnalysisSource, AtomPlan, BranchEnvironment, BranchVerifier, CompileError,
    CompileErrorKind, CompileOptions, ContextRequirement, CoreMapping, ExpandMode,
    LexiconQueryAnalyzer, Morphology, Origin, QueryAnalyzer, QueryAtom, QueryDiagnostic, QueryPlan,
    SurfaceBranch, parse_query,
};
use kfind_data::DerivationRule;
use kfind_morph::{CoarsePos, ParticleTransition, RuleId, generate_predicate_branches};

mod context;
mod normalization;

use context::lexical_context_rule;
use normalization::{DraftBranch, normalize_and_merge, normalize_atom};

const BRANCH_OVERHEAD_BYTES: usize = 64;
const COPULA_CONTRACTED_AOEO_RULE_ID: &str = "ending.aoeo-seo";
const CONNECTIVE_JI_RULE_ID: &str = "ending.connective-ji";
const NOMINALIZER_RULE_IDS: &[&str] = &["ending.nominalizer", "ending.nominalizer-gi"];
const INTERNAL_PROVENANCE_IDS: &[&str] = &[
    "contraction.eu-drop",
    "contraction.h-irregular",
    "contraction.identical-vowel",
];
const MORPH_VERIFIER_RULE_IDS: &[&str] = &[
    "ending.aoeo-seo",
    "particle.additive",
    "ending.connective-do",
    "ending.connective-ya",
    "ending.polite-yo",
    "ending.imperative-ra",
    "ending.polite-declarative",
    "ending.conditional",
    "ending.connective-jiman",
    "ending.connective-neunde",
    "ending.final-da",
    "ending.connective-go",
    "ending.honorific",
    "ending.past",
    "ending.past-adnominal",
    "ending.future-adnominal",
    "ending.coordinate-myeo",
];
const MORPH_PARTICLE_RULE_IDS: &[&str] = &[
    "particle.plural",
    "particle.source.egeseo",
    "particle.source.hanteseo",
    "particle.direction",
    "particle.dative",
    "particle.source",
    "particle.locative",
    "particle.genitive",
    "particle.subject",
    "particle.object",
    "particle.comitative",
    "particle.topic",
    "particle.additive",
    "particle.only",
    "particle.limit.ggaji",
    "particle.from",
    "particle.even.jocha",
    "particle.even.majeo",
];
const ADVERB_PARTICLE_RULE_IDS: &[&str] = &[
    "particle.topic",
    "particle.additive",
    "particle.only",
    "particle.limit.ggaji",
    "particle.from",
    "particle.even.jocha",
    "particle.even.majeo",
];

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

    let known_rule_ids = analyzer
        .lexicons()
        .rules()
        .all_ids()
        .collect::<HashSet<_>>();
    let allowed_predicate_rules = allowed_rules(&known_rule_ids, |_| true);
    let allowed_particle_rules = allowed_rules(&known_rule_ids, |id| id.starts_with("particle."));
    let allowed_adverb_particle_rules =
        allowed_rules(&known_rule_ids, |id| ADVERB_PARTICLE_RULE_IDS.contains(&id));
    let particle_transitions = analyzer
        .lexicons()
        .rules()
        .particles
        .iter()
        .map(|rule| {
            ParticleTransition::new(
                rule.id.clone(),
                rule.next
                    .iter()
                    .cloned()
                    .map(RuleId::from)
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            )
        })
        .collect::<Vec<_>>();
    let mut diagnostics = Vec::new();
    if options.requires_full_pos_lexicon() && !analyzer.lexicons().full_pos_loaded() {
        diagnostics.push(QueryDiagnostic::FullPosLexiconUnavailable);
    }
    let mut excluded_rules = Vec::new();

    let mut atom_plans = Vec::with_capacity(ast.atoms.len());
    let mut total_branches = 0;
    let mut estimated_matcher_bytes = shared_rule_bytes(&[
        &allowed_predicate_rules,
        &allowed_particle_rules,
        &allowed_adverb_particle_rules,
    ]);
    let mut uses_predicate_verifier = false;
    let mut uses_nominal_verifier = false;
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

        let mut drafts = Vec::new();
        for (analysis_index, analysis) in analyses.iter().enumerate() {
            compile_analysis(
                &effective.raw,
                analysis_index as u16,
                analysis,
                options,
                analyzer,
                &allowed_predicate_rules,
                &allowed_particle_rules,
                &allowed_adverb_particle_rules,
                &known_rule_ids,
                &mut excluded_rules,
                &mut drafts,
            )?;
        }
        let branches = normalize_and_merge(
            drafts,
            options.normalization,
            options.boundary,
            one_scalar_atom,
            atom_index,
        )?;
        if branches.is_empty() {
            return Err(CompileError::new(
                Some(atom_index),
                CompileErrorKind::NoSearchableBranches,
            ));
        }
        uses_predicate_verifier |= branches
            .iter()
            .any(|branch| matches!(&branch.verifier, BranchVerifier::Predicate { .. }));
        uses_nominal_verifier |= branches
            .iter()
            .any(|branch| matches!(&branch.verifier, BranchVerifier::NominalParticles { .. }));
        total_branches += branches.len();
        if total_branches > options.limits.max_branches {
            return Err(CompileError::new(
                Some(atom_index),
                CompileErrorKind::TooManyBranches {
                    actual: total_branches,
                    limit: options.limits.max_branches,
                },
            ));
        }
        estimated_matcher_bytes += branches.iter().map(estimate_branch_bytes).sum::<usize>();
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
            branches,
            boundary: options.boundary,
        });
    }

    if uses_predicate_verifier {
        excluded_rules.extend(missing_rules(MORPH_VERIFIER_RULE_IDS, &known_rule_ids));
    }
    if uses_nominal_verifier {
        excluded_rules.extend(missing_rules(MORPH_PARTICLE_RULE_IDS, &known_rule_ids));
    }
    excluded_rules.sort();
    excluded_rules.dedup();
    if !excluded_rules.is_empty() {
        diagnostics.push(QueryDiagnostic::VerifierVocabularyRestricted {
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
        particle_transitions: particle_transitions.into(),
        estimated_matcher_bytes,
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
    adverb_particle_rules: &Arc<[RuleId]>,
    known_rule_ids: &HashSet<&str>,
    excluded_rules: &mut Vec<RuleId>,
    output: &mut Vec<DraftBranch>,
) -> Result<(), CompileError> {
    if options.expand == ExpandMode::Literal {
        output.push(exact_branch(atom_surface, analysis_index, Vec::new(), true));
        return Ok(());
    }

    if matches!(analysis.morphology, Morphology::Exact) {
        let context_requirement = lexical_context_requirement(atom_surface, analysis);
        if options.expand == ExpandMode::Derivation && analysis.coarse_pos == CoarsePos::Adverb {
            output.push(DraftBranch {
                anchor: atom_surface.to_owned(),
                verifier: BranchVerifier::NominalParticles {
                    allowed_rule_ids: Arc::clone(adverb_particle_rules),
                    blocked_rule_ids: Arc::from([]),
                },
                core_mapping: CoreMapping::WholeAnchor,
                origin: Origin {
                    analysis_index,
                    rule_path: Vec::new(),
                },
                smart_left: true,
                context_requirement,
            });
        } else {
            output.push(exact_branch_with_context(
                atom_surface,
                analysis_index,
                Vec::new(),
                true,
                context_requirement,
            ));
        }
        return Ok(());
    }

    match &analysis.morphology {
        Morphology::Predicate(_) => compile_predicate(
            analysis,
            analysis_index,
            Vec::new(),
            predicate_rules,
            known_rule_ids,
            excluded_rules,
            output,
        )?,
        Morphology::Nominal(nominal) => {
            let blocked_rule_ids = blocked_override_rules(nominal);
            output.push(DraftBranch {
                anchor: analysis.lemma.to_string(),
                verifier: BranchVerifier::NominalParticles {
                    allowed_rule_ids: Arc::clone(particle_rules),
                    blocked_rule_ids,
                },
                core_mapping: CoreMapping::WholeAnchor,
                origin: Origin {
                    analysis_index,
                    rule_path: Vec::new(),
                },
                smart_left: true,
                context_requirement: if analysis.coarse_pos == CoarsePos::Noun {
                    ContextRequirement::NominalComponent
                } else {
                    ContextRequirement::None
                },
            });
            for override_form in &nominal.overrides {
                output.push(exact_branch(
                    &override_form.surface,
                    analysis_index,
                    vec![override_form.rule_id.clone()],
                    true,
                ));
            }
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
                        verifier: BranchVerifier::DirectParticle {
                            rule_id: rule_id.clone(),
                        },
                        core_mapping: CoreMapping::WholeAnchor,
                        origin: Origin {
                            analysis_index,
                            rule_path: vec![rule_id.clone()],
                        },
                        smart_left: false,
                        context_requirement: ContextRequirement::None,
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

fn lexical_context_requirement(atom_surface: &str, analysis: &Analysis) -> ContextRequirement {
    if lexical_context_rule(atom_surface, analysis.fine_pos).is_some() {
        ContextRequirement::LexicalContext
    } else {
        ContextRequirement::None
    }
}

fn compile_predicate(
    analysis: &Analysis,
    analysis_index: u16,
    prefix_rules: Vec<RuleId>,
    allowed_rules: &Arc<[RuleId]>,
    known_rule_ids: &HashSet<&str>,
    excluded_rules: &mut Vec<RuleId>,
    output: &mut Vec<DraftBranch>,
) -> Result<(), CompileError> {
    let Morphology::Predicate(predicate) = &analysis.morphology else {
        unreachable!("predicate compile received non-predicate analysis")
    };
    let branches = generate_predicate_branches(predicate)
        .map_err(|error| CompileError::new(None, CompileErrorKind::Generate(error)))?;
    for branch in branches {
        let environment = predicate_environment(predicate, &branch);
        let mut rule_path = prefix_rules.clone();
        rule_path.extend(branch.rule_path);
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
        let smart_left = predicate.alternation != kfind_morph::LexicalAlternation::Copula
            && !(analysis.source == AnalysisSource::Forced
                && rule_path
                    .last()
                    .is_some_and(|rule| rule.as_str() == CONNECTIVE_JI_RULE_ID));
        output.push(DraftBranch {
            anchor: branch.anchor.into(),
            verifier: BranchVerifier::Predicate {
                continuation: branch.continuation,
                pos: predicate.pos,
                allowed_rule_ids: Arc::clone(allowed_rules),
                nominal_particle_transition,
                environment,
            },
            core_mapping: CoreMapping::PrefixBytes(branch.core_len),
            origin: Origin {
                analysis_index,
                rule_path,
            },
            smart_left,
            context_requirement: if predicate.alternation == kfind_morph::LexicalAlternation::Copula
            {
                ContextRequirement::PredicateLexical
            } else {
                ContextRequirement::None
            },
        });
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn compile_derivations(
    analysis: &Analysis,
    analysis_index: u16,
    analyzer: &LexiconQueryAnalyzer,
    predicate_rules: &Arc<[RuleId]>,
    particle_rules: &Arc<[RuleId]>,
    known_rule_ids: &HashSet<&str>,
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
                predicate_rules,
                known_rule_ids,
                excluded_rules,
                output,
            )?;
        } else if rule.result_pos.is_nominal() {
            output.push(DraftBranch {
                anchor: derived_lemma,
                verifier: BranchVerifier::NominalParticles {
                    allowed_rule_ids: Arc::clone(particle_rules),
                    blocked_rule_ids: Arc::from([]),
                },
                core_mapping: CoreMapping::WholeAnchor,
                origin: Origin {
                    analysis_index,
                    rule_path: derivation_path,
                },
                smart_left: true,
                context_requirement: ContextRequirement::NominalComponent,
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
        .map(|override_form| override_form.rule_id.clone())
        .collect::<Vec<_>>();
    rules.sort();
    rules.dedup();
    rules.into()
}

fn predicate_environment(
    predicate: &kfind_morph::PredicateEntry,
    branch: &kfind_morph::SurfaceBranchSpec,
) -> BranchEnvironment {
    let is_contracted_copula_aoeo = predicate.alternation
        == kfind_morph::LexicalAlternation::Copula
        && branch
            .rule_path
            .iter()
            .any(|rule| rule.as_str() == COPULA_CONTRACTED_AOEO_RULE_ID);
    if !is_contracted_copula_aoeo {
        return BranchEnvironment::Unrestricted;
    }

    let Some(stem) = predicate.lemma.strip_suffix('다') else {
        return BranchEnvironment::Unrestricted;
    };
    if branch.anchor.starts_with(stem) {
        BranchEnvironment::Unrestricted
    } else {
        BranchEnvironment::ContractedAfterVowel {
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
    exact_branch_with_context(
        surface,
        analysis_index,
        rule_path,
        smart_left,
        ContextRequirement::None,
    )
}

fn exact_branch_with_context(
    surface: &str,
    analysis_index: u16,
    rule_path: Vec<RuleId>,
    smart_left: bool,
    context_requirement: ContextRequirement,
) -> DraftBranch {
    DraftBranch {
        anchor: surface.to_owned(),
        verifier: BranchVerifier::Exact,
        core_mapping: CoreMapping::WholeAnchor,
        origin: Origin {
            analysis_index,
            rule_path,
        },
        smart_left,
        context_requirement,
    }
}

fn allowed_rules(known: &HashSet<&str>, include: impl Fn(&str) -> bool) -> Arc<[RuleId]> {
    let mut rules = known
        .iter()
        .copied()
        .filter(|id| include(id))
        .map(RuleId::from)
        .collect::<Vec<_>>();
    rules.sort();
    rules.into()
}

fn missing_rules(ids: &[&str], known: &HashSet<&str>) -> impl Iterator<Item = RuleId> {
    ids.iter()
        .copied()
        .filter(|id| !known.contains(id))
        .map(RuleId::from)
}

fn is_known_or_internal(rule: &RuleId, known: &HashSet<&str>) -> bool {
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

fn estimate_branch_bytes(branch: &SurfaceBranch) -> usize {
    BRANCH_OVERHEAD_BYTES
        + branch.anchor.len()
        + branch
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
}

#[cfg(test)]
mod tests;
