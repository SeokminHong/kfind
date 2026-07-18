use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;
use std::sync::Arc;

use grep_matcher::{LineMatchKind, LineTerminator, Match, Matcher, NoCaptures, NoError};
use kfind_data::{ComponentResource, DataFinePos};
use kfind_morph::{
    ConstraintResolver, DEFAULT_LATTICE_NODE_LIMIT, MorphContinuation, ParticleChainModel,
    ParticleVerifier, PredicateStemClass, ProductPolicy, RuleId,
    verify_copula_surface_after_nominal, verify_predicate_continuation,
};
use kfind_query::{
    CandidateConsumption, CandidateDecision, CandidateLeftContext, CandidateProgram, CoreMapping,
    Origin, PhraseMatch, QueryPlan, VerifiedSpan,
};
use unicode_normalization::{UnicodeNormalization, is_nfc};

use crate::boundary::{accepts_requirements, surrounding_token_span};
use crate::{AnchorBuildError, AnchorBuildLimits, AnchorEngine, AnchorHit};

mod candidates;
mod context;
mod phrase;

pub use candidates::LocalAnalysisCandidate;
use context::{
    PreparedStructuralContextAnalysis, PreparedStructuralContextCache, StructuralRequest,
};
use phrase::{PhraseMatchLimit, PhraseSelection, select_phrase_matches};

const MAX_CONSUMPTION_BYTES: usize = 256;
const ADNOMINAL_RULE_IDS: [&str; 4] = [
    "ending.present-adnominal",
    "ending.past-adnominal",
    "ending.future-adnominal",
    "ending.retrospective-adnominal",
];
#[derive(Default)]
struct StructuralCache {
    windows: HashMap<(usize, usize, bool, bool), Option<PreparedStructuralContextAnalysis>>,
    prepared_contexts: PreparedStructuralContextCache,
}

impl StructuralCache {
    fn clear_windows(&mut self) {
        self.windows.clear();
    }
}
/// A query-plan matcher backed by one shared set of unique anchors.
#[derive(Debug)]
pub struct MorphMatcher {
    plan: Arc<QueryPlan>,
    anchor_engine: AnchorEngine,
    anchor_programs: Vec<Box<[ProgramRef]>>,
    max_anchor_bytes: usize,
    is_line_local: bool,
    particle_verifier: ParticleVerifier,
    constraint_resolver: Option<ConstraintResolver>,
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

