use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use grep_matcher::{LineMatchKind, LineTerminator, Match, Matcher, NoCaptures, NoError};
use kfind_data::{ComponentResource, DataFinePos};
use kfind_morph::{
    ConstraintResolver, DEFAULT_LATTICE_NODE_LIMIT, MorphContinuation, ParticleChainModel,
    ParticleVerifier, PredicateStemClass, PreparedTokenGraph, ProductPolicy, RuleId,
    verify_copula_surface_after_nominal, verify_predicate_continuation,
    verify_predicate_continuation_with_prefinal_context,
};
use kfind_query::{
    AtomPlan, BoundaryPolicy, CandidateConsumption, CandidateDecision, CandidateLeftContext,
    CandidateProgram, CoreMapping, Origin, PhraseMatch, QueryPlan, RoutedMatch, VerifiedSpan,
};
use unicode_normalization::{UnicodeNormalization, is_nfc};

use crate::SearchDiagnostics;
use crate::boundary::{
    accepts_requirements, is_boundary_after, is_boundary_before, is_sentence_end_after,
    surrounding_token_span,
};
use crate::{AnchorBuildError, AnchorBuildLimits, AnchorEngine, AnchorHit};

mod candidates;
mod context;
mod fused_copula;
mod line_candidate;
mod phrase;
mod streaming_phrase;

pub use candidates::LocalAnalysisCandidate;
use context::{
    PreparedStructuralContextAnalysis, PreparedStructuralContextCache, StructuralPreparation,
    StructuralRequest,
};
use phrase::{PhraseMatchLimit, PhraseSelection};

const MAX_CONSUMPTION_BYTES: usize = 256;
const MAX_PREPARED_EXACT_TOKEN_PROGRAMS: usize = 8;
const MAX_PREPARED_EXACT_TOKEN_GRAPHS: usize = 64;
const ADNOMINAL_RULE_IDS: [&str; 4] = [
    "ending.present-adnominal",
    "ending.past-adnominal",
    "ending.future-adnominal",
    "ending.retrospective-adnominal",
];
#[derive(Default)]
struct StructuralCache<'a> {
    diagnostics: Option<&'a SearchDiagnostics>,
    windows: HashMap<(usize, usize, bool, bool), Option<PreparedStructuralContextAnalysis<'a>>>,
    prepared_contexts: PreparedStructuralContextCache,
}

impl StructuralCache<'_> {
    fn clear_windows(&mut self) {
        self.windows.clear();
    }
}

impl PreparedExactTokenGraphs {
    fn build(plan: &QueryPlan, anchor_memory_usage: usize) -> Self {
        let program_count = plan.atoms.iter().try_fold(0usize, |count, atom| {
            count
                .checked_add(atom.programs.len())
                .filter(|count| *count <= MAX_PREPARED_EXACT_TOKEN_PROGRAMS)
        });
        if program_count.is_none() {
            return Self::default();
        }
        let mut keys = plan
            .atoms
            .iter()
            .flat_map(|atom| &atom.programs)
            .filter_map(exact_token_graph_key)
            .collect::<Vec<_>>();
        keys.sort_unstable();
        keys.dedup();
        if keys.len() > MAX_PREPARED_EXACT_TOKEN_GRAPHS {
            return Self::default();
        }

        let mut result = Self::default();
        let mut remaining_memory = plan
            .limits
            .max_matcher_bytes
            .saturating_sub(anchor_memory_usage);
        for (text, include_nominal_copula, include_nominal_derivation_predicate) in keys {
            let memory_usage = text
                .len()
                .saturating_add(std::mem::size_of::<PreparedExactTokenGraph>());
            if memory_usage > remaining_memory {
                continue;
            }
            remaining_memory -= memory_usage;
            result.entries
                [prepared_token_slot(include_nominal_copula, include_nominal_derivation_predicate)]
            .push(PreparedExactTokenGraph {
                text: text.into_owned().into_boxed_str(),
                graph: OnceLock::new(),
            });
        }
        for entries in &mut result.entries {
            entries.shrink_to_fit();
        }
        result.remaining_memory = AtomicUsize::new(remaining_memory);
        result.minimum_text_bytes = result
            .entries
            .iter()
            .flatten()
            .map(|entry| entry.text.len())
            .min()
            .unwrap_or(0);
        result.maximum_text_bytes = result
            .entries
            .iter()
            .flatten()
            .map(|entry| entry.text.len())
            .max()
            .unwrap_or(0);
        result
    }

    fn get_or_prepare(
        &self,
        text: &str,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
        resolver: &ConstraintResolver,
        node_limit: usize,
    ) -> Option<Arc<PreparedTokenGraph>> {
        if text.len() < self.minimum_text_bytes || text.len() > self.maximum_text_bytes {
            return None;
        }
        let entries = &self.entries
            [prepared_token_slot(include_nominal_copula, include_nominal_derivation_predicate)];
        let index = entries
            .binary_search_by(|entry| entry.text.as_ref().cmp(text))
            .ok()?;
        entries[index]
            .graph
            .get_or_init(|| {
                let graph = resolver
                    .prepare_token_graph_for_candidate(
                        text,
                        node_limit,
                        include_nominal_copula,
                        include_nominal_derivation_predicate,
                    )
                    .ok()?;
                self.reserve_graph_memory(graph.memory_usage())
                    .then(|| Arc::new(graph))
            })
            .clone()
    }

    fn reserve_graph_memory(&self, memory_usage: usize) -> bool {
        self.remaining_memory
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |remaining| {
                remaining.checked_sub(memory_usage)
            })
            .is_ok()
    }
}

fn exact_token_graph_key(branch: &CandidateProgram) -> Option<(Cow<'_, str>, bool, bool)> {
    let patterns = branch.structural_patterns();
    if patterns.is_empty() {
        return None;
    }
    let raw_text = std::str::from_utf8(&branch.anchor).ok()?;
    let text = if is_nfc(raw_text) {
        Cow::Borrowed(raw_text)
    } else {
        Cow::Owned(raw_text.nfc().collect())
    };
    let include_nominal_derivation_predicate = patterns.iter().any(|pattern| {
        pattern.fine_pos.is_nominal()
            && matches!(pattern.continuation, MorphContinuation::NominalParticles)
            && pattern.component_capability.allows_runtime()
    });
    let include_nominal_copula = matches!(
        branch.consumption,
        CandidateConsumption::PredicateContinuation {
            pos: kfind_morph::PredicatePos::Copula,
            ..
        }
    );
    Some((
        text,
        include_nominal_copula,
        include_nominal_derivation_predicate,
    ))
}

const fn prepared_token_slot(
    include_nominal_copula: bool,
    include_nominal_derivation_predicate: bool,
) -> usize {
    include_nominal_copula as usize | ((include_nominal_derivation_predicate as usize) << 1)
}

/// A query-plan matcher backed by one shared set of unique anchors.
#[derive(Debug)]
pub struct MorphMatcher {
    plan: Arc<QueryPlan>,
    anchor_engine: AnchorEngine,
    anchor_programs: Vec<Box<[ProgramRef]>>,
    fused_copula_anchor: Option<usize>,
    max_anchor_end_from_token_start: usize,
    is_line_local: bool,
    particle_verifier: ParticleVerifier,
    constraint_resolver: Option<ConstraintResolver>,
    prepared_exact_tokens: PreparedExactTokenGraphs,
}

#[derive(Debug, Default)]
struct PreparedExactTokenGraphs {
    entries: [Vec<PreparedExactTokenGraph>; 4],
    minimum_text_bytes: usize,
    maximum_text_bytes: usize,
    remaining_memory: AtomicUsize,
}

#[derive(Debug)]
struct PreparedExactTokenGraph {
    text: Box<str>,
    graph: OnceLock<Option<Arc<PreparedTokenGraph>>>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VerificationCounters {
    pub raw_anchor_hits: usize,
    pub verified_program_hits: usize,
    pub structural_candidate_hits: usize,
    pub unique_structural_windows: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MatchLimitExceeded {
    limit: usize,
}

impl MatchLimitExceeded {
    #[must_use]
    pub const fn limit(self) -> usize {
        self.limit
    }
}

impl Display for MatchLimitExceeded {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "match count exceeds limit {}", self.limit)
    }
}

impl Error for MatchLimitExceeded {}

impl MorphMatcher {
    pub fn new(plan: Arc<QueryPlan>) -> Result<Self, MorphMatcherBuildError> {
        Self::build(plan, None)
    }

    pub fn with_component_resource(
        plan: Arc<QueryPlan>,
        component_resource: Arc<ComponentResource>,
    ) -> Result<Self, MorphMatcherBuildError> {
        let attached_auxiliary = plan
            .atoms
            .iter()
            .flat_map(|atom| &atom.programs)
            .flat_map(CandidateProgram::structural_patterns)
            .any(|pattern| pattern.fine_pos == DataFinePos::Vx);
        Self::build(
            plan,
            Some(
                ConstraintResolver::new(component_resource)
                    .with_attached_auxiliary(attached_auxiliary),
            ),
        )
    }

    fn build(
        plan: Arc<QueryPlan>,
        constraint_resolver: Option<ConstraintResolver>,
    ) -> Result<Self, MorphMatcherBuildError> {
        if plan.atoms.is_empty() {
            return Err(MorphMatcherBuildError::EmptyPlan);
        }
        if let Some((atom_index, _)) = plan
            .atoms
            .iter()
            .enumerate()
            .find(|(_, atom)| atom.programs.is_empty())
        {
            return Err(MorphMatcherBuildError::EmptyAtom { atom_index });
        }
        if plan.requires_component_resource() && constraint_resolver.is_none() {
            return Err(MorphMatcherBuildError::ComponentResourceRequired);
        }

        let (anchors, mut anchor_programs, fused_copula_programs) = unique_anchors(&plan);
        let fused_copula_anchor = (!fused_copula_programs.is_empty()).then(|| {
            let index = anchor_programs.len();
            anchor_programs.push(fused_copula_programs.into_boxed_slice());
            index
        });
        let mut max_anchor_end_from_token_start =
            anchors.iter().map(|anchor| anchor.len()).max().unwrap_or(0);
        if fused_copula_anchor.is_some() {
            max_anchor_end_from_token_start =
                max_anchor_end_from_token_start.max(MAX_CONSUMPTION_BYTES);
        }
        let is_line_local = anchors.iter().all(|anchor| !anchor.contains(&b'\n'));
        let anchor_engine = AnchorEngine::from_unique_anchors(
            &anchors,
            AnchorBuildLimits {
                max_anchors: plan.limits.max_programs,
                max_memory_bytes: plan.limits.max_matcher_bytes,
            },
        )?;
        let particle_model =
            if plan.particle_allomorphs.is_empty() || plan.particle_transitions.is_empty() {
                ParticleChainModel::default()
            } else {
                ParticleChainModel::with_graph(
                    Arc::clone(&plan.particle_allomorphs),
                    Arc::clone(&plan.particle_transitions),
                )
            };
        let particle_verifier = ParticleVerifier::new(particle_model);
        let prepared_exact_tokens = constraint_resolver
            .as_ref()
            .map_or_else(PreparedExactTokenGraphs::default, |_| {
                PreparedExactTokenGraphs::build(&plan, anchor_engine.memory_usage())
            });
        Ok(Self {
            plan,
            anchor_engine,
            anchor_programs,
            fused_copula_anchor,
            max_anchor_end_from_token_start,
            is_line_local,
            particle_verifier,
            constraint_resolver,
            prepared_exact_tokens,
        })
    }

    #[must_use]
    pub fn plan(&self) -> &Arc<QueryPlan> {
        &self.plan
    }

    #[must_use]
    pub fn anchor_engine(&self) -> &AnchorEngine {
        &self.anchor_engine
    }

