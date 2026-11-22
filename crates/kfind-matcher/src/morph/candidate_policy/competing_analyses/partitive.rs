use kfind_morph::{CoarsePos, ConstraintResolver, PredicatePos, PredicatePosSet};
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
    let Some((next_end, next)) = context::next_word_on_line(haystack, span.end) else {
        return false;
    };
    let previous = context::previous_word_with_start(haystack, span.start);
    if analysis == (CoarsePos::Verb, "하다")
        && let Some((comparison_end, comparison)) = context::next_word_on_line(haystack, next_end)
        && comparison == "더"
        && previous.as_ref().is_none_or(|(_, word)| {
            resolver.has_exact_nominal_surface(word)
                || !(resolver.has_subject_marked_nominal(word)
                    || resolver.has_object_marked_nominal(word)
                    || resolver.has_nominal_oblique(word)
                    || resolver.has_predicate_adnominal_surface(word)
                    || resolver.has_exact_adverb_surface(word))
        })
    {
        // '한층 더'의 첫말처럼 합쳐 쓴 전체가 부사인 경우만 판정한다.
        // https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=89052
        let joined = ["한", next.as_ref()].concat();
        if resolver.has_exact_adverb_surface(&joined)
            && !resolver.has_exact_nominal_surface(&joined)
            && has_degree_predicate(haystack, comparison_end, resolver)
        {
            return true;
        }
    }
    let Some((previous_start, previous)) = previous else {
        return false;
    };
    match analysis {
        (CoarsePos::Verb, "하다") if previous == "가능한" => {
            // 국립국어원 '{가능한 한/가능한} 빨리'의 조건 명사 한(限).
            // https://www.korean.go.kr/front/mcfaq/mcfaqView.do?mcfaq_seq=9116
            resolver.has_adnominal_dependent_noun(&previous, "한")
                && !resolver.has_exact_nominal_surface(&next)
                && (resolver.has_exact_adverb_surface(&next)
                    || resolver.has_adjective_adnominal_surface(&next)
                    || resolver.has_complete_connective_path(&next))
        }
        (CoarsePos::Verb, "하다") if matches!(previous.as_ref(), "단" | "오직") => {
            // 한정 표현 뒤의 수 관형사와 단위 명사가 뒤 체언의 수량을 지정한다.
            // https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=29733
            next.strip_suffix('의')
                .is_some_and(|unit| resolver.has_standalone_counted_unit("한", unit))
                && context::next_word_on_line(haystack, next_end).is_some_and(|(_, next)| {
                    next.strip_suffix('만')
                        .is_some_and(|host| resolver.has_exact_nominal_surface(host))
                })
        }
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
                && (resolver.has_inflected_counted_unit("한", &next)
                    || resolver.has_counted_unit_with_copula("한", &next)))
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

fn has_degree_predicate(
    haystack: &[u8],
    after_comparison: usize,
    resolver: &ConstraintResolver,
) -> bool {
    let Some((end, head)) = context::next_word_on_line(haystack, after_comparison) else {
        return false;
    };
    if resolver.has_complete_connective_path(&head)
        || resolver.has_predicate_adnominal_surface(&head)
        || resolver.has_independent_final_predicate(&head)
        || head.char_indices().skip(1).any(|(split, initial)| {
            matches!(initial, '지' | '졌')
                && resolver.has_exact_predicate_connective_path(
                    &head[..split],
                    PredicatePosSet::one(PredicatePos::Adjective),
                )
                && resolver
                    .has_attached_resultative_inflection(&head[split..], PredicatePos::Adjective)
        })
    {
        return !resolver.has_exact_nominal_surface(&head);
    }
    resolver.is_attested_do_object(&head)
        && context::next_word_on_line(haystack, end).is_some_and(|(_, suffix)| {
            resolver
                .has_derived_predicate_adnominal_surface(&[head.as_ref(), suffix.as_ref()].concat())
        })
}
