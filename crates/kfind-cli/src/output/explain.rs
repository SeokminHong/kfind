use std::collections::{BTreeSet, HashSet};
use std::io::{self, Write};

use kfind_morph::{CoarsePos, FinePos, LexicalAlternation};
use kfind_query::{
    AnalysisSource, Morphology, NormalizationMode, QueryDiagnostic, QueryPlan, VerifiedSpan,
};
use kfind_search::SearchLine;

use super::FullPosStatus;
use super::text::write_safe_bytes;
use super::write_safe_path;
use crate::Language;

pub(super) fn write_query_plan(
    writer: &mut impl Write,
    plan: &QueryPlan,
    full_pos: Option<&FullPosStatus>,
    language: Language,
) -> io::Result<()> {
    write_label(writer, language, "query", "쿼리", 0)?;
    write_safe_bytes(writer, plan.raw_query.as_bytes())?;
    writer.write_all(b"\n")?;
    if let Some(full_pos) = full_pos {
        write_full_pos_status(writer, full_pos, language)?;
    }

    for (atom_index, atom) in plan.atoms.iter().enumerate() {
        writeln!(writer, "{}[{atom_index}]:", language.select("atom", "요소"))?;
        write_section_label(writer, language, "analyses", "분석", 2)?;
        for analysis in &atom.analyses {
            writer.write_all(b"    - ")?;
            write_label(writer, language, "lemma", "표제어", 0)?;
            write_safe_bytes(writer, analysis.lemma.as_bytes())?;
            writer.write_all(b"\n")?;
            write_label(writer, language, "pos", "품사", 6)?;
            writeln!(
                writer,
                "{}",
                explain_pos_label(analysis.coarse_pos, language)
            )?;
            write_label(writer, language, "fine_pos", "세부_품사", 6)?;
            writeln!(writer, "{}", fine_pos_label(analysis.fine_pos))?;
            write_label(writer, language, "source", "출처", 6)?;
            writeln!(writer, "{}", source_label(analysis.source))?;
            if let Morphology::Predicate(predicate) = &analysis.morphology {
                write_label(writer, language, "alternation", "교체_규칙", 6)?;
                writeln!(writer, "{}", alternation_label(predicate.alternation))?;
            }
        }
        write_label(writer, language, "branches", "분기_수", 2)?;
        writeln!(writer, "{}", atom.branches.len())?;
        write_section_label(writer, language, "anchors", "앵커", 2)?;
        let anchors = atom
            .branches
            .iter()
            .map(|branch| branch.anchor.as_ref())
            .collect::<BTreeSet<_>>();
        for anchor in anchors {
            writer.write_all(b"    - ")?;
            write_safe_bytes(writer, anchor)?;
            writer.write_all(b"\n")?;
        }
        let verifier_states = atom
            .branches
            .iter()
            .map(|branch| &branch.verifier)
            .collect::<HashSet<_>>()
            .len();
        write_label(writer, language, "verifier_states", "검증기_상태_수", 2)?;
        writeln!(writer, "{verifier_states}")?;
    }
    write_label(writer, language, "max_gap", "최대_거리", 0)?;
    writeln!(writer, "{}", plan.phrase_policy.max_gap)?;
    write_label(writer, language, "normalization", "정규화", 0)?;
    writeln!(writer, "{}", normalization_label(plan.normalization))?;
    write_label(
        writer,
        language,
        "estimated_matcher_bytes",
        "예상_검색기_바이트",
        0,
    )?;
    writeln!(writer, "{}", plan.estimated_matcher_bytes)?;
    if !plan.diagnostics.is_empty() {
        write_section_label(writer, language, "diagnostics", "진단", 0)?;
        for diagnostic in &plan.diagnostics {
            writer.write_all(b"  - ")?;
            write_diagnostic(writer, diagnostic, language)?;
            writer.write_all(b"\n")?;
        }
    }
    Ok(())
}