    /// Finds the next non-overlapping query match without morphology provenance.
    #[must_use]
    pub fn find_span_at(&self, haystack: &[u8], at: usize) -> Option<Range<usize>> {
        self.find_span_at_with_diagnostics(haystack, at, &SearchDiagnostics::default())
    }

    /// Finds a span and records unavailable structural verification in the caller's scope.
    #[must_use]
    pub fn find_span_at_with_diagnostics(
        &self,
        haystack: &[u8],
        at: usize,
        diagnostics: &SearchDiagnostics,
    ) -> Option<Range<usize>> {
        if at > haystack.len() {
            return None;
        }
        if self.plan.atoms.len() == 1 && self.plan.graph.is_none() {
            let mut cache = StructuralCache {
                diagnostics: Some(diagnostics),
                ..Default::default()
            };
            return self
                .find_single_atom_best_with_cache(haystack, at, MatchMetadata::SpanOnly, &mut cache)
                .map(|span| span.token);
        }
        streaming_phrase::select_with_diagnostics(
            self,
            haystack,
            at,
            MatchMetadata::SpanOnly,
            PhraseMatchLimit::First,
            Some(diagnostics),
        )
        .matches
        .into_iter()
        .next()
        .map(|matched| matched.span)
    }

    /// Finds the next non-overlapping query match with its atom metadata.
    #[must_use]
    pub fn find_at_with_meta(&self, haystack: &[u8], at: usize) -> Option<PhraseMatch> {
        if at > haystack.len() {
            return None;
        }
        if self.plan.atoms.len() == 1 && self.plan.graph.is_none() {
            return self.find_single_atom_at(haystack, at);
        }
        self.find_phrase_at(haystack, at)
    }

    /// Recomputes all non-overlapping matches and morphology provenance.
    ///
    /// This is intended for matched lines that need JSON or explain metadata.
    #[must_use]
    pub fn find_all_with_meta(&self, haystack: &[u8]) -> Vec<PhraseMatch> {
        if self.plan.atoms.len() > 1
            || self.plan.graph.is_some()
            || self.fused_copula_anchor.is_some()
        {
            return self
                .find_phrases_with_meta(haystack, PhraseMatchLimit::All)
                .matches;
        }
        let mut matches = Vec::new();
        let mut at = 0;
        let mut structural_cache = StructuralCache::default();
        loop {
            structural_cache.clear_windows();
            let Some(matched) = self.find_single_atom_best_with_cache(
                haystack,
                at,
                MatchMetadata::Provenance,
                &mut structural_cache,
            ) else {
                break;
            };
            let matched = single_atom_phrase(matched);
            at = matched.span.end;
            matches.push(matched);
        }
        matches
    }

    /// Recomputes up to `limit` non-overlapping matches and morphology provenance.
    ///
    /// Returns an error as soon as the matcher proves that another match exists.
    pub fn find_all_with_meta_limit(
        &self,
        haystack: &[u8],
        limit: usize,
    ) -> Result<Vec<PhraseMatch>, MatchLimitExceeded> {
        self.find_all_with_meta_limit_and_diagnostics(
            haystack,
            limit,
            &SearchDiagnostics::default(),
        )
    }

    /// Collects matches while accumulating structural verification diagnostics.
    pub fn find_all_with_meta_limit_and_diagnostics(
        &self,
        haystack: &[u8],
        limit: usize,
        diagnostics: &SearchDiagnostics,
    ) -> Result<Vec<PhraseMatch>, MatchLimitExceeded> {
        if self.plan.atoms.len() > 1
            || self.plan.graph.is_some()
            || self.fused_copula_anchor.is_some()
        {
            let selection = streaming_phrase::select_with_diagnostics(
                self,
                haystack,
                0,
                MatchMetadata::Provenance,
                PhraseMatchLimit::Bounded(limit),
                Some(diagnostics),
            );
            return if selection.limit_exceeded {
                Err(MatchLimitExceeded { limit })
            } else {
                Ok(selection.matches)
            };
        }
        let mut matches = Vec::new();
        let mut at = 0;
        let mut structural_cache = StructuralCache {
            diagnostics: Some(diagnostics),
            ..Default::default()
        };
        loop {
            structural_cache.clear_windows();
            let Some(matched) = self.find_single_atom_best_with_cache(
                haystack,
                at,
                MatchMetadata::Provenance,
                &mut structural_cache,
            ) else {
                break;
            };
            let matched = single_atom_phrase(matched);
            if matches.len() == limit {
                return Err(MatchLimitExceeded { limit });
            }
            at = matched.span.end;
            matches.push(matched);
        }
        Ok(matches)
    }

    /// Finds bounded matches with their chosen query atom route.
    pub fn find_all_with_route_limit_and_diagnostics(
        &self,
        haystack: &[u8],
        limit: usize,
        diagnostics: &SearchDiagnostics,
    ) -> Result<Vec<RoutedMatch>, MatchLimitExceeded> {
        if self.plan.graph.is_none() {
            return self
                .find_all_with_meta_limit_and_diagnostics(haystack, limit, diagnostics)
                .map(|matches| {
                    matches
                        .into_iter()
                        .map(|matched| RoutedMatch {
                            query_atom_indices: (0..matched.atoms.len()).collect(),
                            matched,
                        })
                        .collect()
                });
        }
        let selection = streaming_phrase::select_with_diagnostics(
            self,
            haystack,
            0,
            MatchMetadata::Provenance,
            PhraseMatchLimit::Bounded(limit),
            Some(diagnostics),
        );
        if selection.limit_exceeded {
            return Err(MatchLimitExceeded { limit });
        }
        Ok(selection
            .matches
            .into_iter()
            .zip(selection.routes)
            .map(|(matched, query_atom_indices)| RoutedMatch {
                matched,
                query_atom_indices,
            })
            .collect())
    }

    #[must_use]
    pub fn find_at_with_route(&self, haystack: &[u8], at: usize) -> Option<RoutedMatch> {
        if at > haystack.len() {
            return None;
        }
        if self.plan.graph.is_none() {
            return self
                .find_at_with_meta(haystack, at)
                .map(|matched| RoutedMatch {
                    query_atom_indices: (0..matched.atoms.len()).collect(),
                    matched,
                });
        }
        let selection = streaming_phrase::select(
            self,
            haystack,
            at,
            MatchMetadata::Provenance,
            PhraseMatchLimit::First,
        );
        selection
            .matches
            .into_iter()
            .zip(selection.routes)
            .next()
            .map(|(matched, query_atom_indices)| RoutedMatch {
                matched,
                query_atom_indices,
            })
    }

    fn find_phrases_with_meta(&self, haystack: &[u8], limit: PhraseMatchLimit) -> PhraseSelection {
        streaming_phrase::select(self, haystack, 0, MatchMetadata::Provenance, limit)
    }

    #[must_use]
    pub fn verification_counters(&self, haystack: &[u8]) -> VerificationCounters {
        let mut counters = VerificationCounters::default();
        let mut structural_windows = HashSet::new();
        let mut structural_cache = StructuralCache::default();
        for hit in self.anchor_hits(haystack, 0) {
            counters.raw_anchor_hits += 1;
            for branch_ref in &self.anchor_programs[hit.anchor_index] {
                let atom = &self.plan.atoms[branch_ref.atom_index];
                let branch = &atom.programs[branch_ref.program_index];
                let Some(candidate) = self.execute_program_without_decision(
                    haystack,
                    &hit,
                    branch,
                    MatchMetadata::SpanOnly,
                ) else {
                    continue;
                };
                if !self.accepts_program(haystack, &candidate, branch, atom, &mut structural_cache)
                {
                    if matches!(&branch.decision, CandidateDecision::Structural(_)) {
                        counters.structural_candidate_hits += 1;
                        let window =
                            surrounding_token_span(haystack, candidate.verified.core.clone());
                        structural_windows.insert((window.start, window.end));
                    }
                    continue;
                }
                counters.verified_program_hits += 1;
            }
        }
        counters.unique_structural_windows = structural_windows.len();
        counters
    }

    fn find_single_atom_at(&self, haystack: &[u8], at: usize) -> Option<PhraseMatch> {
        self.find_single_atom_best(haystack, at, MatchMetadata::Provenance)
            .map(single_atom_phrase)
    }

    fn find_single_atom_best(
        &self,
        haystack: &[u8],
        at: usize,
        metadata: MatchMetadata,
    ) -> Option<VerifiedSpan> {
        let mut structural_cache = StructuralCache::default();
        self.find_single_atom_best_with_cache(haystack, at, metadata, &mut structural_cache)
    }

