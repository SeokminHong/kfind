use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;
use std::sync::Arc;

use grep_matcher::{LineMatchKind, LineTerminator, Match, Matcher, NoCaptures, NoError};
use kfind_data::{ComponentResource, DataFinePos};
use kfind_morph::{
    DEFAULT_LATTICE_NODE_LIMIT, FinePos, LocalLatticeDecision, ParticleChainModel,
    ParticleVerifier, RuleId, evaluate_local_component_paths, verify_predicate_continuation,
};
use kfind_query::{
    BranchEnvironment, BranchVerifier, ContextRequirement, CoreMapping, Origin, PhraseMatch,
    QueryPlan, SurfaceBranch, VerifiedSpan, join_phrase_spans,
};
use unicode_normalization::{UnicodeNormalization, is_nfc};

use crate::boundary::{accepts_requirements, surrounding_token_span};
use crate::{AnchorBuildError, AnchorBuildLimits, AnchorEngine, AnchorHit};

mod candidates;

pub use candidates::LocalAnalysisCandidate;

const MAX_VERIFIER_BYTES: usize = 256;

/// A query-plan matcher backed by one shared set of unique anchors.
#[derive(Debug)]
pub struct MorphMatcher {
    plan: Arc<QueryPlan>,
    anchor_engine: AnchorEngine,
    anchor_branches: Vec<Box<[BranchRef]>>,
    max_anchor_bytes: usize,
    is_line_local: bool,
    particle_verifier: ParticleVerifier,
    component_resource: Option<Arc<ComponentResource>>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VerificationCounters {
    pub raw_anchor_hits: usize,
    pub verified_branch_hits: usize,
    pub local_lattice_candidate_hits: usize,
    pub unique_analysis_windows: usize,
    pub nominal_component_candidate_hits: usize,
    pub unique_component_windows: usize,
}

impl MorphMatcher {
    pub fn new(plan: Arc<QueryPlan>) -> Result<Self, MorphMatcherBuildError> {
        Self::build(plan, None)
    }

    pub fn with_component_resource(
        plan: Arc<QueryPlan>,
        component_resource: Arc<ComponentResource>,
    ) -> Result<Self, MorphMatcherBuildError> {
        Self::build(plan, Some(component_resource))
    }

