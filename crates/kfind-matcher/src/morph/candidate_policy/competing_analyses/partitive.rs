use kfind_morph::{CoarsePos, ConstraintResolver};
use kfind_query::{AtomPlan, CandidateProgram};

use crate::boundary::{is_boundary_after, is_boundary_before};

use super::{ExecutedCandidate, context};

pub(super) fn rejects_han(
    haystack: &[u8],
    candidate: &ExecutedCandidate,
    branch: &CandidateProgram,
    atom: &AtomPlan,
    resolver: &ConstraintResolver,
) -> bool {
    let span = &candidate.consumed;
    if candidate.verified.core != *span
        || !matches!(haystack.get(span.clone()), Some(surface)
            if surface == "한".as_bytes() || surface == "한".as_bytes())
        || !is_boundary_before(haystack, span.start)
        || !is_boundary_after(haystack, span.end)
    {
        return false;
    }
    let mut analyses = branch.origins.iter().map(|origin| {
        let analysis = &atom.analyses[usize::from(origin.analysis_index)];
        (analysis.coarse_pos, analysis.lemma.as_ref())
    });
    let Some(analysis) = analyses.next() else {
        return false;
    };
    if !analyses.all(|other| other == analysis) {
        return false;
    }
    let Some((previous_start, previous)) = context::previous_word_with_start(haystack, span.start)
    else {
        return false;
    };
    let Some((next_end, next)) = context::next_word_on_line(haystack, span.end) else {
        return false;
    };
    match analysis {
        (CoarsePos::Verb, "하다") if previous == "중" => {
            let Some((group_start, group)) =
                context::previous_word_with_start(haystack, previous_start)
            else {
                return false;
            };
            let group = group.strip_suffix('들').unwrap_or(&group);
            !context::previous_word(haystack, group_start).is_some_and(|word| {
                resolver.has_subject_marked_nominal(&word)
                    || resolver.has_object_marked_nominal(&word)
            }) && ((resolver.is_person_nominal(group)
                && resolver.has_inflected_counted_unit("한", &next))
                || (resolver.is_spatial_nominal(group)
                    && resolver.has_counted_place_surface("한", &next)))
        }
        (CoarsePos::Determiner, "한") => {
            previous.ends_with('로')
                && resolver.has_nominal_oblique(&previous)
                && context::previous_word(haystack, previous_start)
                    .is_some_and(|object| resolver.has_object_marked_nominal(&object))
                && !resolver.has_inflected_counted_unit("한", &next)
                && !resolver.has_counted_place_surface("한", &next)
                && (resolver.has_subject_marked_nominal(&next)
                    || resolver.has_topic_marked_nominal(&next)
                    || resolver.has_independent_nominal_head(&next)
                        && context::next_word_on_line(haystack, next_end).is_some_and(
                            |(_, head)| {
                                resolver.has_subject_marked_nominal(&head)
                                    || resolver.has_topic_marked_nominal(&head)
                            },
                        ))
        }
        _ => false,
    }
}