    fn find_single_atom_best_with_cache<'a>(
        &self,
        haystack: &'a [u8],
        at: usize,
        metadata: MatchMetadata,
        structural_cache: &mut StructuralCache<'a>,
    ) -> Option<VerifiedSpan> {
        let mut best = None;
        for hit in self.anchor_hits(haystack, at) {
            if best.as_ref().is_some_and(|matched: &VerifiedSpan| {
                hit.span.end
                    > matched
                        .token
                        .start
                        .saturating_add(self.max_anchor_end_from_token_start)
            }) {
                break;
            }
            for branch_ref in &self.anchor_programs[hit.anchor_index] {
                if branch_ref.atom_index != 0 {
                    continue;
                }
                let branch = &self.plan.atoms[0].programs[branch_ref.program_index];
                let Some(candidate) = self.execute_program_with_metadata(
                    haystack,
                    &hit,
                    branch,
                    &self.plan.atoms[0],
                    metadata,
                    structural_cache,
                ) else {
                    continue;
                };
                merge_best_span(&mut best, candidate);
            }
        }
        best
    }

    fn find_phrase_at(&self, haystack: &[u8], at: usize) -> Option<PhraseMatch> {
        self.find_phrase_at_with_metadata(haystack, at, MatchMetadata::Provenance)
    }

    fn find_phrase_at_with_metadata(
        &self,
        haystack: &[u8],
        at: usize,
        metadata: MatchMetadata,
    ) -> Option<PhraseMatch> {
        streaming_phrase::select(self, haystack, at, metadata, PhraseMatchLimit::First)
            .matches
            .into_iter()
            .next()
    }

    fn execute_program_with_metadata<'a>(
        &self,
        haystack: &'a [u8],
        hit: &AnchorHit,
        branch: &CandidateProgram,
        atom: &AtomPlan,
        metadata: MatchMetadata,
        structural_cache: &mut StructuralCache<'a>,
    ) -> Option<VerifiedSpan> {
        let candidate = self.execute_program_without_decision(haystack, hit, branch, metadata)?;
        self.accepts_program(haystack, &candidate, branch, atom, structural_cache)
            .then_some(candidate.verified)
    }

    fn execute_program_without_decision(
        &self,
        haystack: &[u8],
        hit: &AnchorHit,
        branch: &CandidateProgram,
        metadata: MatchMetadata,
    ) -> Option<ExecutedCandidate> {
        let anchor = std::str::from_utf8(haystack.get(hit.span.clone())?).ok()?;
        let core = mapped_core(&hit.span, branch.core_mapping, anchor)?;
        let (consumed_bytes, suffix_rules) = match &branch.consumption {
            CandidateConsumption::Anchor
            | CandidateConsumption::CopulaHostEndingCompose
            | CandidateConsumption::FusedCopulaEnding => (0, Vec::new()),
            CandidateConsumption::PredicateContinuation {
                continuation,
                pos,
                source_positions,
                nominal_particle_transition,
                left_context,
                ..
            } => {
                if !accepts_left_context(left_context, haystack, hit.span.start) {
                    return None;
                }
                let following = valid_utf8_prefix(&haystack[hit.span.end..]);
                let (consumption_anchor, consumption_following) =
                    normalized_consumption_text(anchor, following);
                let matched = verify_predicate_continuation_with_prefinal_context(
                    *continuation,
                    *pos,
                    &consumption_anchor,
                    &consumption_following,
                    || {
                        !branch.decision.is_structural()
                            || is_boundary_before(haystack, core.start)
                            || self.constraint_resolver.as_ref().is_some_and(|resolver| {
                                let whole = surrounding_token_span(haystack, core.clone());
                                if *pos == kfind_morph::PredicatePos::Copula {
                                    return context::normalized_token(
                                        haystack,
                                        whole.start..core.start,
                                    )
                                    .is_some_and(|host| {
                                        resolver.has_complete_nominal_surface(&host)
                                    });
                                }
                                source_aligned_compound_predicate_position(
                                    haystack,
                                    &core,
                                    &whole,
                                    *source_positions,
                                    resolver,
                                )
                            })
                    },
                )?;
                if *pos == kfind_morph::PredicatePos::Copula
                    && *continuation == kfind_morph::ContinuationState::StemEnding
                    && matched.rule_path.iter().any(|rule| {
                        rule.as_str().starts_with("ending.copula-")
                            || rule.as_str().starts_with("ending.hao-")
                    })
                    && is_boundary_before(haystack, hit.span.start)
                {
                    return None;
                }
                if !branch.consumption.allows_rule_path(&matched.rule_path) {
                    return None;
                }
                let mut normalized_consumed = matched.consumed_bytes;
                let mut rule_path = matched.rule_path;
                if let Some(host) = auxiliary_nominal_host(&rule_path)
                    .or_else(|| nominal_particle_transition.then_some(consumption_anchor.as_ref()))
                {
                    let particle = self
                        .particle_verifier
                        .verify_prefix(host, &consumption_following[normalized_consumed..]);
                    normalized_consumed =
                        normalized_consumed.checked_add(particle.consumed_bytes)?;
                    rule_path.extend(particle.rule_path);
                }
                let consumed_bytes = match &consumption_following {
                    Cow::Borrowed(_) => normalized_consumed,
                    Cow::Owned(normalized) => {
                        map_normalized_prefix(following, normalized, normalized_consumed)?
                    }
                };
                (consumed_bytes, rule_path)
            }
            CandidateConsumption::StructuralPredicateEnding {
                pos,
                source_positions,
                flags,
                base_state,
                validate_anchor,
                stem_class,
                ..
            } => {
                let resolver = self.constraint_resolver.as_ref()?;
                let whole = surrounding_token_span(haystack, hit.span.clone());
                if whole.start != hit.span.start
                    || whole.end <= hit.span.end
                    || whole.len() > MAX_CONSUMPTION_BYTES
                {
                    return None;
                }
                let token = std::str::from_utf8(haystack.get(whole.clone())?).ok()?;
                let normalized_token = token.nfc().collect::<String>();
                if *pos == kfind_morph::PredicatePos::Copula
                    && resolver.has_exact_pronoun_case_surface(&normalized_token)
                    && !resolver.has_exact_predicate_ending_path(&normalized_token, *pos)
                {
                    return None;
                }
                let normalized_anchor = anchor.nfc().collect::<String>();
                let normalized_core_len = std::str::from_utf8(haystack.get(core.clone())?)
                    .ok()?
                    .nfc()
                    .collect::<String>()
                    .len();
                let suffix = normalized_token.get(normalized_anchor.len()..)?;
                // This fallback starts at the token boundary. A causal copula needs a host.
                if *pos == kfind_morph::PredicatePos::Copula && suffix.ends_with("므로") {
                    return None;
                }
                let ending_path = branch.consumption.allows_structural_suffix(suffix)
                    && stem_accepts_ending(
                        *pos,
                        *flags,
                        *base_state,
                        *stem_class,
                        &normalized_anchor,
                        suffix,
                    )
                    && if *validate_anchor {
                        source_positions.iter().any(|source_pos| {
                            resolver.supports_predicate_ending_path(
                                &normalized_token,
                                normalized_anchor.len(),
                                source_pos,
                                DEFAULT_LATTICE_NODE_LIMIT,
                            )
                        })
                    } else {
                        resolver.supports_ending_suffix_path(
                            &normalized_token,
                            normalized_anchor.len(),
                            DEFAULT_LATTICE_NODE_LIMIT,
                        )
                    };
                let auxiliary_path = normalized_anchor.ends_with(['아', '어', '여'])
                    && resolver.auxiliary_splits(suffix).into_iter().any(|split| {
                        split == suffix.len()
                            || suffix.get(split..).is_some_and(|ending| {
                                branch.consumption.allows_structural_suffix(ending)
                            })
                    });
                if (!ending_path && !auxiliary_path)
                    || (!auxiliary_path
                        && source_positions.iter().any(|source_pos| {
                            resolver.whole_predicate_conflicts(
                                &normalized_token,
                                normalized_core_len,
                                source_pos,
                            )
                        }))
                {
                    return None;
                }
                (
                    whole.end.checked_sub(hit.span.end)?,
                    vec![RuleId::from("structural.ending-path")],
                )
            }
            CandidateConsumption::NominalParticleChain { .. }
            | CandidateConsumption::NominalCopulaEndingChain { .. } => {
                let following = valid_utf8_prefix(&haystack[hit.span.end..]);
                let (consumption_anchor, consumption_following) =
                    normalized_consumption_text(anchor, following);
                let matched = self
                    .particle_verifier
                    .verify_prefix(&consumption_anchor, &consumption_following);
                if !branch.consumption.allows_rule_path(&matched.rule_path) {
                    return None;
                }
                let consumed_bytes = match &consumption_following {
                    Cow::Borrowed(_) => matched.consumed_bytes,
                    Cow::Owned(normalized) => {
                        map_normalized_prefix(following, normalized, matched.consumed_bytes)?
                    }
                };
                (consumed_bytes, matched.rule_path)
            }
            CandidateConsumption::DirectParticleHost { rule_id } => {
                if requires_direct_particle_host(branch)
                    && !self.accepts_direct_particle(haystack, &hit.span, rule_id)
                {
                    return None;
                }
                (0, Vec::new())
            }
        };
        let token_end = hit.span.end.checked_add(consumed_bytes)?;
        if token_end > haystack.len() || !is_utf8_boundary(haystack, token_end) {
            return None;
        }
        if suffix_rules.last().is_some_and(|rule| {
            matches!(
                rule.as_str(),
                "ending.copula-ya" | "ending.copula-ge" | "ending.final-ge"
            )
        }) && !is_sentence_end_after(haystack, token_end)
        {
            return None;
        }
        let token = hit.span.start..token_end;
        let origins = match metadata {
            MatchMetadata::SpanOnly => Vec::new(),
            MatchMetadata::Provenance => extend_origins(&branch.origins, &suffix_rules),
        };
        Some(ExecutedCandidate {
            anchor: hit.span.clone(),
            consumed: token.clone(),
            suffix_rules,
            verified: VerifiedSpan {
                core,
                token,
                origins,
            },
        })
    }

    fn accepts_program<'a>(
        &self,
        haystack: &'a [u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        atom: &AtomPlan,
        structural_cache: &mut StructuralCache<'a>,
    ) -> bool {
        let boundary = atom.boundary;
        if boundary == BoundaryPolicy::Smart
            && matches!(
                branch.consumption,
                CandidateConsumption::PredicateContinuation {
                    pos: kfind_morph::PredicatePos::Copula,
                    ..
                }
            )
            && is_boundary_before(haystack, candidate.verified.core.start)
            && candidate
                .suffix_rules
                .iter()
                .any(|rule| rule.as_str() == "ending.connective-meuro")
        {
            return false;
        }
        if boundary == BoundaryPolicy::Smart
            && matches!(
                branch.consumption,
                CandidateConsumption::PredicateContinuation {
                    continuation: kfind_morph::ContinuationState::NieunEnding,
                    ..
                }
            )
            && candidate
                .suffix_rules
                .iter()
                .any(|rule| rule.as_str() == "ending.connective-neunde")
            && self.constraint_resolver.as_ref().is_some_and(|resolver| {
                rejects_background_conjunction(haystack, candidate, resolver)
            })
        {
            return false;
        }
        if boundary == BoundaryPolicy::Smart
            && self.constraint_resolver.as_ref().is_some_and(|resolver| {
                has_competing_hao_connective(haystack, candidate, branch, resolver)
                    || has_nominal_quotative_frame(haystack, candidate, branch, resolver)
                    || has_counted_unit_copula(haystack, candidate, branch, resolver)
                    || has_transitive_jiyo_frame(haystack, candidate, branch, resolver)
                    || has_nominal_frame_for_volitional(haystack, candidate, branch, resolver)
                    || has_prayer_object_frame(haystack, candidate, branch, atom, resolver)
                    || has_topic_before_independent_predicate(haystack, candidate, branch, resolver)
                    || has_genitive_nominal_interrogative_frame(
                        haystack, candidate, branch, resolver,
                    )
            })
        {
            return false;
        }
        match &branch.decision {
            CandidateDecision::Boundary(_) => {
                self.accepts_token_boundary(haystack, &candidate.verified, branch)
                    || self.accepts_boundary_adnominal_interrogative(haystack, candidate, branch)
            }
            CandidateDecision::Structural(_) => {
                self.accepts_structural(haystack, candidate, branch, structural_cache)
            }
        }
    }

    fn accepts_boundary_adnominal_interrogative(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
    ) -> bool {
        let Some(resolver) = self.constraint_resolver.as_ref() else {
            return false;
        };
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
        if candidate.consumed.end >= whole.end {
            return false;
        }
        let Some(trailing) = haystack
            .get(candidate.consumed.end..whole.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return false;
        };
        let has_adnominal_rule = branch
            .origins
            .iter()
            .flat_map(|origin| &origin.rule_path)
            .chain(&candidate.suffix_rules)
            .any(|rule| ADNOMINAL_RULE_IDS.contains(&rule.as_str()));
        if !trailing.nfc().eq("가".chars())
            || !has_adnominal_rule
            || !self.supports_source_predicate_trailing(
                haystack, candidate, &whole, trailing, branch, resolver,
            )
        {
            return false;
        }
        let boundary = branch.boundary();
        accepts_requirements(
            haystack,
            candidate.verified.core.clone(),
            candidate.consumed.start..whole.end,
            boundary.require_left,
            boundary.require_right,
        )
    }

    fn accepts_structural<'a>(
        &self,
        haystack: &'a [u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        structural_cache: &mut StructuralCache<'a>,
    ) -> bool {
        let Some(resolver) = self.constraint_resolver.as_ref() else {
            return false;
        };
        let patterns = branch.structural_patterns();
        if patterns.is_empty() {
            return false;
        }
        if matches!(branch.consumption, CandidateConsumption::Anchor)
            && branch.origins.iter().any(|origin| {
                origin
                    .rule_path
                    .iter()
                    .any(|rule| rule.as_str() == "contraction.dependent-subject-ge")
            })
        {
            return self.accepts_token_boundary(haystack, &candidate.verified, branch)
                && context::normalized_token(haystack, candidate.anchor.clone()).is_some_and(
                    |token| {
                        context::previous_word(haystack, candidate.anchor.start).is_some_and(
                            |previous| resolver.has_dependent_noun_subject_path(&previous, &token),
                        )
                    },
                );
        }
        if matches!(
            branch.consumption,
            CandidateConsumption::CopulaHostEndingCompose | CandidateConsumption::FusedCopulaEnding
        ) {
            return self
                .accepts_lost_span_copula_ending(haystack, candidate, branch, resolver, patterns);
        }
        if self.accepts_pronoun_copula_ending(haystack, candidate, branch, resolver, patterns) {
            return true;
        }
        if self.accepts_product_copula_frame(haystack, candidate, branch, resolver, patterns) {
            return true;
        }
        if self.accepts_lexical_adverb_particle_frame(haystack, candidate, branch, patterns) {
            return true;
        }
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
        if matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        ) && candidate.verified.core.start == whole.start
            && candidate.verified.core.end < whole.end
            && patterns
                .iter()
                .any(|pattern| matches!(pattern.fine_pos, DataFinePos::Nnb | DataFinePos::Nr))
            && context::previous_word(haystack, whole.start).is_some_and(|previous| {
                let digits = previous.strip_suffix('여').unwrap_or(&previous);
                !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
            })
            && context::normalized_token(haystack, whole.clone()).is_some_and(|token| {
                context::normalized_token(haystack, candidate.verified.core.clone()).is_some_and(
                    |unit| resolver.has_numeric_unit_before_dependent_noun(&token, &unit),
                )
            })
        {
            return true;
        }
        if self.has_lexicalized_time_modifier(haystack, candidate, branch, &whole, resolver) {
            return false;
        }
        let licensed_trailing =
            self.licensed_structural_trailing(haystack, candidate, branch, resolver);
        if licensed_trailing.is_none()
            && has_conflicting_whole_predicate(haystack, candidate, branch, resolver)
        {
            return false;
        }
        if licensed_trailing.is_none()
            && matches!(
                branch.consumption,
                CandidateConsumption::NominalParticleChain { .. }
            )
            && candidate.consumed.end < whole.end
            && contracted_copula_surface_follows(
                haystack,
                candidate.consumed.end,
                whole.end,
                patterns
                    .iter()
                    .any(|pattern| pattern.fine_pos == DataFinePos::Nr),
            )
        {
            return false;
        }
        let copula_program = matches!(
            branch.consumption,
            CandidateConsumption::PredicateContinuation {
                pos: kfind_morph::PredicatePos::Copula,
                ..
            }
        );
        let include_nominal_copula = copula_program
            || (patterns.iter().any(|pattern| pattern.fine_pos.is_nominal())
                && nominal_copula_surface_at(haystack, candidate.verified.core.end, whole.end))
            || (candidate.verified.core.start > whole.start
                && patterns
                    .iter()
                    .any(|pattern| pattern.fine_pos.is_predicate())
                && nominal_copula_surface_at(haystack, candidate.verified.core.start, whole.end));
        let include_nominal_derivation_predicate = patterns.iter().any(|pattern| {
            pattern.fine_pos.is_nominal()
                && matches!(pattern.continuation, MorphContinuation::NominalParticles)
                && pattern.component_capability.allows_runtime()
        });
        let consumed = licensed_trailing.unwrap_or_else(|| candidate.consumed.clone());
        let rejected_suffix =
            self.has_rejected_structural_suffix(haystack, candidate, &consumed, branch, resolver);
        let nominal_particle_chain = matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        );
        if rejected_suffix
            && (!nominal_particle_chain
                || self.has_terminal_particle_conflict(haystack, candidate, &whole))
        {
            return false;
        }
        let window = whole.clone();
        let context = structural_cache
            .windows
            .entry((
                window.start,
                window.end,
                include_nominal_copula,
                include_nominal_derivation_predicate,
            ))
            .or_insert_with(|| {
                PreparedStructuralContextAnalysis::extract(
                    haystack,
                    candidate.verified.core.clone(),
                    StructuralPreparation {
                        resolver,
                        node_limit: DEFAULT_LATTICE_NODE_LIMIT,
                        include_nominal_copula,
                        include_nominal_derivation_predicate,
                        prepared_exact_tokens: &self.prepared_exact_tokens,
                    },
                    &mut structural_cache.prepared_contexts,
                )
            });
        let Some(context) = context.as_ref() else {
            if let Some(diagnostics) = structural_cache.diagnostics {
                diagnostics.record_unavailable();
            }
            return false;
        };
        if rejected_suffix && !context.has_nominal_copula_host(candidate.verified.core.clone()) {
            return false;
        }
        let decision = context.resolve(StructuralRequest {
            candidate: &candidate.verified,
            anchor: candidate.anchor.clone(),
            consumed,
            patterns,
        });
        if decision.as_ref().is_none_or(|decision| {
            matches!(
                decision.outcome,
                kfind_morph::ConstraintOutcome::Unavailable(_)
            )
        }) && let Some(diagnostics) = structural_cache.diagnostics
        {
            diagnostics.record_unavailable();
        }
        decision.is_some_and(|decision| ProductPolicy::RecallFirst.accepts(&decision))
    }

    fn has_lexicalized_time_modifier(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        whole: &Range<usize>,
        resolver: &ConstraintResolver,
    ) -> bool {
        if candidate.verified.core.start != whole.start
            || candidate.consumed.end >= whole.end
            || !matches!(
                branch.consumption,
                CandidateConsumption::PredicateContinuation { .. }
            )
        {
            return false;
        }
        let trailing = candidate.consumed.end..whole.end;
        if !haystack.get(trailing.clone()).is_some_and(|bytes| {
            bytes.starts_with("때".as_bytes()) || bytes.starts_with("ᄄ".as_bytes())
        }) || !branch.origins.iter().any(|origin| {
            candidate
                .suffix_rules
                .last()
                .or_else(|| origin.rule_path.last())
                .is_some_and(|rule| ADNOMINAL_RULE_IDS.contains(&rule.as_str()))
        }) {
            return false;
        }
        let Some(trailing) = context::normalized_token(haystack, trailing) else {
            return false;
        };
        let Some(particles) = trailing.strip_prefix('때') else {
            return false;
        };
        let Some(prefix) = context::normalized_token(haystack, whole.start..candidate.consumed.end)
        else {
            return false;
        };
        let modifier = format!("{prefix}때");
        resolver.has_whole_modifier(&modifier)
            && self
                .particle_verifier
                .verify_prefix(&modifier, particles)
                .consumed_bytes
                == particles.len()
    }

    fn licensed_structural_trailing(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        resolver: &ConstraintResolver,
    ) -> Option<Range<usize>> {
        if matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        ) {
            return self.licensed_nominal_copula_trailing(haystack, candidate, resolver);
        }
        if !matches!(
            branch.consumption,
            CandidateConsumption::PredicateContinuation { .. }
        ) {
            return None;
        }
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
        if candidate.consumed.end >= whole.end {
            return None;
        }
        let trailing =
            std::str::from_utf8(haystack.get(candidate.consumed.end..whole.end)?).ok()?;
        let has_rule = |expected: &str| {
            branch
                .origins
                .iter()
                .flat_map(|origin| &origin.rule_path)
                .chain(&candidate.suffix_rules)
                .any(|rule| rule.as_str() == expected)
        };
        let source_predicate_continuation = self.supports_source_predicate_trailing(
            haystack, candidate, &whole, trailing, branch, resolver,
        );
        let source_attached_auxiliary = candidate.suffix_rules.is_empty()
            && has_rule("ending.aoeo")
            && match branch.consumption {
                CandidateConsumption::PredicateContinuation {
                    source_positions, ..
                } => {
                    let visible_connective = candidate.consumed.end > candidate.verified.core.end;
                    let token = haystack
                        .get(whole.clone())
                        .and_then(|token| std::str::from_utf8(token).ok());
                    let complete_attached_path = token.is_some_and(|token| {
                        resolver
                            .has_complete_attached_auxiliary_path(token, DEFAULT_LATTICE_NODE_LIMIT)
                    });
                    let source_auxiliary_suffix =
                        resolver.supports_auxiliary_sequence(trailing, DEFAULT_LATTICE_NODE_LIMIT);
                    let unambiguous_contraction = haystack
                        .get(candidate.verified.core.start..candidate.consumed.end)
                        .and_then(|surface| std::str::from_utf8(surface).ok())
                        .is_some_and(|surface| {
                            resolver.has_unambiguous_predicate_surface(surface, source_positions)
                        });
                    let extended_predicate_prefix = token.is_some_and(|token| {
                        resolver.has_extended_predicate_prefix_path(
                            token,
                            candidate.verified.core.len(),
                            source_positions,
                            DEFAULT_LATTICE_NODE_LIMIT,
                        )
                    });
                    let unambiguous_whole_path = token.is_some_and(|token| {
                        resolver.has_unambiguous_attached_auxiliary_whole_path(token)
                    });
                    let resultative_contraction = starts_with_resultative_auxiliary(trailing);
                    let aligned_auxiliary = complete_attached_path
                        && resolver.supports_auxiliary_with_fused_prefinal(
                            trailing,
                            DEFAULT_LATTICE_NODE_LIMIT,
                        )
                        && haystack
                            .get(whole.start..candidate.consumed.end)
                            .and_then(|surface| std::str::from_utf8(surface).ok())
                            .is_some_and(|surface| {
                                resolver
                                    .has_exact_predicate_connective_path(surface, source_positions)
                            });
                    (complete_attached_path
                        && (visible_connective
                            || unambiguous_contraction
                            || resultative_contraction))
                        || (source_auxiliary_suffix
                            && (unambiguous_contraction
                                || aligned_auxiliary
                                || extended_predicate_prefix
                                || unambiguous_whole_path
                                || resultative_contraction))
                }
                _ => false,
            };
        let licensed = (matches!(trailing, "까" | "까요") && has_rule("ending.future-adnominal"))
            || (trailing == "서도" && has_rule("ending.connective-go"))
            || (trailing == "도" && has_rule("ending.connective-neunde"))
            || (trailing.starts_with("잖") && has_rule("ending.past"))
            || (trailing.starts_with(['아', '어', '여'])
                && has_rule("ending.past")
                && resolver.supports_ending_suffix_path(trailing, 0, DEFAULT_LATTICE_NODE_LIMIT))
            || (ADNOMINAL_RULE_IDS.iter().any(|rule| has_rule(rule))
                && ["때", "게"].iter().any(|noun| trailing.starts_with(noun)))
            || source_attached_auxiliary
            || source_predicate_continuation;
        licensed.then_some(candidate.consumed.start..whole.end)
    }

    fn accepts_product_copula_frame(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        resolver: &ConstraintResolver,
        patterns: &[kfind_morph::QueryMorphPattern],
    ) -> bool {
        let CandidateConsumption::PredicateContinuation { pos, .. } = branch.consumption else {
            return false;
        };
        if pos != kfind_morph::PredicatePos::Copula
            || !patterns
                .iter()
                .any(|pattern| pattern.fine_pos == DataFinePos::Vcp)
        {
            return false;
        }
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
        if candidate.verified.core.start <= whole.start || candidate.consumed.end != whole.end {
            return false;
        }
        let Some(token) = haystack
            .get(whole.clone())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return false;
        };
        let token = token.nfc().collect::<String>();
        if resolver.has_whole_modifier(&token) {
            return false;
        }
        let Some(host) = haystack
            .get(whole.start..candidate.verified.core.start)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return false;
        };
        let host = host.nfc().collect::<String>();
        resolver.has_complete_nominal_surface(&host)
            || host
                .char_indices()
                .map(|(offset, _)| offset)
                .skip(1)
                .any(|split| {
                    let nominal = &host[..split];
                    let particles = &host[split..];
                    resolver.has_complete_nominal_surface(nominal)
                        && self
                            .particle_verifier
                            .verify_exact(nominal, particles)
                            .is_some()
                })
    }

    fn accepts_lexical_adverb_particle_frame(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        patterns: &[kfind_morph::QueryMorphPattern],
    ) -> bool {
        if !matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        ) || candidate.suffix_rules.is_empty()
            || !patterns.iter().any(|pattern| {
                matches!(pattern.fine_pos, DataFinePos::Mag | DataFinePos::Maj)
                    && matches!(
                        pattern.continuation,
                        kfind_morph::MorphContinuation::NominalParticles
                    )
            })
        {
            return false;
        }
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
        if candidate.verified.core.start != whole.start
            || candidate.anchor != candidate.verified.core
            || candidate.consumed != whole
        {
            return false;
        }
        let boundary = branch.boundary();
        accepts_requirements(
            haystack,
            candidate.verified.core.clone(),
            candidate.consumed.clone(),
            boundary.require_left,
            boundary.require_right,
        )
    }

    fn accepts_pronoun_copula_ending(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        resolver: &ConstraintResolver,
        patterns: &[kfind_morph::QueryMorphPattern],
    ) -> bool {
        if !matches!(
            branch.consumption,
            CandidateConsumption::NominalCopulaEndingChain { .. }
        ) || !patterns.iter().any(|pattern| {
            pattern.fine_pos == DataFinePos::Np
                && matches!(
                    pattern.continuation,
                    kfind_morph::MorphContinuation::NominalCopulaEnding
                )
        }) {
            return false;
        }
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
        if candidate.anchor != candidate.verified.core || candidate.consumed != whole {
            return false;
        }
        let Some(anchor) = haystack
            .get(candidate.anchor.clone())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return false;
        };
        let normalized = anchor.nfc().collect::<String>();
        resolver.has_exact_pronoun_copula_ending_path(&normalized)
            && self.accepts_token_boundary(haystack, &candidate.verified, branch)
    }

    fn accepts_lost_span_copula_ending(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        resolver: &ConstraintResolver,
        patterns: &[kfind_morph::QueryMorphPattern],
    ) -> bool {
        if !patterns
            .iter()
            .any(|pattern| pattern.fine_pos == DataFinePos::Vcp)
        {
            return false;
        }
        let Some(token) = haystack
            .get(candidate.verified.core.clone())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return false;
        };
        let normalized = if is_nfc(token) {
            Cow::Borrowed(token)
        } else {
            Cow::Owned(token.nfc().collect::<String>())
        };
        if !self.accepts_token_boundary(haystack, &candidate.verified, branch) {
            return false;
        }
        resolver.has_exact_lost_span_copula_ending_path(&normalized)
            || (matches!(branch.consumption, CandidateConsumption::FusedCopulaEnding)
                && (fused_copula::accepts_fused_retrospective(&normalized, resolver)
                    || fused_copula::accepts_omitted_copula(
                        &normalized,
                        resolver,
                        || is_sentence_end_after(haystack, candidate.verified.token.end),
                        || {
                            follows_predicate_adnominal(
                                haystack,
                                candidate.verified.token.start,
                                resolver,
                            )
                        },
                        |unit| {
                            context::previous_word(haystack, candidate.verified.token.start)
                                .is_some_and(|previous| {
                                    resolver.has_counted_unit_frame(&previous, unit)
                                })
                        },
                    )))
    }

    fn licensed_nominal_copula_trailing(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        resolver: &ConstraintResolver,
    ) -> Option<Range<usize>> {
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
        if candidate.verified.core.start != whole.start
            || candidate.consumed.end < candidate.verified.core.end
            || candidate.consumed.end >= whole.end
        {
            return None;
        }
        let suffix = haystack
            .get(candidate.consumed.end..whole.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())?;
        let suffix = if is_nfc(suffix) {
            Cow::Borrowed(suffix)
        } else {
            Cow::Owned(suffix.nfc().collect::<String>())
        };
        let token = haystack
            .get(whole.clone())
            .and_then(|bytes| std::str::from_utf8(bytes).ok())?;
        let token = if is_nfc(token) {
            Cow::Borrowed(token)
        } else {
            Cow::Owned(token.nfc().collect::<String>())
        };
        if resolver.has_whole_modifier(&token) {
            return None;
        }
        if kfind_morph::is_omitted_copula_ending(&suffix) {
            if (matches!(suffix.as_ref(), "게" | "네" | "네요")
                && !is_sentence_end_after(haystack, whole.end))
                || (suffix == "게" && resolver.has_exact_nominal_surface(&token))
            {
                return None;
            }
            let host = std::str::from_utf8(
                haystack.get(candidate.consumed.start..candidate.consumed.end)?,
            )
            .ok()?
            .nfc()
            .collect::<String>();
            if resolver.has_exact_action_predicate_surface(&host) {
                return None;
            }
        }
        let preceding = haystack
            .get(candidate.consumed.start..candidate.consumed.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .and_then(|consumed| consumed.nfc().last());
        if !preceding
            .is_some_and(|preceding| verify_copula_surface_after_nominal(preceding, &suffix))
        {
            return None;
        }
        Some(candidate.consumed.start..whole.end)
    }

    fn supports_source_predicate_trailing(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        whole: &Range<usize>,
        trailing: &str,
        branch: &CandidateProgram,
        resolver: &ConstraintResolver,
    ) -> bool {
        let CandidateConsumption::PredicateContinuation {
            continuation,
            pos,
            source_positions,
            ..
        } = branch.consumption
        else {
            return false;
        };
        let declarative_adnominal = trailing.nfc().eq("는".chars())
            && haystack
                .get(candidate.consumed.clone())
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .and_then(|consumed| consumed.nfc().last())
                == Some('다');
        let has_rule = |expected: &str| {
            branch
                .origins
                .iter()
                .flat_map(|origin| &origin.rule_path)
                .chain(&candidate.suffix_rules)
                .any(|rule| rule.as_str() == expected)
        };
        let forbids_ending_auxiliary_particles = [
            "ending.declarative",
            "ending.interrogative-neunya",
            "ending.connective-ni",
            "ending.conditional",
            "ending.prospective-final",
            "ending.hao-final-so",
            "ending.hao-final",
            "ending.hao-final-u",
        ]
        .iter()
        .any(|rule| has_rule(rule));
        let prospective_quotative_topic = has_rule("ending.prospective-quotative");
        let ending_auxiliary_particles = !declarative_adnominal
            && !forbids_ending_auxiliary_particles
            && haystack
                .get(candidate.consumed.clone())
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .is_some_and(|ending| {
                    let ending = ending.nfc().collect::<String>();
                    let trailing = trailing.nfc().collect::<String>();
                    self.particle_verifier
                        .verify_exact(&ending, &trailing)
                        .is_some_and(|matched| {
                            (!prospective_quotative_topic
                                || matched
                                    .rule_path
                                    .first()
                                    .is_some_and(|rule| rule.as_str() == "particle.topic"))
                                && matched.rule_path.first().is_some_and(|rule| {
                                    contains_rule(
                                        &self.plan.predicate_ending_initial_particle_rules,
                                        rule,
                                    )
                                })
                                && matched.rule_path.iter().all(|rule| {
                                    contains_rule(&self.plan.auxiliary_particle_rules, rule)
                                })
                        })
                });
        let has_adnominal_rule = ADNOMINAL_RULE_IDS.iter().any(|rule| has_rule(rule));
        let adnominal_dependent_noun_particle =
            trailing.nfc().next() == Some('지') && has_adnominal_rule;
        let adnominal_interrogative = trailing.nfc().eq("가".chars()) && has_adnominal_rule;
        let source_continuation = matches!(
            continuation,
            kfind_morph::ContinuationState::Past
                | kfind_morph::ContinuationState::Future
                | kfind_morph::ContinuationState::Eu
        ) && !has_adnominal_rule
            && licensed_source_ending_surface(trailing);
        let normalized_trailing = trailing.nfc().collect::<String>();
        let source_ending_trailing = source_continuation
            && resolver.supports_ending_suffix_path(
                &normalized_trailing,
                0,
                DEFAULT_LATTICE_NODE_LIMIT,
            );
        let source_nominalizer_particle_split = source_continuation.then_some(()).and_then(|()| {
            trailing.char_indices().skip(1).find_map(|(split, _)| {
                let ending = trailing.get(..split)?;
                let particles = trailing.get(split..)?;
                (resolver.has_exact_nominalizing_ending_surface(ending)
                    && self
                        .particle_verifier
                        .verify_exact(ending, particles)
                        .is_some())
                .then_some(split)
            })
        });
        let licensed_non_ending_trailing = declarative_adnominal
            || ending_auxiliary_particles
            || adnominal_dependent_noun_particle
            || adnominal_interrogative;
        let source_aligned_compound = source_aligned_compound_predicate_position(
            haystack,
            &candidate.verified.core,
            whole,
            source_positions,
            resolver,
        );
        if !licensed_non_ending_trailing
            && !source_ending_trailing
            && source_nominalizer_particle_split.is_none()
            && !source_aligned_compound
        {
            return false;
        }
        let valid_position = if pos == kfind_morph::PredicatePos::Copula {
            candidate.verified.core.start > whole.start
        } else {
            (continuation != kfind_morph::ContinuationState::Terminal
                || licensed_non_ending_trailing
                || source_ending_trailing
                || source_nominalizer_particle_split.is_some())
                && (candidate.verified.core.start == whole.start || source_aligned_compound)
        };
        if !valid_position
            || (pos != kfind_morph::PredicatePos::Copula
                && !licensed_non_ending_trailing
                && self
                    .particle_verifier
                    .model()
                    .allomorphs
                    .iter()
                    .any(|form| trailing.starts_with(form.surface.as_ref())))
        {
            return false;
        }

        let token = haystack
            .get(candidate.verified.core.start..whole.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok());
        let core = haystack
            .get(candidate.verified.core.clone())
            .and_then(|bytes| std::str::from_utf8(bytes).ok());
        let ending = haystack
            .get(candidate.verified.core.start..candidate.consumed.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok());
        token
            .zip(core)
            .zip(ending)
            .is_some_and(|((token, core), ending)| {
                if is_nfc(token) {
                    if resolver.has_whole_modifier(token) && !adnominal_interrogative {
                        return false;
                    }
                    return if ending_auxiliary_particles {
                        source_positions.iter().any(|source_pos| {
                            resolver.supports_predicate_ending_particle_path(
                                token,
                                core.len(),
                                ending.len(),
                                source_pos,
                                DEFAULT_LATTICE_NODE_LIMIT,
                            )
                        })
                    } else if let Some(split) = source_nominalizer_particle_split {
                        source_positions.iter().any(|source_pos| {
                            resolver.supports_predicate_ending_particle_path(
                                token,
                                core.len(),
                                ending.len() + split,
                                source_pos,
                                DEFAULT_LATTICE_NODE_LIMIT,
                            )
                        })
                    } else if adnominal_dependent_noun_particle {
                        source_positions.iter().any(|source_pos| {
                            resolver.supports_adnominal_dependent_noun_particle_path(
                                token,
                                core.len(),
                                ending.len(),
                                source_pos,
                                DEFAULT_LATTICE_NODE_LIMIT,
                            ) || resolver.has_exact_predicate_ending_path(token, source_pos)
                        })
                    } else {
                        source_positions.iter().any(|source_pos| {
                            resolver.supports_predicate_ending_path(
                                token,
                                core.len(),
                                source_pos,
                                DEFAULT_LATTICE_NODE_LIMIT,
                            )
                        })
                    };
                }
                let normalized = token.nfc().collect::<String>();
                let core_len = core.nfc().map(char::len_utf8).sum();
                let ending_len = ending.nfc().map(char::len_utf8).sum();
                if resolver.has_whole_modifier(&normalized) && !adnominal_interrogative {
                    return false;
                }
                if ending_auxiliary_particles {
                    source_positions.iter().any(|source_pos| {
                        resolver.supports_predicate_ending_particle_path(
                            &normalized,
                            core_len,
                            ending_len,
                            source_pos,
                            DEFAULT_LATTICE_NODE_LIMIT,
                        )
                    })
                } else if let Some(split) = source_nominalizer_particle_split {
                    let nominalizer_len = trailing
                        .get(..split)
                        .map(|surface| surface.nfc().map(char::len_utf8).sum::<usize>())
                        .unwrap_or_default();
                    source_positions.iter().any(|source_pos| {
                        resolver.supports_predicate_ending_particle_path(
                            &normalized,
                            core_len,
                            ending_len + nominalizer_len,
                            source_pos,
                            DEFAULT_LATTICE_NODE_LIMIT,
                        )
                    })
                } else if adnominal_dependent_noun_particle {
                    source_positions.iter().any(|source_pos| {
                        resolver.supports_adnominal_dependent_noun_particle_path(
                            &normalized,
                            core_len,
                            ending_len,
                            source_pos,
                            DEFAULT_LATTICE_NODE_LIMIT,
                        ) || resolver.has_exact_predicate_ending_path(&normalized, source_pos)
                    })
                } else {
                    source_positions.iter().any(|source_pos| {
                        resolver.supports_predicate_ending_path(
                            &normalized,
                            core_len,
                            source_pos,
                            DEFAULT_LATTICE_NODE_LIMIT,
                        )
                    })
                }
            })
    }

    fn has_rejected_structural_suffix(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        consumed: &Range<usize>,
        branch: &CandidateProgram,
        resolver: &ConstraintResolver,
    ) -> bool {
        if !matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
                | CandidateConsumption::PredicateContinuation { .. }
        ) {
            return false;
        }
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
        if consumed.end >= whole.end {
            return false;
        }
        let Some(suffix) = haystack
            .get(consumed.end..whole.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return true;
        };
        let normalized = suffix.nfc().collect::<String>();
        let particle_shaped = self
            .particle_verifier
            .model()
            .allomorphs
            .iter()
            .any(|form| normalized.starts_with(form.surface.as_ref()));
        if particle_shaped {
            return true;
        }
        let CandidateConsumption::PredicateContinuation {
            source_positions,
            nominal_particle_transition,
            ..
        } = branch.consumption
        else {
            return false;
        };
        if nominal_particle_transition {
            return false;
        }
        let trailing = std::str::from_utf8(&haystack[consumed.end..whole.end]).unwrap_or_default();
        trailing.char_indices().skip(1).any(|(offset, _)| {
            let split = consumed.end + offset;
            let remainder = trailing.get(offset..).unwrap_or_default();
            let particle_remainder = self
                .particle_verifier
                .model()
                .allomorphs
                .iter()
                .any(|form| remainder.starts_with(form.surface.as_ref()));
            if !particle_remainder {
                return false;
            }
            let Some(prefix) = haystack
                .get(candidate.verified.core.start..split)
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
            else {
                return false;
            };
            let normalized_prefix = prefix.nfc().collect::<String>();
            let Some(core) = haystack
                .get(candidate.verified.core.clone())
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
            else {
                return false;
            };
            let normalized_core_len = core.nfc().collect::<String>().len();
            source_positions.iter().any(|source_pos| {
                resolver.supports_predicate_ending_path(
                    &normalized_prefix,
                    normalized_core_len,
                    source_pos,
                    DEFAULT_LATTICE_NODE_LIMIT,
                )
            })
        })
    }

    fn has_terminal_particle_conflict(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        whole: &Range<usize>,
    ) -> bool {
        let model = self.particle_verifier.model();
        let is_terminal = |rule_id: &RuleId| {
            model
                .transitions
                .iter()
                .find(|transition| &transition.rule_id == rule_id)
                .is_some_and(|transition| transition.next.is_empty())
        };
        if candidate.suffix_rules.iter().any(is_terminal) {
            return true;
        }
        let Some(trailing) = haystack
            .get(candidate.consumed.end..whole.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return false;
        };
        let trailing = trailing.nfc().collect::<String>();
        model
            .allomorphs
            .iter()
            .any(|form| is_terminal(&form.rule_id) && trailing.starts_with(form.surface.as_ref()))
    }

    fn accepts_token_boundary(
        &self,
        haystack: &[u8],
        candidate: &VerifiedSpan,
        branch: &CandidateProgram,
    ) -> bool {
        let boundary = branch.boundary();
        accepts_requirements(
            haystack,
            candidate.core.clone(),
            candidate.token.clone(),
            boundary.require_left,
            boundary.require_right,
        )
    }

    fn accepts_direct_particle(
        &self,
        haystack: &[u8],
        anchor_span: &Range<usize>,
        rule_id: &RuleId,
    ) -> bool {
        let Some(anchor_bytes) = haystack.get(anchor_span.clone()) else {
            return false;
        };
        let Ok(anchor) = std::str::from_utf8(anchor_bytes) else {
            return false;
        };
        let Some(left_bytes) = haystack.get(..anchor_span.start) else {
            return false;
        };
        let normalized_anchor = anchor.nfc().collect::<String>();
        let normalized_left = valid_utf8_suffix(left_bytes).nfc().collect::<String>();
        let Some(previous) = normalized_left.chars().next_back() else {
            return false;
        };
        if !crate::is_token_character(previous) {
            return false;
        }
        let normalized_context = format!("{normalized_left}{normalized_anchor}");
        if self
            .particle_verifier
            .model()
            .allomorphs
            .iter()
            .any(|form| {
                &form.rule_id == rule_id
                    && form.surface.len() > normalized_anchor.len()
                    && form.surface.ends_with(&normalized_anchor)
                    && normalized_context.ends_with(form.surface.as_ref())
            })
        {
            return false;
        }

        self.particle_verifier
            .model()
            .allomorphs
            .iter()
            .any(|form| {
                form.surface.as_ref() == normalized_anchor
                    && &form.rule_id == rule_id
                    && form.condition.accepts(previous)
            })
    }
}

