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
    AnalysisSource, AtomPlan, BoundaryPolicy, CandidateConsumption, CandidateDecision,
    CandidateLeftContext, CandidateProgram, CoreMapping, Origin, PhraseMatch, QueryComposition,
    QueryPlan, RoutedMatch, VerifiedSpan,
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
    anchor_programs: AnchorPrograms,
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

        let (anchors, anchor_programs, fused_copula_anchor) = unique_anchors(&plan);
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
            for branch_ref in self.anchor_programs.get(hit.anchor_index) {
                let atom = &self.plan.atoms[branch_ref.atom_index];
                let branch = &atom.programs[branch_ref.program_index];
                let Some(mut candidate) = self.execute_program_without_decision(
                    haystack,
                    &hit,
                    branch,
                    MatchMetadata::SpanOnly,
                ) else {
                    continue;
                };
                if !self.accepts_program(
                    haystack,
                    &mut candidate,
                    branch,
                    atom,
                    &mut structural_cache,
                ) {
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
            for branch_ref in self.anchor_programs.get(hit.anchor_index) {
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
        let mut candidate =
            self.execute_program_without_decision(haystack, hit, branch, metadata)?;
        if !self.accepts_program(haystack, &mut candidate, branch, atom, structural_cache) {
            return None;
        }
        let mut verified = candidate.verified;
        if matches!(branch.consumption, CandidateConsumption::FusedCopulaEnding) {
            let digits = haystack[hit.span.clone()]
                .iter()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            verified.core.start += digits;
            verified.token.start += digits;
        }
        Some(verified)
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
                let mut normalized_consumed = matched.consumed_bytes;
                let mut rule_path = matched.rule_path;
                if consumption_following[normalized_consumed..]
                    .strip_prefix('도')
                    .is_some_and(|tail| !tail.starts_with(crate::is_token_character))
                    && rule_path.last().map_or_else(
                        || {
                            !branch.origins.is_empty()
                                && branch.origins.iter().all(|origin| {
                                    origin.rule_path.last().is_some_and(|rule| {
                                        rule.as_str() == "ending.simultaneous-myeonseo"
                                    })
                                })
                        },
                        |rule| rule.as_str() == "ending.simultaneous-myeonseo",
                    )
                {
                    normalized_consumed += '도'.len_utf8();
                    rule_path.push(RuleId::from("particle.additive"));
                }
                if !branch.consumption.allows_rule_path(&rule_path) {
                    return None;
                }
                let remaining = &consumption_following[normalized_consumed..];
                if let Some(tail) = remaining.strip_prefix("부터")
                    && rule_path.last().map_or_else(
                        || {
                            !branch.origins.is_empty()
                                && branch.origins.iter().all(|origin| {
                                    origin.rule_path.last().is_some_and(|rule| {
                                        rule.as_str() == "ending.simultaneous-myeonseo"
                                    })
                                })
                        },
                        |rule| rule.as_str() == "ending.simultaneous-myeonseo",
                    )
                {
                    let end = tail
                        .find(|character| !crate::is_token_character(character))
                        .unwrap_or(tail.len());
                    let ending = &tail[..end];
                    if matches!(ending, "" | "다" | "이다") {
                        normalized_consumed += "부터".len() + end;
                        rule_path.push(RuleId::from("particle.from"));
                        if !ending.is_empty() {
                            rule_path.push(RuleId::from("ending.final-da"));
                        }
                    }
                }
                if let Some(host) = continuation_nominal_host(
                    &rule_path,
                    &consumption_following[..normalized_consumed],
                )
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
                if matches!(
                    rule_path.as_slice(),
                    [.., connective, particle]
                        if connective.as_str() == "ending.connective-ji"
                            && matches!(particle.as_str(), "particle.topic" | "particle.object" | "particle.additive")
                ) {
                    let end = hit.span.end + consumed_bytes;
                    if !context::next_word(haystack, end).is_some_and(|next| {
                        has_negative_auxiliary(&next, *pos, self.constraint_resolver.as_ref())
                    }) {
                        return None;
                    }
                }
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
            CandidateConsumption::NominalParticleChain {
                allowed_rule_ids, ..
            }
            | CandidateConsumption::NominalCopulaEndingChain {
                allowed_rule_ids, ..
            } => 'nominal: {
                let following = valid_utf8_prefix(&haystack[hit.span.end..]);
                let (consumption_anchor, consumption_following) =
                    normalized_consumption_text(anchor, following);
                let adverb_host = *allowed_rule_ids == self.plan.auxiliary_particle_rules;
                if adverb_host
                    && branch.origins.iter().any(|origin| {
                        origin
                            .rule_path
                            .iter()
                            .any(|rule| rule.as_str() == "structural.adverb-copula-nieun")
                    })
                {
                    if !is_boundary_before(haystack, hit.span.start) {
                        return None;
                    }
                    let tail = consumption_following
                        .split(|character: char| !crate::is_token_character(character))
                        .next()
                        .filter(|tail| !tail.is_empty())?;
                    let host = kfind_morph::drop_final(consumption_anchor.chars().next_back()?)?;
                    if !verify_copula_surface_after_nominal(host, &format!("인{tail}")) {
                        return None;
                    }
                    let consumed_bytes = match &consumption_following {
                        Cow::Borrowed(_) => tail.len(),
                        Cow::Owned(normalized) => {
                            map_normalized_prefix(following, normalized, tail.len())?
                        }
                    };
                    break 'nominal (
                        consumed_bytes,
                        vec![RuleId::from("structural.adverb-copula")],
                    );
                }
                if adverb_host
                    && consumption_following.starts_with(['이', '인', '일', '임', '입', '였'])
                    && is_boundary_before(haystack, hit.span.start)
                    && let Some(suffix) = consumption_following
                        .split(|character: char| !crate::is_token_character(character))
                        .next()
                    && consumption_anchor
                        .chars()
                        .next_back()
                        .is_some_and(|host| verify_copula_surface_after_nominal(host, suffix))
                {
                    let consumed_bytes = match &consumption_following {
                        Cow::Borrowed(_) => suffix.len(),
                        Cow::Owned(normalized) => {
                            map_normalized_prefix(following, normalized, suffix.len())?
                        }
                    };
                    let token_end = hit.span.end.checked_add(consumed_bytes)?;
                    if !self.constraint_resolver.as_ref().is_some_and(|resolver| {
                        context::normalized_token(haystack, hit.span.start..token_end)
                            .is_some_and(|word| resolver.has_whole_modifier(&word))
                    }) {
                        break 'nominal (
                            consumed_bytes,
                            vec![RuleId::from("structural.adverb-copula")],
                        );
                    }
                }
                let mut matched = if adverb_host {
                    // A trailing jamo can belong to the last syllable of the adverb.
                    if matches!(&consumption_anchor, Cow::Owned(_))
                        && consumption_anchor
                            .chars()
                            .next_back()
                            .zip(consumption_following.chars().next())
                            .is_some_and(|(last, next)| {
                                unicode_normalization::char::compose(last, next).is_some()
                            })
                    {
                        return None;
                    }
                    self.particle_verifier
                        .verify_non_nominal_prefix(&consumption_anchor, &consumption_following)
                } else {
                    self.particle_verifier
                        .verify_prefix(&consumption_anchor, &consumption_following)
                };
                let mut consumed_name_suffix = false;
                if !adverb_host
                    && matches!(
                        branch.consumption,
                        CandidateConsumption::NominalParticleChain { .. }
                    )
                    && let Some(tail) = consumption_following.strip_prefix('이')
                    && tail.starts_with(crate::is_token_character)
                    && consumption_anchor
                        .chars()
                        .next_back()
                        .is_some_and(|last| kfind_morph::drop_final(last).is_some())
                    && is_boundary_before(haystack, core.start)
                    && let Some(resolver) = self.constraint_resolver.as_ref()
                    && resolver.has_exact_proper_name_surface(&consumption_anchor)
                {
                    let name = format!("{consumption_anchor}이");
                    if !resolver.has_exact_nominal_surface(&name) {
                        let with_suffix = self.particle_verifier.verify_prefix(&name, tail);
                        let consumed = '이'.len_utf8() + with_suffix.consumed_bytes;
                        if consumed > matched.consumed_bytes
                            && with_suffix.consumed_bytes > 0
                            && !tail[with_suffix.consumed_bytes..]
                                .starts_with(crate::is_token_character)
                            && branch.consumption.allows_rule_path(&with_suffix.rule_path)
                        {
                            matched = with_suffix;
                            matched.consumed_bytes = consumed;
                            consumed_name_suffix = true;
                        }
                    }
                }
                if let Some((prefix, trailing)) =
                    consumption_following.split_at_checked(matched.consumed_bytes)
                    && trailing
                        .chars()
                        .next()
                        .is_some_and(crate::is_token_character)
                    && let Some(uncontracted_prefix) =
                        ParticleVerifier::before_contracted_case_topic(prefix, &matched.rule_path)
                {
                    matched = self
                        .particle_verifier
                        .verify_prefix(&consumption_anchor, uncontracted_prefix);
                }
                if matches!(
                    branch.consumption,
                    CandidateConsumption::NominalParticleChain { .. }
                ) && consumption_following.starts_with('서')
                    && matched
                        .rule_path
                        .first()
                        .is_some_and(|rule| rule.as_str() == "particle.source")
                {
                    let tail = &consumption_following[matched.consumed_bytes..];
                    let competing_lexeme = branch.decision.is_structural()
                        && self.constraint_resolver.as_ref().is_some_and(|resolver| {
                            let whole = surrounding_token_span(haystack, core.clone());
                            context::normalized_token(haystack, whole).is_some_and(|token| {
                                let aligned_pronoun =
                                    branch.structural_patterns().iter().any(|pattern| {
                                        pattern.fine_pos == DataFinePos::Np
                                            && resolver
                                                .has_complete_nominal_surface(&consumption_anchor)
                                    });
                                resolver.has_exact_predicate_ending_path(
                                    &token,
                                    kfind_morph::PredicatePos::Verb,
                                ) || (!aligned_pronoun
                                    && (resolver.has_whole_modifier(&token)
                                        || resolver.has_exact_nominal_surface(&token)
                                        || token.strip_suffix('의').is_some_and(|host| {
                                            resolver.has_exact_nominal_surface(host)
                                        })))
                            })
                        });
                    if competing_lexeme || tail.starts_with(crate::is_token_character) {
                        // An incomplete shortened case chain can also start a compound component.
                        matched = self
                            .particle_verifier
                            .verify_prefix(&consumption_anchor, "");
                    }
                }
                if !branch.consumption.allows_rule_path(&matched.rule_path) {
                    return None;
                }

                if let [direction, genitive, ..] = matched.rule_path.as_slice()
                    && direction.as_str() == "particle.direction"
                    && genitive.as_str() == "particle.genitive"
                    && let Some(direction_surface) = ["으로", "로"]
                        .into_iter()
                        .find(|form| consumption_following.starts_with(form))
                {
                    let accepts = branch.decision.is_structural()
                        && self.constraint_resolver.as_ref().is_some_and(|resolver| {
                            let host = format!("{consumption_anchor}{direction_surface}");
                            !resolver.has_exact_nominal_surface(&host)
                                && !resolver.has_whole_modifier(&host)
                        });
                    if !accepts {
                        let prefix = if branch.decision.is_structural() {
                            ""
                        } else {
                            direction_surface
                        };
                        matched = self
                            .particle_verifier
                            .verify_prefix(&consumption_anchor, prefix);
                    }
                }

                let subject_starts_copula = branch.decision.is_structural()
                    && matches!(
                        branch.consumption,
                        CandidateConsumption::NominalParticleChain { .. }
                    )
                    && matches!(matched.rule_path.as_slice(), [rule] if rule.as_str() == "particle.subject")
                    && consumption_following.get(..matched.consumed_bytes) == Some("이")
                    && consumption_following[matched.consumed_bytes..]
                        .chars()
                        .next()
                        .is_some_and(crate::is_token_character)
                    && core == hit.span
                    && consumption_anchor
                        .chars()
                        .next_back()
                        .is_some_and(|preceding| {
                            let whole = surrounding_token_span(haystack, core.clone());
                            (whole.start == core.start
                                || (haystack[whole.start..core.start]
                                    .iter()
                                    .all(u8::is_ascii_digit)
                                    && self.constraint_resolver.as_ref().is_some_and(|resolver| {
                                        context::normalized_token(haystack, whole.start..core.end)
                                            .is_some_and(|host| {
                                                resolver.has_numeric_unit_surface(&host)
                                            })
                                    }))
                                || self.constraint_resolver.as_ref().is_some_and(|resolver| {
                                    has_nominal_component_before_copula(
                                        haystack,
                                        &whole,
                                        &core,
                                        branch.structural_patterns(),
                                        resolver,
                                    )
                                }))
                                && context::normalized_token(haystack, core.end..whole.end)
                                    .is_some_and(|ending| {
                                        verify_copula_surface_after_nominal(preceding, &ending)
                                    })
                        });
                if subject_starts_copula {
                    (0, Vec::new())
                } else {
                    let consumed_bytes = match &consumption_following {
                        Cow::Borrowed(_) => matched.consumed_bytes,
                        Cow::Owned(normalized) => {
                            map_normalized_prefix(following, normalized, matched.consumed_bytes)?
                        }
                    };
                    if consumed_name_suffix {
                        matched
                            .rule_path
                            .insert(0, RuleId::from("suffix.proper-name-i"));
                    }
                    (consumed_bytes, matched.rule_path)
                }
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
        if suffix_rules.last().is_some_and(|rule| match rule.as_str() {
            "ending.copula-ya" | "ending.copula-ge" | "ending.final-ge" => {
                !is_sentence_end_after(haystack, token_end)
            }
            "ending.interrogative-ni" => {
                branch.boundary().require_right && !is_boundary_after(haystack, token_end)
            }
            _ => false,
        }) {
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
        candidate: &mut ExecutedCandidate,
        branch: &CandidateProgram,
        atom: &AtomPlan,
        structural_cache: &mut StructuralCache<'a>,
    ) -> bool {
        let boundary = atom.boundary;
        if boundary == BoundaryPolicy::Smart
            && candidate
                .suffix_rules
                .last()
                .is_some_and(|rule| rule.as_str() == "ending.directive-ram")
            && self.constraint_resolver.as_ref().is_some_and(|resolver| {
                let token = &candidate.consumed;
                context::normalized_token(haystack, token.clone())
                    .is_some_and(|word| resolver.has_exact_nominal_surface(&word))
                    && context::next_word_on_line(haystack, token.end).is_some_and(|(end, next)| {
                        resolver.has_nominal_with_optional_particles(&next)
                            || (resolver.has_predicate_adnominal_surface(&next)
                                && context::next_word_on_line(haystack, end).is_some_and(
                                    |(_, head)| resolver.has_nominal_with_optional_particles(&head),
                                ))
                    })
            })
        {
            return false;
        }
        if boundary == BoundaryPolicy::Smart
            && candidate
                .suffix_rules
                .last()
                .is_some_and(|rule| rule.as_str() == "ending.propositive-se")
            && !is_boundary_before(haystack, candidate.verified.core.start)
            && !self.constraint_resolver.as_ref().is_some_and(|resolver| {
                context::normalized_token(
                    haystack,
                    surrounding_token_span(haystack, candidate.verified.core.clone()),
                )
                .is_some_and(|token| {
                    resolver
                        .has_complete_attached_auxiliary_path(&token, DEFAULT_LATTICE_NODE_LIMIT)
                })
            })
        {
            return false;
        }
        if boundary == BoundaryPolicy::Smart
            && candidate.suffix_rules.iter().any(|rule| {
                matches!(
                    rule.as_str(),
                    "ending.concessive-nieundeul" | "ending.counterfactual-deondeul"
                )
            })
            && !branch
                .structural_patterns()
                .iter()
                .any(|pattern| pattern.fine_pos == DataFinePos::Vcp)
            && (!is_boundary_before(haystack, candidate.verified.core.start)
                || self.constraint_resolver.as_ref().is_some_and(|resolver| {
                    context::normalized_token(haystack, candidate.consumed.clone()).is_some_and(
                        |word| {
                            let plural_nominal = word.strip_suffix('들').is_some_and(|host| {
                                resolver.has_exact_nominal_surface(host)
                                    && !resolver.has_predicate_adnominal_surface(host)
                                    && !resolver.has_exact_predicate_connective_path(
                                        &word,
                                        kfind_morph::PredicatePosSet::one(
                                            kfind_morph::PredicatePos::Verb,
                                        ) | kfind_morph::PredicatePosSet::one(
                                            kfind_morph::PredicatePos::Adjective,
                                        ),
                                    )
                            });
                            plural_nominal
                                || (resolver.has_exact_non_copula_predicate_surface(&word)
                                    && context::next_word(haystack, candidate.consumed.end)
                                        .is_some_and(|next| {
                                            resolver.has_adnominal_dependent_noun(&word, &next)
                                                || (resolver.has_predicate_adnominal_surface(&word)
                                                    && resolver.has_independent_nominal_head(&next))
                                        }))
                        },
                    )
                }))
        {
            return false;
        }
        if boundary == BoundaryPolicy::Smart
            && !is_boundary_before(haystack, candidate.verified.core.start)
            && !is_boundary_after(haystack, candidate.consumed.end)
            && branch
                .origins
                .iter()
                .flat_map(|origin| &origin.rule_path)
                .chain(&candidate.suffix_rules)
                .any(|rule| rule.as_str() == "ending.connective-doe")
        {
            return false;
        }
        if boundary == BoundaryPolicy::Smart
            && !branch.origins.is_empty()
            && branch.origins.iter().all(|origin| {
                origin.rule_path.last().is_some_and(|rule| {
                    matches!(
                        rule.as_str(),
                        "ending.prohibitive-short" | "ending.prohibitive-short-polite"
                    )
                })
            })
        {
            return self.accepts_token_boundary(haystack, &candidate.verified, branch)
                && context::previous_word(haystack, candidate.verified.core.start).is_some_and(
                    |previous| {
                        previous.ends_with('지')
                            && self.constraint_resolver.as_ref().is_some_and(|resolver| {
                                resolver.has_negative_connective_host(&previous)
                            })
                    },
                );
        }
        if boundary == BoundaryPolicy::Smart
            && let Some(resolver) = self.constraint_resolver.as_ref()
            && let Some(accepts) = decide_request_dalla(haystack, candidate, branch, atom, resolver)
        {
            return accepts;
        }
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
                (branch.origins.iter().any(|origin| {
                    origin
                        .rule_path
                        .iter()
                        .any(|rule| rule.as_str() == "contraction.ha-gi")
                }) && context::normalized_token(haystack, candidate.anchor.clone())
                    .is_some_and(|anchor| resolver.has_exact_nominal_surface(&anchor)))
                    || (candidate
                        .suffix_rules
                        .iter()
                        .any(|rule| rule.as_str() == "particle.comitative.hago")
                        && has_hago_predicate_frame(haystack, &candidate.verified.token, resolver))
                    || has_competing_iran_particle_analysis(haystack, candidate, resolver)
                    || lacks_contracted_quotative_head(haystack, candidate, branch, resolver)
                    || has_competing_hao_connective(haystack, candidate, branch, resolver)
                    || has_competing_nominal_with_gu(haystack, candidate, resolver)
                    || has_nominal_quotative_frame(haystack, candidate, branch, resolver)
                    || has_counted_unit_copula(haystack, candidate, branch, resolver)
                    || has_elapsed_measure_frame(haystack, candidate, branch, atom, resolver)
                    || has_quantifying_adverb_frame(haystack, candidate, branch, atom, resolver)
                    || has_jiyo_object_frame(haystack, candidate, branch, atom, resolver)
                    || has_whole_alternative_conjunction(haystack, candidate, branch, resolver)
                    || has_nama_proper_name_frame(haystack, candidate, branch, resolver)
                    || has_conflicting_honorific(haystack, candidate, branch, atom, resolver)
                    || has_possession_argument(haystack, candidate, branch, atom, resolver)
                    || has_object_inside_adnominal_result(haystack, candidate, branch, resolver)
                    || has_nominal_frame_for_volitional(haystack, candidate, branch, resolver)
                    || has_humble_object_frame(haystack, candidate, branch, atom, resolver)
                    || has_topic_before_independent_predicate(haystack, candidate, branch, resolver)
                    || has_nominal_interrogative_frame(haystack, candidate, branch, resolver)
                    || has_nonpredicate_report_frame(haystack, candidate, branch, resolver)
                    || has_comparative_nominal_frame(haystack, candidate, branch, resolver)
            })
        {
            return false;
        }
        if boundary == BoundaryPolicy::Smart
            && matches!(
                branch.consumption,
                CandidateConsumption::NominalParticleChain { .. }
            )
            && let Some(resolver) = self.constraint_resolver.as_ref()
            && has_dependent_quotative_frame(haystack, candidate, resolver)
        {
            candidate.consumed.end = candidate.verified.core.end;
            candidate.verified.token.end = candidate.verified.core.end;
            if !candidate.verified.origins.is_empty() {
                candidate.verified.origins.clone_from(&branch.origins);
            }
            candidate.suffix_rules.clear();
        }
        let accepted = match &branch.decision {
            CandidateDecision::Boundary(_) => {
                self.accepts_token_boundary(haystack, &candidate.verified, branch)
                    || self.accepts_boundary_adnominal_interrogative(haystack, candidate, branch)
            }
            CandidateDecision::Structural(_) => {
                self.accepts_structural(haystack, candidate, branch, structural_cache)
            }
        };
        accepted
            || (self.plan.composition == QueryComposition::Phrase
                && self.plan.atoms.len() > 1
                && self.accepts_joined_nominal_phrase_component(haystack, candidate, branch, atom))
    }

    fn accepts_joined_nominal_phrase_component(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        atom: &AtomPlan,
    ) -> bool {
        let core = &candidate.verified.core;
        if atom.boundary != BoundaryPolicy::Smart
            || !matches!(
                branch.consumption,
                CandidateConsumption::NominalParticleChain { .. }
            )
        {
            return false;
        }
        let Some(resolver) = self.constraint_resolver.as_ref() else {
            return false;
        };
        let split = if *core == candidate.consumed
            && branch.origins.iter().any(|origin| {
                atom.analyses
                    .get(usize::from(origin.analysis_index))
                    .is_some_and(|analysis| analysis.source == AnalysisSource::Heuristic)
            })
            && is_boundary_before(haystack, core.start)
            && !is_boundary_after(haystack, core.end)
        {
            core.end
        } else if !candidate.suffix_rules.is_empty()
            && branch.structural_patterns().iter().any(|pattern| {
                pattern.fine_pos == DataFinePos::Nng
                    && pattern.component_capability.allows_runtime()
            })
            && !is_boundary_before(haystack, core.start)
            && is_boundary_after(haystack, candidate.consumed.end)
            && context::normalized_token(haystack, core.clone())
                .is_some_and(|head| resolver.has_exact_nominal_surface(&head))
        {
            core.start
        } else {
            return false;
        };
        let whole = surrounding_token_span(haystack, core.clone());
        let parts = [whole.start..split, split..whole.end, whole];
        if let [Some(modifier), Some(head), Some(whole)] =
            parts.map(|span| context::normalized_token(haystack, span))
        {
            resolver.has_joined_nominal_modifier(&modifier, &head, &whole)
        } else {
            false
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
        candidate: &mut ExecutedCandidate,
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
        let core = &candidate.verified.core;
        if matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        ) && candidate.consumed == *core
            && is_boundary_before(haystack, core.start)
            && haystack.get(core.end) == Some(&b'(')
            && patterns
                .iter()
                .any(|pattern| pattern.fine_pos == DataFinePos::Nnp)
            && let Some(host) = context::normalized_token(haystack, core.clone())
            && resolver.has_exact_proper_name_surface(&host)
            && let Some(tail) = after_parenthesized_hanja(haystack, core.end)
            && let Some(particles) = tail.split(|ch| !crate::is_token_character(ch)).next()
            && !particles.is_empty()
            && self
                .particle_verifier
                .verify_exact(&host, particles)
                .is_some()
        {
            return true;
        }
        if matches!(branch.consumption, CandidateConsumption::Anchor)
            && branch.origins.iter().any(|origin| {
                origin
                    .rule_path
                    .last()
                    .is_some_and(|rule| rule.as_str() == "contraction.uri-attributive-ul")
            })
        {
            let span = &candidate.verified.token;
            return self.accepts_token_boundary(haystack, &candidate.verified, branch)
                && context::next_word(haystack, span.end)
                    .is_some_and(|next| resolver.has_independent_nominal_head(&next));
        }
        if matches!(branch.consumption, CandidateConsumption::Anchor)
            && branch.origins.iter().any(|origin| {
                origin
                    .rule_path
                    .last()
                    .is_some_and(|rule| rule.as_str().starts_with("contraction.pronoun-topic-"))
            })
        {
            let span = &candidate.verified.token;
            return self.accepts_token_boundary(haystack, &candidate.verified, branch)
                && context::normalized_token(haystack, span.clone()).is_some_and(|token| {
                    resolver.has_pronoun_topic_contraction(&token)
                        && !context::previous_word(haystack, span.start).is_some_and(|host| {
                            (resolver.has_nominal_genitive_surface(&host)
                                && resolver.has_exact_nominal_surface(&token))
                                || (resolver.has_unambiguous_proper_name_surface(&host)
                                    && resolver.has_whole_determiner(&token)
                                    && context::next_word(haystack, span.end).is_some_and(|head| {
                                        resolver.has_nominal_with_optional_particles(&head)
                                    }))
                        })
                });
        }
        if matches!(branch.consumption, CandidateConsumption::Anchor)
            && !branch.origins.is_empty()
            && branch.origins.iter().all(|origin| {
                origin
                    .rule_path
                    .last()
                    .is_some_and(|rule| rule.as_str() == "particle.genitive")
            })
        {
            let span = &candidate.verified.token;
            return self.accepts_token_boundary(haystack, &candidate.verified, branch)
                && context::normalized_token(haystack, span.clone()).is_some_and(|token| {
                    context::next_word(haystack, span.end).is_some_and(|next| {
                        (resolver.has_nominal_with_optional_particles(&next)
                            || resolver.has_explicit_copula_after_nominal(&next))
                            && (!resolver.has_whole_determiner(&next)
                                || resolver.has_exact_nominal_surface(&next))
                            && (token != "네"
                                || !resolver.has_inflected_counted_unit(&token, &next))
                    })
                });
        }
        if patterns.iter().any(|pattern| {
            pattern.fine_pos == DataFinePos::Mag
                && pattern.lexical_form.ends_with('로')
                && context::previous_word(haystack, candidate.anchor.start).is_some_and(
                    |previous| {
                        !resolver.has_object_marked_nominal(&previous)
                            && resolver.has_predicate_adnominal_surface(&previous)
                    },
                )
                && resolver.has_nominal_oblique(&pattern.lexical_form)
                && context::next_word(haystack, candidate.consumed.end).is_some_and(|next| {
                    resolver.has_independent_final_predicate(&next)
                        || resolver.has_exact_predicate_connective_path(
                            &next,
                            kfind_morph::PredicatePosSet::one(kfind_morph::PredicatePos::Verb),
                        )
                        || next
                            .strip_suffix('고')
                            .is_some_and(|stem| resolver.has_exact_action_predicate_surface(stem))
                })
        }) {
            return false;
        }
        if matches!(branch.consumption, CandidateConsumption::Anchor)
            && branch.origins.iter().any(|origin| {
                origin
                    .rule_path
                    .iter()
                    .any(|rule| rule.as_str() == "contraction.adverb-topic-neun")
            })
        {
            return self.accepts_token_boundary(haystack, &candidate.verified, branch)
                && context::normalized_token(haystack, candidate.anchor.clone()).is_some_and(
                    |token| {
                        patterns.iter().any(|pattern| {
                            pattern.fine_pos == DataFinePos::Mag
                                && resolver
                                    .has_adverb_topic_contraction(&pattern.lexical_form, &token)
                        })
                    },
                );
        }
        if matches!(branch.consumption, CandidateConsumption::Anchor)
            && branch.origins.iter().any(|origin| {
                origin.rule_path.iter().any(|rule| {
                    matches!(
                        rule.as_str(),
                        "contraction.dependent-subject-ge"
                            | "contraction.dependent-subject-ge-from-geot"
                    )
                })
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
        if matches!(branch.consumption, CandidateConsumption::Anchor)
            && patterns
                .iter()
                .any(|pattern| pattern.fine_pos == DataFinePos::Nnb)
            && let Some(rule) = branch
                .origins
                .iter()
                .filter_map(|origin| origin.rule_path.last())
                .find(|rule| {
                    matches!(
                        rule.as_str(),
                        "contraction.nominal-topic-neun"
                            | "contraction.dependent-topic-geot"
                            | "contraction.dependent-object-geo"
                            | "contraction.dependent-object-geot"
                    )
                })
        {
            let object = matches!(
                rule.as_str(),
                "contraction.dependent-object-geo" | "contraction.dependent-object-geot"
            );
            return self.accepts_token_boundary(haystack, &candidate.verified, branch)
                && context::normalized_token(haystack, candidate.anchor.clone()).is_some_and(
                    |token| {
                        context::previous_word(haystack, candidate.anchor.start).is_some_and(
                            |previous| {
                                resolver.has_dependent_noun_particle_contraction(
                                    &previous,
                                    &token,
                                    object,
                                    DEFAULT_LATTICE_NODE_LIMIT,
                                )
                            },
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
        if self.accepts_nominal_copula_ending(haystack, candidate, branch, resolver, patterns) {
            return true;
        }
        if self.accepts_product_copula_frame(haystack, candidate, branch, resolver, patterns) {
            return true;
        }
        if self.accepts_lexical_adverb_particle_frame(haystack, candidate, branch, patterns) {
            return true;
        }
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
        let core = &candidate.verified.core;
        if matches!(
            branch.consumption,
            CandidateConsumption::PredicateContinuation { .. }
        ) && candidate.consumed.end < whole.end
            && haystack
                .get(candidate.consumed.clone())
                .is_some_and(|surface| {
                    surface.ends_with("고".as_bytes()) || surface.ends_with("고".as_bytes())
                })
            && context::normalized_token(haystack, candidate.consumed.end..whole.end)
                .is_some_and(|tail| tail.starts_with(['프', '픈', '플', '픕', '픔', '파', '팠']))
            && context::normalized_token(haystack, whole.clone())
                .is_some_and(|token| has_lexical_gopeuda_adjective(&token, resolver))
        {
            return false;
        }
        if *core == whole
            && candidate.consumed == whole
            && matches!(
                branch.consumption,
                CandidateConsumption::NominalParticleChain { .. }
            )
            && patterns.iter().any(|pattern| {
                pattern.fine_pos == DataFinePos::Nnb
                    && resolver.has_scope_dependent_noun(&pattern.lexical_form)
            })
        {
            if let Some(host) = context::previous_word(haystack, whole.start) {
                if resolver.has_scope_nominal_host(&host) {
                    return true;
                }
                if resolver.has_scope_competing_connective(&host) {
                    return false;
                }
            }
            if context::next_word(haystack, whole.end)
                .is_some_and(|next| resolver.has_exact_adverb_surface(&next))
            {
                return false;
            }
        }
        if core.start > whole.start
            && patterns.iter().any(|pattern| {
                pattern.fine_pos.is_nominal() && pattern.lexical_form.as_ref() == "시"
            })
            && context::normalized_token(haystack, whole.start..core.end).is_some_and(|host| {
                context::normalized_token(haystack, core.end..whole.end).is_some_and(|tail| {
                    resolver.has_bound_si_suffix(&host, &tail, candidate.consumed.end == whole.end)
                })
            })
        {
            return false;
        }
        if matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        ) && core.start > whole.start
            && *core == candidate.consumed
            && core.end < whole.end
            && context::normalized_token(haystack, core.end..whole.end).is_some_and(|tail| {
                tail.chars().next().is_some_and(|initial| {
                    matches!(
                        kfind_morph::drop_final(initial).unwrap_or(initial),
                        '이' | '여' | '예'
                    )
                }) && has_nominal_component_before_copula(
                    haystack, &whole, core, patterns, resolver,
                ) && context::normalized_token(haystack, core.clone())
                    .and_then(|surface| surface.chars().next_back())
                    .is_some_and(|preceding| verify_copula_surface_after_nominal(preceding, &tail))
            })
        {
            return true;
        }
        if matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        ) && core.start > whole.start
            && core.end < whole.end
            && candidate.consumed.end == whole.end
            && patterns.iter().any(|pattern| {
                pattern.fine_pos == DataFinePos::Nng
                    && pattern.component_capability.allows_runtime()
                    && ConstraintResolver::has_attested_nominal_suffix(&pattern.lexical_form)
                    && context::normalized_token(haystack, whole.start..core.end).is_some_and(
                        |word| {
                            ConstraintResolver::has_attested_nominal_suffix_usage(
                                &word,
                                &pattern.lexical_form,
                            )
                        },
                    )
            })
        {
            return true;
        }
        if matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        ) && *core == candidate.consumed
            && core.start == whole.start
            && core.end < whole.end
            && (haystack[core.end..whole.end].starts_with("적".as_bytes())
                || haystack[core.end..whole.end].starts_with("ᄌ".as_bytes()))
            && patterns.iter().any(|pattern| {
                pattern.fine_pos == DataFinePos::Nng
                    && pattern.component_capability.allows_runtime()
                    && pattern.lexical_form.chars().count() == 1
            })
            && context::normalized_token(haystack, core.clone()).is_some_and(|root| {
                context::normalized_token(haystack, core.end..whole.end).is_some_and(|suffix| {
                    suffix.strip_prefix('적').is_some_and(|trailing| {
                        let particle = (!trailing.is_empty()
                            && !verify_copula_surface_after_nominal('적', trailing))
                        .then_some(trailing);
                        resolver.has_attested_short_nominal_derivation(&root, "적", particle)
                            && particle.is_none_or(|particle| {
                                self.particle_verifier
                                    .verify_exact("적", particle)
                                    .is_some()
                            })
                    })
                })
            })
        {
            return true;
        }
        if matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        ) && *core == candidate.consumed
            && whole.start < core.start
            && core.end < whole.end
            && haystack[whole.start..core.start]
                .iter()
                .all(u8::is_ascii_digit)
            && patterns
                .iter()
                .any(|pattern| matches!(pattern.fine_pos, DataFinePos::Nnb | DataFinePos::Nr))
            && context::normalized_token(haystack, whole.start..core.end).is_some_and(|host| {
                resolver.has_numeric_unit_surface(&host)
                    && context::normalized_token(haystack, core.end..whole.end).is_some_and(
                        |ending| {
                            (!matches!(ending.as_ref(), "게" | "네" | "네요")
                                || is_sentence_end_after(haystack, whole.end))
                                && host.chars().next_back().is_some_and(|preceding| {
                                    verify_copula_surface_after_nominal(preceding, &ending)
                                })
                        },
                    )
            })
        {
            return true;
        }
        if matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        ) && *core == candidate.consumed
            && whole.start < core.start
            && core.end < whole.end
            && patterns.iter().any(|pattern| {
                pattern.fine_pos == DataFinePos::Nng
                    && pattern.component_capability.allows_runtime()
            })
        {
            let parts = [whole.start..core.start, core.clone(), core.end..whole.end];
            if let [Some(prefix), Some(noun), Some(suffix)] =
                parts.map(|span| context::normalized_token(haystack, span))
                && resolver.has_prefixed_nominal_derivation_copula(&prefix, &noun, &suffix)
            {
                return true;
            }
        }
        if matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        ) && patterns
            .iter()
            .any(|pattern| pattern.fine_pos == DataFinePos::Nnb)
            && self.accepts_numeric_unit(haystack, candidate, &whole, resolver)
        {
            return true;
        }
        if matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        ) && candidate.consumed == *core
            && haystack.get(core.end..whole.end).is_some_and(|tail| {
                tail.starts_with("간".as_bytes()) || tail.starts_with("간".as_bytes())
            })
            && patterns.iter().any(|pattern| pattern.fine_pos.is_nominal())
            && let Some(unit) = context::normalized_token(haystack, core.clone())
        {
            let attached = core.start != whole.start;
            let count = if !attached {
                context::previous_word(haystack, whole.start)
            } else {
                context::normalized_token(haystack, whole.start..core.start)
            };
            if resolver.has_duration_suffix_host(&unit, count.as_deref(), attached)
                && context::normalized_token(haystack, core.end..whole.end).is_some_and(|tail| {
                    tail.strip_prefix('간').is_some_and(|particles| {
                        self.particle_verifier
                            .verify_exact("간", particles)
                            .is_some()
                    })
                })
            {
                return true;
            }
        }
        if matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        ) && candidate.consumed == *core
            && haystack.get(core.end..whole.end).is_some_and(|tail| {
                tail.starts_with("째".as_bytes()) || tail.starts_with("ᄍ".as_bytes())
            })
            && patterns.iter().any(|pattern| pattern.fine_pos.is_nominal())
            && context::normalized_token(haystack, core.clone()).is_some_and(|unit| {
                let counted = if core.start == whole.start {
                    context::previous_word(haystack, whole.start).is_some_and(|count| {
                        (count == "첫" && unit == "번")
                            || resolver.has_standalone_counted_unit(&count, &unit)
                    })
                } else {
                    context::normalized_token(haystack, whole.start..core.start).is_some_and(
                        |count| {
                            (count.bytes().all(|b| b.is_ascii_digit())
                                || ConstraintResolver::is_mixed_cardinal(&count))
                                && resolver.has_exact_counter(&unit)
                        },
                    )
                };
                counted
                    && context::normalized_token(haystack, core.end..whole.end).is_some_and(
                        |tail| {
                            tail.strip_prefix('째').is_some_and(|particles| {
                                self.particle_verifier
                                    .verify_exact("째", particles)
                                    .is_some()
                            })
                        },
                    )
            })
        {
            candidate.verified.token.end = whole.end;
            return true;
        }
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
        if *core == whole
            && candidate.consumed == whole
            && matches!(
                branch.consumption,
                CandidateConsumption::NominalParticleChain { .. }
            )
            && patterns.iter().any(|pattern| {
                pattern.fine_pos == DataFinePos::Nng && pattern.lexical_form.ends_with('가')
            })
            && has_interrogative_auxiliary_frame(haystack, &whole, resolver)
        {
            return false;
        }
        let (licensed_trailing, attached_auxiliary) =
            self.licensed_structural_trailing(haystack, candidate, branch, resolver);
        if licensed_trailing.is_none()
            && has_conflicting_whole_predicate(haystack, candidate, branch, resolver)
        {
            return false;
        }
        let contracted_copula = licensed_trailing.is_none()
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
            );
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
        let consumed_end = consumed.end;
        let rejected_suffix =
            self.has_rejected_structural_suffix(haystack, candidate, &consumed, branch, resolver);
        let nominal_particle_chain = matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        );
        if nominal_particle_chain
            && candidate.suffix_rules.iter().any(|rule| {
                matches!(
                    rule.as_str(),
                    "particle.alternative.ideun-deun" | "particle.alternative.ideunji-deunji"
                )
            })
            && context::normalized_token(haystack, whole.clone()).is_some_and(|word| {
                resolver.has_unambiguous_proper_name_surface(&word)
                    || (has_parenthesized_latin_name(haystack, whole.end)
                        && resolver.has_exact_nominal_surface(&word))
            })
        {
            return false;
        }
        let terminal_particle_conflict = rejected_suffix
            && nominal_particle_chain
            && self.has_terminal_particle_conflict(haystack, candidate, &whole);
        let possible_compound_prefix = nominal_particle_chain
            && candidate.verified.core.start == whole.start
            && candidate.consumed.end < whole.end
            && patterns.iter().any(|pattern| {
                pattern.fine_pos == DataFinePos::Nng
                    && pattern.component_capability.allows_runtime()
                    && pattern.lexical_form.chars().nth(1).is_some()
            })
            && context::normalized_token(haystack, candidate.consumed.end..whole.end).is_some_and(
                |suffix| {
                    !self
                        .particle_verifier
                        .model()
                        .allomorphs
                        .iter()
                        .any(|form| suffix == form.surface.as_ref())
                },
            );
        if (contracted_copula && !possible_compound_prefix)
            || (rejected_suffix
                && (!nominal_particle_chain
                    || (terminal_particle_conflict && !possible_compound_prefix)))
        {
            return false;
        }
        if possible_compound_prefix
            && rejected_suffix
            && context::normalized_token(haystack, whole.clone()).is_some_and(|token| {
                context::previous_word(haystack, whole.start).is_some_and(|previous| {
                    token.char_indices().skip(1).any(|(end, character)| {
                        matches!(character, '이' | '인' | '입')
                            && resolver.has_counted_unit_frame(&previous, &token[..end])
                            && token[..end].chars().next_back().is_some_and(|last| {
                                verify_copula_surface_after_nominal(last, &token[end..])
                                    || (character == '인' && token[end..].starts_with("인가"))
                            })
                    })
                })
            })
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
        let compound_prefix = possible_compound_prefix
            && context.has_compound_nominal_prefix(
                candidate.verified.core.clone(),
                candidate.consumed != candidate.verified.core,
            );
        if !compound_prefix
            && (contracted_copula
                || (rejected_suffix
                    && (terminal_particle_conflict
                        || !context.has_nominal_copula_host(candidate.verified.core.clone()))))
        {
            return false;
        }
        let trim_particle = compound_prefix && candidate.verified.token != candidate.verified.core;
        let decision = context.resolve(
            StructuralRequest {
                candidate: &candidate.verified,
                anchor: candidate.anchor.clone(),
                consumed: if trim_particle {
                    candidate.verified.core.clone()
                } else {
                    consumed
                },
                patterns,
            },
            || context::has_proper_name_subject_frame(haystack, &whole, resolver),
        );
        if decision.as_ref().is_none_or(|decision| {
            matches!(
                decision.outcome,
                kfind_morph::ConstraintOutcome::Unavailable(_)
            )
        }) && let Some(diagnostics) = structural_cache.diagnostics
        {
            diagnostics.record_unavailable();
        }
        let accepted =
            decision.is_some_and(|decision| ProductPolicy::RecallFirst.accepts(&decision));
        if accepted && trim_particle {
            candidate.verified.token = candidate.verified.core.clone();
            if !candidate.verified.origins.is_empty() {
                candidate.verified.origins.clone_from(&branch.origins);
            }
            candidate.suffix_rules.clear();
        } else if accepted && attached_auxiliary {
            candidate.verified.token.end = consumed_end;
        }
        accepted
    }

    fn accepts_numeric_unit(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        whole: &Range<usize>,
        resolver: &ConstraintResolver,
    ) -> bool {
        let core = &candidate.verified.core;
        let complete = candidate.consumed.end == whole.end;
        if !complete && candidate.consumed != *core {
            return false;
        }
        let prefix = &haystack[whole.start..core.start];
        let digits = |bytes: &[u8]| !bytes.is_empty() && bytes.iter().all(u8::is_ascii_digit);
        if complete
            && (prefix.starts_with("제".as_bytes()) || prefix.starts_with("ᄌ".as_bytes()))
            && context::normalized_token(haystack, whole.start..core.start).is_some_and(|prefix| {
                prefix
                    .strip_prefix('제')
                    .is_some_and(|number| digits(number.as_bytes()))
            })
        {
            return context::normalized_token(haystack, core.clone())
                .is_some_and(|unit| resolver.has_exact_numeric_unit(&unit));
        }
        let Some(unit) = context::normalized_token(haystack, core.clone()) else {
            return false;
        };
        if complete
            && prefix.is_empty()
            && context::previous_word(haystack, whole.start)
                .is_some_and(|previous| resolver.has_standalone_counted_unit(&previous, &unit))
        {
            return true;
        }
        if complete
            && prefix.is_empty()
            && context::previous_word(haystack, whole.start)
                .is_some_and(|previous| ConstraintResolver::is_mixed_cardinal(&previous))
            && resolver.has_exact_numeric_unit(&unit)
        {
            return true;
        }
        if !complete
            && prefix.is_empty()
            && has_counted_unit_copula_ending(haystack, core.end..whole.end)
            && context::previous_word(haystack, whole.start)
                .is_some_and(|previous| resolver.has_counted_unit_frame(&previous, &unit))
        {
            return true;
        }
        let supported = if complete {
            ConstraintResolver::is_dictionary_numeric_unit(&unit)
        } else {
            let tail = core.end..whole.end;
            haystack[tail.clone()].starts_with("씩".as_bytes())
                || haystack[tail].starts_with("ᄊ".as_bytes())
        };
        if !supported {
            return false;
        }
        let preceded_by_number = if prefix.is_empty() {
            context::previous_word(haystack, whole.start)
                .is_some_and(|previous| digits(previous.as_bytes()))
        } else {
            digits(prefix)
        };
        if !preceded_by_number {
            return false;
        }
        if complete {
            return true;
        }
        context::normalized_token(haystack, core.end..whole.end).is_some_and(|suffix| {
            suffix.strip_prefix('씩').is_some_and(|particles| {
                resolver.has_exact_numeric_unit(&unit)
                    && self
                        .particle_verifier
                        .verify_exact("씩", particles)
                        .is_some()
            })
        })
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
                .verify_non_nominal_prefix(&modifier, particles)
                .consumed_bytes
                == particles.len()
    }

    fn licensed_structural_trailing(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        resolver: &ConstraintResolver,
    ) -> (Option<Range<usize>>, bool) {
        if matches!(
            branch.consumption,
            CandidateConsumption::NominalParticleChain { .. }
        ) {
            return (
                self.licensed_nominal_copula_trailing(haystack, candidate, resolver),
                false,
            );
        }
        if !matches!(
            branch.consumption,
            CandidateConsumption::PredicateContinuation { .. }
        ) {
            return (None, false);
        }
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
        if candidate.consumed.end >= whole.end {
            return (None, false);
        }
        let Some(trailing) = haystack
            .get(candidate.consumed.end..whole.end)
            .and_then(|bytes| std::str::from_utf8(bytes).ok())
        else {
            return (None, false);
        };
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
                    let contracted_host = whole.start == candidate.verified.core.start
                        && has_rule("contraction.ha-yeo");
                    let supported_connective = source_auxiliary_suffix
                        && trailing.starts_with(['주', '줘', '줬', '준', '줄', '줌', '줍'])
                        && token.is_some_and(|token| {
                            !resolver.has_competing_nominal_after_connective(
                                token,
                                candidate.consumed.end - whole.start,
                            ) || (contracted_host
                                && has_contracted_auxiliary_host_context(
                                    haystack, &whole, trailing, resolver,
                                ))
                        })
                        && haystack
                            .get(whole.start..candidate.consumed.end)
                            .and_then(|surface| std::str::from_utf8(surface).ok())
                            .is_some_and(|surface| {
                                resolver.has_derived_predicate_connective_surface(
                                    surface,
                                    source_positions,
                                ) || (contracted_host
                                    && resolver.has_exact_predicate_connective_path(
                                        surface,
                                        source_positions,
                                    ))
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
                        || supported_connective
                        || token.is_some_and(|token| {
                            let auxiliary_start = candidate.consumed.end - whole.start;
                            let host = &token[..auxiliary_start];
                            if !resolver.has_exact_predicate_connective_path(host, source_positions)
                                || branch.origins.iter().all(|origin| {
                                    origin
                                        .rule_path
                                        .iter()
                                        .any(|rule| rule.as_str() == "lexical.dictionary-voice")
                                })
                            {
                                return false;
                            }
                            trailing.char_indices().skip(1).any(|(split, _)| {
                                let (auxiliary, particles) = trailing.split_at(split);
                                self.particle_verifier
                                    .verify_exact(auxiliary, particles)
                                    .is_some()
                                    && resolver.has_nominalized_attached_auxiliary_path(
                                        &token[..auxiliary_start + split],
                                        DEFAULT_LATTICE_NODE_LIMIT,
                                    )
                            })
                        })
                }
                _ => false,
            };
        let licensed = (matches!(trailing, "까" | "까요") && has_rule("ending.future-adnominal"))
            || (trailing == "서도" && has_rule("ending.connective-go"))
            || (matches!(trailing, "가" | "가")
                && has_rule("ending.aoeo-seo")
                && context::next_word(haystack, whole.end)
                    .is_some_and(|next| resolver.has_negative_copula_after_focus(&next)))
            || (trailing == "도" && has_rule("ending.connective-neunde"))
            || (trailing.starts_with("잖") && has_rule("ending.past"))
            || (trailing.starts_with(['아', '어', '여'])
                && has_rule("ending.past")
                && resolver.supports_ending_suffix_path(trailing, 0, DEFAULT_LATTICE_NODE_LIMIT))
            || (ADNOMINAL_RULE_IDS.iter().any(|rule| has_rule(rule))
                && (trailing.starts_with(['때', '게'])
                    || context::normalized_token(haystack, candidate.consumed.end..whole.end)
                        .is_some_and(|tail| {
                            resolver.has_adnominal_auxiliary_tail(
                                &tail,
                                has_rule("ending.future-adnominal"),
                                has_rule("ending.present-adnominal")
                                    || has_rule("ending.past-adnominal"),
                            )
                        })))
            || source_attached_auxiliary
            || source_predicate_continuation;
        (
            licensed.then_some(candidate.consumed.start..whole.end),
            source_attached_auxiliary,
        )
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
        if candidate.verified.core.start <= whole.start {
            return false;
        }
        if candidate.consumed.end != whole.end {
            return candidate.consumed.end < whole.end
                && candidate
                    .suffix_rules
                    .last()
                    .is_some_and(|rule| rule.as_str() == "ending.interrogative-nga")
                && context::normalized_token(haystack, candidate.consumed.end..whole.end)
                    .is_some_and(|quote| matches!(quote.as_ref(), "라고" | "라고요" | "라는"))
                && context::normalized_token(haystack, whole.start..candidate.verified.core.start)
                    .is_some_and(|host| resolver.has_complete_nominal_surface(&host));
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
        if host.ends_with('하')
            && context::previous_word(haystack, whole.start)
                .is_some_and(|previous| resolver.has_object_marked_nominal(&previous))
            && token
                .get(host.len()..)
                .is_some_and(|ending| resolver.has_derived_ha_past(&host, ending))
        {
            return false;
        }
        ((host.bytes().all(|byte| byte.is_ascii_digit())
            || resolver.has_numeric_unit_surface(&host))
            && token.get(host.len()..).is_some_and(|ending| {
                host.chars()
                    .next_back()
                    .is_some_and(|preceding| verify_copula_surface_after_nominal(preceding, ending))
            }))
            || resolver.has_complete_nominal_surface(&host)
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

    fn accepts_nominal_copula_ending(
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
            matches!(pattern.fine_pos, DataFinePos::Np | DataFinePos::Nnb)
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
        self.accepts_token_boundary(haystack, &candidate.verified, branch)
            && ((patterns
                .iter()
                .any(|pattern| pattern.fine_pos == DataFinePos::Np)
                && resolver.has_exact_pronoun_copula_ending_path(&normalized))
                || (patterns
                    .iter()
                    .any(|pattern| pattern.fine_pos == DataFinePos::Nnb)
                    && follows_predicate_adnominal(haystack, whole.start, resolver)
                    && resolver.has_exact_dependent_copula_ending_path(&normalized)))
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
        if (normalized.ends_with("더군") || normalized.ends_with("더군요"))
            && context::previous_word(haystack, candidate.verified.token.start)
                .is_some_and(|previous| resolver.has_object_marked_nominal(&previous))
        {
            return false;
        }
        if let Some(stem) = normalized
            .strip_suffix("렷다")
            .or_else(|| normalized.strip_suffix('랴'))
            && resolver.has_exact_action_predicate_surface(stem)
            && has_directional_adverbial_frame(haystack, candidate.verified.token.start, resolver)
        {
            return false;
        }
        if matches!(
            branch.consumption,
            CandidateConsumption::CopulaHostEndingCompose
        ) && branch.origins.iter().any(|origin| {
            origin
                .rule_path
                .last()
                .is_some_and(|rule| rule.as_str() == "contraction.geos-copula-nieun-go")
        }) {
            return is_sentence_end_after(haystack, candidate.verified.token.end)
                && context::previous_word(haystack, candidate.verified.token.start).is_some_and(
                    |previous| resolver.has_adnominal_dependent_noun(&previous, "것"),
                );
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
                        || has_quotative_adnominal_head(haystack, candidate.consumed.end, resolver),
                        |unit| {
                            context::previous_word(haystack, candidate.verified.token.start)
                                .is_some_and(|previous| {
                                    resolver.has_counted_unit_frame(&previous, unit)
                                })
                        },
                        |noun| {
                            context::previous_word_with_start(
                                haystack,
                                candidate.verified.token.start,
                            )
                            .is_some_and(|(start, previous)| {
                                resolver.has_quotative_adnominal_dependent_noun(&previous, noun)
                                    || (context::previous_word(haystack, start).is_some_and(
                                        |object| resolver.has_object_marked_nominal(&object),
                                    ) && resolver.has_adnominal_dependent_noun(&previous, noun))
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
        if !preceding.is_some_and(|preceding| {
            verify_copula_surface_after_nominal(preceding, &suffix)
                || suffix
                    .strip_suffix('요')
                    .filter(|form| form.ends_with("니다"))
                    .is_some_and(|form| verify_copula_surface_after_nominal(preceding, form))
        }) {
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
                    let matched = self
                        .particle_verifier
                        .verify_non_nominal_prefix(&ending, &trailing);
                    matched.consumed_bytes == trailing.len()
                        && (!prospective_quotative_topic
                            || matched
                                .rule_path
                                .first()
                                .is_some_and(|rule| rule.as_str() == "particle.topic"))
                        && matched.rule_path.first().is_some_and(|rule| {
                            contains_rule(&self.plan.predicate_ending_initial_particle_rules, rule)
                        })
                        && matched
                            .rule_path
                            .iter()
                            .all(|rule| contains_rule(&self.plan.auxiliary_particle_rules, rule))
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
            .any(|form| {
                normalized.starts_with(form.surface.as_ref())
                    && is_structural_suffix_particle(form, branch)
            });
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
                .any(|form| {
                    remainder.starts_with(form.surface.as_ref())
                        && is_structural_suffix_particle(form, branch)
                });
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

fn has_directional_adverbial_frame(
    haystack: &[u8],
    at: usize,
    resolver: &ConstraintResolver,
) -> bool {
    let Some((start, adjacent)) = context::previous_word_with_start(haystack, at) else {
        return false;
    };
    let is_directional = |word: &str| word.ends_with('로') && resolver.has_nominal_oblique(word);
    context::previous_word(haystack, start).is_some_and(|previous| {
        (is_directional(&adjacent) && resolver.has_exact_adverb_surface(&previous))
            || (is_directional(&previous) && resolver.has_exact_adverb_surface(&adjacent))
    })
}

fn follows_predicate_adnominal(haystack: &[u8], at: usize, resolver: &ConstraintResolver) -> bool {
    context::previous_word_with_start(haystack, at).is_some_and(|(start, previous)| {
        resolver.has_predicate_adnominal_surface(&previous)
            && !(resolver.has_object_marked_nominal(&previous)
                && context::previous_word(haystack, start)
                    .is_some_and(|modifier| resolver.has_predicate_adnominal_surface(&modifier)))
    })
}

fn lacks_contracted_quotative_head(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    matches!(
        branch.consumption,
        CandidateConsumption::PredicateContinuation {
            pos: kfind_morph::PredicatePos::Copula,
            ..
        }
    ) && candidate
        .suffix_rules
        .last()
        .is_some_and(|rule| rule.as_str() == "ending.copula-quotative-adnominal")
        && std::str::from_utf8(&haystack[candidate.anchor.end..candidate.consumed.end])
            .is_ok_and(|suffix| suffix.nfc().eq("란".chars()))
        && !has_quotative_adnominal_head(haystack, candidate.consumed.end, resolver)
}

fn has_quotative_adnominal_head(
    haystack: &[u8],
    end: usize,
    resolver: &ConstraintResolver,
) -> bool {
    context::next_word_with_end(haystack, end).is_some_and(|(next_end, next)| {
        // Enumerative (이)며 takes a nominal coordinate, not a clause starting with an adverb.
        if next
            .strip_suffix("이며")
            .is_some_and(|host| resolver.has_complete_nominal_surface(host))
            && context::next_word(haystack, next_end)
                .is_some_and(|following| resolver.has_exact_adverb_surface(&following))
        {
            return false;
        }
        if is_sentence_end_after(haystack, next_end)
            && (resolver.has_independent_final_predicate(&next)
                || resolver.has_final_copula_after_nominal(&next))
        {
            return false;
        }
        (!resolver.has_exact_adverb_surface(&next)
            && !resolver.has_nominal_genitive_surface(&next)
            && !next.strip_suffix(['과', '와']).is_some_and(|host| {
                resolver.has_complete_nominal_surface(host)
                    && context::next_word(haystack, next_end).is_some_and(|coordinate| {
                        resolver.has_nominal_with_optional_particles(&coordinate)
                    })
            })
            && resolver.has_nominal_with_optional_particles(&next))
            || (resolver.has_predicate_adnominal_surface(&next)
                && context::next_word(haystack, next_end)
                    .is_some_and(|head| resolver.has_nominal_with_optional_particles(&head)))
    })
}

fn has_competing_iran_particle_analysis(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    resolver: &ConstraintResolver,
) -> bool {
    if !candidate
        .suffix_rules
        .last()
        .is_some_and(|rule| rule.as_str() == "particle.topic.iran-ran")
    {
        return false;
    }
    let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
    let before = &haystack[..whole.start];
    if whole.start == candidate.verified.core.start
        && (before.ends_with(b"\"") || before.ends_with("”".as_bytes()))
    {
        return true;
    }
    context::normalized_token(haystack, whole.clone()).is_some_and(|token| {
        (resolver.has_exact_nominal_surface(&token)
            && context::next_word(haystack, whole.end)
                .is_some_and(|next| resolver.has_nominal_genitive_surface(&next)))
            || (!token.ends_with("이란")
                && resolver.has_predicate_adnominal_surface(&token)
                && has_quotative_adnominal_head(haystack, whole.end, resolver)
                && context::previous_word(haystack, whole.start).is_some_and(|previous| {
                    resolver.has_nominal_oblique(&previous)
                        || resolver.has_object_marked_nominal(&previous)
                }))
    })
}

fn has_dependent_quotative_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    resolver: &ConstraintResolver,
) -> bool {
    let core = &candidate.verified.core;
    matches!(candidate.suffix_rules.as_slice(), [rule] if rule.as_str() == "particle.topic.iran-ran")
        && is_boundary_before(haystack, core.start)
        && is_boundary_after(haystack, candidate.consumed.end)
        && context::normalized_token(haystack, core.clone()).is_some_and(|nominal| {
            context::previous_word(haystack, core.start)
                .is_some_and(|previous| resolver.has_adnominal_dependent_noun(&previous, &nominal))
        })
        && has_quotative_adnominal_head(haystack, candidate.consumed.end, resolver)
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
        || !has_counted_unit_copula_ending(haystack, core.end..candidate.consumed.end)
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

fn has_counted_unit_copula_ending(haystack: &[u8], suffix: Range<usize>) -> bool {
    has_jiyo_suffix(haystack, suffix.clone())
        || matches!(haystack.get(suffix), Some(bytes) if bytes == "냐".as_bytes() || bytes == "냐".as_bytes())
}

fn has_elapsed_measure_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let core = &candidate.verified.core;
    let consumed = &candidate.consumed;
    if (branch.anchor.as_ref() != "만".as_bytes()
        && branch.anchor.as_ref() != "\u{1106}\u{1161}\u{11ab}".as_bytes())
        || !candidate
            .suffix_rules
            .first()
            .is_some_and(|rule| matches!(rule.as_str(), "particle.locative" | "particle.genitive"))
        || !branch.origins.iter().all(|origin| {
            atom.analyses
                .get(usize::from(origin.analysis_index))
                .is_some_and(|analysis| analysis.coarse_pos == kfind_morph::CoarsePos::Numeral)
        })
        || !is_boundary_before(haystack, core.start)
        || !is_boundary_after(haystack, consumed.end)
    {
        return false;
    }
    context::previous_word_with_start(haystack, core.start).is_some_and(|(start, measure)| {
        let count = context::previous_word(haystack, start);
        resolver.has_elapsed_measure(&measure, count.as_deref())
    })
}

fn has_quantifying_adverb_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let core = &candidate.verified.core;
    if (branch.anchor.as_ref() != "불과".as_bytes()
        && branch.anchor.as_ref() != "\u{1107}\u{116e}\u{11af}\u{1100}\u{116a}".as_bytes())
        || *core != candidate.consumed
        || !is_boundary_before(haystack, core.start)
        || !is_boundary_after(haystack, core.end)
        || !branch.origins.iter().all(|origin| {
            atom.analyses
                .get(usize::from(origin.analysis_index))
                .is_some_and(|analysis| analysis.coarse_pos == kfind_morph::CoarsePos::Noun)
        })
    {
        return false;
    }
    context::next_word_with_end(haystack, core.end).is_some_and(|(end, next)| {
        let unit = context::next_word(haystack, end);
        resolver.has_quantity_expression(&next, unit.as_deref())
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

fn has_jiyo_object_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let core = &candidate.verified.core;
    let tail = &haystack[core.end..];
    if !(tail.starts_with("지요".as_bytes())
        || tail.starts_with("죠".as_bytes())
        || tail.starts_with("ᄌ".as_bytes()))
        || branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            atom.analyses
                .get(usize::from(origin.analysis_index))
                .is_some_and(|analysis| analysis.coarse_pos == kfind_morph::CoarsePos::Adjective)
        })
    {
        return false;
    }
    let span = surrounding_token_span(haystack, core.clone());
    if core.start != span.start
        || candidate.consumed.end > span.end
        || !is_sentence_end_after(haystack, span.end)
        || !has_jiyo_suffix(haystack, core.end..span.end)
        || !context::normalized_token(haystack, core.clone())
            .is_some_and(|stem| resolver.has_exact_action_predicate_surface(&stem))
    {
        return false;
    }
    let limit = span
        .start
        .saturating_sub(crate::DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes);
    let mut before = span.start;
    let scheduled_verb = has_only_lemma(branch, atom, "늦다");
    let mut has_schedule = false;
    let mut has_person = false;
    while let Some((start, word)) = context::previous_word_with_start(haystack, before) {
        if start < limit || haystack[start..before].contains(&b'\n') {
            break;
        }
        if resolver.has_object_marked_nominal(&word) {
            return true;
        }
        if scheduled_verb
            && !has_schedule
            && let Some(noun) = word.strip_suffix('에')
        {
            has_schedule = resolver.has_scheduled_complement(noun, "에");
            if !has_schedule
                && let Some((head_start, head)) = context::previous_word_with_start(haystack, start)
                && head_start >= limit
                && !haystack[head_start..start].contains(&b'\n')
                && resolver.has_scheduled_complement(&format!("{head} {noun}"), "에")
            {
                has_schedule = true;
                before = head_start;
                continue;
            }
        }
        if scheduled_verb && resolver.has_person_subject(&word) {
            has_person = true;
        } else if (!has_schedule && !resolver.has_exact_adverb_surface(&word))
            || (has_schedule && resolver.has_independent_final_predicate(&word))
        {
            break;
        }
        if has_schedule && has_person {
            return true;
        }
        before = start;
    }
    false
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
        && context::next_word_with_end(haystack, end).is_some_and(|(next_end, next)| {
            resolver.has_independent_final_predicate(&next)
                || (context::previous_word(haystack, span.start).is_none()
                    && resolver.has_exact_nominal_surface(&next)
                    && !resolver.has_exact_non_copula_predicate_surface(&next)
                    && context::next_word_with_end(haystack, next_end).is_some_and(
                        |(predicate_end, predicate)| {
                            is_sentence_end_after(haystack, predicate_end)
                                && (resolver.has_final_copula_after_nominal(&predicate)
                                    || predicate.strip_suffix('다').is_some_and(|noun| {
                                        noun.chars().next_back().is_some_and(|last| {
                                            kfind_morph::decompose_syllable(last)
                                                .is_some_and(|syllable| syllable.jongseong == 0)
                                        }) && resolver.has_exact_nominal_surface(noun)
                                    }))
                        },
                    ))
        })
}

fn has_interrogative_auxiliary_frame(
    haystack: &[u8],
    token: &Range<usize>,
    resolver: &ConstraintResolver,
) -> bool {
    use kfind_morph::{PredicatePos, decompose_syllable, hangul::JONG_NIEUN};

    let Some(text) = context::normalized_token(haystack, token.clone()) else {
        return false;
    };
    if !text
        .strip_suffix('가')
        .and_then(|head| head.chars().next_back())
        .and_then(decompose_syllable)
        .is_some_and(|syllable| syllable.jongseong == JONG_NIEUN)
    {
        return false;
    }
    context::next_word_on_line(haystack, token.end).is_some_and(|(end, next)| {
        is_sentence_end_after(haystack, end) && resolver.has_interrogative_auxiliary_surface(&next)
    }) && context::previous_word(haystack, token.start)
        .is_some_and(|previous| resolver.has_subject_marked_nominal(&previous))
        && (resolver.has_exact_predicate_ending_path(&text, PredicatePos::Adjective)
            || resolver.has_exact_lost_span_copula_ending_path(&text))
}

fn has_competing_nominal_with_gu(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    resolver: &ConstraintResolver,
) -> bool {
    let span = &candidate.consumed;
    candidate
        .suffix_rules
        .last()
        .is_some_and(|rule| rule.as_str() == "ending.connective-go")
        && *span == candidate.verified.token
        && haystack
            .get(span.clone())
            .is_some_and(|text| text.ends_with("구".as_bytes()) || text.ends_with("구".as_bytes()))
        && context::normalized_token(haystack, span.clone()).is_some_and(|token| {
            resolver.has_exact_nominal_surface(&token)
                && !context::previous_word(haystack, span.start).is_some_and(|previous| {
                    resolver.has_object_marked_nominal(&previous)
                        || (resolver.has_exact_adverb_surface(&previous)
                            && !resolver.has_exact_proper_name_surface(&token))
                })
        })
}

fn has_nominal_interrogative_frame(
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
    if !is_boundary_before(haystack, span.start)
        || !is_boundary_after(haystack, end)
        || !context::normalized_token(haystack, span.start..end)
            .is_some_and(|token| resolver.has_exact_nominal_surface(&token))
    {
        return false;
    }
    let Some((next_end, next)) = context::next_word_with_end(haystack, end) else {
        return false;
    };
    (context::previous_word(haystack, span.start)
        .is_some_and(|previous| resolver.has_nominal_genitive_surface(&previous))
        && resolver.has_complete_nominal_surface(&next))
        || context::next_word(haystack, next_end).is_some_and(|counter| {
            counter.strip_suffix(['을', '를']).is_some_and(|unit| {
                resolver.has_counted_unit_frame(&next, unit)
                    && resolver.has_object_marked_nominal(&counter)
            })
        })
}

fn has_nonpredicate_report_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let end = candidate.consumed.end;
    let Some(tail) = end
        .checked_sub(3)
        .and_then(|start| haystack.get(start..end))
    else {
        return false;
    };
    if !["대", "래", "재", "냬", "ᅢ", "ᅤ"]
        .iter()
        .any(|ending| tail == ending.as_bytes())
    {
        return false;
    }
    let is_report = |rule: &RuleId| rule.as_str() == "ending.report-contraction";
    if !(candidate.suffix_rules.iter().any(is_report)
        || (!branch.origins.is_empty()
            && branch
                .origins
                .iter()
                .all(|origin| origin.rule_path.iter().any(is_report))))
        || !is_boundary_before(haystack, candidate.consumed.start)
        || !is_boundary_after(haystack, candidate.consumed.end)
    {
        return false;
    }
    let Some((next_end, next)) = context::next_word_with_end(haystack, candidate.consumed.end)
    else {
        return false;
    };
    let Some(token) = context::normalized_token(haystack, candidate.consumed.clone()) else {
        return false;
    };
    if resolver.has_exact_adverb_surface(&token) && resolver.has_independent_final_predicate(&next)
    {
        return true;
    }
    if !resolver.has_exact_nominal_surface(&token) {
        return false;
    }
    context::next_word(haystack, next_end).is_some_and(|following| {
        let unit = following
            .strip_suffix(['이', '가', '을', '를', '은', '는'])
            .unwrap_or(&following);
        resolver.has_counted_unit_frame(&next, unit)
            || (resolver.has_complete_nominal_surface(&next)
                && resolver.has_independent_final_predicate(&following))
    })
}

fn has_comparative_nominal_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let span = &candidate.consumed;
    if !candidate.suffix_rules.is_empty()
        || !matches!(
            branch.consumption,
            CandidateConsumption::PredicateContinuation {
                continuation: kfind_morph::ContinuationState::AOrEo,
                ..
            }
        )
        || !is_boundary_before(haystack, span.start)
        || !is_boundary_after(haystack, span.end)
    {
        return false;
    }
    context::next_word(haystack, span.end)
        .is_some_and(|next| resolver.has_comparative_nominal_surface(&next))
        && context::normalized_token(haystack, span.clone())
            .is_some_and(|token| resolver.has_exact_nominal_surface(&token))
        && !context::previous_word(haystack, span.start).is_some_and(|previous| {
            resolver.has_nominal_oblique(&previous) || resolver.has_object_marked_nominal(&previous)
        })
}

fn has_nama_proper_name_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    let whole = &candidate.verified.token;
    let surface = &haystack[whole.clone()];
    if candidate.consumed.start != whole.start
        || !(surface.ends_with("나마".as_bytes()) || surface.ends_with("나마".as_bytes()))
        || !branch.origins.iter().any(|origin| {
            origin
                .rule_path
                .last()
                .is_some_and(|rule| rule.as_str() == "ending.concessive-nama")
        }) && !matches!(
            branch.consumption,
            CandidateConsumption::PredicateContinuation {
                continuation: kfind_morph::ContinuationState::AOrEo,
                ..
            }
        )
    {
        return false;
    }
    context::normalized_token(haystack, whole.clone())
        .is_some_and(|name| name.ends_with("나마") && resolver.has_exact_proper_name_surface(&name))
        && context::next_word(haystack, whole.end)
            .is_some_and(|head| resolver.has_nominal_with_optional_particles(&head))
        && !context::previous_word(haystack, whole.start)
            .is_some_and(|object| resolver.has_object_marked_nominal(&object))
}

fn has_whole_alternative_conjunction(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    resolver: &ConstraintResolver,
) -> bool {
    if !matches!(candidate.suffix_rules.as_slice(), [rule] if rule.as_str() == "particle.topic")
        || !branch
            .structural_patterns()
            .iter()
            .any(|pattern| matches!(pattern.fine_pos, DataFinePos::Mag | DataFinePos::Maj))
    {
        return false;
    }
    let span = &candidate.consumed;
    is_boundary_before(haystack, span.start)
        && is_boundary_after(haystack, span.end)
        && context::normalized_token(haystack, span.clone())
            .is_some_and(|token| resolver.has_exact_conjunctive_adverb(&token))
        && context::previous_word(haystack, span.start)
            .is_some_and(|previous| resolver.has_alternative_connective(&previous))
}

fn has_object_inside_adnominal_result(
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
    ) || !branch.origins.iter().any(|origin| {
        candidate
            .suffix_rules
            .last()
            .or_else(|| origin.rule_path.last())
            .is_some_and(|rule| ADNOMINAL_RULE_IDS.contains(&rule.as_str()))
    }) {
        return false;
    }
    let span = &candidate.consumed;
    let Some((end, next)) = context::next_word_with_end(haystack, span.end) else {
        return false;
    };
    resolver.has_adnominal_result_complement(&next)
        && is_boundary_before(haystack, span.start)
        && context::previous_word(haystack, span.start)
            .is_some_and(|previous| resolver.has_object_marked_nominal(&previous))
        && context::next_word_with_end(haystack, end).is_some_and(|(next_end, next)| {
            let predicate = if resolver.has_subject_marked_nominal(&next) {
                context::next_word_with_end(haystack, next_end)
            } else {
                Some((next_end, next))
            };
            predicate.is_some_and(|(predicate_end, predicate)| {
                is_sentence_end_after(haystack, predicate_end)
                    && resolver.has_independent_final_predicate_without_object(&predicate)
            })
        })
}

fn has_contracted_auxiliary_host_context(
    haystack: &[u8],
    token: &Range<usize>,
    auxiliary: &str,
    resolver: &ConstraintResolver,
) -> bool {
    context::previous_word(haystack, token.start).is_some_and(|previous| {
        resolver.has_exact_predicate_connective_path(
            &previous,
            kfind_morph::PredicatePosSet::one(kfind_morph::PredicatePos::Verb),
        ) || (previous
            .strip_suffix("도록")
            .is_some_and(|stem| resolver.has_exact_non_copula_predicate_surface(stem)))
            || (is_sentence_end_after(haystack, token.end)
                && (resolver.has_exact_adverb_surface(&previous)
                    || resolver.has_object_marked_nominal(&previous)))
            || (resolver.has_predicate_adnominal_surface(auxiliary)
                && (resolver.has_subject_marked_nominal(&previous)
                    || resolver.has_nominal_oblique(&previous))
                && context::next_word(haystack, token.end)
                    .is_some_and(|next| resolver.has_independent_nominal_head(&next)))
    })
}

fn has_conflicting_honorific(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    if branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            let mut rules = origin.rule_path.iter().map(RuleId::as_str);
            let honorific = rules.any(|rule| rule == "ending.honorific");
            honorific
                && rules
                    .chain(candidate.suffix_rules.iter().map(RuleId::as_str))
                    .any(|rule| {
                        matches!(
                            rule,
                            "ending.polite-imperative"
                                | "ending.final-ne"
                                | "ending.aoeo-seo"
                                | "ending.connective-do"
                        )
                    })
        })
    {
        return false;
    }
    let token = surrounding_token_span(haystack, candidate.verified.core.clone());
    let start = candidate.verified.core.start;
    if token.start < start {
        return context::normalized_token(haystack, token.clone()).is_some_and(|word| {
            resolver.has_exact_non_copula_predicate_surface(&word)
                && context::normalized_token(haystack, token.start..start)
                    .is_some_and(|prefix| !resolver.has_vowel_connective_host(&prefix))
        });
    }
    if has_only_lemma(branch, atom, "갈다")
        && context::normalized_token(haystack, token.clone()).is_some_and(|surface| {
            surface.strip_prefix('가').is_some_and(|tail| {
                tail.starts_with("세요") || tail.starts_with("셔") || tail.starts_with("시")
            })
        })
        && context::previous_word_with_start(haystack, start).is_some_and(|(at, previous)| {
            !haystack[at..start].contains(&b'\n')
                && resolver.has_place_destination(&previous)
                && context::previous_word(haystack, at)
                    .is_none_or(|word| !resolver.has_object_marked_nominal(&word))
        })
    {
        return true;
    }
    // Auxiliary 주다 takes an -아/-어 connective. Its honorific overlaps with 줄다.
    branch.origins.iter().all(|origin| {
        atom.analyses
            .get(usize::from(origin.analysis_index))
            .is_some_and(|analysis| analysis.lemma.as_ref() == "줄다")
    }) && context::previous_word(haystack, start)
        .is_some_and(|previous| resolver.has_request_auxiliary_host(&previous))
}

