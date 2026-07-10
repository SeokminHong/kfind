use std::collections::BTreeSet;
use std::io::{self, Write};

use kfind_morph::{CoarsePos, FinePos, LexicalAlternation};
use kfind_query::{AnalysisSource, Morphology, QueryDiagnostic, QueryPlan, VerifiedSpan};
use kfind_search::SearchLine;

use super::text::write_safe_bytes;

pub(super) fn write_query_plan(writer: &mut impl Write, plan: &QueryPlan) -> io::Result<()> {
    writer.write_all(b"query: ")?;
    write_safe_bytes(writer, plan.raw_query.as_bytes())?;
    writer.write_all(b"\n")?;

    for (atom_index, atom) in plan.atoms.iter().enumerate() {
        writeln!(writer, "atom[{atom_index}]:")?;
        writer.write_all(b"  analyses:\n")?;
        for analysis in &atom.analyses {
            writer.write_all(b"    - lemma: ")?;
            write_safe_bytes(writer, analysis.lemma.as_bytes())?;
            writeln!(writer, "\n      pos: {}", pos_label(analysis.coarse_pos))?;
            writeln!(
                writer,
                "      fine_pos: {}",
                fine_pos_label(analysis.fine_pos)
            )?;
            writeln!(writer, "      source: {}", source_label(analysis.source))?;
            if let Morphology::Predicate(predicate) = &analysis.morphology {
                writeln!(
                    writer,
                    "      alternation: {}",
                    alternation_label(predicate.alternation)
                )?;
            }
        }
        writeln!(writer, "  branches: {}", atom.branches.len())?;
        writer.write_all(b"  anchors:\n")?;
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
    }
    writeln!(writer, "max_gap: {}", plan.phrase_policy.max_gap)?;
    writeln!(
        writer,
        "estimated_matcher_bytes: {}",
        plan.estimated_matcher_bytes
    )?;
    if !plan.diagnostics.is_empty() {
        writer.write_all(b"diagnostics:\n")?;
        for diagnostic in &plan.diagnostics {
            writer.write_all(b"  - ")?;
            write_diagnostic(writer, diagnostic)?;
            writer.write_all(b"\n")?;
        }
    }
    Ok(())
}

pub(super) fn write_match_explanations(
    writer: &mut impl Write,
    line: &SearchLine,
    plan: &QueryPlan,
) -> io::Result<()> {
    for (match_index, matched) in line.matches.iter().enumerate() {
        writeln!(writer, "  match[{match_index}]:")?;
        for (atom_index, span) in matched.atoms.iter().enumerate() {
            writeln!(writer, "    atom[{atom_index}]:")?;
            writer.write_all(b"      token: ")?;
            write_span(writer, &line.bytes, span, true)?;
            writer.write_all(b"\n      core: ")?;
            write_span(writer, &line.bytes, span, false)?;
            writer.write_all(b"\n      origins:\n")?;
            for origin in &span.origins {
                let analysis = plan
                    .atoms
                    .get(atom_index)
                    .and_then(|atom| atom.analyses.get(usize::from(origin.analysis_index)));
                writer.write_all(b"        - generated_from: ")?;
                if let Some(analysis) = analysis {
                    write_safe_bytes(writer, analysis.lemma.as_bytes())?;
                    writeln!(
                        writer,
                        "\n          pos: {}",
                        pos_label(analysis.coarse_pos)
                    )?;
                } else {
                    writeln!(writer, "analysis[{}]", origin.analysis_index)?;
                }
                writer.write_all(b"          rules:")?;
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
) -> io::Result<()> {
    let range = if token { &span.token } else { &span.core };
    if let Some(surface) = bytes.get(range.clone()) {
        write_safe_bytes(writer, surface)
    } else {
        writer.write_all(b"<invalid-span>")
    }
}

fn write_diagnostic(writer: &mut impl Write, diagnostic: &QueryDiagnostic) -> io::Result<()> {
    match diagnostic {
        QueryDiagnostic::FullPosLexiconUnavailable => {
            writer.write_all(b"full POS lexicon unavailable")
        }
        QueryDiagnostic::UnregisteredDaLiteralOnly { atom_index, lemma } => {
            write!(
                writer,
                "atom[{atom_index}] unregistered -da lemma is literal-only: "
            )?;
            write_safe_bytes(writer, lemma.as_bytes())
        }
        QueryDiagnostic::VerifierVocabularyRestricted { excluded_rule_ids } => {
            writer.write_all(b"verifier vocabulary excluded rules:")?;
            for rule in excluded_rule_ids {
                write!(writer, " {rule}")?;
            }
            Ok(())
        }
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