fn rejects_background_conjunction(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    resolver: &ConstraintResolver,
) -> bool {
    let consumed = &candidate.consumed;
    haystack.get(consumed.end) == Some(&b',')
        && haystack[..consumed.start]
            .iter()
            .rev()
            .copied()
            .find(|byte| !matches!(byte, b' ' | b'\t'))
            .is_none_or(|byte| matches!(byte, b'.' | b'!' | b'?' | b'\n' | b'\r'))
        && std::str::from_utf8(&haystack[consumed.clone()]).is_ok_and(|token| {
            resolver.has_exact_conjunctive_adverb(&token.nfc().collect::<String>())
        })
}

fn nominal_copula_surface_at(haystack: &[u8], start: usize, end: usize) -> bool {
    haystack
        .get(start..end)
        .and_then(|suffix| std::str::from_utf8(suffix).ok())
        .and_then(|suffix| suffix.nfc().next())
        .is_some_and(|character| matches!(character, '이' | '입'))
}

fn contracted_copula_surface_follows(
    haystack: &[u8],
    start: usize,
    end: usize,
    preserve_numeral_da: bool,
) -> bool {
    haystack
        .get(start..end)
        .and_then(|suffix| std::str::from_utf8(suffix).ok())
        .and_then(|suffix| suffix.nfc().next())
        .is_some_and(|character| {
            matches!(character, '였' | '여') || (character == '다' && !preserve_numeral_da)
        })
}

