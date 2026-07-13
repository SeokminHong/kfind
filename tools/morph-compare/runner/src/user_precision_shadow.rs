use std::collections::BTreeSet;
use std::ops::Range;

use kfind_data::ComponentResource;
use kfind_matcher::is_token_character;
use kfind_morph::CoarsePos;
use kfind_query::{Morphology, PhraseMatch, QueryPlan};
use serde::Serialize;

use super::Span;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub(super) struct UserPrecisionShadow {
    policy: &'static str,
    projected_spans: Vec<Span>,
    removed_matches: usize,
    matched_coarse_pos: Vec<&'static str>,
    whole_token_lexical: Vec<WholeTokenLexicalEvidence>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
struct WholeTokenLexicalEvidence {
    match_span: Span,
    atom_index: usize,
    matched_span: Span,
    whole_token_span: Span,
    whole_token: String,
    exact_pos: Vec<String>,
    strict_subspan: bool,
    all_exact_non_predicate: bool,
    predicate_origins: usize,
    retained_origins: usize,
    rejected: bool,
}

pub(super) fn diagnose_user_precision_shadow(
    plan: &QueryPlan,
    matches: &[PhraseMatch],
    text: &str,
    resource: &ComponentResource,
) -> (Vec<Span>, UserPrecisionShadow) {
    project_with_lookup(plan, matches, text, |token| {
        exact_component_pos(resource, token)
    })
}

fn project_with_lookup(
    plan: &QueryPlan,
    matches: &[PhraseMatch],
    text: &str,
    mut exact_pos: impl FnMut(&[u8]) -> Vec<String>,
) -> (Vec<Span>, UserPrecisionShadow) {
    let mut baseline_spans = Vec::new();
    let mut projected_spans = Vec::new();
    let mut removed_matches = 0;
    let mut matched_coarse_pos = BTreeSet::new();
    let mut whole_token_lexical = Vec::new();

    for matched in matches {
        let mut rejected_match = false;
        for (atom_index, atom) in matched.atoms.iter().enumerate() {
            baseline_spans.push(span(atom.token.clone()));
            let analyses = &plan.atoms[atom_index].analyses;
            for origin in &atom.origins {
                if let Some(analysis) = analyses.get(usize::from(origin.analysis_index)) {
                    matched_coarse_pos.insert(analysis.coarse_pos);
                }
            }

            let whole_token = surrounding_token_span(text, atom.token.clone());
            let exact_pos = exact_pos(&text.as_bytes()[whole_token.clone()]);
            if exact_pos.is_empty() {
                continue;
            }

            let predicate_origins = atom
                .origins
                .iter()
                .filter(|origin| {
                    analyses
                        .get(usize::from(origin.analysis_index))
                        .is_some_and(|analysis| {
                            matches!(analysis.morphology, Morphology::Predicate(_))
                        })
                })
                .count();
            if predicate_origins == 0 {
                continue;
            }
            let strict_subspan = whole_token != atom.token;
            let all_exact_non_predicate = exact_pos.iter().all(|pos| !is_predicate_pos(pos));
            let retained_origins = atom.origins.len().saturating_sub(predicate_origins);
            let rejected = strict_subspan && all_exact_non_predicate && retained_origins == 0;
            rejected_match |= rejected;
            whole_token_lexical.push(WholeTokenLexicalEvidence {
                match_span: span(matched.span.clone()),
                atom_index,
                matched_span: span(atom.token.clone()),
                whole_token_span: span(whole_token.clone()),
                whole_token: text[whole_token].to_owned(),
                exact_pos,
                strict_subspan,
                all_exact_non_predicate,
                predicate_origins,
                retained_origins,
                rejected,
            });
        }
        if rejected_match {
            removed_matches += 1;
        } else {
            projected_spans.extend(matched.atoms.iter().map(|atom| span(atom.token.clone())));
        }
    }

    sort_and_deduplicate(&mut baseline_spans);
    sort_and_deduplicate(&mut projected_spans);
    (
        baseline_spans,
        UserPrecisionShadow {
            policy: "whole-token-lexical",
            projected_spans,
            removed_matches,
            matched_coarse_pos: matched_coarse_pos
                .into_iter()
                .map(coarse_pos_name)
                .collect(),
            whole_token_lexical,
        },
    )
}

fn exact_component_pos(resource: &ComponentResource, token: &[u8]) -> Vec<String> {
    let mut exact_pos = Vec::new();
    resource.common_prefixes(token, |length, analyses| {
        if length == token.len() {
            exact_pos.extend(analyses.iter().map(|analysis| analysis.pos.to_owned()));
        }
    });
    exact_pos.sort();
    exact_pos.dedup();
    exact_pos
}

fn is_predicate_pos(pos: &str) -> bool {
    pos.split('+')
        .any(|tag| matches!(tag, "VV" | "VA" | "VX" | "VCP" | "VCN" | "XSV" | "XSA"))
}

fn surrounding_token_span(text: &str, span: Range<usize>) -> Range<usize> {
    let mut start = span.start;
    while let Some((offset, character)) = text[..start].char_indices().next_back() {
        if !is_token_character(character) {
            break;
        }
        start = offset;
    }

    let mut end = span.end;
    while let Some(character) = text[end..].chars().next() {
        if !is_token_character(character) {
            break;
        }
        end += character.len_utf8();
    }
    start..end
}

fn sort_and_deduplicate(spans: &mut Vec<Span>) {
    spans.sort_by_key(|span| (span.byte_start, span.byte_end));
    spans.dedup_by_key(|span| (span.byte_start, span.byte_end));
}

fn span(range: Range<usize>) -> Span {
    Span {
        byte_start: range.start,
        byte_end: range.end,
    }
}

const fn coarse_pos_name(pos: CoarsePos) -> &'static str {
    match pos {
        CoarsePos::Noun => "noun",
        CoarsePos::Pronoun => "pronoun",
        CoarsePos::Numeral => "numeral",
        CoarsePos::Verb => "verb",
        CoarsePos::Adjective => "adjective",
        CoarsePos::Determiner => "determiner",
        CoarsePos::Adverb => "adverb",
        CoarsePos::Particle => "particle",
        CoarsePos::Interjection => "interjection",
        CoarsePos::Literal => "literal",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use kfind_query::{
        CompileOptions, LexiconQueryAnalyzer, Lexicons, Origin, VerifiedSpan, compile_query,
    };

    use super::*;

    #[test]
    fn removes_predicate_suffix_when_whole_token_is_only_non_predicate() {
        let plan = plan("이다");
        let predicate = predicate_analysis(&plan);
        let matches = vec![phrase_match(3..6, &[predicate])];

        let (_, shadow) =
            project_with_lookup(&plan, &matches, "매일", |_| vec!["MAG".to_owned()]);

        assert!(shadow.projected_spans.is_empty());
        assert_eq!(shadow.removed_matches, 1);
        assert!(shadow.whole_token_lexical[0].rejected);
    }

    #[test]
    fn records_whole_token_predicate_homonym_without_removing_it() {
        let plan = plan("살다");
        let predicate = predicate_analysis(&plan);
        let matches = vec![phrase_match(0..6, &[predicate])];

        let (_, shadow) =
            project_with_lookup(&plan, &matches, "사실", |_| vec!["NNG".to_owned()]);

        assert_eq!(shadow.projected_spans, vec![span(0..6)]);
        assert_eq!(shadow.removed_matches, 0);
        assert!(!shadow.whole_token_lexical[0].strict_subspan);
    }

    #[test]
    fn retains_match_for_predicate_or_mixed_query_evidence() {
        let mut plan = plan("이다");
        let predicate = predicate_analysis(&plan);
        let mut non_predicate_analysis = plan.atoms[0].analyses[usize::from(predicate)].clone();
        non_predicate_analysis.morphology = Morphology::Exact;
        plan.atoms[0].analyses.push(non_predicate_analysis);
        let non_predicate = u16::try_from(plan.atoms[0].analyses.len() - 1).unwrap();
        let predicate_exact = vec![phrase_match(3..6, &[predicate])];
        let mixed_origins = vec![phrase_match(3..6, &[predicate, non_predicate])];

        let (_, predicate_shadow) = project_with_lookup(&plan, &predicate_exact, "매일", |_| {
            vec!["MAG".to_owned(), "VCP".to_owned()]
        });
        let (_, mixed_shadow) =
            project_with_lookup(&plan, &mixed_origins, "매일", |_| vec!["MAG".to_owned()]);

        assert_eq!(predicate_shadow.projected_spans, vec![span(3..6)]);
        assert_eq!(mixed_shadow.projected_spans, vec![span(3..6)]);
        assert!(!mixed_shadow.whole_token_lexical[0].rejected);
    }

    fn plan(query: &str) -> QueryPlan {
        let analyzer = LexiconQueryAnalyzer::new(Arc::new(Lexicons::embedded().unwrap()));
        compile_query(query, &CompileOptions::default(), &analyzer).unwrap()
    }

    fn predicate_analysis(plan: &QueryPlan) -> u16 {
        plan.atoms[0]
            .analyses
            .iter()
            .position(|analysis| matches!(analysis.morphology, Morphology::Predicate(_)))
            .expect("predicate analysis") as u16
    }

    fn phrase_match(token: Range<usize>, analyses: &[u16]) -> PhraseMatch {
        PhraseMatch {
            span: token.clone(),
            atoms: vec![VerifiedSpan {
                core: token.clone(),
                token,
                origins: analyses
                    .iter()
                    .map(|analysis_index| Origin {
                        analysis_index: *analysis_index,
                        rule_path: Vec::new(),
                    })
                    .collect(),
            }],
        }
    }
}
