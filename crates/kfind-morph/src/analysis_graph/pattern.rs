use std::ops::Range;
use std::sync::Arc;

use kfind_data::DataFinePos;

use crate::{ContinuationState, FinePos};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CandidateTokenRelation {
    Whole,
    PrefixWithContinuation,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ComponentCapability {
    WholeOnly,
    Source,
    SourceAndRuntime,
}

impl ComponentCapability {
    #[must_use]
    pub const fn allows_source(self) -> bool {
        matches!(self, Self::Source | Self::SourceAndRuntime)
    }

    #[must_use]
    pub const fn allows_runtime(self) -> bool {
        matches!(self, Self::SourceAndRuntime)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MorphContinuation {
    Exact,
    Predicate {
        state: ContinuationState,
        nominal_particles: bool,
    },
    NominalParticles,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AdjacentSide {
    Previous,
    Next,
    Either,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CopularFrameRole {
    Nominal,
    Copula,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AdjacentTokenConstraint {
    RepeatedToken { side: AdjacentSide },
    CopularFrame { role: CopularFrameRole },
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct QueryMorphPattern {
    pub fine_pos: DataFinePos,
    pub lexical_form: Arc<str>,
    pub token_relation: CandidateTokenRelation,
    pub continuation: MorphContinuation,
    pub component_capability: ComponentCapability,
    pub adjacent: Arc<[AdjacentTokenConstraint]>,
}

impl QueryMorphPattern {
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        !self.lexical_form.is_empty()
            && matches!(
                (self.token_relation, self.continuation),
                (CandidateTokenRelation::Whole, MorphContinuation::Exact)
                    | (
                        CandidateTokenRelation::PrefixWithContinuation,
                        MorphContinuation::Predicate { .. } | MorphContinuation::NominalParticles
                    )
            )
    }

    #[must_use]
    pub fn from_fine_pos(fine_pos: FinePos, lexical_form: &str) -> Vec<Self> {
        let fine_pos = match fine_pos {
            FinePos::CommonNoun => DataFinePos::Nng,
            FinePos::ProperNoun => DataFinePos::Nnp,
            FinePos::DependentNoun => DataFinePos::Nnb,
            FinePos::Pronoun => DataFinePos::Np,
            FinePos::Numeral => DataFinePos::Nr,
            FinePos::Verb => DataFinePos::Vv,
            FinePos::Adjective => {
                return vec![
                    Self::new(DataFinePos::Va, lexical_form),
                    Self::new(DataFinePos::Vcn, lexical_form),
                ];
            }
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
            | FinePos::Literal => return Vec::new(),
        };
        vec![Self::new(fine_pos, lexical_form)]
    }

    #[must_use]
    pub fn with_branch_contract(
        mut self,
        token_relation: CandidateTokenRelation,
        continuation: MorphContinuation,
        component_capability: ComponentCapability,
    ) -> Self {
        self.token_relation = token_relation;
        self.continuation = continuation;
        self.component_capability = component_capability;
        self
    }

    #[must_use]
    pub fn new(fine_pos: DataFinePos, lexical_form: &str) -> Self {
        Self {
            fine_pos,
            lexical_form: Arc::from(lexical_form),
            token_relation: CandidateTokenRelation::Whole,
            continuation: MorphContinuation::Exact,
            component_capability: ComponentCapability::WholeOnly,
            adjacent: default_adjacent_constraints(fine_pos),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateSpans {
    pub core: Range<usize>,
    pub anchor: Range<usize>,
    pub consumed: Range<usize>,
    pub token: Range<usize>,
}

impl CandidateSpans {
    #[must_use]
    pub fn is_valid_for(&self, text: &str) -> bool {
        valid_span(text, &self.core)
            && valid_span(text, &self.anchor)
            && valid_span(text, &self.consumed)
            && valid_span(text, &self.token)
            && contains(&self.anchor, &self.core)
            && contains(&self.consumed, &self.anchor)
            && contains(&self.token, &self.consumed)
    }
}

fn default_adjacent_constraints(fine_pos: DataFinePos) -> Arc<[AdjacentTokenConstraint]> {
    match fine_pos {
        DataFinePos::Mag => Arc::from([AdjacentTokenConstraint::RepeatedToken {
            side: AdjacentSide::Either,
        }]),
        DataFinePos::Nng
        | DataFinePos::Nnp
        | DataFinePos::Nnb
        | DataFinePos::Nr
        | DataFinePos::Np => Arc::from([AdjacentTokenConstraint::CopularFrame {
            role: CopularFrameRole::Nominal,
        }]),
        DataFinePos::Vcp => Arc::from([AdjacentTokenConstraint::CopularFrame {
            role: CopularFrameRole::Copula,
        }]),
        _ => Arc::from([]),
    }
}

fn valid_span(text: &str, span: &Range<usize>) -> bool {
    span.start < span.end
        && span.end <= text.len()
        && text.is_char_boundary(span.start)
        && text.is_char_boundary(span.end)
}

fn contains(outer: &Range<usize>, inner: &Range<usize>) -> bool {
    outer.start <= inner.start && inner.end <= outer.end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_query_pos_to_structural_patterns() {
        let patterns = QueryMorphPattern::from_fine_pos(FinePos::Adjective, "좋");

        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0].fine_pos, DataFinePos::Va);
        assert_eq!(patterns[1].fine_pos, DataFinePos::Vcn);
        assert_eq!(
            patterns[0].component_capability,
            ComponentCapability::WholeOnly
        );
    }

    #[test]
    fn assigns_context_selectors_without_lexical_surfaces() {
        let adverb = QueryMorphPattern::from_fine_pos(FinePos::GeneralAdverb, "빨리");
        let nominal = QueryMorphPattern::from_fine_pos(FinePos::CommonNoun, "학교");

        assert_eq!(
            adverb[0].adjacent.as_ref(),
            [AdjacentTokenConstraint::RepeatedToken {
                side: AdjacentSide::Either
            }]
        );
        assert_eq!(
            nominal[0].adjacent.as_ref(),
            [AdjacentTokenConstraint::CopularFrame {
                role: CopularFrameRole::Nominal
            }]
        );
    }

    #[test]
    fn validates_candidate_span_nesting() {
        let spans = CandidateSpans {
            core: 0..3,
            anchor: 0..6,
            consumed: 0..9,
            token: 0..9,
        };

        assert!(spans.is_valid_for("걸었다"));
        assert!(
            !CandidateSpans {
                token: 0..6,
                ..spans
            }
            .is_valid_for("걸었다")
        );
    }

    #[test]
    fn rejects_inconsistent_token_and_continuation_contracts() {
        let invalid = QueryMorphPattern::new(DataFinePos::Nng, "학교").with_branch_contract(
            CandidateTokenRelation::Whole,
            MorphContinuation::NominalParticles,
            ComponentCapability::WholeOnly,
        );

        assert!(!invalid.is_well_formed());
        assert!(QueryMorphPattern::new(DataFinePos::Nng, "학교").is_well_formed());
    }
}