fn follows_predicate_adnominal(haystack: &[u8], at: usize, resolver: &ConstraintResolver) -> bool {
    context::previous_word(haystack, at)
        .is_some_and(|previous| resolver.has_predicate_adnominal_surface(&previous))
}

fn has_counted_unit_copula(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let (CandidateConsumption::PredicateContinuation { pos, .. }
    | CandidateConsumption::StructuralPredicateEnding { pos, .. }) = branch.consumption
    else {
        return false;
    };
    let core = &candidate.verified.core;
    if pos == kfind_morph::PredicatePos::Copula
        || !has_jiyo_suffix(haystack, core.end..candidate.consumed.end)
        || !is_boundary_before(haystack, core.start)
    {
        return false;
    }
    let Some(previous) = context::previous_word(haystack, core.start) else {
        return false;
    };
    haystack
        .get(core.clone())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .is_some_and(|unit| {
            resolver.has_counted_unit_frame(&previous, &unit.nfc().collect::<String>())
        })
}

fn has_jiyo_suffix(haystack: &[u8], suffix: Range<usize>) -> bool {
    haystack
        .get(suffix)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .is_some_and(|suffix| {
            matches!(suffix, "죠" | "지요")
                || (suffix.starts_with('ᄌ')
                    && (suffix.nfc().eq("죠".chars()) || suffix.nfc().eq("지요".chars())))
        })
}

