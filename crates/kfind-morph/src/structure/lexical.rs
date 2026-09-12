use crate::PredicatePos;
use crate::structure::BoundedTokenContext;
use kfind_data::{ComponentPos as StructuralPos, ComponentResource, DataFinePos};
use std::ops::Range;

pub(super) fn predicate_pos_matches(actual: &str, expected: PredicatePos) -> bool {
    match expected {
        PredicatePos::Verb => actual == "VV",
        PredicatePos::Adjective => matches!(actual, "VA" | "VCN"),
        PredicatePos::AuxiliaryVerb | PredicatePos::AuxiliaryAdjective => actual == "VX",
        PredicatePos::Copula => actual == "VCP",
    }
}

pub(super) fn structural_predicate_pos_matches(
    actual: StructuralPos,
    expected: PredicatePos,
) -> bool {
    match expected {
        PredicatePos::Verb => actual == StructuralPos::VV,
        PredicatePos::Adjective => matches!(actual, StructuralPos::VA | StructuralPos::VCN),
        PredicatePos::AuxiliaryVerb | PredicatePos::AuxiliaryAdjective => {
            actual == StructuralPos::VX
        }
        PredicatePos::Copula => actual == StructuralPos::VCP,
    }
}
#[derive(Clone, Debug)]
pub(super) struct NumericUnitPath {
    pub(super) unit: Range<usize>,
    pub(super) dependent_tail: Option<Range<usize>>,
}

pub(super) fn numeric_unit_path(
    resource: &ComponentResource,
    text: &str,
) -> Option<NumericUnitPath> {
    let numeric_end = text.bytes().take_while(u8::is_ascii_digit).count();
    if numeric_end == 0 || numeric_end == text.len() {
        return None;
    }
    let mut selected = None;
    resource.common_prefix_positions(&text.as_bytes()[numeric_end..], |unit_length, positions| {
        if !matches!(
            positions,
            [StructuralPos::NNB | StructuralPos::NNBC | StructuralPos::NR]
        ) {
            return;
        }
        let unit_end = numeric_end + unit_length;
        if complete_suffix(resource, &text[unit_end..], StructuralPos::is_particle) {
            select_numeric_unit_path(
                &mut selected,
                NumericUnitPath {
                    unit: numeric_end..unit_end,
                    dependent_tail: None,
                },
            );
        }
        resource.common_prefix_positions(&text.as_bytes()[unit_end..], |tail_length, positions| {
            if !matches!(positions, [StructuralPos::NNB | StructuralPos::NNBC]) {
                return;
            }
            let tail_end = unit_end + tail_length;
            if !complete_suffix(resource, &text[tail_end..], StructuralPos::is_particle) {
                return;
            }
            select_numeric_unit_path(
                &mut selected,
                NumericUnitPath {
                    unit: numeric_end..unit_end,
                    dependent_tail: Some(unit_end..tail_end),
                },
            );
        });
    });
    selected
}

fn select_numeric_unit_path(selected: &mut Option<NumericUnitPath>, candidate: NumericUnitPath) {
    let rank = |path: &NumericUnitPath| {
        (
            path.dependent_tail
                .as_ref()
                .map_or(path.unit.end, |tail| tail.end),
            path.unit.end,
            path.dependent_tail.is_some(),
        )
    };
    if selected
        .as_ref()
        .is_none_or(|current| rank(&candidate) > rank(current))
    {
        *selected = Some(candidate);
    }
}

pub(super) fn adnominal_suffix_is_supported(resource: &ComponentResource, text: &str) -> bool {
    let surface_shape = text.ends_with("는") || text.ends_with("던");
    surface_shape
        && text.char_indices().map(|(offset, _)| offset).any(|start| {
            has_exact_sequence(resource, &text[start..], &[StructuralPos::ETM])
                || has_exact_sequence(
                    resource,
                    &text[start..],
                    &[StructuralPos::EP, StructuralPos::ETM],
                )
        })
}

pub(super) fn exact_analysis_starts_with_pos(
    resource: &ComponentResource,
    text: &str,
    accepts: impl Fn(StructuralPos) -> bool,
) -> bool {
    let mut matched = false;
    resource.common_prefix_positions(text.as_bytes(), |length, positions| {
        if length == text.len() {
            matched |= positions.first().copied().is_some_and(&accepts);
        }
    });
    matched
}