    fn build(
        plan: Arc<QueryPlan>,
        component_resource: Option<Arc<ComponentResource>>,
    ) -> Result<Self, MorphMatcherBuildError> {
        if plan.atoms.is_empty() {
            return Err(MorphMatcherBuildError::EmptyPlan);
        }
        if let Some((atom_index, _)) = plan
            .atoms
            .iter()
            .enumerate()
            .find(|(_, atom)| atom.branches.is_empty())
        {
            return Err(MorphMatcherBuildError::EmptyAtom { atom_index });
        }

        let (anchors, anchor_branches) = unique_anchors(&plan);
        let max_anchor_bytes = anchors.iter().map(|anchor| anchor.len()).max().unwrap_or(0);
        let is_line_local = anchors.iter().all(|anchor| !anchor.contains(&b'\n'));
        let anchor_engine = AnchorEngine::new_with_limits(
            &anchors,
            AnchorBuildLimits {
                max_anchors: plan.limits.max_branches,
                max_memory_bytes: plan.limits.max_matcher_bytes,
            },
        )?;
        let particle_verifier = ParticleVerifier::new(ParticleChainModel {
            transitions: Arc::clone(&plan.particle_transitions),
            ..ParticleChainModel::default()
        });
        Ok(Self {
            plan,
            anchor_engine,
            anchor_branches,
            max_anchor_bytes,
            is_line_local,
            particle_verifier,
            component_resource,
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
            return self.find_all_phrases_with_meta(haystack);
        }
        let mut matches = Vec::new();
        let mut at = 0;
        while let Some(matched) = self.find_at_with_meta(haystack, at) {
            at = matched.span.end;
            matches.push(matched);
        }
        matches
    }

    fn find_all_phrases_with_meta(&self, haystack: &[u8]) -> Vec<PhraseMatch> {
        let text = phrase_join_text(haystack);
        let atom_spans = self.collect_atom_spans(haystack, 0, MatchMetadata::Provenance);
        let Ok(mut candidates) = join_phrase_spans(&text, &atom_spans, self.plan.phrase_policy)
        else {
            return Vec::new();
        };
        candidates.sort_by(compare_phrase_matches);

        let mut matches = Vec::new();
        let mut at = 0;
        for matched in candidates {
            if matched.span.start < at {
                continue;
            }
            at = matched.span.end;
            matches.push(matched);
        }
        matches
    }

    #[must_use]
    pub fn verification_counters(&self, haystack: &[u8]) -> VerificationCounters {
        let mut counters = VerificationCounters::default();
        let mut analysis_windows = HashSet::new();
        let mut component_windows = HashSet::new();
        for hit in self.anchor_engine.hits(haystack, 0) {
            counters.raw_anchor_hits += 1;
            for branch_ref in &self.anchor_branches[hit.anchor_index] {
                let branch =
                    &self.plan.atoms[branch_ref.atom_index].branches[branch_ref.branch_index];
                let Some(candidate) = self.verify_branch_without_boundary(
                    haystack,
                    &hit,
                    branch,
                    MatchMetadata::SpanOnly,
                ) else {
                    continue;
                };
                if !self.accepts_token_boundary(haystack, &candidate, branch) {
                    if branch.context_requirement == ContextRequirement::NominalComponent {
                        counters.nominal_component_candidate_hits += 1;
                        let window = surrounding_token_span(haystack, candidate.core);
                        component_windows.insert((window.start, window.end));
                    }
                    continue;
                }
                counters.verified_branch_hits += 1;
                if branch.context_requirement == ContextRequirement::EojeolLattice {
                    counters.local_lattice_candidate_hits += 1;
                    let window = surrounding_token_span(haystack, candidate.token);
                    analysis_windows.insert((window.start, window.end));
                }
            }
        }
        counters.unique_analysis_windows = analysis_windows.len();
        counters.unique_component_windows = component_windows.len();
        counters
    }

    fn find_single_atom_at(&self, haystack: &[u8], at: usize) -> Option<PhraseMatch> {
        self.find_single_atom_best(haystack, at, MatchMetadata::Provenance)
            .map(|span| PhraseMatch {
                span: span.token.clone(),
                atoms: vec![span],
            })
    }

    fn find_single_atom_best(
        &self,
        haystack: &[u8],
        at: usize,
        metadata: MatchMetadata,
    ) -> Option<VerifiedSpan> {
        let mut best = None;
        for hit in self.anchor_engine.hits(haystack, at) {
            if best.as_ref().is_some_and(|matched: &VerifiedSpan| {
                hit.span.end > matched.token.start.saturating_add(self.max_anchor_bytes)
            }) {
                break;
            }
            for branch_ref in &self.anchor_branches[hit.anchor_index] {
                if branch_ref.atom_index != 0 {
                    continue;
                }
                let branch = &self.plan.atoms[0].branches[branch_ref.branch_index];
                let Some(candidate) = self.verify_branch_with_metadata(
                    haystack,
                    &hit,
                    branch_ref.atom_index,
                    branch,
                    metadata,
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
        let matches = join_phrase_spans(&text, &atom_spans, self.plan.phrase_policy).ok()?;
        matches.into_iter().min_by(compare_phrase_matches)
    }

    fn collect_atom_spans(
        &self,
        haystack: &[u8],
        at: usize,
        metadata: MatchMetadata,
    ) -> Vec<Vec<VerifiedSpan>> {
        let mut atom_spans = vec![Vec::new(); self.plan.atoms.len()];
        for hit in self.anchor_engine.hits(haystack, at) {
            for branch_ref in &self.anchor_branches[hit.anchor_index] {
                let atom = &self.plan.atoms[branch_ref.atom_index];
                let branch = &atom.branches[branch_ref.branch_index];
                if let Some(span) = self.verify_branch_with_metadata(
                    haystack,
                    &hit,
                    branch_ref.atom_index,
                    branch,
                    metadata,
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

    fn verify_branch_with_metadata(
        &self,
        haystack: &[u8],
        hit: &AnchorHit,
        atom_index: usize,
        branch: &SurfaceBranch,
        metadata: MatchMetadata,
    ) -> Option<VerifiedSpan> {
        let candidate = self.verify_branch_without_boundary(haystack, hit, branch, metadata)?;
        self.accepts_branch(haystack, &candidate, atom_index, branch)
            .then_some(candidate)
    }

    fn verify_branch_without_boundary(
        &self,
        haystack: &[u8],
        hit: &AnchorHit,
        branch: &SurfaceBranch,
        metadata: MatchMetadata,
    ) -> Option<VerifiedSpan> {
        let anchor = std::str::from_utf8(haystack.get(hit.span.clone())?).ok()?;
        let core = mapped_core(&hit.span, branch.core_mapping, anchor)?;
        let (consumed_bytes, suffix_rules) = match &branch.verifier {
            BranchVerifier::Exact => (0, Vec::new()),
            BranchVerifier::Predicate {
                continuation,
                pos,
                environment,
                ..
            } => {
                if !accepts_environment(environment, haystack, hit.span.start) {
                    return None;
                }
                let following = valid_utf8_prefix(&haystack[hit.span.end..]);
                let (verifier_anchor, verifier_following) =
                    normalized_verifier_text(anchor, following);
                let matched = verify_predicate_continuation(
                    *continuation,
                    *pos,
                    &verifier_anchor,
                    &verifier_following,
                )?;
                if !branch.verifier.accepts_rule_path(&matched.rule_path) {
                    return None;
                }
                let consumed_bytes = match &verifier_following {
                    Cow::Borrowed(_) => matched.consumed_bytes,
                    Cow::Owned(normalized) => {
                        map_normalized_prefix(following, normalized, matched.consumed_bytes)?
                    }
                };
                (consumed_bytes, matched.rule_path)
            }
            BranchVerifier::NominalParticles { .. } => {
                let following = valid_utf8_prefix(&haystack[hit.span.end..]);
                let (verifier_anchor, verifier_following) =
                    normalized_verifier_text(anchor, following);
                let matched = self
                    .particle_verifier
                    .verify_prefix(&verifier_anchor, &verifier_following);
                if !branch.verifier.accepts_rule_path(&matched.rule_path) {
                    return None;
                }
                let consumed_bytes = match &verifier_following {
                    Cow::Borrowed(_) => matched.consumed_bytes,
                    Cow::Owned(normalized) => {
                        map_normalized_prefix(following, normalized, matched.consumed_bytes)?
                    }
                };
                (consumed_bytes, matched.rule_path)
            }
            BranchVerifier::DirectParticle { rule_id } => {
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
        Some(VerifiedSpan {
            core,
            token,
            origins: match metadata {
                MatchMetadata::SpanOnly => Vec::new(),
                MatchMetadata::Provenance => extend_origins(&branch.origins, &suffix_rules),
            },
        })
    }

    fn accepts_branch(
        &self,
        haystack: &[u8],
        candidate: &VerifiedSpan,
        atom_index: usize,
        branch: &SurfaceBranch,
    ) -> bool {
        self.accepts_token_boundary(haystack, candidate, branch)
            || (branch.context_requirement == ContextRequirement::NominalComponent
                && self.accepts_nominal_component(haystack, candidate, atom_index, branch))
    }

    fn accepts_token_boundary(
        &self,
        haystack: &[u8],
        candidate: &VerifiedSpan,
        branch: &SurfaceBranch,
    ) -> bool {
        accepts_requirements(
            haystack,
            candidate.core.clone(),
            candidate.token.clone(),
            branch.boundary.require_left,
            branch.boundary.require_right,
        )
    }

    fn accepts_nominal_component(
        &self,
        haystack: &[u8],
        candidate: &VerifiedSpan,
        atom_index: usize,
        branch: &SurfaceBranch,
    ) -> bool {
        let Some(resource) = &self.component_resource else {
            return false;
        };
        let Ok(window) = crate::AnalysisWindow::extract(
            haystack,
            candidate.core.clone(),
            crate::DEFAULT_ANALYSIS_WINDOW_LIMITS,
        ) else {
            return false;
        };
        let Some(query_span) = window.normalized_span(candidate.core.clone()) else {
            return false;
        };
        if self.has_rejected_particle_suffix(candidate, window.normalized(), &query_span) {
            return false;
        }
        let Some(atom) = self.plan.atoms.get(atom_index) else {
            return false;
        };
        let query_positions = branch
            .origins
            .iter()
            .filter_map(|origin| atom.analyses.get(usize::from(origin.analysis_index)))
            .filter_map(|analysis| component_pos(analysis.fine_pos))
            .collect::<HashSet<_>>();
        query_positions.into_iter().any(|query_pos| {
            evaluate_local_component_paths(
                resource.as_ref(),
                window.normalized(),
                query_span.clone(),
                query_pos,
                DEFAULT_LATTICE_NODE_LIMIT,
            )
            .is_ok_and(|report| report.decision == LocalLatticeDecision::Accept)
        })
    }

    fn has_rejected_particle_suffix(
        &self,
        candidate: &VerifiedSpan,
        normalized_window: &str,
        query_span: &Range<usize>,
    ) -> bool {
        if candidate.token.end != candidate.core.end {
            return false;
        }
        let Some(suffix) = normalized_window.get(query_span.end..) else {
            return false;
        };
        self.particle_verifier
            .model()
            .allomorphs
            .iter()
            .any(|form| suffix == form.surface.as_ref())
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

fn component_pos(pos: FinePos) -> Option<DataFinePos> {
    Some(match pos {
        FinePos::CommonNoun => DataFinePos::Nng,
        FinePos::ProperNoun => DataFinePos::Nnp,
        FinePos::DependentNoun => DataFinePos::Nnb,
        FinePos::Pronoun => DataFinePos::Np,
        FinePos::Numeral => DataFinePos::Nr,
        FinePos::Verb => DataFinePos::Vv,
        FinePos::Adjective => DataFinePos::Va,
        FinePos::AuxiliaryVerb | FinePos::AuxiliaryAdjective => DataFinePos::Vx,
        FinePos::Copula => DataFinePos::Vcp,
        FinePos::Determiner => DataFinePos::Mm,
        FinePos::GeneralAdverb => DataFinePos::Mag,
        FinePos::ConjunctiveAdverb => DataFinePos::Maj,
        FinePos::Interjection => DataFinePos::Ic,
        FinePos::Particle
        | FinePos::Foreign
        | FinePos::Number
        | FinePos::Code
        | FinePos::Literal => return None,
    })
}

fn normalized_verifier_text<'a>(
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

fn requires_direct_particle_host(branch: &SurfaceBranch) -> bool {
    !branch.boundary.require_left && branch.boundary.require_right
}

fn accepts_environment(
    environment: &BranchEnvironment,
    haystack: &[u8],
    anchor_start: usize,
) -> bool {
    match environment {
        BranchEnvironment::Unrestricted => true,
        BranchEnvironment::ContractedAfterVowel {
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

#[derive(Debug, Clone, Copy)]
struct BranchRef {
    atom_index: usize,
    branch_index: usize,
}

type AnchorsAndBranches = (Vec<Box<[u8]>>, Vec<Box<[BranchRef]>>);

fn unique_anchors(plan: &QueryPlan) -> AnchorsAndBranches {
    let mut anchor_indices = HashMap::<Box<[u8]>, usize>::new();
    let mut anchors = Vec::<Box<[u8]>>::new();
    let mut branch_lists = Vec::<Vec<BranchRef>>::new();
    for (atom_index, atom) in plan.atoms.iter().enumerate() {
        for (branch_index, branch) in atom.branches.iter().enumerate() {
            let anchor_index = if let Some(index) = anchor_indices.get(branch.anchor.as_ref()) {
                *index
            } else {
                let index = anchors.len();
                let anchor = branch.anchor.clone();
                anchor_indices.insert(anchor.clone(), index);
                anchors.push(anchor);
                branch_lists.push(Vec::new());
                index
            };
            branch_lists[anchor_index].push(BranchRef {
                atom_index,
                branch_index,
            });
        }
    }
    (
        anchors,
        branch_lists
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
    let mut end = bytes.len().min(MAX_VERIFIER_BYTES);
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
    let mut remaining = &bytes[bytes.len().saturating_sub(MAX_VERIFIER_BYTES)..];
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

fn compare_phrase_matches(left: &PhraseMatch, right: &PhraseMatch) -> std::cmp::Ordering {
    (left.span.start, std::cmp::Reverse(left.span.end))
        .cmp(&(right.span.start, std::cmp::Reverse(right.span.end)))
}

#[derive(Debug)]
pub enum MorphMatcherBuildError {
    EmptyPlan,
    EmptyAtom { atom_index: usize },
    Anchor(AnchorBuildError),
}

impl Display for MorphMatcherBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPlan => {
                formatter.write_str("a morphology matcher requires at least one atom")
            }
            Self::EmptyAtom { atom_index } => {
                write!(formatter, "query atom {atom_index} has no search branches")
            }
            Self::Anchor(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for MorphMatcherBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Anchor(error) => Some(error),
            Self::EmptyPlan | Self::EmptyAtom { .. } => None,
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
