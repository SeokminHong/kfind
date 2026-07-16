use std::ops::Range;
use std::sync::{Arc, OnceLock};

use kfind_data::{ComponentResource, DataFinePos};

use super::unknown::UnknownDictionary;
use super::{
    LocalLatticeCosts, LocalLatticeDecision, LocalLatticeError, best_costs, build_nodes,
    validate_query_span,
};

#[derive(Debug)]
pub struct LocalComponentEvaluator {
    resource: Arc<ComponentResource>,
    unknown: OnceLock<Result<UnknownDictionary, LocalLatticeError>>,
}

impl LocalComponentEvaluator {
    #[must_use]
    pub fn new(resource: Arc<ComponentResource>) -> Self {
        Self {
            resource,
            unknown: OnceLock::new(),
        }
    }

    #[must_use]
    pub fn resource(&self) -> &ComponentResource {
        &self.resource
    }

    #[must_use]
    pub fn resource_arc(&self) -> Arc<ComponentResource> {
        Arc::clone(&self.resource)
    }

    pub fn evaluate_decision(
        &self,
        text: &str,
        query_span: Range<usize>,
        query_pos: DataFinePos,
        node_limit: usize,
    ) -> Result<LocalLatticeDecision, LocalLatticeError> {
        self.evaluate_costs(text, query_span, query_pos, node_limit)?
            .decision()
    }

    pub fn supports_component(
        &self,
        text: &str,
        query_span: Range<usize>,
        query_pos: DataFinePos,
        node_limit: usize,
        max_cost_penalty: u32,
    ) -> Result<bool, LocalLatticeError> {
        self.evaluate_costs(text, query_span, query_pos, node_limit)?
            .supports_query(max_cost_penalty)
    }

    fn evaluate_costs(
        &self,
        text: &str,
        query_span: Range<usize>,
        query_pos: DataFinePos,
        node_limit: usize,
    ) -> Result<LocalLatticeCosts, LocalLatticeError> {
        validate_query_span(text, &query_span)?;
        let nodes = build_nodes(
            self.resource.as_ref(),
            text,
            &query_span,
            query_pos,
            self.unknown()?,
            node_limit,
        )?;
        best_costs(self.resource.as_ref(), text.len(), &nodes)
    }

    fn unknown(&self) -> Result<&UnknownDictionary, LocalLatticeError> {
        self.unknown
            .get_or_init(|| UnknownDictionary::parse(self.resource.as_ref()))
            .as_ref()
            .map_err(Clone::clone)
    }
}