pub(super) fn complete_ha_predicate_path(resource: &ComponentResource, text: &str) -> bool {
    ["하", "해", "했"].into_iter().any(|surface| {
        text.starts_with(surface)
            && exact_analysis_starts_with_pos(resource, surface, |pos| pos == StructuralPos::VV)
            && complete_suffix(resource, &text[surface.len()..], StructuralPos::is_ending)
    })
}

pub(super) fn complete_predicate_ending_path(resource: &ComponentResource, text: &str) -> bool {
    text.char_indices()
        .map(|(offset, _)| offset)
        .skip(1)
        .any(|split| {
            exact_analysis_starts_with_pos(resource, &text[..split], |pos| pos.is_predicate_tag())
                && complete_suffix(resource, &text[split..], StructuralPos::is_ending)
        })
}

pub(super) fn complete_predicate_connective_ji_path(
    resource: &ComponentResource,
    text: &str,
) -> bool {
    let Some(predicate) = text.strip_suffix('지') else {
        return false;
    };
    !predicate.is_empty()
        && exact_analysis_starts_with_pos(resource, predicate, StructuralPos::is_predicate_tag)
        && has_exact_sequence(resource, "지", &[StructuralPos::EC])
}

pub(super) fn has_copular_adnominal_split(resource: &ComponentResource, current: &str) -> bool {
    current
        .char_indices()
        .map(|(offset, _)| offset)
        .skip(1)
        .any(|split| {
            has_exact_fine_pos(resource, &current[..split], DataFinePos::is_nominal)
                && has_exact_sequence(
                    resource,
                    &current[split..],
                    &[StructuralPos::VCP, StructuralPos::ETM],
                )
        })
}

pub(super) fn copular_frame(
    resource: &ComponentResource,
    context: BoundedTokenContext<'_>,
) -> Option<(Range<usize>, Range<usize>)> {
    let previous = context.previous?;
    let next = context.next?;
    if !complete_pos_sequence(resource, previous, &[StructuralPos::VCN, StructuralPos::EC])
        || !starts_with_pos(resource, next, |pos| {
            matches!(pos, StructuralPos::NNB | StructuralPos::NNBC)
        })
    {
        return None;
    }
    let split = unique_copular_split(resource, context.current)?;
    Some((0..split, split..context.current.len()))
}

fn unique_copular_split(resource: &ComponentResource, current: &str) -> Option<usize> {
    let mut matches = current
        .char_indices()
        .map(|(offset, _)| offset)
        .skip(1)
        .filter(|&split| {
            has_exact_fine_pos(resource, &current[..split], DataFinePos::is_nominal)
                && (has_exact_sequence(resource, &current[split..], &[StructuralPos::VCP])
                    || has_exact_sequence(
                        resource,
                        &current[split..],
                        &[StructuralPos::VCP, StructuralPos::ETM],
                    ))
        });
    let split = matches.next()?;
    matches.next().is_none().then_some(split)
}

pub(super) fn nominal_particle_hosts(
    resource: &ComponentResource,
    current: &str,
) -> Vec<Range<usize>> {
    current
        .char_indices()
        .map(|(offset, _)| offset)
        .skip(1)
        .filter(|&split| {
            has_exact_fine_pos(resource, &current[..split], DataFinePos::is_nominal)
                && complete_suffix(resource, &current[split..], StructuralPos::is_particle)
        })
        .map(|end| 0..end)
        .collect()
}

pub(super) fn complete_nominal_particle_host(
    resource: &ComponentResource,
    current: &str,
) -> Option<Range<usize>> {
    current
        .char_indices()
        .map(|(offset, _)| offset)
        .skip(1)
        .filter(|&split| {
            complete_nominal_host(resource, &current[..split])
                && complete_suffix(resource, &current[split..], StructuralPos::is_particle)
        })
        .max()
        .map(|end| 0..end)
}

pub(super) fn complete_nominal_host(resource: &ComponentResource, text: &str) -> bool {
    let mut visited = vec![[false; 2]; text.len() + 1];
    let mut pending = vec![(0, false)];
    while let Some((start, has_nominal)) = pending.pop() {
        if start == text.len() {
            if has_nominal {
                return true;
            }
            continue;
        }
        resource.common_prefix_positions(&text.as_bytes()[start..], |length, positions| {
            if length == 0 || start + length > text.len() {
                return;
            }
            let mut next_has_nominal = has_nominal;
            let valid = positions.iter().all(|&pos| {
                if pos.is_nominal() {
                    next_has_nominal = true;
                    true
                } else {
                    matches!(
                        pos,
                        StructuralPos::XPN | StructuralPos::XSN | StructuralPos::XR
                    )
                }
            });
            let end = start + length;
            let state = usize::from(next_has_nominal);
            if valid && !visited[end][state] {
                visited[end][state] = true;
                pending.push((end, next_has_nominal));
            }
        });
    }
    false
}

