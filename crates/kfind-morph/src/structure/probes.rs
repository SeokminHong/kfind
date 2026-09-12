use crate::structure::ConstraintResolver;
use crate::structure::evidence::TokenEvidence;
use crate::structure::graph::EdgeGraph;
use crate::structure::lexical::{
    complete_dependent_noun_particle_suffix, complete_nominal_host, complete_suffix,
    predicate_pos_matches, structural_predicate_pos_matches,
};
use crate::structure::paths::predicate::{
    has_complete_attached_auxiliary_path, is_attached_auxiliary_whole_path,
};
use crate::{PredicatePos, PredicatePosSet};
use kfind_data::ComponentPos as StructuralPos;
use std::ops::Range;

impl ConstraintResolver {
    #[must_use]
    pub fn has_whole_modifier(&self, text: &str) -> bool {
        let mut matched = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length == text.len() {
                    matched |= positions.first().is_some_and(|pos| {
                        matches!(
                            *pos,
                            StructuralPos::MM | StructuralPos::MAG | StructuralPos::MAJ
                        )
                    });
                }
            });
        matched
    }

    #[must_use]
    pub fn supports_predicate_ending_path(
        &self,
        text: &str,
        anchor_len: usize,
        pos: PredicatePos,
        node_limit: usize,
    ) -> bool {
        self.supports_predicate_ending_path_with_terminal(
            text, anchor_len, pos, node_limit, None, false,
        )
    }

    #[must_use]
    pub fn has_exact_predicate_ending_path(&self, text: &str, pos: PredicatePos) -> bool {
        let mut matched = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length != text.len() {
                    return;
                }
                let Some((first, endings)) = positions.split_first() else {
                    return;
                };
                matched |= structural_predicate_pos_matches(*first, pos)
                    && !endings.is_empty()
                    && endings.iter().all(|position| position.is_ending());
            });
        matched
    }

    #[must_use]
    pub fn has_exact_pronoun_copula_ending_path(&self, text: &str) -> bool {
        let mut matched = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length != text.len() {
                    return;
                }
                matched |= positions.len() >= 3
                    && positions[0] == StructuralPos::NP
                    && positions[1] == StructuralPos::VCP
                    && positions[2..].iter().all(|pos| pos.is_ending())
                    && positions
                        .last()
                        .is_some_and(|pos| matches!(*pos, StructuralPos::EC | StructuralPos::EF));
            });
        matched
    }

    #[must_use]
    pub fn has_exact_lost_span_copula_ending_path(&self, text: &str) -> bool {
        let mut matched = false;
        self.resource
            .common_prefix_analysis_refs(text.as_bytes(), |length, analysis| {
                if length != text.len() {
                    return;
                }
                let positions = analysis.positions();
                let Some(copula_index) =
                    positions.iter().position(|pos| *pos == StructuralPos::VCP)
                else {
                    return;
                };
                matched |= copula_index > 0
                    && positions[..copula_index].iter().all(|pos| pos.is_nominal())
                    && positions[copula_index + 1..]
                        .iter()
                        .all(|pos| pos.is_ending())
                    && positions
                        .last()
                        .is_some_and(|pos| matches!(*pos, StructuralPos::EC | StructuralPos::EF))
                    && !analysis
                        .components()
                        .any(|component| component.pos == "VCP");
            });
        matched
    }

    fn supports_predicate_ending_path_with_terminal(
        &self,
        text: &str,
        anchor_len: usize,
        pos: PredicatePos,
        node_limit: usize,
        required_terminal: Option<StructuralPos>,
        allow_complete_anchor: bool,
    ) -> bool {
        if anchor_len == 0
            || if allow_complete_anchor {
                anchor_len > text.len()
            } else {
                anchor_len >= text.len()
            }
            || !text.is_char_boundary(anchor_len)
        {
            return false;
        }
        let mut visited = vec![[[false; 2]; 2]; text.len() + 1];
        let mut pending = Vec::new();
        let mut nodes = 0;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length != anchor_len {
                    return;
                }
                nodes += 1;
                let Some((first, endings)) = positions.split_first() else {
                    return;
                };
                if structural_predicate_pos_matches(*first, pos)
                    && endings.iter().all(|ending| ending.is_ending())
                {
                    let has_ending = !endings.is_empty();
                    let terminal_matches = endings
                        .last()
                        .is_some_and(|ending| required_terminal.is_none_or(|tag| *ending == tag));
                    if !visited[length][usize::from(has_ending)][usize::from(terminal_matches)] {
                        visited[length][usize::from(has_ending)][usize::from(terminal_matches)] =
                            true;
                        pending.push((length, has_ending, terminal_matches));
                    }
                }
            });
        while let Some((start, has_ending, terminal_matches)) = pending.pop() {
            if nodes > node_limit {
                return false;
            }
            if start == text.len()
                && has_ending
                && required_terminal.is_none_or(|_| terminal_matches)
            {
                return true;
            }
            self.resource
                .common_prefix_positions(&text.as_bytes()[start..], |length, endings| {
                    if length == 0 || start + length > text.len() {
                        return;
                    }
                    nodes += 1;
                    if !endings.is_empty() && endings.iter().all(|ending| ending.is_ending()) {
                        let end = start + length;
                        let terminal_matches = endings.last().is_some_and(|ending| {
                            required_terminal.is_none_or(|tag| *ending == tag)
                        });
                        if !visited[end][1][usize::from(terminal_matches)] {
                            visited[end][1][usize::from(terminal_matches)] = true;
                            pending.push((end, true, terminal_matches));
                        }
                    }
                });
        }
        false
    }

    #[must_use]
    pub fn supports_adnominal_dependent_noun_particle_path(
        &self,
        text: &str,
        anchor_len: usize,
        adnominal_len: usize,
        pos: PredicatePos,
        node_limit: usize,
    ) -> bool {
        if anchor_len == 0
            || anchor_len > adnominal_len
            || adnominal_len >= text.len()
            || !text.is_char_boundary(anchor_len)
            || !text.is_char_boundary(adnominal_len)
        {
            return false;
        }
        self.supports_predicate_ending_path_with_terminal(
            &text[..adnominal_len],
            anchor_len,
            pos,
            node_limit,
            Some(StructuralPos::ETM),
            true,
        ) && complete_dependent_noun_particle_suffix(
            &self.resource,
            &text[adnominal_len..],
            node_limit,
        )
    }

    #[must_use]
    pub fn supports_predicate_ending_particle_path(
        &self,
        text: &str,
        anchor_len: usize,
        ending_len: usize,
        pos: PredicatePos,
        node_limit: usize,
    ) -> bool {
        if anchor_len == 0
            || anchor_len >= ending_len
            || ending_len >= text.len()
            || !text.is_char_boundary(anchor_len)
            || !text.is_char_boundary(ending_len)
        {
            return false;
        }
        self.supports_predicate_ending_path(&text[..ending_len], anchor_len, pos, node_limit)
            && complete_suffix(
                &self.resource,
                &text[ending_len..],
                StructuralPos::is_particle,
            )
    }

    #[must_use]
    pub fn supports_ending_suffix_path(&self, text: &str, start: usize, node_limit: usize) -> bool {
        if start >= text.len() || !text.is_char_boundary(start) {
            return false;
        }
        let mut visited = vec![false; text.len() + 1];
        let mut pending = vec![start];
        let mut nodes = 0;
        while let Some(position) = pending.pop() {
            if nodes > node_limit {
                return false;
            }
            if position == text.len() {
                return true;
            }
            self.resource.common_prefix_positions(
                &text.as_bytes()[position..],
                |length, positions| {
                    if length == 0 || position + length > text.len() {
                        return;
                    }
                    nodes += 1;
                    if positions.iter().all(|pos| pos.is_ending()) {
                        let end = position + length;
                        if !visited[end] {
                            visited[end] = true;
                            pending.push(end);
                        }
                    }
                },
            );
        }
        false
    }

    #[must_use]
    pub fn has_exact_ending_surface(&self, text: &str) -> bool {
        let mut supported = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length == text.len()
                    && !positions.is_empty()
                    && positions.iter().all(|pos| pos.is_ending())
                {
                    supported = true;
                }
            });
        supported
    }

    #[must_use]
    pub fn has_exact_nominalizing_ending_surface(&self, text: &str) -> bool {
        let mut supported = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length == text.len()
                    && !positions.is_empty()
                    && positions.iter().all(|pos| pos.is_ending())
                    && positions.last() == Some(&StructuralPos::ETN)
                {
                    supported = true;
                }
            });
        supported
    }

    #[must_use]
    pub fn auxiliary_splits(&self, text: &str) -> Vec<usize> {
        let mut splits = Vec::new();
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                let Some((first, endings)) = positions.split_first() else {
                    return;
                };
                if *first != StructuralPos::VX || !endings.iter().all(|pos| pos.is_ending()) {
                    return;
                }
                if length == text.len() || endings.is_empty() {
                    splits.push(length);
                }
            });
        splits.sort_unstable();
        splits.dedup();
        splits
    }

    #[must_use]
    pub fn supports_auxiliary_sequence(&self, text: &str, node_limit: usize) -> bool {
        if text.is_empty() {
            return false;
        }
        let mut visited = vec![false; text.len() + 1];
        let mut pending = vec![0];
        let mut nodes = 0;
        while let Some(start) = pending.pop() {
            if nodes > node_limit {
                return false;
            }
            if start == text.len() && start > 0 {
                return true;
            }
            self.resource.common_prefix_positions(
                &text.as_bytes()[start..],
                |length, positions| {
                    if length == 0 || start + length > text.len() {
                        return;
                    }
                    nodes += 1;
                    let allowed = if start == 0 {
                        positions.first() == Some(&StructuralPos::VX)
                            && positions[1..].iter().all(|pos| pos.is_ending())
                    } else {
                        positions.iter().all(|pos| pos.is_ending())
                    };
                    if allowed {
                        let end = start + length;
                        if !visited[end] {
                            visited[end] = true;
                            pending.push(end);
                        }
                    }
                },
            );
        }
        false
    }

    #[must_use]
    pub fn whole_predicate_conflicts(
        &self,
        text: &str,
        anchor_len: usize,
        pos: PredicatePos,
    ) -> bool {
        self.whole_predicate_conflicts_at(text, 0..anchor_len, pos)
    }

    #[must_use]
    pub fn whole_predicate_conflicts_at(
        &self,
        text: &str,
        anchor: Range<usize>,
        pos: PredicatePos,
    ) -> bool {
        if anchor.is_empty()
            || anchor.end > text.len()
            || !text.is_char_boundary(anchor.start)
            || !text.is_char_boundary(anchor.end)
        {
            return false;
        }
        let mut whole_predicate = false;
        let mut aligned_query_stem = false;
        self.resource
            .common_prefix_analysis_refs(text.as_bytes(), |length, analysis| {
                if length != text.len() {
                    return;
                }
                let Some(first) = analysis.positions().first() else {
                    return;
                };
                if !first.is_predicate() {
                    return;
                }
                whole_predicate = true;
                aligned_query_stem |= analysis.components().any(|component| {
                    component.span == anchor && predicate_pos_matches(component.pos, pos)
                });
            });
        whole_predicate && !aligned_query_stem
    }

    #[must_use]
    pub fn has_source_aligned_compound_predicate_component(
        &self,
        text: &str,
        anchor: Range<usize>,
        source_positions: PredicatePosSet,
        node_limit: usize,
    ) -> bool {
        if anchor.is_empty()
            || anchor.start == 0
            || anchor.end > text.len()
            || !text.is_char_boundary(anchor.start)
            || !text.is_char_boundary(anchor.end)
        {
            return false;
        }
        TokenEvidence::collect(&self.resource, text, node_limit, false, false, false).is_ok_and(
            |evidence| {
                !evidence.has_whole_modifier()
                    && evidence
                        .compound_predicate_components
                        .iter()
                        .filter(|component| {
                            component.start == anchor.start && component.end == anchor.end
                        })
                        .any(|component| {
                            source_positions
                                .iter()
                                .any(|pos| predicate_pos_matches(component.pos.as_str(), pos))
                        })
            },
        )
    }

    #[must_use]
    pub fn has_attached_auxiliary_whole_path(&self, text: &str) -> bool {
        let mut supported = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length == text.len() {
                    supported |= is_attached_auxiliary_whole_path(positions);
                }
            });
        supported
    }

    #[must_use]
    pub fn has_unambiguous_predicate_surface(
        &self,
        text: &str,
        source_positions: PredicatePosSet,
    ) -> bool {
        let mut found = false;
        let mut conflicting = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length != text.len() {
                    return;
                }
                let compatible = positions.split_first().is_some_and(|(first, endings)| {
                    source_positions
                        .iter()
                        .any(|pos| structural_predicate_pos_matches(*first, pos))
                        && endings.iter().all(|pos| pos.is_ending())
                });
                let predicate_only = positions.split_first().is_some_and(|(first, endings)| {
                    first.is_predicate() && endings.iter().all(|pos| pos.is_ending())
                });
                found |= compatible;
                conflicting |= !predicate_only;
            });
        found && !conflicting
    }

    #[must_use]
    pub fn has_extended_predicate_prefix_path(
        &self,
        text: &str,
        core_len: usize,
        source_positions: PredicatePosSet,
        node_limit: usize,
    ) -> bool {
        if core_len >= text.len() || !text.is_char_boundary(core_len) {
            return false;
        }
        let mut prefix_ends = Vec::new();
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length > core_len
                    && length < text.len()
                    && positions.first().is_some_and(|actual| {
                        positions.len() == 1
                            && source_positions
                                .iter()
                                .any(|pos| structural_predicate_pos_matches(*actual, pos))
                    })
                {
                    prefix_ends.push(length);
                }
            });
        prefix_ends
            .into_iter()
            .any(|end| self.supports_ending_suffix_path(text, end, node_limit))
    }

    #[must_use]
    pub fn has_unambiguous_attached_auxiliary_whole_path(&self, text: &str) -> bool {
        let mut found = false;
        let mut conflicting = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length != text.len() {
                    return;
                }
                let compatible = is_attached_auxiliary_whole_path(positions);
                found |= compatible;
                conflicting |= !compatible;
            });
        found && !conflicting
    }

    #[must_use]
    pub fn has_complete_attached_auxiliary_path(&self, text: &str, node_limit: usize) -> bool {
        EdgeGraph::collect(&self.resource, text, node_limit)
            .is_ok_and(|graph| has_complete_attached_auxiliary_path(text.len(), &graph))
    }

    #[must_use]
    pub fn has_complete_nominal_surface(&self, text: &str) -> bool {
        !text.is_empty() && complete_nominal_host(&self.resource, text)
    }
}