        let (anchors, anchor_programs) = unique_anchors(&plan);
        let max_anchor_bytes = anchors.iter().map(|anchor| anchor.len()).max().unwrap_or(0);
        let is_line_local = anchors.iter().all(|anchor| !anchor.contains(&b'\n'));
        let anchor_engine = AnchorEngine::new_with_limits(
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
        Ok(Self {
            plan,
            anchor_engine,
            anchor_programs,
            max_anchor_bytes,
            is_line_local,
            particle_verifier,
            constraint_resolver,
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
        if at > haystack.len() {
            return None;
        }
        if self.plan.atoms.len() == 1 {
            return self
                .find_single_atom_best(haystack, at, MatchMetadata::SpanOnly)
                .map(|span| span.token);
        }
        self.find_phrase_at_with_metadata(haystack, at, MatchMetadata::SpanOnly)
            .map(|matched| matched.span)
    }

    /// Finds the next non-overlapping query match with its atom metadata.
    #[must_use]
    pub fn find_at_with_meta(&self, haystack: &[u8], at: usize) -> Option<PhraseMatch> {
        if at > haystack.len() {
            return None;
        }
        if self.plan.atoms.len() == 1 {
            return self.find_single_atom_at(haystack, at);
        }
        self.find_phrase_at(haystack, at)
    }

    /// Recomputes all non-overlapping matches and morphology provenance.
    ///
    /// This is intended for matched lines that need JSON or explain metadata.
    #[must_use]
    pub fn find_all_with_meta(&self, haystack: &[u8]) -> Vec<PhraseMatch> {
        if self.plan.atoms.len() > 1 {
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
        if self.plan.atoms.len() > 1 {
            let selection = self.find_phrases_with_meta(haystack, PhraseMatchLimit::Bounded(limit));
            return if selection.limit_exceeded {
                Err(MatchLimitExceeded { limit })
            } else {
                Ok(selection.matches)
            };
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
            if matches.len() == limit {
                return Err(MatchLimitExceeded { limit });
            }
            at = matched.span.end;
            matches.push(matched);
        }
        Ok(matches)
    }

    fn find_phrases_with_meta(&self, haystack: &[u8], limit: PhraseMatchLimit) -> PhraseSelection {
        let text = phrase_join_text(haystack);
        let atom_spans = self.collect_atom_spans(haystack, 0, MatchMetadata::Provenance);
        select_phrase_matches(&text, &atom_spans, self.plan.phrase_policy, limit)
    }

    #[must_use]
    pub fn verification_counters(&self, haystack: &[u8]) -> VerificationCounters {
        let mut counters = VerificationCounters::default();
        let mut structural_windows = HashSet::new();
        let mut structural_cache = StructuralCache::default();
        for hit in self.anchor_engine.hits(haystack, 0) {
            counters.raw_anchor_hits += 1;
            for branch_ref in &self.anchor_programs[hit.anchor_index] {
                let branch =
                    &self.plan.atoms[branch_ref.atom_index].programs[branch_ref.program_index];
                let Some(candidate) = self.execute_program_without_decision(
                    haystack,
                    &hit,
                    branch,
                    MatchMetadata::SpanOnly,
                ) else {
                    continue;
                };
                if !self.accepts_program(haystack, &candidate, branch, &mut structural_cache) {
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

    fn find_single_atom_best_with_cache(
        &self,
        haystack: &[u8],
        at: usize,
        metadata: MatchMetadata,
        structural_cache: &mut StructuralCache,
    ) -> Option<VerifiedSpan> {
        let mut best = None;
        for hit in self.anchor_engine.hits(haystack, at) {
            if best.as_ref().is_some_and(|matched: &VerifiedSpan| {
                hit.span.end > matched.token.start.saturating_add(self.max_anchor_bytes)
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
        let text = phrase_join_text(haystack);
        let atom_spans = self.collect_atom_spans(haystack, at, metadata);
        select_phrase_matches(
            &text,
            &atom_spans,
            self.plan.phrase_policy,
            PhraseMatchLimit::First,
        )
        .matches
        .into_iter()
        .next()
    }

    fn collect_atom_spans(
        &self,
        haystack: &[u8],
        at: usize,
        metadata: MatchMetadata,
    ) -> Vec<Vec<VerifiedSpan>> {
        let mut atom_spans = vec![Vec::new(); self.plan.atoms.len()];
        let mut structural_cache = StructuralCache::default();
        for hit in self.anchor_engine.hits(haystack, at) {
            for branch_ref in &self.anchor_programs[hit.anchor_index] {
                let atom = &self.plan.atoms[branch_ref.atom_index];
                let branch = &atom.programs[branch_ref.program_index];
                if let Some(span) = self.execute_program_with_metadata(
                    haystack,
                    &hit,
                    branch,
                    metadata,
                    &mut structural_cache,
                ) {
                    atom_spans[branch_ref.atom_index].push(span);
                }
            }
        }
        for spans in &mut atom_spans {
            merge_duplicate_spans(spans);
        }
        atom_spans
    }

    fn execute_program_with_metadata(
        &self,
        haystack: &[u8],
        hit: &AnchorHit,
        branch: &CandidateProgram,
        metadata: MatchMetadata,
        structural_cache: &mut StructuralCache,
    ) -> Option<VerifiedSpan> {
        let candidate = self.execute_program_without_decision(haystack, hit, branch, metadata)?;
        self.accepts_program(haystack, &candidate, branch, structural_cache)
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
            CandidateConsumption::Anchor => (0, Vec::new()),
            CandidateConsumption::PredicateContinuation {
                continuation,
                pos,
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
                let matched = verify_predicate_continuation(
                    *continuation,
                    *pos,
                    &consumption_anchor,
                    &consumption_following,
                )?;
                if !branch.consumption.allows_rule_path(&matched.rule_path) {
                    return None;
                }
                let mut normalized_consumed = matched.consumed_bytes;
                let mut rule_path = matched.rule_path;
                if *nominal_particle_transition {
                    let particle = self.particle_verifier.verify_prefix(
                        &consumption_anchor,
                        &consumption_following[normalized_consumed..],
                    );
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
                let normalized_anchor = anchor.nfc().collect::<String>();
                let normalized_core_len = std::str::from_utf8(haystack.get(core.clone())?)
                    .ok()?
                    .nfc()
                    .collect::<String>()
                    .len();
                let suffix = normalized_token.get(normalized_anchor.len()..)?;
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

    fn accepts_program(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        structural_cache: &mut StructuralCache,
    ) -> bool {
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

    fn accepts_structural(
        &self,
        haystack: &[u8],
        candidate: &ExecutedCandidate,
        branch: &CandidateProgram,
        structural_cache: &mut StructuralCache,
    ) -> bool {
        let Some(resolver) = self.constraint_resolver.as_ref() else {
            return false;
        };
        let patterns = branch.structural_patterns();
        if patterns.is_empty() {
            return false;
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
        let licensed_trailing =
            self.licensed_structural_trailing(haystack, candidate, branch, resolver);
        if licensed_trailing.is_none()
            && has_conflicting_whole_predicate(haystack, candidate, branch, resolver)
        {
            return false;
        }
        let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
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
                && nominal_copula_surface_follows(haystack, candidate));
        let include_nominal_derivation_predicate = patterns.iter().any(|pattern| {
            pattern.fine_pos.is_nominal()
                && matches!(pattern.continuation, MorphContinuation::NominalParticles)
                && pattern.component_capability.allows_runtime()
        });
        let consumed = licensed_trailing.unwrap_or_else(|| candidate.consumed.clone());
        let rejected_suffix =
            self.has_rejected_structural_suffix(haystack, candidate, &consumed, branch, resolver);
        if rejected_suffix
            && !matches!(
                branch.consumption,
                CandidateConsumption::NominalParticleChain { .. }
            )
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
                    resolver,
                    DEFAULT_LATTICE_NODE_LIMIT,
                    include_nominal_copula,
                    include_nominal_derivation_predicate,
                    &mut structural_cache.prepared_contexts,
                )
            });
        let Some(context) = context.as_ref() else {
            return false;
        };
        if rejected_suffix && !context.has_nominal_copula_host(candidate.verified.core.clone()) {
            return false;
        }
        context
            .resolve(StructuralRequest {
                candidate: &candidate.verified,
                anchor: candidate.anchor.clone(),
                consumed,
                patterns,
            })
            .is_some_and(|decision| ProductPolicy::RecallFirst.accepts(&decision))
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
        let licensed = (matches!(trailing, "까" | "까요") && has_rule("ending.future-adnominal"))
            || (trailing == "서도" && has_rule("ending.connective-go"))
            || (trailing == "도" && has_rule("ending.connective-neunde"))
            || (trailing.starts_with("잖") && has_rule("ending.past"))
            || (trailing.starts_with(['아', '어', '여'])
                && has_rule("ending.past")
                && resolver.supports_ending_suffix_path(trailing, 0, DEFAULT_LATTICE_NODE_LIMIT))
            || (ADNOMINAL_RULE_IDS.iter().any(|rule| has_rule(rule))
                && ["때", "게"].iter().any(|noun| trailing.starts_with(noun)))
            || (candidate.suffix_rules.is_empty()
                && has_rule("ending.aoeo")
                && resolver.supports_auxiliary_sequence(trailing, DEFAULT_LATTICE_NODE_LIMIT))
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
        let ending_auxiliary_particles = !declarative_adnominal
            && haystack
                .get(candidate.consumed.clone())
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .is_some_and(|ending| {
                    let ending = ending.nfc().collect::<String>();
                    let trailing = trailing.nfc().collect::<String>();
                    self.particle_verifier
                        .verify_exact(&ending, &trailing)
                        .is_some_and(|matched| {
                            matched.rule_path.first().is_some_and(|rule| {
                                contains_rule(
                                    &self.plan.predicate_ending_initial_particle_rules,
                                    rule,
                                )
                            }) && matched.rule_path.iter().all(|rule| {
                                contains_rule(&self.plan.auxiliary_particle_rules, rule)
                            })
                        })
                });
        let has_rule = |expected: &str| {
            branch
                .origins
                .iter()
                .flat_map(|origin| &origin.rule_path)
                .chain(&candidate.suffix_rules)
                .any(|rule| rule.as_str() == expected)
        };
        let has_adnominal_rule = ADNOMINAL_RULE_IDS.iter().any(|rule| has_rule(rule));
        let adnominal_dependent_noun_particle =
            trailing.nfc().next() == Some('지') && has_adnominal_rule;
        let adnominal_interrogative = trailing.nfc().eq("가".chars()) && has_adnominal_rule;
        let licensed_non_ending_trailing = declarative_adnominal
            || ending_auxiliary_particles
            || adnominal_dependent_noun_particle
            || adnominal_interrogative;
        let source_aligned_compound = source_aligned_compound_predicate_position(
            haystack,
            candidate,
            whole,
            source_positions,
            resolver,
        );
        let valid_position = if pos == kfind_morph::PredicatePos::Copula {
            candidate.verified.core.start > whole.start
        } else {
            (continuation != kfind_morph::ContinuationState::Terminal
                || licensed_non_ending_trailing)
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

fn nominal_copula_surface_follows(haystack: &[u8], candidate: &ExecutedCandidate) -> bool {
    let whole = surrounding_token_span(haystack, candidate.verified.core.clone());
    haystack
        .get(candidate.verified.core.end..whole.end)
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
    if !internal_core
        && (candidate.anchor != candidate.verified.core
            || candidate.consumed != candidate.verified.core)
    {
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
        candidate,
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
    candidate: &ExecutedCandidate,
    whole: &Range<usize>,
    source_positions: kfind_morph::PredicatePosSet,
    resolver: &ConstraintResolver,
) -> bool {
    if candidate.verified.core.start <= whole.start || candidate.verified.core.end > whole.end {
        return false;
    }
    let Some(token) = haystack
        .get(whole.clone())
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
    else {
        return false;
    };
    let Some(prefix) = haystack
        .get(whole.start..candidate.verified.core.start)
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
    if class == PredicateStemClass::Consonant
        && [
            "니", "니까", "니깐", "며", "면서", "면", "려", "리", "세", "셔", "시", "십",
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
        Ok(self
            .anchor_engine
            .hits(haystack, 0)
            .next()
            .map(|hit| LineMatchKind::Candidate(hit.span.start)))
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

type AnchorsAndPrograms = (Vec<Box<[u8]>>, Vec<Box<[ProgramRef]>>);

fn unique_anchors(plan: &QueryPlan) -> AnchorsAndPrograms {
    let mut anchor_indices = HashMap::<Box<[u8]>, usize>::new();
    let mut anchors = Vec::<Box<[u8]>>::new();
    let mut program_lists = Vec::<Vec<ProgramRef>>::new();
    for (atom_index, atom) in plan.atoms.iter().enumerate() {
        for (program_index, branch) in atom.programs.iter().enumerate() {
            let anchor_index = if let Some(index) = anchor_indices.get(branch.anchor.as_ref()) {
                *index
            } else {
                let index = anchors.len();
                let anchor = branch.anchor.clone();
                anchor_indices.insert(anchor.clone(), index);
                anchors.push(anchor);
                program_lists.push(Vec::new());
                index
            };
            program_lists[anchor_index].push(ProgramRef {
                atom_index,
                program_index,
            });
        }
    }
    (
        anchors,
        program_lists
            .into_iter()
            .map(Vec::into_boxed_slice)
            .collect(),
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

fn merge_duplicate_spans(spans: &mut Vec<VerifiedSpan>) {
    spans.sort_by_key(|span| {
        (
            span.token.start,
            span.token.end,
            span.core.start,
            span.core.end,
        )
    });
    let mut merged = Vec::<VerifiedSpan>::with_capacity(spans.len());
    for span in spans.drain(..) {
        if let Some(previous) = merged
            .last_mut()
            .filter(|previous| same_span(previous, &span))
        {
            merge_origins(&mut previous.origins, span.origins);
        } else {
            merged.push(span);
        }
    }
    *spans = merged;
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
