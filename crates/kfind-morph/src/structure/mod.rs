use crate::structure::evidence::TokenEvidence;
use crate::structure::paths::nominal::copula_surface_begins_at;
use crate::structure::selection::{StructureSelection, select_structure};
use crate::structure::support::collect_pattern_supports;
use crate::{CandidateSpans, MorphContinuation, QueryMorphPattern, StructuralSignature};
use kfind_data::{ComponentResource, DataFinePos};
use std::ops::Range;
use std::sync::Arc;

mod evidence;
mod graph;
mod lexical;
mod paths;
mod probes;
mod selection;
mod support;

#[derive(Clone, Copy, Debug)]
pub struct BoundedTokenContext<'a> {
    pub previous: Option<&'a str>,
    pub current: &'a str,
    pub next: Option<&'a str>,
}

impl<'a> BoundedTokenContext<'a> {
    #[must_use]
    pub const fn current(current: &'a str) -> Self {
        Self {
            previous: None,
            current,
            next: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StructuralEvidence {
    Whole,
    SourceComponent,
    RuntimeComponent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstraintUnavailable {
    InvalidSpans,
    NodeLimit { actual: usize, limit: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstraintOutcome {
    Supported,
    Contradicted,
    Ambiguous,
    Unavailable(ConstraintUnavailable),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConstraintSupport {
    pub pattern_index: usize,
    pub evidence: StructuralEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConstraintDecision {
    pub outcome: ConstraintOutcome,
    pub supported: Vec<ConstraintSupport>,
}

impl ConstraintDecision {
    fn unavailable(reason: ConstraintUnavailable) -> Self {
        Self {
            outcome: ConstraintOutcome::Unavailable(reason),
            supported: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductPolicy {
    RecallFirst,
    Unambiguous,
}

impl ProductPolicy {
    #[must_use]
    pub fn accepts(self, decision: &ConstraintDecision) -> bool {
        !decision.supported.is_empty()
            && match self {
                Self::RecallFirst => matches!(
                    decision.outcome,
                    ConstraintOutcome::Supported | ConstraintOutcome::Ambiguous
                ),
                Self::Unambiguous => decision.outcome == ConstraintOutcome::Supported,
            }
    }
}

#[derive(Debug)]
pub struct ConstraintResolver {
    resource: Arc<ComponentResource>,
    attached_auxiliary: bool,
}

impl ConstraintResolver {
    #[must_use]
    pub fn new(resource: Arc<ComponentResource>) -> Self {
        Self {
            resource,
            attached_auxiliary: false,
        }
    }

    #[must_use]
    pub const fn with_attached_auxiliary(mut self, enabled: bool) -> Self {
        self.attached_auxiliary = enabled;
        self
    }

    #[must_use]
    pub fn resource(&self) -> &ComponentResource {
        &self.resource
    }

    #[must_use]
    pub fn resolve_candidate(
        &self,
        context: BoundedTokenContext<'_>,
        spans: CandidateSpans,
        patterns: &[QueryMorphPattern],
        node_limit: usize,
    ) -> ConstraintDecision {
        let include_attached_auxiliary = self.attached_auxiliary
            || patterns
                .iter()
                .any(|pattern| pattern.fine_pos == DataFinePos::Vx);
        let include_nominal_copula = patterns.iter().any(|pattern| pattern.fine_pos.is_nominal())
            && copula_surface_begins_at(context.current, spans.core.end);
        let include_nominal_derivation_predicate = patterns.iter().any(|pattern| {
            pattern.fine_pos.is_nominal()
                && matches!(pattern.continuation, MorphContinuation::NominalParticles)
                && pattern.component_capability.allows_runtime()
        });
        let prepared = match self.prepare_context_inner(
            context,
            node_limit,
            include_attached_auxiliary,
            include_nominal_copula,
            include_nominal_derivation_predicate,
        ) {
            Ok(prepared) => prepared,
            Err(reason) => return ConstraintDecision::unavailable(reason),
        };
        prepared.resolve_candidate(spans, patterns)
    }

    pub fn prepare_context(
        &self,
        context: BoundedTokenContext<'_>,
        node_limit: usize,
    ) -> Result<PreparedStructuralContext, ConstraintUnavailable> {
        self.prepare_context_inner(context, node_limit, self.attached_auxiliary, false, true)
    }

    pub fn prepare_context_for_candidate(
        &self,
        context: BoundedTokenContext<'_>,
        node_limit: usize,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
    ) -> Result<PreparedStructuralContext, ConstraintUnavailable> {
        self.prepare_context_inner(
            context,
            node_limit,
            self.attached_auxiliary,
            include_nominal_copula,
            include_nominal_derivation_predicate,
        )
    }

    fn prepare_context_inner(
        &self,
        context: BoundedTokenContext<'_>,
        node_limit: usize,
        include_attached_auxiliary: bool,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
    ) -> Result<PreparedStructuralContext, ConstraintUnavailable> {
        let token = self.prepare_token_graph_inner(
            context.current,
            node_limit,
            include_attached_auxiliary,
            include_nominal_copula,
            include_nominal_derivation_predicate,
        )?;
        let selection = select_structure(&self.resource, context, &token.evidence);
        Ok(PreparedStructuralContext {
            token: PreparedTokenGraphStorage::Owned(token),
            selection,
        })
    }

    pub fn prepare_token_graph_for_candidate(
        &self,
        current: &str,
        node_limit: usize,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
    ) -> Result<PreparedTokenGraph, ConstraintUnavailable> {
        self.prepare_token_graph_inner(
            current,
            node_limit,
            self.attached_auxiliary,
            include_nominal_copula,
            include_nominal_derivation_predicate,
        )
    }

    pub fn prepare_context_with_token_graph(
        &self,
        context: BoundedTokenContext<'_>,
        token: Arc<PreparedTokenGraph>,
    ) -> Result<PreparedStructuralContext, ConstraintUnavailable> {
        if token.text.as_ref() != context.current {
            return Err(ConstraintUnavailable::InvalidSpans);
        }
        let selection = select_structure(&self.resource, context, &token.evidence);
        Ok(PreparedStructuralContext {
            token: PreparedTokenGraphStorage::Shared(token),
            selection,
        })
    }

    fn prepare_token_graph_inner(
        &self,
        current: &str,
        node_limit: usize,
        include_attached_auxiliary: bool,
        include_nominal_copula: bool,
        include_nominal_derivation_predicate: bool,
    ) -> Result<PreparedTokenGraph, ConstraintUnavailable> {
        let evidence = TokenEvidence::collect(
            &self.resource,
            current,
            node_limit,
            include_attached_auxiliary,
            include_nominal_copula,
            include_nominal_derivation_predicate,
        )?;
        Ok(PreparedTokenGraph {
            text: current.into(),
            evidence,
        })
    }
}

#[derive(Debug)]
pub struct PreparedTokenGraph {
    text: Box<str>,
    evidence: TokenEvidence,
}

impl PreparedTokenGraph {
    #[must_use]
    pub fn memory_usage(&self) -> usize {
        std::mem::size_of::<Self>() + self.text.len() + self.evidence.memory_usage()
    }
}

#[derive(Debug)]
pub struct PreparedStructuralContext {
    token: PreparedTokenGraphStorage,
    selection: StructureSelection,
}

#[derive(Debug)]
// The inline owned variant preserves the allocation-free one-shot preparation path.
#[allow(clippy::large_enum_variant)]
enum PreparedTokenGraphStorage {
    Owned(PreparedTokenGraph),
    Shared(Arc<PreparedTokenGraph>),
}

impl PreparedTokenGraphStorage {
    fn as_ref(&self) -> &PreparedTokenGraph {
        match self {
            Self::Owned(token) => token,
            Self::Shared(token) => token,
        }
    }
}

impl PreparedStructuralContext {
    #[must_use]
    pub fn has_nominal_copula_host(&self, span: &Range<usize>) -> bool {
        self.token.as_ref().evidence.has_nominal_copula_host(span)
    }

    #[must_use]
    pub fn resolve_candidate(
        &self,
        spans: CandidateSpans,
        patterns: &[QueryMorphPattern],
    ) -> ConstraintDecision {
        let token = self.token.as_ref();
        if !spans.is_valid_for(&token.text)
            || spans.token != (0..token.text.len())
            || patterns.iter().any(|pattern| !pattern.is_well_formed())
        {
            return ConstraintDecision::unavailable(ConstraintUnavailable::InvalidSpans);
        }
        let raw = collect_pattern_supports(
            &token.evidence,
            &spans,
            patterns,
            self.selection.graph_nominal_host(),
        );
        if raw.is_empty() {
            return ConstraintDecision {
                outcome: ConstraintOutcome::Contradicted,
                supported: Vec::new(),
            };
        }
        let mut supported = raw
            .into_iter()
            .filter(|support| {
                self.selection
                    .accepts(support, &spans, patterns, &token.text, &token.evidence)
            })
            .collect::<Vec<_>>();
        supported.sort_unstable_by_key(|support| (support.pattern_index, support.evidence as u8));
        supported.dedup();
        if supported.is_empty() {
            return ConstraintDecision {
                outcome: ConstraintOutcome::Contradicted,
                supported,
            };
        }
        let signature_count = distinct_signature_count(&supported, patterns);
        ConstraintDecision {
            outcome: if signature_count > 1 {
                ConstraintOutcome::Ambiguous
            } else {
                ConstraintOutcome::Supported
            },
            supported,
        }
    }
}

fn distinct_signature_count(
    supports: &[ConstraintSupport],
    patterns: &[QueryMorphPattern],
) -> usize {
    let mut signatures = Vec::<StructuralSignature<'_>>::new();
    for support in supports {
        let signature = patterns[support.pattern_index].structural_signature();
        if !signatures.contains(&signature) {
            signatures.push(signature);
        }
    }
    signatures.len()
}

#[cfg(test)]
mod tests;
