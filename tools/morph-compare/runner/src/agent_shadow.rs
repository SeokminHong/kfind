use std::ops::Range;
use std::sync::Arc;

use anyhow::{Context, Result};
use kfind_data::DecodedMorphologyResource;
use kfind_matcher::{AnalysisWindow, MorphMatcher, is_token_character};
use kfind_query::{
    AnalysisSource, BoundaryPolicy, CompileOptionOverrides, CompileOptions, LexiconQueryAnalyzer,
    QueryPlan, VerifiedSpan, compile_query,
};
use serde::Serialize;

use super::shadow::{ShadowLatticeEvidence, diagnose_agent_candidate, fine_pos_name};
use super::{Case, Span, coarse_pos_name, parse_pos};

#[derive(Debug, Serialize)]
pub(super) struct AgentShadowSummary {
    profile: &'static str,
    boundary: &'static str,
    morphology_artifact_sha256: String,
    results: Vec<AgentShadowCase>,
}

#[derive(Debug, Serialize)]
struct AgentShadowCase {
    id: String,
    query_pos: String,
    matches: Vec<AgentShadowMatch>,
}

#[derive(Debug, Serialize)]
struct AgentShadowMatch {
    atom_index: usize,
    core: Span,
    token: Span,
    whole_token: Span,
    origins: Vec<AgentShadowOrigin>,
    exact_whole_token_analyses: Vec<ExactAnalysis>,
    lattice: Vec<ShadowLatticeEvidence>,
}

#[derive(Debug, Serialize)]
struct AgentShadowOrigin {
    analysis_index: u16,
    lemma: String,
    coarse_pos: &'static str,
    fine_pos: &'static str,
    source: &'static str,
    rule_path: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ExactAnalysis {
    pos: String,
    left_id: u16,
    right_id: u16,
    word_cost: i32,
    analysis_type: String,
    expression: String,
}

pub(super) fn diagnose_agent_shadow(
    cases: &[Case],
    analyzer: &LexiconQueryAnalyzer,
    morphology: &DecodedMorphologyResource<'_>,
    morphology_artifact_sha256: String,
) -> Result<AgentShadowSummary> {
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        let options = CompileOptions::resolve(CompileOptionOverrides {
            boundary: Some(BoundaryPolicy::Any),
            pos: Some(parse_pos(&case.pos)?),
            ..CompileOptionOverrides::default()
        })?;
        let plan = Arc::new(
            compile_query(&case.query, &options, analyzer)
                .with_context(|| format!("failed to compile Agent shadow case {}", case.id))?,
        );
        let matcher = MorphMatcher::new(Arc::clone(&plan))?;
        let matches = matcher
            .find_all_with_meta(case.text.as_bytes())
            .into_iter()
            .flat_map(|matched| matched.atoms.into_iter().enumerate())
            .map(|(atom_index, matched)| {
                diagnose_match(&case.text, atom_index, &matched, &plan, morphology)
            })
            .collect::<Result<Vec<_>>>()?;
        results.push(AgentShadowCase {
            id: case.id.clone(),
            query_pos: case.pos.clone(),
            matches,
        });
    }
    Ok(AgentShadowSummary {
        profile: "embedded",
        boundary: "any",
        morphology_artifact_sha256,
        results,
    })
}

fn diagnose_match(
    text: &str,
    atom_index: usize,
    matched: &VerifiedSpan,
    plan: &QueryPlan,
    morphology: &DecodedMorphologyResource<'_>,
) -> Result<AgentShadowMatch> {
    let whole_token = surrounding_token_span(text, matched.token.clone());
    let token_bytes = text
        .as_bytes()
        .get(whole_token.clone())
        .context("Agent shadow whole-token span is invalid")?;
    let mut exact_whole_token_analyses = Vec::new();
    morphology.common_prefixes(token_bytes, |length, analyses| {
        if length != token_bytes.len() {
            return;
        }
        exact_whole_token_analyses.extend(analyses.iter().map(|analysis| ExactAnalysis {
            pos: analysis.pos.to_owned(),
            left_id: analysis.left_id,
            right_id: analysis.right_id,
            word_cost: analysis.word_cost,
            analysis_type: analysis.analysis_type.to_owned(),
            expression: analysis.expression.to_owned(),
        }));
    });

    let atom = plan
        .atoms
        .get(atom_index)
        .context("Agent shadow atom index is invalid")?;
    let origins = matched
        .origins
        .iter()
        .map(|origin| {
            let analysis = atom
                .analyses
                .get(usize::from(origin.analysis_index))
                .context("Agent shadow analysis index is invalid")?;
            Ok(AgentShadowOrigin {
                analysis_index: origin.analysis_index,
                lemma: analysis.lemma.to_string(),
                coarse_pos: coarse_pos_name(analysis.coarse_pos),
                fine_pos: fine_pos_name(analysis.fine_pos),
                source: analysis_source_name(analysis.source),
                rule_path: origin
                    .rule_path
                    .iter()
                    .map(|rule| rule.as_str().to_owned())
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let lattice = matched
        .origins
        .iter()
        .map(|origin| {
            let analysis = &atom.analyses[usize::from(origin.analysis_index)];
            diagnose_agent_candidate(
                atom_index,
                origin.analysis_index,
                origin.rule_path.clone(),
                analysis.fine_pos,
                matched.core.clone(),
                AnalysisWindow::extract(
                    text.as_bytes(),
                    matched.core.clone(),
                    kfind_matcher::DEFAULT_ANALYSIS_WINDOW_LIMITS,
                ),
                morphology,
            )
        })
        .collect();

    Ok(AgentShadowMatch {
        atom_index,
        core: span(matched.core.clone()),
        token: span(matched.token.clone()),
        whole_token: span(whole_token),
        origins,
        exact_whole_token_analyses,
        lattice,
    })
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
    for (offset, character) in text[span.end..].char_indices() {
        if !is_token_character(character) {
            break;
        }
        end = span.end + offset + character.len_utf8();
    }
    start..end
}

fn span(range: Range<usize>) -> Span {
    Span {
        byte_start: range.start,
        byte_end: range.end,
    }
}

const fn analysis_source_name(source: AnalysisSource) -> &'static str {
    match source {
        AnalysisSource::BuiltinLexicon => "builtin-lexicon",
        AnalysisSource::FullPosLexicon => "full-pos-lexicon",
        AnalysisSource::UserLexicon => "user-lexicon",
        AnalysisSource::ProductiveSuffix => "productive-suffix",
        AnalysisSource::Heuristic => "heuristic",
        AnalysisSource::Forced => "forced",
    }
}
