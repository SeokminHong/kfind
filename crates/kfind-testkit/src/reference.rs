use std::cmp::Reverse;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;
use std::sync::Arc;

use kfind_morph::{ParticleVerifier, RuleId, verify_predicate_continuation};
use kfind_query::{
    BranchEnvironment, BranchVerifier, CoreMapping, Origin, PhraseJoinError, PhraseMatch,
    QueryPlan, SurfaceBranch, VerifiedSpan, join_phrase_spans,
};
use unicode_general_category::{GeneralCategory, get_general_category};
use unicode_normalization::{UnicodeNormalization, is_nfc};

#[derive(Debug)]
pub struct ReferenceMatcher {
    plan: Arc<QueryPlan>,
    particle_verifier: ParticleVerifier,
}

impl ReferenceMatcher {
    pub fn new(plan: Arc<QueryPlan>) -> Result<Self, ReferenceMatcherBuildError> {
        if plan.atoms.is_empty() {
            return Err(ReferenceMatcherBuildError::EmptyPlan);
        }
        for (atom_index, atom) in plan.atoms.iter().enumerate() {
            if atom.branches.is_empty() {
                return Err(ReferenceMatcherBuildError::EmptyAtom { atom_index });
            }
            for (branch_index, branch) in atom.branches.iter().enumerate() {
                if branch.anchor.is_empty() {
                    return Err(ReferenceMatcherBuildError::EmptyAnchor {
                        atom_index,
                        branch_index,
                    });
                }
                if std::str::from_utf8(&branch.anchor).is_err() {
                    return Err(ReferenceMatcherBuildError::InvalidAnchorUtf8 {
                        atom_index,
                        branch_index,
                    });
                }
            }
        }
        Ok(Self {
            plan,
            particle_verifier: ParticleVerifier::default(),
        })
    }

    #[must_use]
    pub fn plan(&self) -> &Arc<QueryPlan> {
        &self.plan
    }

    pub fn find_at_with_meta(
        &self,
        text: &str,
        at: usize,
    ) -> Result<Option<PhraseMatch>, ReferenceMatcherError> {
        if at > text.len() {
            return Ok(None);
        }
        let atom_spans = self.collect_atom_spans(text, at);
        if self.plan.atoms.len() == 1 {
            let matched = atom_spans[0]
                .iter()
                .min_by_key(|span| span_order(span))
                .cloned();
            return Ok(matched.map(|span| PhraseMatch {
                span: span.token.clone(),
                atoms: vec![span],
            }));
        }

        let matches = join_phrase_spans(text, &atom_spans, self.plan.phrase_policy)
            .map_err(ReferenceMatcherError::PhraseJoin)?;
        Ok(matches
            .into_iter()
            .min_by_key(|matched| (matched.span.start, Reverse(matched.span.end))))
    }

    pub fn find_all_with_meta(
        &self,
        text: &str,
    ) -> Result<Vec<PhraseMatch>, ReferenceMatcherError> {
        let mut matches = Vec::new();
        let mut at = 0;
        while let Some(matched) = self.find_at_with_meta(text, at)? {
            at = matched.span.end;
            matches.push(matched);
        }
        Ok(matches)
    }

