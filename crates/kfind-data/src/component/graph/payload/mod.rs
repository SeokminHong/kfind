mod decode;
mod encode;

pub(in crate::component::graph) use decode::GraphPayloadLayout;
pub(in crate::component::graph) use encode::encode_graph_payload;

const ANALYSIS_BYTES: usize = 36;
const COMPONENT_BYTES: usize = 16;
const NO_SPAN: u32 = u32::MAX;
