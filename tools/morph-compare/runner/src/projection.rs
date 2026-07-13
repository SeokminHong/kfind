use kfind_matcher::MorphMatcher;
use serde::Serialize;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(super) struct PolicyCandidate {
    atom_index: usize,
    core: ByteSpan,
    token: ByteSpan,
    origins: Vec<PolicyOrigin>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct PolicyOrigin {
    analysis_index: u16,
    rule_path: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct ByteSpan {
    byte_start: usize,
    byte_end: usize,
}

pub(super) fn policy_candidates(matcher: &MorphMatcher, text: &str) -> Vec<PolicyCandidate> {
    matcher
        .diagnostic_atom_candidates(text.as_bytes())
        .into_iter()
        .enumerate()
        .flat_map(|(atom_index, spans)| {
            spans.into_iter().map(move |span| PolicyCandidate {
                atom_index,
                core: ByteSpan {
                    byte_start: span.core.start,
                    byte_end: span.core.end,
                },
                token: ByteSpan {
                    byte_start: span.token.start,
                    byte_end: span.token.end,
                },
                origins: span
                    .origins
                    .into_iter()
                    .map(|origin| PolicyOrigin {
                        analysis_index: origin.analysis_index,
                        rule_path: origin
                            .rule_path
                            .into_iter()
                            .map(|rule| rule.as_str().to_owned())
                            .collect(),
                    })
                    .collect(),
            })
        })
        .collect()
}