fn has_only_lemma(branch: &CandidateProgram, atom: &AtomPlan, lemma: &str) -> bool {
    !branch.origins.is_empty()
        && branch.origins.iter().all(|origin| {
            atom.analyses
                .get(usize::from(origin.analysis_index))
                .is_some_and(|analysis| analysis.lemma.as_ref() == lemma)
        })
}

fn has_possession_argument(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    if !candidate
        .suffix_rules
        .iter()
        .any(|rule| rule.as_str() == "ending.auxiliary-jida")
        || !has_only_lemma(branch, atom, "가다")
    {
        return false;
    }
    let token = surrounding_token_span(haystack, candidate.verified.core.clone());
    if candidate.verified.core.start != token.start {
        return false;
    }
    context::previous_word_with_start(haystack, token.start).is_some_and(|(at, previous)| {
        !haystack[at..token.start].contains(&b'\n')
            && resolver.has_possession_object(&previous)
            && context::normalized_token(haystack, token)
                .is_some_and(|surface| resolver.has_possessive_predicate_surface(&surface))
    })
}

fn decide_request_dalla(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> Option<bool> {
    use kfind_morph::CoarsePos;

    let start = candidate.verified.core.start;
    let tail = haystack.get(start..)?;
    if !(tail.starts_with("달라".as_bytes()) || tail.starts_with("달라".as_bytes())) {
        return None;
    }
    let (first, rest) = branch.origins.split_first()?;
    let analysis = atom.analyses.get(usize::from(first.analysis_index))?;
    let is_request = match (analysis.lemma.as_ref(), analysis.coarse_pos) {
        ("달다", CoarsePos::Verb) => true,
        ("다르다", CoarsePos::Adjective) => false,
        _ => return None,
    };
    if rest.iter().any(|origin| {
        atom.analyses
            .get(usize::from(origin.analysis_index))
            .is_none_or(|other| {
                other.lemma != analysis.lemma || other.coarse_pos != analysis.coarse_pos
            })
    }) {
        return None;
    }

    let span = surrounding_token_span(haystack, candidate.verified.core.clone());
    let token = context::normalized_token(haystack, start..span.end)?;
    let request_quote = || {
        token.strip_prefix('달').is_some_and(|ending| {
            verifies_directive("달", ending, |rule| rule == "ending.directive-quotative")
        })
    };
    if span.start < start {
        return (!is_request
            && (token == "달라" || request_quote())
            && context::normalized_token(haystack, span.start..start)
                .is_some_and(|host| resolver.has_request_auxiliary_host(&host)))
        .then_some(false);
    }
    if token != "달라" {
        return (!is_request && request_quote()).then_some(false);
    }
    let next = context::next_word(haystack, span.end);
    if next
        .as_deref()
        .is_some_and(|word| resolver.has_boi_predicate(word))
    {
        return is_request.then_some(false);
    }
    if !is_request {
        let mut before = start;
        let context_start =
            start.saturating_sub(crate::DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes);
        while before > context_start {
            let (word_start, word) = context::previous_word_with_start(haystack, before)?;
            if word_start < context_start {
                break;
            }
            if resolver.has_object_marked_nominal(&word) {
                return Some(false);
            }
            if !resolver.has_exact_adverb_surface(&word) {
                break;
            }
            before = word_start;
        }
        return None;
    }
    if candidate.consumed == span
        && matches!(candidate.suffix_rules.as_slice(), [rule] if rule.as_str() == "ending.directive-ra")
        && next
            .as_deref()
            .and_then(|word| word.strip_prefix('하'))
            .is_some_and(|ending| {
                verifies_directive("하", ending, |rule| {
                    matches!(rule, "ending.directive-ra" | "ending.directive-quotative")
                })
            })
    {
        return Some(true);
    }
    None
}

fn verifies_directive(stem: &str, ending: &str, accepts_rule: impl FnOnce(&str) -> bool) -> bool {
    use kfind_morph::{ContinuationState, PredicatePos};

    verify_predicate_continuation(ContinuationState::Eu, PredicatePos::Verb, stem, ending)
        .is_some_and(|matched| {
            matched.consumed_bytes == ending.len()
                && matches!(matched.rule_path.as_slice(), [rule] if accepts_rule(rule.as_str()))
        })
}

fn has_humble_object_frame(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    if branch.origins.is_empty()
        || !branch.origins.iter().all(|origin| {
            atom.analyses
                .get(usize::from(origin.analysis_index))
                .is_some_and(|analysis| analysis.lemma.as_ref() == "비다")
        })
    {
        return false;
    }
    let core = &candidate.verified.core;
    let whole = surrounding_token_span(haystack, core.clone());
    if whole.start != core.start
        || !is_sentence_end_after(haystack, whole.end)
        || !context::normalized_token(haystack, core.end..whole.end).is_some_and(|ending| {
            matches!(
                ending.as_ref(),
                "나이다" | "나이까" | "옵나이다" | "옵나이까"
            )
        })
    {
        return false;
    }
    context::previous_word_with_start(haystack, whole.start).is_some_and(|(start, previous)| {
        resolver.has_object_marked_nominal(&previous)
            && previous.strip_suffix(['을', '를']).is_some_and(|host| {
                let count = context::previous_word(haystack, start);
                !resolver.has_elapsed_measure(host, count.as_deref())
            })
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
        && (context::next_word_on_line(haystack, whole.end)
            .is_some_and(|(_, next)| resolver.has_causal_nominal_surface(&next))
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

fn has_nominal_component_before_copula(
    haystack: &[u8],
    whole: &Range<usize>,
    core: &Range<usize>,
    patterns: &[kfind_morph::QueryMorphPattern],
    resolver: &ConstraintResolver,
) -> bool {
    core.start > whole.start
        && patterns.iter().any(|pattern| {
            pattern.fine_pos == DataFinePos::Nng
                && pattern.component_capability.allows_runtime()
                && context::normalized_token(haystack, whole.start..core.end).is_some_and(|host| {
                    let suffix = &pattern.lexical_form;
                    ConstraintResolver::has_attested_nominal_suffix_usage(&host, suffix)
                        || resolver.has_unambiguous_final_nominal_component(&host, suffix)
                })
        })
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
            "ending.auxiliary-gada"
                | "ending.auxiliary-jida"
                | "ending.auxiliary-boda"
                | "ending.auxiliary-gopeuda"
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
    if candidate
        .suffix_rules
        .iter()
        .any(|rule| rule.as_str() == "ending.auxiliary-gopeuda")
        && has_lexical_gopeuda_adjective(&normalized_token, resolver)
    {
        return true;
    }
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

fn has_lexical_gopeuda_adjective(token: &str, resolver: &ConstraintResolver) -> bool {
    token.match_indices('고').any(|(start, connective)| {
        let end = start + connective.len();
        verify_predicate_continuation(
            kfind_morph::ContinuationState::StemEnding,
            kfind_morph::PredicatePos::Verb,
            &token[..start],
            &token[start..],
        )
        .is_some_and(|matched| {
            matched.token_end == token.len()
                && matched
                    .rule_path
                    .iter()
                    .any(|rule| rule.as_str() == "ending.auxiliary-gopeuda")
                && resolver.has_exact_adjective_surface(&format!("{}프", &token[..end]))
        })
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
    if pos.is_action()
        && suffix == "답시고"
        && !matches!(
            base_state,
            kfind_morph::ContinuationState::Past | kfind_morph::ContinuationState::Future
        )
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

fn has_negative_auxiliary(
    word: &str,
    host: kfind_morph::PredicatePos,
    resolver: Option<&ConstraintResolver>,
) -> bool {
    use kfind_morph::{ContinuationState, PredicatePos};
    let (anchor, state) = match word.chars().next() {
        Some('않') if word.starts_with("않았") => ("않았", ContinuationState::Past),
        Some('않') if word.starts_with("않아") => ("않아", ContinuationState::AOrEo),
        Some('않') => ("않", ContinuationState::StemEnding),
        Some('못') => match word.chars().nth(1) {
            Some('하') => ("못하", ContinuationState::StemEnding),
            Some('했') => ("못했", ContinuationState::Past),
            Some('해') => ("못해", ContinuationState::AOrEo),
            _ => return false,
        },
        Some('마') if host.is_action() => {
            return matches!(word, "마" | "마요" | "마라" | "마세요" | "마십시오");
        }
        Some('말') if host.is_action() => ("말", ContinuationState::StemEnding),
        _ => return false,
    };
    let suffix = &word[anchor.len()..];
    let (anchor, suffix, state) = match (state, suffix.strip_prefix('는')) {
        (ContinuationState::StemEnding, Some(tail)) if tail.starts_with('데') => (
            &word[..anchor.len() + '는'.len_utf8()],
            tail,
            ContinuationState::NieunEnding,
        ),
        _ => (anchor, suffix, state),
    };
    (state == ContinuationState::AOrEo || !suffix.is_empty())
        && (verify_predicate_continuation(state, PredicatePos::AuxiliaryVerb, anchor, suffix)
            .is_some_and(|matched| matched.token_end == word.len())
            || resolver.is_some_and(|resolver| {
                !resolver.has_subject_marked_nominal(word)
                    && resolver.supports_ending_suffix_path(
                        word,
                        anchor.len(),
                        DEFAULT_LATTICE_NODE_LIMIT,
                    )
            }))
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

#[derive(Debug)]
struct AnchorPrograms {
    offsets: Vec<usize>,
    programs: Vec<ProgramRef>,
}

impl AnchorPrograms {
    fn get(&self, anchor_index: usize) -> &[ProgramRef] {
        &self.programs[self.offsets[anchor_index]..self.offsets[anchor_index + 1]]
    }
}

fn unique_anchors(plan: &QueryPlan) -> (Vec<&[u8]>, AnchorPrograms, Option<usize>) {
    let program_count = plan.atoms.iter().map(|atom| atom.programs.len()).sum();
    let mut anchor_indices = HashMap::<&[u8], usize>::with_capacity(program_count);
    let mut anchors = Vec::<&[u8]>::with_capacity(program_count);
    let mut links = Vec::with_capacity(program_count);
    let mut counts = Vec::<usize>::with_capacity(program_count);
    let mut fused_count = 0;
    for (atom_index, atom) in plan.atoms.iter().enumerate() {
        for (program_index, branch) in atom.programs.iter().enumerate() {
            let program = ProgramRef {
                atom_index,
                program_index,
            };
            if matches!(branch.consumption, CandidateConsumption::FusedCopulaEnding) {
                links.push((None, program));
                fused_count += 1;
                continue;
            }
            let anchor_index = *anchor_indices.entry(&branch.anchor).or_insert_with(|| {
                let index = anchors.len();
                anchors.push(&branch.anchor);
                counts.push(0);
                index
            });
            links.push((Some(anchor_index), program));
            counts[anchor_index] += 1;
        }
    }
    let fused_anchor = (fused_count != 0).then(|| {
        let index = counts.len();
        counts.push(fused_count);
        index
    });
    let mut offsets = Vec::with_capacity(counts.len() + 1);
    offsets.push(0);
    for count in &mut counts {
        let start = *offsets.last().expect("initial offset");
        offsets.push(start + *count);
        *count = start;
    }
    let mut programs = vec![
        ProgramRef {
            atom_index: 0,
            program_index: 0
        };
        program_count
    ];
    for (anchor_index, program) in links {
        let index = anchor_index.or(fused_anchor).expect("fused anchor");
        programs[counts[index]] = program;
        counts[index] += 1;
    }
    (anchors, AnchorPrograms { offsets, programs }, fused_anchor)
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

fn after_parenthesized_hanja(haystack: &[u8], end: usize) -> Option<&str> {
    let bytes = haystack.get(end..)?.strip_prefix(b"(")?;
    let bounded = &bytes[..bytes
        .len()
        .min(crate::DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes)];
    let (name, tail) = valid_utf8_prefix(bounded).split_once(')')?;
    (!name.is_empty()
        && name.chars().all(|ch| {
            matches!(ch, '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}' | '\u{f900}'..='\u{faff}')
        }))
    .then_some(tail)
}

fn has_parenthesized_latin_name(haystack: &[u8], end: usize) -> bool {
    let Some(tail) = haystack.get(end..).and_then(|tail| tail.strip_prefix(b"(")) else {
        return false;
    };
    let tail = &tail[..tail
        .len()
        .min(crate::DEFAULT_ANALYSIS_WINDOW_LIMITS.max_raw_bytes)];
    let mut words = tail.splitn(2, |byte| *byte == b')');
    let Some(name) = words.next().filter(|_| words.next().is_some()) else {
        return false;
    };
    let mut parts = name
        .split(|byte| *byte == b' ')
        .filter(|part| !part.is_empty());
    let is_name_part = |part: &[u8]| {
        part.first().is_some_and(u8::is_ascii_uppercase) && part.iter().all(u8::is_ascii_alphabetic)
    };
    parts.next().is_some_and(is_name_part)
        && parts.next().is_some_and(is_name_part)
        && parts.all(is_name_part)
}

fn is_short_source_particle(form: &kfind_morph::ParticleAllomorph) -> bool {
    form.rule_id.as_str() == "particle.source" && form.surface.as_ref() == "서"
}

fn is_structural_suffix_particle(
    form: &kfind_morph::ParticleAllomorph,
    branch: &CandidateProgram,
) -> bool {
    !is_short_source_particle(form)
        && !(form.rule_id.as_str() == "particle.comitative.hago"
            && matches!(
                branch.consumption,
                CandidateConsumption::PredicateContinuation { .. }
            ))
}

fn has_hago_predicate_frame(
    haystack: &[u8],
    token: &Range<usize>,
    resolver: &ConstraintResolver,
) -> bool {
    let Some(surface) = context::normalized_token(haystack, token.clone()) else {
        return false;
    };
    let Some(stem) = surface
        .strip_suffix("고요")
        .or_else(|| surface.strip_suffix('고'))
    else {
        return false;
    };
    let previous = context::previous_word(haystack, token.start);
    let next = context::next_word(haystack, token.end);
    (previous.as_deref().is_some_and(|word| {
        resolver.has_object_marked_nominal(word) || resolver.has_negative_connective_host(word)
    }) || next.as_deref().is_some_and(|word| {
        word.starts_with('있') && resolver.has_exact_non_copula_predicate_surface("있")
    })) && resolver.has_exact_action_predicate_surface(stem)
        || (previous
            .as_deref()
            .is_some_and(|word| resolver.has_subject_marked_nominal(word))
            || next
                .as_deref()
                .is_some_and(|word| resolver.has_subject_marked_nominal(word)))
            && resolver.has_exact_adjective_surface(stem)
}

fn starts_with_resultative_auxiliary(surface: &str) -> bool {
    surface
        .chars()
        .next()
        .is_some_and(|first| matches!(first, '지' | '졌' | '진' | '질' | '져' | '집' | '짐'))
}

fn continuation_nominal_host<'a>(rule_path: &[RuleId], consumed: &'a str) -> Option<&'a str> {
    let nominalized = matches!(
        rule_path.last().map(RuleId::as_str),
        Some("ending.nominalizer" | "ending.nominalizer-gi")
    );
    nominalized
        .then(|| {
            consumed
                .char_indices()
                .next_back()
                .map(|(start, _)| &consumed[start..])
        })
        .flatten()
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