fn has_topic_before_independent_predicate(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let span = &candidate.consumed;
    let end = match candidate.suffix_rules.as_slice() {
        [prefix @ .., last]
            if last.as_str() == "ending.present-adnominal"
                && prefix
                    .iter()
                    .any(|rule| rule.as_str() == "ending.auxiliary-jida") =>
        {
            span.end
        }
        [] if branch.origins.iter().all(|origin| {
            origin
                .rule_path
                .last()
                .is_some_and(|rule| rule.as_str() == "ending.connective-ji")
        }) =>
        {
            let token = surrounding_token_span(haystack, span.clone());
            if !context::normalized_token(haystack, span.end..token.end)
                .is_some_and(|suffix| suffix == "는")
            {
                return false;
            }
            token.end
        }
        _ => return false,
    };
    is_boundary_before(haystack, span.start)
        && is_boundary_after(haystack, end)
        && context::normalized_token(haystack, span.start..end).is_some_and(|token| {
            token
                .strip_suffix('는')
                .is_some_and(|host| resolver.has_exact_nominal_surface(host))
        })
        && context::next_word(haystack, end)
            .is_some_and(|next| resolver.has_independent_final_predicate(&next))
}

fn has_genitive_nominal_interrogative_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let rule = candidate.suffix_rules.last().or_else(|| {
        let (first, rest) = branch.origins.split_first()?;
        let rule = first.rule_path.last()?;
        rest.iter()
            .all(|origin| origin.rule_path.last() == Some(rule))
            .then_some(rule)
    });
    let Some(rule) = rule.map(RuleId::as_str) else {
        return false;
    };
    let span = &candidate.consumed;
    let end = if rule == "ending.interrogative-nga" {
        span.end
    } else if ADNOMINAL_RULE_IDS.contains(&rule) {
        if !haystack.get(span.end..).is_some_and(|tail| {
            tail.starts_with("가".as_bytes()) || tail.starts_with("ᄀ".as_bytes())
        }) {
            return false;
        }
        let token = surrounding_token_span(haystack, span.clone());
        if !context::normalized_token(haystack, span.end..token.end)
            .is_some_and(|suffix| suffix == "가")
        {
            return false;
        }
        token.end
    } else {
        return false;
    };
    is_boundary_before(haystack, span.start)
        && is_boundary_after(haystack, end)
        && context::normalized_token(haystack, span.start..end)
            .is_some_and(|token| resolver.has_exact_nominal_surface(&token))
        && context::previous_word(haystack, span.start)
            .is_some_and(|previous| resolver.has_nominal_genitive_surface(&previous))
        && context::next_word(haystack, end)
            .is_some_and(|next| resolver.has_complete_nominal_surface(&next))
}

fn has_transitive_jiyo_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    use kfind_morph::PredicatePos;
    if !matches!(
        branch.consumption,
        CandidateConsumption::PredicateContinuation {
            pos: PredicatePos::Adjective,
            ..
        } | CandidateConsumption::StructuralPredicateEnding {
            pos: PredicatePos::Adjective,
            ..
        }
    ) {
        return false;
    }
    let core = &candidate.verified.core;
    let Some(stem) = context::normalized_token(haystack, core.clone()) else {
        return false;
    };
    // The direct object of these transitive senses denotes a text or a physical target.
    let objects: &[&str] = match stem.as_ref() {
        "쓰" => &["글", "동화", "소설", "수필", "편지"],
        "차" => &["공", "축구공", "제기", "깡통"],
        _ => return false,
    };
    let whole = surrounding_token_span(haystack, core.clone());
    if whole.start != core.start || !has_jiyo_suffix(haystack, core.end..whole.end) {
        return false;
    }
    let Some((start, previous)) = context::previous_word_with_start(haystack, core.start) else {
        return false;
    };
    let object = if stem == "차" && previous == "힘껏" {
        let Some(object) = context::previous_word(haystack, start) else {
            return false;
        };
        object
    } else {
        previous
    };
    object
        .strip_suffix(['을', '를'])
        .is_some_and(|noun| objects.contains(&noun))
        && resolver.has_object_marked_nominal(&object)
}