fn write_full_pos_status(
    writer: &mut impl Write,
    status: &FullPosStatus,
    language: Language,
) -> io::Result<()> {
    write_section_label(writer, language, "full_pos", "전체_품사_사전", 0)?;
    match status {
        FullPosStatus::Loaded { path } => {
            write_label(writer, language, "status", "상태", 2)?;
            writeln!(writer, "{}", language.select("loaded", "불러옴"))?;
            write_label(writer, language, "path", "경로", 2)?;
            write_safe_path(writer, path)?;
            writer.write_all(b"\n")
        }
        FullPosStatus::Preview { candidate_paths } => {
            write_label(writer, language, "status", "상태", 2)?;
            writeln!(
                writer,
                "{}",
                language.select(
                    "preview (core lexicon only)",
                    "미리보기 (core lexicon만 사용)"
                )
            )?;
            write_section_label(writer, language, "candidate_paths", "후보_경로", 2)?;
            for path in candidate_paths {
                writer.write_all(b"    - ")?;
                write_safe_path(writer, path)?;
                writer.write_all(b"\n")?;
            }
            Ok(())
        }
        FullPosStatus::NotRequired => {
            write_label(writer, language, "status", "상태", 2)?;
            writeln!(
                writer,
                "{}",
                language.select("not required (literal query)", "불필요 (literal 쿼리)")
            )
        }
    }
}

