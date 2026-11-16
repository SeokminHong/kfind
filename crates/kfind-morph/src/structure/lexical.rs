use crate::PredicatePos;
use crate::predicate::ha_aspirates_after;
use crate::structure::{BoundedTokenContext, ConstraintResolver};
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
    numeric_unit_path_at(resource, text, numeric_end)
}

pub(super) fn numeric_unit_path_at(
    resource: &ComponentResource,
    text: &str,
    numeric_end: usize,
) -> Option<NumericUnitPath> {
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
            let (unit, dependent_tail) =
                if ConstraintResolver::is_dictionary_numeric_unit(&text[numeric_end..tail_end]) {
                    (numeric_end..tail_end, None)
                } else {
                    (numeric_end..unit_end, Some(unit_end..tail_end))
                };
            select_numeric_unit_path(
                &mut selected,
                NumericUnitPath {
                    unit,
                    dependent_tail,
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

pub(super) fn calendar_chain_last_unit(
    text: &str,
    target: Option<Range<usize>>,
) -> Option<Range<usize>> {
    const UNITS: [char; 6] = ['년', '월', '일', '시', '분', '초'];
    let mut end = 0;
    let mut next_rank = 0;
    let mut matched = target.is_none();
    let mut count = 0;
    let mut unit = 0..0;
    while text.as_bytes().get(end).is_some_and(u8::is_ascii_digit) {
        let number = text.as_bytes()[end..]
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        let start = end + number;
        let character = text[start..].chars().next()?;
        let rank = UNITS[next_rank..]
            .iter()
            .position(|&item| item == character)?;
        next_rank += rank + 1;
        end = start + character.len_utf8();
        unit = start..end;
        matched |= target.as_ref() == Some(&unit);
        count += 1;
    }
    (count > 1 && matched).then_some(unit)
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

pub(super) fn complete_predicate_adnominal_path(resource: &ComponentResource, text: &str) -> bool {
    predicate_inflected_path(resource, text, StructuralPos::is_predicate_tag, |pos| {
        pos == StructuralPos::ETM
    })
}

pub(super) fn complete_past_verbal_adnominal_path(
    resource: &ComponentResource,
    text: &str,
) -> bool {
    if text.ends_with('는')
        || !text
            .chars()
            .next_back()
            .and_then(crate::hangul::decompose_syllable)
            .is_some_and(|syllable| syllable.jongseong == crate::hangul::JONG_NIEUN)
    {
        return false;
    }
    let adnominal = |pos| pos == StructuralPos::ETM;
    predicate_inflected_path(
        resource,
        text,
        |pos| matches!(pos, StructuralPos::VV | StructuralPos::VX),
        adnominal,
    ) || derived_inflected_path(resource, text, |pos| pos == StructuralPos::XSV, adnominal)
}

pub(super) fn predicate_inflected_path(
    resource: &ComponentResource,
    text: &str,
    accepts_stem: impl Fn(StructuralPos) -> bool,
    accepts_ending: impl Fn(StructuralPos) -> bool,
) -> bool {
    let mut matched = false;
    resource.common_prefix_positions(text.as_bytes(), |length, positions| {
        let Some((&predicate, endings)) = positions.split_first() else {
            return;
        };
        if matched || !accepts_stem(predicate) {
            return;
        }
        matched = complete_inflected_ending(resource, endings, &text[length..], &accepts_ending);
    });
    matched
}

pub(super) fn complete_derived_adnominal_path(resource: &ComponentResource, text: &str) -> bool {
    derived_inflected_path(resource, text, |_| true, |pos| pos == StructuralPos::ETM)
}

pub(super) fn complete_connective_path(resource: &ComponentResource, text: &str) -> bool {
    let connective = |pos| pos == StructuralPos::EC;
    predicate_inflected_path(resource, text, StructuralPos::is_predicate_tag, connective)
        || derived_inflected_path(resource, text, |_| true, connective)
}

pub(super) fn complete_interrogative_ji_path(resource: &ComponentResource, text: &str) -> bool {
    let ending = |pos| matches!(pos, StructuralPos::EC | StructuralPos::EF);
    predicate_inflected_path(resource, text, StructuralPos::is_predicate_tag, ending)
        || derived_inflected_path(resource, text, |_| true, ending)
}

pub(super) fn derived_inflected_path(
    resource: &ComponentResource,
    text: &str,
    accepts_derivation: impl Fn(StructuralPos) -> bool,
    accepts_ending: impl Fn(StructuralPos) -> bool,
) -> bool {
    let mut matched = false;
    resource.common_prefix_positions(text.as_bytes(), |root_end, root| {
        if matched || !matches!(root, [StructuralPos::NNG | StructuralPos::XR]) {
            return;
        }
        let suffix = &text[root_end..];
        resource.common_prefix_positions(suffix.as_bytes(), |end, positions| {
            let [
                derivation @ (StructuralPos::XSV | StructuralPos::XSA),
                endings @ ..,
            ] = positions
            else {
                return;
            };
            matched |= accepts_derivation(*derivation)
                && complete_inflected_ending(resource, endings, &suffix[end..], &accepts_ending);
        });
    });
    matched
}

pub(super) fn adjective_adnominal_path(resource: &ComponentResource, text: &str) -> bool {
    let adnominal = |pos| pos == StructuralPos::ETM;
    predicate_inflected_path(resource, text, |pos| pos == StructuralPos::VA, adnominal)
        || derived_inflected_path(resource, text, |pos| pos == StructuralPos::XSA, adnominal)
}

pub(super) fn complete_modifier_prefix(resource: &ComponentResource, text: &str) -> bool {
    let connective = |pos| pos == StructuralPos::EC;
    let has_connective = |surface: &str| {
        predicate_inflected_path(
            resource,
            surface,
            StructuralPos::is_predicate_tag,
            connective,
        ) || derived_inflected_path(resource, surface, |_| true, connective)
    };
    let has_adnominal = |surface| {
        complete_predicate_adnominal_path(resource, surface)
            || complete_derived_adnominal_path(resource, surface)
    };
    has_adnominal(text)
        || has_connective(text)
        || text.char_indices().skip(1).any(|(boundary, _)| {
            has_connective(&text[..boundary]) && has_adnominal(&text[boundary..])
        })
}

pub(super) fn complete_inflected_ending(
    resource: &ComponentResource,
    endings: &[StructuralPos],
    suffix: &str,
    accepts: impl Fn(StructuralPos) -> bool,
) -> bool {
    let is_complete = |positions: &[StructuralPos]| {
        positions.split_last().is_some_and(|(last, prefinals)| {
            accepts(*last) && prefinals.iter().all(|pos| *pos == StructuralPos::EP)
        })
    };
    if suffix.is_empty() {
        return is_complete(endings);
    }
    endings.iter().all(|pos| *pos == StructuralPos::EP)
        && suffix.char_indices().any(|(split, _)| {
            let tail = &suffix[split..];
            let mut complete = false;
            resource.common_prefix_positions(tail.as_bytes(), |length, positions| {
                complete |= length == tail.len() && is_complete(positions);
            });
            complete
                && (split == 0
                    || complete_suffix(resource, &suffix[..split], |pos| pos == StructuralPos::EP))
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

pub(super) fn has_verb_nominalization(resource: &ComponentResource, text: &str) -> bool {
    let exact = |surface: &str| {
        let mut matched = false;
        resource.common_prefix_positions(surface.as_bytes(), |length, positions| {
            if length == surface.len() {
                matched |= matches!(positions, [StructuralPos::VV, middle @ .., StructuralPos::ETN]
                    if middle.iter().all(|pos| pos.is_ending()));
            }
        });
        matched
    };
    exact(text)
        || text.strip_suffix(['기', '음']).is_some_and(|stem| {
            !stem.is_empty()
                && exact_analysis_starts_with_pos(resource, stem, |pos| pos == StructuralPos::VV)
                && has_exact_sequence(resource, &text[stem.len()..], &[StructuralPos::ETN])
        })
}

pub(super) fn has_auxiliary_adnominal_split(resource: &ComponentResource, text: &str) -> bool {
    text.char_indices().skip(1).any(|(split, _)| {
        has_exact_sequence(
            resource,
            &text[..split],
            &[StructuralPos::VV, StructuralPos::EC],
        ) && has_exact_sequence(
            resource,
            &text[split..],
            &[StructuralPos::VX, StructuralPos::ETM],
        )
    })
}

pub(super) fn complete_predicate_connective_ji_path(
    resource: &ComponentResource,
    text: &str,
) -> bool {
    if let Some(base) = text.strip_suffix('치') {
        return ha_aspirates_after(base)
            && complete_nominal_host(resource, base)
            && (has_exact_sequence(resource, "하", &[StructuralPos::XSV])
                || has_exact_sequence(resource, "하", &[StructuralPos::XSA]))
            && has_exact_sequence(resource, "지", &[StructuralPos::EC]);
    }
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
            has_exact_sequence(
                resource,
                &current[split..],
                &[StructuralPos::VCP, StructuralPos::ETM],
            ) && complete_nominal_host(resource, &current[..split])
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
    complete_nominal_particle_host_with(resource, current, |_, _| true)
}

pub(super) fn complete_nominal_particle_host_with(
    resource: &ComponentResource,
    current: &str,
    accepts: impl Fn(&str, &str) -> bool,
) -> Option<Range<usize>> {
    current
        .char_indices()
        .map(|(offset, _)| offset)
        .skip(1)
        .filter(|&split| {
            let (host, particle) = current.split_at(split);
            complete_nominal_host(resource, host)
                && complete_suffix(resource, particle, StructuralPos::is_particle)
                && accepts(host, particle)
        })
        .max()
        .map(|end| 0..end)
}

pub(super) fn has_complete_object_phrase(resource: &ComponentResource, text: &str) -> bool {
    let Some(host) = text.strip_suffix(['을', '를']) else {
        return false;
    };
    !host.is_empty()
        && (has_exact_fine_pos(resource, host, DataFinePos::is_nominal)
            || complete_nominal_host(resource, host))
        && has_exact_sequence(resource, &text[host.len()..], &[StructuralPos::JKO])
}

pub(super) fn has_complete_subject_phrase(resource: &ComponentResource, text: &str) -> bool {
    let Some(host) = text.strip_suffix(['이', '가']) else {
        return false;
    };
    host.chars().next_back().is_some_and(|last| {
        crate::drop_final(last).is_some() == text.ends_with('이')
            && (has_exact_fine_pos(resource, host, DataFinePos::is_nominal)
                || complete_nominal_host(resource, host))
            && has_exact_sequence(resource, &text[host.len()..], &[StructuralPos::JKS])
    })
}

pub(super) fn has_adnominal_result_complement(resource: &ComponentResource, text: &str) -> bool {
    [("것", "으로"), ("걸", "로"), ("거", "로")]
        .into_iter()
        .any(|(noun, particle)| {
            text.strip_prefix(noun)
                .and_then(|rest| rest.strip_prefix(particle))
                .is_some_and(|tail| {
                    has_exact_sequence(resource, noun, &[StructuralPos::NNB])
                        && has_exact_sequence(resource, particle, &[StructuralPos::JKB])
                        && (tail.is_empty()
                            || complete_suffix(resource, tail, StructuralPos::is_particle))
                })
        })
}

pub(super) fn has_unambiguous_proper_name(resource: &ComponentResource, text: &str) -> bool {
    let proper_name = |stem| {
        has_exact_fine_pos(resource, stem, |pos| pos == DataFinePos::Nnp)
            && !exact_analysis_starts_with_pos(resource, stem, |pos| pos != StructuralPos::NNP)
    };
    proper_name(text)
        || nominal_particle_hosts(resource, text)
            .iter()
            .any(|host| proper_name(&text[host.clone()]))
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

pub(super) fn has_exact_sequence(
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