pub(super) fn complete_suffix(
    resource: &ComponentResource,
    suffix: &str,
    accepts: impl Copy + Fn(StructuralPos) -> bool,
) -> bool {
    let mut reachable = vec![false; suffix.len() + 1];
    reachable[0] = true;
    for start in 0..suffix.len() {
        if !reachable[start] {
            continue;
        }
        resource.common_prefix_positions(&suffix.as_bytes()[start..], |length, positions| {
            let Some(end) = start
                .checked_add(length)
                .filter(|&end| length > 0 && end <= suffix.len())
            else {
                return;
            };
            if positions.iter().copied().all(accepts) {
                reachable[end] = true;
            }
        });
    }
    reachable[suffix.len()]
}

pub(super) fn complete_dependent_noun_particle_suffix(
    resource: &ComponentResource,
    suffix: &str,
    node_limit: usize,
) -> bool {
    let mut visited = vec![[false; 3]; suffix.len() + 1];
    let mut pending = vec![(0, 0_usize)];
    let mut nodes = 0;
    while let Some((start, state)) = pending.pop() {
        if nodes > node_limit {
            return false;
        }
        if start == suffix.len() {
            if state == 2 {
                return true;
            }
            continue;
        }
        resource.common_prefix_positions(&suffix.as_bytes()[start..], |length, positions| {
            if length == 0 || start + length > suffix.len() {
                return;
            }
            nodes += 1;
            let mut next_state = state;
            let valid = positions.iter().copied().all(|position| match next_state {
                0 if matches!(position, StructuralPos::NNB | StructuralPos::NNBC) => {
                    next_state = 1;
                    true
                }
                1 | 2 if position.is_particle() => {
                    next_state = 2;
                    true
                }
                _ => false,
            });
            let end = start + length;
            if valid && !visited[end][next_state] {
                visited[end][next_state] = true;
                pending.push((end, next_state));
            }
        });
    }
    false
}

pub(super) fn has_exact_fine_pos(
    resource: &ComponentResource,
    text: &str,
    accepts: impl Fn(DataFinePos) -> bool,
) -> bool {
    let mut matched = false;
    resource.common_prefix_positions(text.as_bytes(), |length, positions| {
        if length == text.len() {
            matched |= match positions {
                [position] => position.fine_pos().is_some_and(&accepts),
                _ => false,
            };
        }
    });
    matched
}

fn has_exact_sequence(
    resource: &ComponentResource,
    text: &str,
    expected: &[StructuralPos],
) -> bool {
    let mut matched = false;
    resource.common_prefix_positions(text.as_bytes(), |length, positions| {
        if length == text.len() {
            matched |= positions == expected;
        }
    });
    matched
}

fn complete_pos_sequence(
    resource: &ComponentResource,
    text: &str,
    expected: &[StructuralPos],
) -> bool {
    if text.is_empty() || expected.is_empty() {
        return text.is_empty() && expected.is_empty();
    }
    let mut next = Vec::new();
    resource.common_prefix_positions(text.as_bytes(), |length, positions| {
        if length > 0 && expected.starts_with(positions) {
            next.push((length, positions.len()));
        }
    });
    next.into_iter().any(|(length, consumed)| {
        complete_pos_sequence(resource, &text[length..], &expected[consumed..])
    })
}

pub(super) fn exact_analysis_ends_with_pos(
    resource: &ComponentResource,
    text: &str,
    accepts: impl Copy + Fn(StructuralPos) -> bool,
) -> bool {
    let mut matched = false;
    resource.common_prefix_positions(text.as_bytes(), |length, positions| {
        if length == text.len() {
            matched |= positions.last().copied().is_some_and(accepts);
        }
    });
    matched
}

fn starts_with_pos(
    resource: &ComponentResource,
    text: &str,
    accepts: impl Fn(StructuralPos) -> bool,
) -> bool {
    let mut matched = false;
    resource.common_prefix_positions(text.as_bytes(), |_, positions| {
        matched |= positions.first().copied().is_some_and(&accepts);
    });
    matched
}