    fn collect_atom_spans(&self, text: &str, at: usize) -> Vec<Vec<VerifiedSpan>> {
        let mut atom_spans = vec![Vec::new(); self.plan.atoms.len()];
        for start in text
            .char_indices()
            .map(|(index, _)| index)
            .filter(|start| *start >= at)
        {
            for (atom_index, atom) in self.plan.atoms.iter().enumerate() {
                for branch in &atom.branches {
                    if let Some(span) = self.verify_branch(text, start, branch) {
                        atom_spans[atom_index].push(span);
                    }
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
        text: &str,
        start: usize,
        branch: &SurfaceBranch,
    ) -> Option<VerifiedSpan> {
        let anchor = std::str::from_utf8(&branch.anchor).ok()?;
        let anchor_end = start.checked_add(anchor.len())?;
        if !text.get(start..)?.starts_with(anchor) {
            return None;
        }
        let core = mapped_core(start..anchor_end, branch.core_mapping, anchor)?;
        let following = &text[anchor_end..];
        let normalized_anchor = (!is_nfc(anchor)).then(|| anchor.nfc().collect::<String>());
        let normalized_following = normalized_anchor
            .is_some()
            .then(|| following.nfc().collect::<String>());
        let verifier_anchor = normalized_anchor.as_deref().unwrap_or(anchor);
        let verifier_following = normalized_following.as_deref().unwrap_or(following);
        let (normalized_consumed, suffix_rules) = match &branch.verifier {
            BranchVerifier::Exact => (0, Vec::new()),
            BranchVerifier::Predicate {
                continuation,
                pos,
                environment,
                ..
            } => {
                if !accepts_environment(environment, text, start) {
                    return None;
                }
                let matched = verify_predicate_continuation(
                    *continuation,
                    *pos,
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
            BranchVerifier::DirectParticle { rule_id } => {
                if requires_direct_particle_host(branch)
                    && !self.accepts_direct_particle(text, start, anchor_end, rule_id)
                {
                    return None;
                }
                (0, Vec::new())
            }
        };
        let consumed = if let Some(normalized) = normalized_following.as_deref() {
            map_normalized_prefix(following, normalized, normalized_consumed)?
        } else {
            normalized_consumed
        };
        let token = start..anchor_end.checked_add(consumed)?;
        if !accepts_boundaries(text, &core, &token, branch) {
            return None;
        }
        Some(VerifiedSpan {
            core,
            token,
            origins: extend_origins(&branch.origins, &suffix_rules),
        })
    }

    fn accepts_direct_particle(
        &self,
        text: &str,
        start: usize,
        anchor_end: usize,
        rule_id: &RuleId,
    ) -> bool {
        let Some(anchor) = text.get(start..anchor_end) else {
            return false;
        };
        let Some(left) = text.get(..start) else {
            return false;
        };
        let normalized_anchor = anchor.nfc().collect::<String>();
        let normalized_left = left.nfc().collect::<String>();
        let Some(previous) = normalized_left.chars().next_back() else {
            return false;
        };
        if !is_reference_token_character(previous) {
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

fn requires_direct_particle_host(branch: &SurfaceBranch) -> bool {
    !branch.boundary.require_left && branch.boundary.require_right
}

fn accepts_environment(environment: &BranchEnvironment, text: &str, start: usize) -> bool {
    match environment {
        BranchEnvironment::Unrestricted => true,
        BranchEnvironment::ContractedAfterVowel {
            uncontracted_prefix,
        } => {
            let normalized_left = text[..start].nfc().collect::<String>();
            let normalized_prefix = uncontracted_prefix.nfc().collect::<String>();
            let Some(previous) = normalized_left.chars().next_back() else {
                return false;
            };
            if kfind_morph::has_final(previous) {
                return false;
            }
            normalized_left
                .strip_suffix(&normalized_prefix)
                .and_then(|host| host.chars().next_back())
                .is_none_or(|host_final| !kfind_morph::has_final(host_final))
        }
    }
}

fn mapped_core(
    anchor_span: Range<usize>,
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

fn accepts_boundaries(
    text: &str,
    core: &Range<usize>,
    token: &Range<usize>,
    branch: &SurfaceBranch,
) -> bool {
    let valid = token.start <= core.start
        && core.start <= core.end
        && core.end <= token.end
        && token.end <= text.len()
        && text.is_char_boundary(token.start)
        && text.is_char_boundary(token.end);
    valid
        && (!branch.boundary.require_left
            || text[..token.start]
                .chars()
                .next_back()
                .is_none_or(|character| !is_reference_token_character(character)))
        && (!branch.boundary.require_right
            || text[token.end..]
                .chars()
                .next()
                .is_none_or(|character| !is_reference_token_character(character)))
}

fn is_reference_token_character(character: char) -> bool {
    character == '_'
        || character.is_alphanumeric()
        || matches!(
            get_general_category(character),
            GeneralCategory::NonspacingMark
                | GeneralCategory::SpacingMark
                | GeneralCategory::EnclosingMark
        )
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

fn merge_duplicate_spans(spans: &mut Vec<VerifiedSpan>) {
    spans.sort_by_key(span_order);
    let mut merged = Vec::<VerifiedSpan>::with_capacity(spans.len());
    for span in spans.drain(..) {
        if let Some(previous) = merged
            .last_mut()
            .filter(|previous| previous.core == span.core && previous.token == span.token)
        {
            previous.origins.extend(span.origins);
            previous.origins.sort();
            previous.origins.dedup();
        } else {
            merged.push(span);
        }
    }
    *spans = merged;
}

fn span_order(span: &VerifiedSpan) -> (usize, Reverse<usize>, usize, Reverse<usize>) {
    (
        span.token.start,
        Reverse(span.token.end),
        span.core.start,
        Reverse(span.core.end),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceMatcherBuildError {
    EmptyPlan,
    EmptyAtom {
        atom_index: usize,
    },
    EmptyAnchor {
        atom_index: usize,
        branch_index: usize,
    },
    InvalidAnchorUtf8 {
        atom_index: usize,
        branch_index: usize,
    },
}

impl Display for ReferenceMatcherBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPlan => {
                formatter.write_str("a reference matcher requires at least one atom")
            }
            Self::EmptyAtom { atom_index } => {
                write!(
                    formatter,
                    "query atom {atom_index} has no reference branches"
                )
            }
            Self::EmptyAnchor {
                atom_index,
                branch_index,
            } => write!(
                formatter,
                "query atom {atom_index} branch {branch_index} has an empty anchor"
            ),
            Self::InvalidAnchorUtf8 {
                atom_index,
                branch_index,
            } => write!(
                formatter,
                "query atom {atom_index} branch {branch_index} has a non-UTF-8 anchor"
            ),
        }
    }
}

impl Error for ReferenceMatcherBuildError {}

#[derive(Debug)]
pub enum ReferenceMatcherError {
    PhraseJoin(PhraseJoinError),
}

impl Display for ReferenceMatcherError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhraseJoin(error) => write!(formatter, "reference phrase join failed: {error}"),
        }
    }
}

impl Error for ReferenceMatcherError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PhraseJoin(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kfind_matcher::MorphMatcher;
    use kfind_query::{CompileOptions, LexiconQueryAnalyzer, Lexicons, compile_query};

    const REPRESENTATIVE_CORPUS: &str = concat!(
        "길을 걸어 갔다. 사용자의 권한을 검증했다.\n",
        "전화를 걸어 보았다. 예쁜 꽃과 파란 하늘을 보았다.\n",
        "사용자권한은 별도 식별자다. 권한을 다시 검증했습니다.\n",
        "학생인 친구는 학교여서 가까운 길로 갔다. 집으로 돌아왔다.\n",
    );

    #[test]
    fn reference_backend_matches_optimized_results() {
        let analyzer = LexiconQueryAnalyzer::new(Arc::new(Lexicons::embedded().unwrap()));
        for query in [
            "걷다",
            "예쁘다",
            "n:사용자 n:권한",
            "n:권한 v:검증하다",
            "lit:걸어",
            "이다",
            "는",
            "로",
        ] {
            let plan = Arc::new(
                compile_query(query, &CompileOptions::default(), &analyzer)
                    .unwrap_or_else(|error| panic!("failed to compile {query:?}: {error}")),
            );
            let optimized = MorphMatcher::new(Arc::clone(&plan)).unwrap();
            let reference = ReferenceMatcher::new(plan).unwrap();

            assert_eq!(
                optimized.find_all_with_meta(REPRESENTATIVE_CORPUS.as_bytes()),
                reference.find_all_with_meta(REPRESENTATIVE_CORPUS).unwrap(),
                "optimized and reference results differ for {query:?}",
            );
        }
    }
}