fn has_prayer_object_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    use kfind_morph::{ContinuationState, PredicatePos};
    let core = &candidate.verified.core;
    let consumed = &candidate.consumed;
    if !matches!(
        branch.consumption,
        CandidateConsumption::PredicateContinuation {
            pos: PredicatePos::Verb,
            ..
        } | CandidateConsumption::StructuralPredicateEnding {
            pos: PredicatePos::Verb,
            ..
        }
    ) || branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            atom.analyses
                .get(usize::from(origin.analysis_index))
                .is_some_and(|analysis| analysis.lemma.as_ref() == "비다")
        })
        || !is_boundary_before(haystack, core.start)
        || !is_boundary_after(haystack, consumed.end)
        || context::normalized_token(haystack, core.clone()).as_deref() != Some("비")
    {
        return false;
    }
    let Some(previous) = context::previous_word(haystack, core.start) else {
        return false;
    };
    // An accusative-marked duration can also precede the intransitive verb.
    if !previous.strip_suffix(['을', '를']).is_some_and(|noun| {
        matches!(
            noun,
            "안녕" | "평안" | "만수무강" | "건강" | "행복" | "소원" | "용서"
        )
    }) || !resolver.has_object_marked_nominal(&previous)
    {
        return false;
    }
    let Some(ending) = context::normalized_token(haystack, core.end..consumed.end) else {
        return false;
    };
    // The structural fallback carries a generic rule ID. Verify its ending too.
    [ContinuationState::StemEnding, ContinuationState::Eu]
        .into_iter()
        .any(|state| {
            verify_predicate_continuation(state, PredicatePos::Verb, "비", &ending).is_some_and(
                |matched| {
                    matched.consumed_bytes == ending.len()
                        && matched
                            .rule_path
                            .last()
                            .is_some_and(|rule| rule.as_str() == "ending.archaic-naida")
                },
            )
        })
}

fn has_nominal_frame_for_volitional(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let anchor_rule = branch
        .origins
        .first()
        .and_then(|origin| origin.rule_path.last());
    let Some(rule) = candidate.suffix_rules.last().or(anchor_rule) else {
        return false;
    };
    if !matches!(
        rule.as_str(),
        "ending.volitional-mse" | "ending.nominalizer"
    ) {
        return false;
    }
    let consumed = &candidate.consumed;
    if !is_boundary_before(haystack, consumed.start) {
        return false;
    }
    let whole = if rule.as_str() == "ending.volitional-mse" {
        if !is_boundary_after(haystack, consumed.end) {
            return false;
        }
        consumed.clone()
    } else {
        // The same surface may also be reached by consuming only the nominalizer.
        if !haystack.get(consumed.end..).is_some_and(|suffix| {
            suffix.starts_with("세".as_bytes()) || suffix.starts_with("ᄉ".as_bytes())
        }) {
            return false;
        }
        let whole = surrounding_token_span(haystack, consumed.clone());
        if !haystack
            .get(consumed.end..whole.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .is_some_and(|suffix| suffix.nfc().eq("세".chars()))
        {
            return false;
        }
        whole
    };
    (candidate.suffix_rules.last().is_some()
        || branch
            .origins
            .iter()
            .all(|origin| origin.rule_path.last() == Some(rule)))
        && (context::next_word(haystack, whole.end).as_deref() == Some("때문에")
            || context::previous_word(haystack, whole.start).is_some_and(|previous| {
                resolver.has_predicate_adnominal_surface(&previous)
                    || resolver.has_nominal_genitive_surface(&previous)
            }))
        && haystack
            .get(whole)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .is_some_and(|token| {
                resolver.has_exact_nominal_surface(&token.nfc().collect::<String>())
            })
}

fn has_nominal_quotative_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let (CandidateConsumption::PredicateContinuation { pos, .. }
    | CandidateConsumption::StructuralPredicateEnding { pos, .. }) = branch.consumption
    else {
        return false;
    };
    let core = &candidate.verified.core;
    if pos == kfind_morph::PredicatePos::Copula {
        return false;
    }
    let Some(suffix) = haystack
        .get(core.end..candidate.consumed.end)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    else {
        return false;
    };
    if !suffix.starts_with(['라', 'ᄅ'])
        || !["라고", "라고요", "라는"]
            .iter()
            .any(|ending| suffix.nfc().eq(ending.chars()))
        || !is_boundary_before(haystack, core.start)
    {
        return false;
    }
    haystack
        .get(core.clone())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .is_some_and(|nominal| {
            resolver.has_exact_nominal_surface(&nominal.nfc().collect::<String>())
        })
        && follows_predicate_adnominal(haystack, core.start, resolver)
}

fn has_competing_hao_connective(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let CandidateConsumption::PredicateContinuation {
        continuation,
        pos,
        source_positions,
        ..
    } = branch.consumption
    else {
        return false;
    };
    let respectful = candidate
        .suffix_rules
        .last()
        .is_some_and(|rule| rule.as_str() == "ending.hao-respectful-connective");
    let contracted_aoeo = continuation == kfind_morph::ContinuationState::AOrEo
        && candidate.consumed == candidate.verified.core
        && ["오", "ᄋ"]
            .iter()
            .any(|syllable| haystack[candidate.consumed.end..].starts_with(syllable.as_bytes()));
    if !respectful && !contracted_aoeo {
        return false;
    }
    let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
    if candidate.verified.core.start != whole.start {
        return false;
    }
    if !respectful
        && !haystack
            .get(candidate.consumed.end..whole.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
            .is_some_and(|tail| tail.nfc().eq("오면".chars()) || tail.nfc().eq("오니".chars()))
    {
        return false;
    }
    let Some(token) = haystack
        .get(whole)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    else {
        return false;
    };
    let normalized = token.nfc().collect::<String>();
    let core_len = token[..candidate.verified.core.len()]
        .nfc()
        .map(char::len_utf8)
        .sum();
    resolver.has_extended_predicate_prefix_path(
        &normalized,
        core_len,
        source_positions | kfind_morph::PredicatePosSet::one(pos),
        DEFAULT_LATTICE_NODE_LIMIT,
    )
}

fn has_conflicting_whole_predicate(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let CandidateConsumption::PredicateContinuation {
        source_positions, ..
    } = branch.consumption
    else {
        return false;
    };
    let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
    if whole == candidate.verified.core {
        return false;
    }
    let internal_core = whole.start < candidate.verified.core.start;
    let generated_auxiliary = candidate.suffix_rules.iter().any(|rule| {
        matches!(
            rule.as_str(),
            "ending.auxiliary-gada" | "ending.auxiliary-jida" | "ending.auxiliary-boda"
        )
    });
    let extends_non_internal_core = !internal_core
        && (candidate.anchor != candidate.verified.core
            || candidate.consumed != candidate.verified.core);
    if extends_non_internal_core && !generated_auxiliary {
        return false;
    }
    let Some(token) = haystack
        .get(whole.clone())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    else {
        return false;
    };
    let Some(core) = haystack
        .get(candidate.verified.core.clone())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    else {
        return false;
    };
    let normalized_token = token.nfc().collect::<String>();
    if extends_non_internal_core && resolver.has_attached_auxiliary_whole_path(&normalized_token) {
        return false;
    }
    if extends_non_internal_core
        && candidate.consumed == whole
        && candidate
            .suffix_rules
            .iter()
            .any(|rule| rule.as_str() == "ending.auxiliary-jida")
        && branch
            .origins
            .iter()
            .flat_map(|origin| &origin.rule_path)
            .any(|rule| rule.as_str() == "contraction.u-eo")
        && resolver
            .has_exact_predicate_connective_path(&core.nfc().collect::<String>(), source_positions)
    {
        return false;
    }
    let Some(prefix) = haystack
        .get(whole.start..candidate.verified.core.start)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    else {
        return false;
    };
    let normalized_prefix = prefix.nfc().collect::<String>();
    if internal_core && normalized_prefix.ends_with(['아', '어', '여']) {
        return false;
    }
    let normalized_core_start = normalized_prefix.len();
    let normalized_core_end = normalized_core_start + core.nfc().collect::<String>().len();
    if source_aligned_compound_predicate_position(
        haystack,
        &candidate.verified.core,
        &whole,
        source_positions,
        resolver,
    ) {
        return false;
    }
    let attached_auxiliary = internal_core
        && branch.structural_patterns().iter().any(|pattern| {
            pattern.fine_pos == kfind_data::DataFinePos::Vx && pattern.lexical_form.as_ref() == "지"
        })
        && resolver.has_attached_auxiliary_whole_path(&normalized_token);
    if attached_auxiliary {
        return false;
    }
    source_positions.iter().any(|source_pos| {
        resolver.whole_predicate_conflicts_at(
            &normalized_token,
            normalized_core_start..normalized_core_end,
            source_pos,
        )
    })
}

fn source_aligned_compound_predicate_position(
    haystack: &[u8],
    core: &Range<usize>,
    whole: &Range<usize>,
    source_positions: kfind_morph::PredicatePosSet,
    resolver: &ConstraintResolver,
) -> bool {
    if core.start <= whole.start || core.end > whole.end {
        return false;
    }
    let Some(token) = haystack
        .get(whole.clone())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    else {
        return false;
    };
    let Some(prefix) = haystack
        .get(whole.start..core.start)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    else {
        return false;
    };
    let Some(core) = haystack
        .get(core.clone())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    else {
        return false;
    };
    let normalized_token = token.nfc().collect::<String>();
    let normalized_core_start = prefix.nfc().map(char::len_utf8).sum::<usize>();
    let normalized_core_end = normalized_core_start + core.nfc().map(char::len_utf8).sum::<usize>();
    resolver.has_source_aligned_compound_predicate_component(
        &normalized_token,
        normalized_core_start..normalized_core_end,
        source_positions,
        DEFAULT_LATTICE_NODE_LIMIT,
    )
}

fn stem_accepts_ending(
    pos: kfind_morph::PredicatePos,
    flags: kfind_morph::PredicateFlags,
    base_state: kfind_morph::ContinuationState,
    class: PredicateStemClass,
    anchor: &str,
    suffix: &str,
) -> bool {
    if suffix.is_empty() {
        return false;
    }
    let Some(first) = suffix.chars().next() else {
        return false;
    };
    if kfind_morph::decompose_syllable(first).is_some_and(|syllable| syllable.choseong == 11) {
        return false;
    }
    if matches!(suffix.chars().next(), Some('이' | '인' | '일' | '임')) {
        return false;
    }
    if !pos.is_action()
        && ["거라", "고자", "느냐", "너라", "려", "자"]
            .iter()
            .any(|prefix| suffix.starts_with(prefix))
    {
        return false;
    }
    let declarative_continuation = ["다면", "다며", "다면서", "다니", "다는데", "다지"]
        .iter()
        .any(|prefix| suffix.starts_with(prefix));
    if ((pos.is_action()
        && !matches!(
            base_state,
            kfind_morph::ContinuationState::Past | kfind_morph::ContinuationState::Future
        ))
        || flags.contains(kfind_morph::PredicateFlags::NO_DECLARATIVE_CONTINUATION))
        && declarative_continuation
    {
        return false;
    }
    if suffix.starts_with("너라") && !anchor.ends_with('오') {
        return false;
    }
    if class != PredicateStemClass::Consonant
        && matches!(suffix, "습니다" | "습니까" | "습디까" | "습디다")
    {
        return false;
    }
    if class == PredicateStemClass::Consonant
        && [
            "니", "니까", "니깐", "라", "며", "면서", "면", "려", "리", "세", "셔", "시", "십",
        ]
        .iter()
        .any(|prefix| suffix.starts_with(prefix))
    {
        return false;
    }
    if class == PredicateStemClass::Rieul
        && ["느", "는", "니", "세", "셔", "시", "십"]
            .iter()
            .any(|prefix| suffix.starts_with(prefix))
    {
        return false;
    }
    true
}

fn normalized_consumption_text<'a>(
    anchor: &'a str,
    following: &'a str,
) -> (Cow<'a, str>, Cow<'a, str>) {
    if is_nfc(anchor) {
        return (Cow::Borrowed(anchor), Cow::Borrowed(following));
    }
    (
        Cow::Owned(anchor.nfc().collect()),
        Cow::Owned(following.nfc().collect()),
    )
}

