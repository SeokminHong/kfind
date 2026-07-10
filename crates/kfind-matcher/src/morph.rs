use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;
use std::sync::Arc;

use grep_matcher::{Match, Matcher, NoCaptures, NoError};
use kfind_morph::{ParticleVerifier, RuleId, verify_predicate_continuation};
use kfind_query::{
    BranchVerifier, CoreMapping, Origin, PhraseMatch, QueryPlan, SurfaceBranch, VerifiedSpan,
    join_phrase_spans,
};
use unicode_normalization::{UnicodeNormalization, is_nfc};

use crate::boundary::accepts_requirements;
use crate::{AnchorBuildError, AnchorBuildLimits, AnchorEngine, AnchorHit};

const MAX_VERIFIER_BYTES: usize = 256;

/// A query-plan matcher backed by one shared set of unique anchors.
#[derive(Debug)]
pub struct MorphMatcher {
    plan: Arc<QueryPlan>,
    anchor_engine: AnchorEngine,
    anchor_branches: Vec<Box<[BranchRef]>>,
    particle_verifier: ParticleVerifier,
}

impl MorphMatcher {
    pub fn new(plan: Arc<QueryPlan>) -> Result<Self, MorphMatcherBuildError> {
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
        let anchor_engine = AnchorEngine::new_with_limits(
            &anchors,
            AnchorBuildLimits {
                max_anchors: plan.limits.max_branches,
                max_memory_bytes: plan.limits.max_matcher_bytes,
            },
        )?;
        Ok(Self {
            plan,
            anchor_engine,
            anchor_branches,
            particle_verifier: ParticleVerifier::default(),
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
        let mut matches = Vec::new();
        let mut at = 0;
        while let Some(matched) = self.find_at_with_meta(haystack, at) {
            at = matched.span.end;
            matches.push(matched);
        }
        matches
    }

    fn find_single_atom_at(&self, haystack: &[u8], at: usize) -> Option<PhraseMatch> {
        let mut best = None;
        for hit in self.anchor_engine.hits(haystack, at) {
            for branch_ref in &self.anchor_branches[hit.anchor_index] {
                if branch_ref.atom_index != 0 {
                    continue;
                }
                let branch = &self.plan.atoms[0].branches[branch_ref.branch_index];
                let Some(candidate) = self.verify_branch(haystack, &hit, branch) else {
                    continue;
                };
                merge_best_span(&mut best, candidate);
            }
        }
        best.map(|span| PhraseMatch {
            span: span.token.clone(),
            atoms: vec![span],
        })
    }

    fn find_phrase_at(&self, haystack: &[u8], at: usize) -> Option<PhraseMatch> {
        let text = std::str::from_utf8(haystack).ok()?;
        let atom_spans = self.collect_atom_spans(haystack, at);
        let matches = join_phrase_spans(text, &atom_spans, self.plan.phrase_policy).ok()?;
        matches.into_iter().min_by(compare_phrase_matches)
    }

    fn collect_atom_spans(&self, haystack: &[u8], at: usize) -> Vec<Vec<VerifiedSpan>> {
        let mut atom_spans = vec![Vec::new(); self.plan.atoms.len()];
        for hit in self.anchor_engine.hits(haystack, at) {
            for branch_ref in &self.anchor_branches[hit.anchor_index] {
                let atom = &self.plan.atoms[branch_ref.atom_index];
                let branch = &atom.branches[branch_ref.branch_index];
                if let Some(span) = self.verify_branch(haystack, &hit, branch) {
                    atom_spans[branch_ref.atom_index].push(span);
                }
            }
        }
        for spans in &mut atom_spans {
            merge_duplicate_spans(spans);
        }
        atom_spans
    }

    fn verify_branch(
        &self,
        haystack: &[u8],
        hit: &AnchorHit,
        branch: &SurfaceBranch,
    ) -> Option<VerifiedSpan> {
        let anchor = std::str::from_utf8(haystack.get(hit.span.clone())?).ok()?;
        let core = mapped_core(&hit.span, branch.core_mapping, anchor)?;
        let following = valid_utf8_prefix(&haystack[hit.span.end..]);
        let normalized_anchor = (!is_nfc(anchor)).then(|| anchor.nfc().collect::<String>());
        let normalized_following = normalized_anchor
            .is_some()
            .then(|| following.nfc().collect::<String>());
        let verifier_anchor = normalized_anchor.as_deref().unwrap_or(anchor);
        let verifier_following = normalized_following.as_deref().unwrap_or(following);
        let (normalized_consumed_bytes, suffix_rules) = match &branch.verifier {
            BranchVerifier::Exact => (0, Vec::new()),
            BranchVerifier::Predicate { continuation, .. } => {
                let matched = verify_predicate_continuation(
                    *continuation,
                    verifier_anchor,
                    verifier_following,
                )?;
                if !branch.verifier.accepts_rule_path(&matched.rule_path) {
                    return None;
                }
                (matched.consumed_bytes, matched.rule_path)
            }
            BranchVerifier::NominalParticles { .. } => {
                let matched = self
                    .particle_verifier
                    .verify_prefix(verifier_anchor, verifier_following);
                if !branch.verifier.accepts_rule_path(&matched.rule_path) {
                    return None;
                }
                (matched.consumed_bytes, matched.rule_path)
            }
        };
        let consumed_bytes = if let Some(normalized) = normalized_following.as_deref() {
            map_normalized_prefix(following, normalized, normalized_consumed_bytes)?
        } else {
            normalized_consumed_bytes
        };
        let token_end = hit.span.end.checked_add(consumed_bytes)?;
        if token_end > haystack.len() || !is_utf8_boundary(haystack, token_end) {
            return None;
        }
        let token = hit.span.start..token_end;
        if !accepts_requirements(
            haystack,
            core.clone(),
            token.clone(),
            branch.boundary.require_left,
            branch.boundary.require_right,
        ) {
            return None;
        }

        Some(VerifiedSpan {
            core,
            token,
            origins: extend_origins(&branch.origins, &suffix_rules),
        })
    }
}

impl Matcher for MorphMatcher {
    type Captures = NoCaptures;
    type Error = NoError;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>, Self::Error> {
        Ok(self
            .find_at_with_meta(haystack, at)
            .map(|matched| Match::new(matched.span.start, matched.span.end)))
    }

    fn new_captures(&self) -> Result<Self::Captures, Self::Error> {
        Ok(NoCaptures::new())
    }
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
