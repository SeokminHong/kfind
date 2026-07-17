use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;

use kfind_morph::{CoarsePos, FinePos, PredicateEntry, PredicatePos, RuleId};

use crate::lexicons::predicate_shape_alternation;
use crate::{Lexicons, QueryAtom};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisSource {
    BuiltinLexicon,
    EnrichedLexicon,
    FullPosLexicon,
    UserLexicon,
    ProductiveSuffix,
    Heuristic,
    Forced,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NominalOverride {
    pub rule_id: RuleId,
    pub surface: Box<str>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NominalMorphology {
    pub overrides: Box<[NominalOverride]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticleMorphology {
    pub variants: Box<[Box<str>]>,
    pub rule_id: Option<RuleId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Morphology {
    Predicate(PredicateEntry),
    Nominal(NominalMorphology),
    Particle(ParticleMorphology),
    Exact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analysis {
    pub lemma: Box<str>,
    pub coarse_pos: CoarsePos,
    pub fine_pos: FinePos,
    pub morphology: Morphology,
    pub source: AnalysisSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnalyzeError {
    InvalidForcedPredicateLemma { lemma: Box<str>, pos: CoarsePos },
}

impl Display for AnalyzeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidForcedPredicateLemma { lemma, pos } => write!(
                formatter,
                "forced {pos:?} query `{lemma}` must be a non-empty -다 lemma"
            ),
        }
    }
}

impl Error for AnalyzeError {}

pub trait QueryAnalyzer: Send + Sync {
    fn analyze(&self, atom: &QueryAtom) -> Result<Vec<Analysis>, AnalyzeError>;
}

#[derive(Debug, Clone)]
pub struct LexiconQueryAnalyzer {
    lexicons: Arc<Lexicons>,
}

impl LexiconQueryAnalyzer {
    #[must_use]
    pub fn new(lexicons: Arc<Lexicons>) -> Self {
        Self { lexicons }
    }

    #[must_use]
    pub fn lexicons(&self) -> &Arc<Lexicons> {
        &self.lexicons
    }
}

impl QueryAnalyzer for LexiconQueryAnalyzer {
    fn analyze(&self, atom: &QueryAtom) -> Result<Vec<Analysis>, AnalyzeError> {
        if atom.quoted_literal {
            return Ok(vec![exact_analysis(
                &atom.raw,
                atom.forced_pos.unwrap_or(CoarsePos::Literal),
                AnalysisSource::Forced,
            )]);
        }

        if let Some(forced_pos) = atom.forced_pos {
            return self.analyze_forced(&atom.raw, forced_pos);
        }

        let exact = self.lexicons.lookup_with_surface_fallback(&atom.raw);
        if !exact.is_empty() {
            return Ok(exact);
        }
        if let Some(productive) = self.lexicons.productive_predicate(&atom.raw) {
            return Ok(vec![productive]);
        }
        if atom.raw.ends_with('다') {
            return Ok(vec![exact_analysis(
                &atom.raw,
                CoarsePos::Literal,
                AnalysisSource::Heuristic,
            )]);
        }
        if is_hangul_atom(&atom.raw) {
            return Ok(vec![
                Analysis {
                    lemma: atom.raw.clone(),
                    coarse_pos: CoarsePos::Noun,
                    fine_pos: FinePos::CommonNoun,
                    morphology: Morphology::Nominal(NominalMorphology::default()),
                    source: AnalysisSource::Heuristic,
                },
                exact_analysis(&atom.raw, CoarsePos::Literal, AnalysisSource::Heuristic),
            ]);
        }
        Ok(vec![exact_analysis(
            &atom.raw,
            CoarsePos::Literal,
            AnalysisSource::Heuristic,
        )])
    }
}

impl LexiconQueryAnalyzer {
    fn analyze_forced(
        &self,
        lemma: &str,
        forced_pos: CoarsePos,
    ) -> Result<Vec<Analysis>, AnalyzeError> {
        let candidates = self.lexicons.lookup_with_surface_fallback(lemma);
        let mut matching = Vec::new();
        let mut includes_full_pos = false;
        for analysis in candidates
            .iter()
            .filter(|analysis| analysis.coarse_pos == forced_pos)
        {
            includes_full_pos |= analysis.source == AnalysisSource::FullPosLexicon;
            let mut analysis = analysis.clone();
            analysis.source = AnalysisSource::Forced;
            matching.push(analysis);
        }
        if !matching.is_empty() {
            if forced_pos == CoarsePos::Noun && includes_full_pos {
                append_missing_forced_noun_analyses(lemma, &mut matching);
            }
            if forced_pos == CoarsePos::Verb {
                append_missing_forced_auxiliary_verb(lemma, &mut matching);
            }
            return Ok(matching);
        }

        if matches!(forced_pos, CoarsePos::Verb | CoarsePos::Adjective) {
            if let Some(mut productive) = self.lexicons.productive_predicate(lemma)
                && productive.coarse_pos == forced_pos
            {
                productive.source = AnalysisSource::Forced;
                let mut analyses = vec![productive];
                if forced_pos == CoarsePos::Verb {
                    append_missing_forced_auxiliary_verb(lemma, &mut analyses);
                }
                return Ok(analyses);
            }
            let stem = lemma.strip_suffix('다').filter(|stem| !stem.is_empty());
            if stem.is_none() {
                return Err(AnalyzeError::InvalidForcedPredicateLemma {
                    lemma: lemma.into(),
                    pos: forced_pos,
                });
            }
            let predicate_pos = if forced_pos == CoarsePos::Verb {
                PredicatePos::Verb
            } else {
                PredicatePos::Adjective
            };
            let predicate = PredicateEntry::new(
                lemma,
                predicate_pos,
                predicate_shape_alternation(lemma, forced_pos)
                    .unwrap_or(kfind_morph::LexicalAlternation::Regular),
            );
            let mut analyses = vec![Analysis {
                lemma: lemma.into(),
                coarse_pos: forced_pos,
                fine_pos: predicate_pos.fine(),
                morphology: Morphology::Predicate(predicate),
                source: AnalysisSource::Forced,
            }];
            if forced_pos == CoarsePos::Verb {
                append_missing_forced_auxiliary_verb(lemma, &mut analyses);
            }
            return Ok(analyses);
        }

        Ok(forced_non_predicates(lemma, forced_pos))
    }
}

fn append_missing_forced_noun_analyses(lemma: &str, analyses: &mut Vec<Analysis>) {
    for fine_pos in [
        FinePos::CommonNoun,
        FinePos::ProperNoun,
        FinePos::DependentNoun,
    ] {
        if analyses
            .iter()
            .all(|analysis| analysis.fine_pos != fine_pos)
        {
            analyses.push(forced_non_predicate_analysis(
                lemma,
                CoarsePos::Noun,
                fine_pos,
            ));
        }
    }
}

fn append_missing_forced_auxiliary_verb(lemma: &str, analyses: &mut Vec<Analysis>) {
    if analyses
        .iter()
        .any(|analysis| analysis.fine_pos == FinePos::AuxiliaryVerb)
    {
        return;
    }
    let Some(mut predicate) = analyses.iter().find_map(|analysis| {
        if analysis.fine_pos != FinePos::Verb {
            return None;
        }
        match &analysis.morphology {
            Morphology::Predicate(predicate) => Some(predicate.clone()),
            _ => None,
        }
    }) else {
        return;
    };
    predicate.pos = PredicatePos::AuxiliaryVerb;
    analyses.push(Analysis {
        lemma: lemma.into(),
        coarse_pos: CoarsePos::Verb,
        fine_pos: FinePos::AuxiliaryVerb,
        morphology: Morphology::Predicate(predicate),
        source: AnalysisSource::Forced,
    });
}

fn forced_non_predicates(lemma: &str, pos: CoarsePos) -> Vec<Analysis> {
    let fine_positions = match pos {
        CoarsePos::Noun => &[
            FinePos::CommonNoun,
            FinePos::ProperNoun,
            FinePos::DependentNoun,
        ][..],
        CoarsePos::Pronoun => &[FinePos::Pronoun],
        CoarsePos::Numeral => &[FinePos::Numeral],
        CoarsePos::Determiner => &[FinePos::Determiner],
        CoarsePos::Adverb => &[FinePos::GeneralAdverb],
        CoarsePos::Particle => &[FinePos::Particle],
        CoarsePos::Interjection => &[FinePos::Interjection],
        CoarsePos::Literal => &[FinePos::Literal],
        CoarsePos::Verb | CoarsePos::Adjective => unreachable!("predicate handled separately"),
    };
    fine_positions
        .iter()
        .copied()
        .map(|fine_pos| forced_non_predicate_analysis(lemma, pos, fine_pos))
        .collect()
}

fn forced_non_predicate_analysis(lemma: &str, pos: CoarsePos, fine_pos: FinePos) -> Analysis {
    Analysis {
        lemma: lemma.into(),
        coarse_pos: pos,
        fine_pos,
        morphology: match pos {
            CoarsePos::Noun | CoarsePos::Pronoun | CoarsePos::Numeral => {
                Morphology::Nominal(NominalMorphology::default())
            }
            CoarsePos::Particle => Morphology::Particle(ParticleMorphology {
                variants: vec![Box::<str>::from(lemma)].into_boxed_slice(),
                rule_id: None,
            }),
            CoarsePos::Determiner
            | CoarsePos::Adverb
            | CoarsePos::Interjection
            | CoarsePos::Literal => Morphology::Exact,
            CoarsePos::Verb | CoarsePos::Adjective => {
                unreachable!("predicate handled separately")
            }
        },
        source: AnalysisSource::Forced,
    }
}

fn exact_analysis(lemma: &str, pos: CoarsePos, source: AnalysisSource) -> Analysis {
    let fine_pos = match pos {
        CoarsePos::Noun => FinePos::CommonNoun,
        CoarsePos::Pronoun => FinePos::Pronoun,
        CoarsePos::Numeral => FinePos::Numeral,
        CoarsePos::Verb => FinePos::Verb,
        CoarsePos::Adjective => FinePos::Adjective,
        CoarsePos::Determiner => FinePos::Determiner,
        CoarsePos::Adverb => FinePos::GeneralAdverb,
        CoarsePos::Particle => FinePos::Particle,
        CoarsePos::Interjection => FinePos::Interjection,
        CoarsePos::Literal => FinePos::Literal,
    };
    Analysis {
        lemma: lemma.into(),
        coarse_pos: pos,
        fine_pos,
        morphology: Morphology::Exact,
        source,
    }
}

fn is_hangul_atom(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|character| {
        matches!(
            character as u32,
            0x1100..=0x11ff | 0x3130..=0x318f | 0xa960..=0xa97f | 0xac00..=0xd7a3 | 0xd7b0..=0xd7ff
        )
    })
}