fn requires_direct_particle_host(branch: &CandidateProgram) -> bool {
    let boundary = branch.boundary();
    !boundary.require_left && boundary.require_right
}

fn accepts_left_context(
    left_context: &CandidateLeftContext,
    haystack: &[u8],
    anchor_start: usize,
) -> bool {
    match left_context {
        CandidateLeftContext::Any => true,
        CandidateLeftContext::ContractedAfterVowel {
            uncontracted_prefix,
        } => {
            let Some(left_bytes) = haystack.get(..anchor_start) else {
                return false;
            };
            let left = valid_utf8_suffix(left_bytes);
            let normalized_left = left.nfc().collect::<String>();
            let normalized_prefix = uncontracted_prefix.nfc().collect::<String>();
            accepts_contracted_after_vowel(&normalized_left, &normalized_prefix)
        }
    }
}

fn accepts_contracted_after_vowel(left: &str, uncontracted_prefix: &str) -> bool {
    let Some(previous) = left.chars().next_back() else {
        return false;
    };
    if kfind_morph::has_final(previous) {
        return false;
    }

    left.strip_suffix(uncontracted_prefix)
        .and_then(|host| host.chars().next_back())
        .is_none_or(|host_final| !kfind_morph::has_final(host_final))
}

impl Matcher for MorphMatcher {
    type Captures = NoCaptures;
    type Error = NoError;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>, Self::Error> {
        Ok(self
            .find_span_at(haystack, at)
            .map(|span| Match::new(span.start, span.end)))
    }

    fn new_captures(&self) -> Result<Self::Captures, Self::Error> {
        Ok(NoCaptures::new())
    }

    fn line_terminator(&self) -> Option<LineTerminator> {
        self.is_line_local.then(|| LineTerminator::byte(b'\n'))
    }

    fn find_candidate_line(&self, haystack: &[u8]) -> Result<Option<LineMatchKind>, Self::Error> {
        Ok(line_candidate::find(self, haystack))
    }
}

#[derive(Clone, Copy)]
enum MatchMetadata {
    SpanOnly,
    Provenance,
}

fn single_atom_phrase(span: VerifiedSpan) -> PhraseMatch {
    PhraseMatch {
        span: span.token.clone(),
        atoms: vec![span],
    }
}

struct ExecutedCandidate {
    anchor: Range<usize>,
    consumed: Range<usize>,
    suffix_rules: Vec<RuleId>,
    verified: VerifiedSpan,
}

#[derive(Debug, Clone, Copy)]
struct ProgramRef {
    atom_index: usize,
    program_index: usize,
}

type AnchorsAndPrograms<'a> = (Vec<&'a [u8]>, Vec<Box<[ProgramRef]>>, Vec<ProgramRef>);

fn unique_anchors(plan: &QueryPlan) -> AnchorsAndPrograms<'_> {
    let mut anchor_indices = HashMap::<&[u8], usize>::new();
    let mut anchors = Vec::<&[u8]>::new();
    let mut program_lists = Vec::<Vec<ProgramRef>>::new();
    let mut fused_copula = Vec::new();
    for (atom_index, atom) in plan.atoms.iter().enumerate() {
        for (program_index, branch) in atom.programs.iter().enumerate() {
            let program = ProgramRef {
                atom_index,
                program_index,
            };
            if matches!(branch.consumption, CandidateConsumption::FusedCopulaEnding) {
                fused_copula.push(program);
                continue;
            }
            let anchor_index = *anchor_indices.entry(&branch.anchor).or_insert_with(|| {
                let index = anchors.len();
                anchors.push(&branch.anchor);
                program_lists.push(Vec::new());
                index
            });
            program_lists[anchor_index].push(program);
        }
    }
    (
        anchors,
        program_lists
            .into_iter()
            .map(Vec::into_boxed_slice)
            .collect(),
        fused_copula,
    )
}

fn mapped_core(
    anchor_span: &Range<usize>,
    mapping: CoreMapping,
    anchor: &str,
) -> Option<Range<usize>> {
    let length = match mapping {
        CoreMapping::WholeAnchor => anchor.len(),
        CoreMapping::PrefixBytes(length) => length,
    };
    if length == 0 || length > anchor.len() || !anchor.is_char_boundary(length) {
        return None;
    }
    Some(anchor_span.start..anchor_span.start + length)
}

fn valid_utf8_prefix(bytes: &[u8]) -> &str {
    let mut end = bytes.len().min(MAX_CONSUMPTION_BYTES);
    while end < bytes.len() && end > 0 && bytes[end] & 0b1100_0000 == 0b1000_0000 {
        end -= 1;
    }
    match std::str::from_utf8(&bytes[..end]) {
        Ok(text) => text,
        Err(error) => std::str::from_utf8(&bytes[..error.valid_up_to()])
            .expect("from_utf8 valid_up_to is always valid UTF-8"),
    }
}

fn phrase_join_text(bytes: &[u8]) -> Cow<'_, str> {
    if let Ok(text) = std::str::from_utf8(bytes) {
        return Cow::Borrowed(text);
    }

    let mut sanitized = bytes.to_vec();
    let mut offset = 0;
    while let Err(error) = std::str::from_utf8(&sanitized[offset..]) {
        let invalid_start = offset + error.valid_up_to();
        let invalid_len = error
            .error_len()
            .unwrap_or_else(|| sanitized.len().saturating_sub(invalid_start))
            .max(1);
        let invalid_end = invalid_start
            .saturating_add(invalid_len)
            .min(sanitized.len());
        sanitized[invalid_start..invalid_end].fill(b'\n');
        offset = invalid_end;
    }
    Cow::Owned(
        String::from_utf8(sanitized)
            .expect("replacing every invalid byte with ASCII yields valid UTF-8"),
    )
}

fn valid_utf8_suffix(bytes: &[u8]) -> &str {
    let mut remaining = &bytes[bytes.len().saturating_sub(MAX_CONSUMPTION_BYTES)..];
    loop {
        match std::str::from_utf8(remaining) {
            Ok(text) => return text,
            Err(error) => {
                let invalid_len = error
                    .error_len()
                    .unwrap_or_else(|| remaining.len().saturating_sub(error.valid_up_to()));
                let skip = error.valid_up_to().saturating_add(invalid_len);
                if skip >= remaining.len() {
                    return "";
                }
                remaining = &remaining[skip..];
            }
        }
    }
}

fn map_normalized_prefix(
    original: &str,
    normalized: &str,
    normalized_bytes: usize,
) -> Option<usize> {
    if normalized_bytes == 0 {
        return Some(0);
    }
    let expected = normalized.get(..normalized_bytes)?;
    original
        .char_indices()
        .map(|(start, character)| start + character.len_utf8())
        .find(|&end| original[..end].nfc().eq(expected.chars()))
}

fn is_utf8_boundary(bytes: &[u8], at: usize) -> bool {
    at <= bytes.len() && (at == 0 || at == bytes.len() || bytes[at] & 0b1100_0000 != 0b1000_0000)
}

fn extend_origins(origins: &[Origin], suffix_rules: &[RuleId]) -> Vec<Origin> {
    let mut extended = origins
        .iter()
        .map(|origin| {
            let mut rule_path = origin.rule_path.clone();
            rule_path.extend_from_slice(suffix_rules);
            Origin {
                analysis_index: origin.analysis_index,
                rule_path,
            }
        })
        .collect::<Vec<_>>();
    extended.sort();
    extended.dedup();
    extended
}

fn merge_best_span(best: &mut Option<VerifiedSpan>, candidate: VerifiedSpan) {
    let Some(current) = best else {
        *best = Some(candidate);
        return;
    };
    if same_span(current, &candidate) {
        merge_origins(&mut current.origins, candidate.origins);
    } else if is_better_span(&candidate, current) {
        *current = candidate;
    }
}

fn is_better_span(candidate: &VerifiedSpan, current: &VerifiedSpan) -> bool {
    (
        candidate.token.start,
        std::cmp::Reverse(candidate.token.end),
        candidate.core.start,
        std::cmp::Reverse(candidate.core.end),
    ) < (
        current.token.start,
        std::cmp::Reverse(current.token.end),
        current.core.start,
        std::cmp::Reverse(current.core.end),
    )
}

fn same_span(left: &VerifiedSpan, right: &VerifiedSpan) -> bool {
    left.core == right.core && left.token == right.token
}

fn contains_rule(rules: &[RuleId], rule: &RuleId) -> bool {
    rules
        .binary_search_by_key(&rule.as_str(), |known| known.as_str())
        .is_ok()
}

fn licensed_source_ending_surface(surface: &str) -> bool {
    const EU_INITIAL_ENDINGS: &[&str] = &[
        "으니까는",
        "으니까",
        "으니깐",
        "으니",
        "으면서",
        "으면",
        "으며",
        "으나",
        "으므로",
        "으려고",
        "으려는",
        "으리라고",
        "으리라",
        "으되",
        "을",
        "음",
    ];

    !surface.starts_with('으')
        || EU_INITIAL_ENDINGS
            .iter()
            .any(|ending| surface.starts_with(ending))
}

fn starts_with_resultative_auxiliary(surface: &str) -> bool {
    surface
        .chars()
        .next()
        .is_some_and(|first| matches!(first, '지' | '졌' | '진' | '질' | '져' | '집' | '짐'))
}

fn auxiliary_nominal_host(rule_path: &[RuleId]) -> Option<&'static str> {
    match rule_path {
        [auxiliary, nominalizer] => match (auxiliary.as_str(), nominalizer.as_str()) {
            ("ending.auxiliary-jida" | "ending.auxiliary-boda", "ending.nominalizer-gi") => {
                Some("기")
            }
            ("ending.auxiliary-jida", "ending.nominalizer") => Some("짐"),
            ("ending.auxiliary-boda", "ending.nominalizer") => Some("봄"),
            _ => None,
        },
        _ => None,
    }
}

fn merge_origins(origins: &mut Vec<Origin>, additional: Vec<Origin>) {
    origins.extend(additional);
    origins.sort();
    origins.dedup();
}

#[derive(Debug)]
pub enum MorphMatcherBuildError {
    EmptyPlan,
    EmptyAtom { atom_index: usize },
    ComponentResourceRequired,
    Anchor(AnchorBuildError),
}

impl Display for MorphMatcherBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPlan => {
                formatter.write_str("a morphology matcher requires at least one atom")
            }
            Self::EmptyAtom { atom_index } => {
                write!(formatter, "query atom {atom_index} has no search programs")
            }
            Self::ComponentResourceRequired => {
                formatter.write_str("component resource is required for this query plan")
            }
            Self::Anchor(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for MorphMatcherBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Anchor(error) => Some(error),
            Self::EmptyPlan | Self::EmptyAtom { .. } | Self::ComponentResourceRequired => None,
        }
    }
}

impl From<AnchorBuildError> for MorphMatcherBuildError {
    fn from(error: AnchorBuildError) -> Self {
        Self::Anchor(error)
    }
}

#[cfg(test)]
mod tests;