pub(super) fn write_match_explanations(
    writer: &mut impl Write,
    line: &SearchLine,
    plan: &QueryPlan,
    language: Language,
) -> io::Result<()> {
    for (match_index, matched) in line.matches.iter().enumerate() {
        writeln!(
            writer,
            "  {}[{match_index}]:",
            language.select("match", "일치")
        )?;
        for (atom_index, span) in matched.atoms.iter().enumerate() {
            writeln!(
                writer,
                "    {}[{atom_index}]:",
                language.select("atom", "요소")
            )?;
            write_label(writer, language, "token", "토큰", 6)?;
            write_span(writer, &line.bytes, span, true, language)?;
            writer.write_all(b"\n")?;
            write_label(writer, language, "core", "핵심", 6)?;
            write_span(writer, &line.bytes, span, false, language)?;
            writer.write_all(b"\n")?;
            write_section_label(writer, language, "origins", "생성_근거", 6)?;
            for origin in &span.origins {
                let analysis = plan
                    .atoms
                    .get(atom_index)
                    .and_then(|atom| atom.analyses.get(usize::from(origin.analysis_index)));
                writer.write_all(b"        - ")?;
                write_label(writer, language, "generated_from", "생성_표제어", 0)?;
                if let Some(analysis) = analysis {
                    write_safe_bytes(writer, analysis.lemma.as_bytes())?;
                    writer.write_all(b"\n")?;
                    write_label(writer, language, "pos", "품사", 10)?;
                    writeln!(
                        writer,
                        "{}",
                        explain_pos_label(analysis.coarse_pos, language)
                    )?;
                } else {
                    writeln!(
                        writer,
                        "{}[{}]",
                        language.select("analysis", "분석"),
                        origin.analysis_index
                    )?;
                }
                write_label(writer, language, "rules", "규칙", 10)?;
                if origin.rule_path.is_empty() {
                    writer.write_all(b" []\n")?;
                } else {
                    writer.write_all(b"\n")?;
                    for rule in &origin.rule_path {
                        writeln!(writer, "            - {rule}")?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn write_span(
    writer: &mut impl Write,
    bytes: &[u8],
    span: &VerifiedSpan,
    token: bool,
    language: Language,
) -> io::Result<()> {
    let range = if token { &span.token } else { &span.core };
    if let Some(surface) = bytes.get(range.clone()) {
        write_safe_bytes(writer, surface)
    } else {
        writer.write_all(
            language
                .select("<invalid-span>", "<올바르지-않은-span>")
                .as_bytes(),
        )
    }
}

fn write_diagnostic(
    writer: &mut impl Write,
    diagnostic: &QueryDiagnostic,
    language: Language,
) -> io::Result<()> {
    match diagnostic {
        QueryDiagnostic::FullPosLexiconUnavailable => writer.write_all(
            language
                .select(
                    "full POS lexicon unavailable",
                    "full POS lexicon을 사용할 수 없습니다",
                )
                .as_bytes(),
        ),
        QueryDiagnostic::UnregisteredDaLiteralOnly { atom_index, lemma } => {
            write!(writer, "{}[{atom_index}] ", language.select("atom", "요소"))?;
            writer.write_all(
                language
                    .select(
                        "unregistered -da lemma is literal-only: ",
                        "등록되지 않은 `다` 표제어는 literal로만 검색합니다: ",
                    )
                    .as_bytes(),
            )?;
            write_safe_bytes(writer, lemma.as_bytes())
        }
        QueryDiagnostic::VerifierVocabularyRestricted { excluded_rule_ids } => {
            writer.write_all(
                language
                    .select(
                        "verifier vocabulary excluded rules:",
                        "verifier vocabulary에서 제외된 규칙:",
                    )
                    .as_bytes(),
            )?;
            for rule in excluded_rule_ids {
                write!(writer, " {rule}")?;
            }
            Ok(())
        }
    }
}

fn write_label(
    writer: &mut impl Write,
    language: Language,
    english: &'static str,
    korean: &'static str,
    indent: usize,
) -> io::Result<()> {
    for _ in 0..indent {
        writer.write_all(b" ")?;
    }
    writer.write_all(language.select(english, korean).as_bytes())?;
    writer.write_all(b": ")
}

fn write_section_label(
    writer: &mut impl Write,
    language: Language,
    english: &'static str,
    korean: &'static str,
    indent: usize,
) -> io::Result<()> {
    for _ in 0..indent {
        writer.write_all(b" ")?;
    }
    writer.write_all(language.select(english, korean).as_bytes())?;
    writer.write_all(b":\n")
}

const fn explain_pos_label(pos: CoarsePos, language: Language) -> &'static str {
    match language {
        Language::English => pos_label(pos),
        Language::Korean => match pos {
            CoarsePos::Noun => "명사",
            CoarsePos::Pronoun => "대명사",
            CoarsePos::Numeral => "수사",
            CoarsePos::Verb => "동사",
            CoarsePos::Adjective => "형용사",
            CoarsePos::Determiner => "관형사",
            CoarsePos::Adverb => "부사",
            CoarsePos::Particle => "조사",
            CoarsePos::Interjection => "감탄사",
            CoarsePos::Literal => "literal",
        },
    }
}

const fn normalization_label(normalization: NormalizationMode) -> &'static str {
    match normalization {
        NormalizationMode::Nfc => "nfc",
        NormalizationMode::Canonical => "canonical",
        NormalizationMode::None => "none",
    }
}

pub(super) const fn pos_label(pos: CoarsePos) -> &'static str {
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

const fn source_label(source: AnalysisSource) -> &'static str {
    match source {
        AnalysisSource::BuiltinLexicon => "builtin-lexicon",
        AnalysisSource::FullPosLexicon => "full-pos-lexicon",
        AnalysisSource::UserLexicon => "user-lexicon",
        AnalysisSource::ProductiveSuffix => "productive-suffix",
        AnalysisSource::Heuristic => "heuristic",
        AnalysisSource::Forced => "forced",
    }
}

const fn fine_pos_label(pos: FinePos) -> &'static str {
    match pos {
        FinePos::CommonNoun => "common-noun",
        FinePos::ProperNoun => "proper-noun",
        FinePos::DependentNoun => "dependent-noun",
        FinePos::Pronoun => "pronoun",
        FinePos::Numeral => "numeral",
        FinePos::Verb => "verb",
        FinePos::Adjective => "adjective",
        FinePos::AuxiliaryVerb => "auxiliary-verb",
        FinePos::AuxiliaryAdjective => "auxiliary-adjective",
        FinePos::Copula => "copula",
        FinePos::Determiner => "determiner",
        FinePos::GeneralAdverb => "general-adverb",
        FinePos::ConjunctiveAdverb => "conjunctive-adverb",
        FinePos::Particle => "particle",
        FinePos::Interjection => "interjection",
        FinePos::Foreign => "foreign",
        FinePos::Number => "number",
        FinePos::Code => "code",
        FinePos::Literal => "literal",
    }
}

const fn alternation_label(alternation: LexicalAlternation) -> &'static str {
    match alternation {
        LexicalAlternation::Regular => "regular",
        LexicalAlternation::DToL => "d-to-l",
        LexicalAlternation::DropS => "drop-s",
        LexicalAlternation::BToWa => "b-to-wa",
        LexicalAlternation::BToWo => "b-to-wo",
        LexicalAlternation::DropH => "drop-h",
        LexicalAlternation::ReuDoubleL => "reu-double-l",
        LexicalAlternation::Reo => "reo",
        LexicalAlternation::Ha => "ha",
        LexicalAlternation::UToEo => "u-to-eo",
        LexicalAlternation::Copula => "copula",
        LexicalAlternation::Suppletive => "suppletive",
    }
}
