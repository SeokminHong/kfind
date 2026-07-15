mod decode;
mod encode;

pub(in crate::component::graph) use decode::GraphPayloadLayout;
pub(in crate::component::graph) use encode::encode_graph_payload;

const ANALYSIS_BYTES: usize = 28;
const COMPONENT_BYTES: usize = 16;
const PAYLOAD_HEADER_BYTES: usize = 20;
const TRANSITION_BYTES: usize = 8;
const NO_SPAN: u32 = u32::MAX;
