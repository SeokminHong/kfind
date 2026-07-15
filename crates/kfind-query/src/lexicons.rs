use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use kfind_data::{
    DataAlternation, DataError, DataFinePos, DecodedPosLexicon, DerivationRule, LexiconData,
    LexiconSources, PosLexiconEntry, RuleSet, RuleSources, UserLexicon, decode_pos_lexicon,
    parse_lexicons, parse_predicates_tsv, parse_rule_set, validate_predicates,
};
use kfind_morph::{
    CoarsePos, ContinuationState, FinePos, LexicalAlternation, PredicateEntry, PredicateFlags,
    PredicatePos, RuleId, SurfaceOverride,
};

use crate::{
    Analysis, AnalysisSource, Morphology, NominalMorphology, NominalOverride, ParticleMorphology,
};

const PREDICATES: &str = include_str!("../../../data/lexicon/predicates.tsv");
const NOMINALS: &str = include_str!("../../../data/lexicon/nominals.tsv");
const MODIFIERS: &str = include_str!("../../../data/lexicon/modifiers.tsv");
const PARTICLES: &str = include_str!("../../../data/lexicon/particles.tsv");
const ENDINGS: &str = include_str!("../../../data/rules/endings.toml");
const ALTERNATIONS: &str = include_str!("../../../data/rules/alternations.toml");
const CONTRACTIONS: &str = include_str!("../../../data/rules/contractions.toml");
const DERIVATIONS: &str = include_str!("../../../data/rules/derivations.toml");
const PARTICLE_RULES: &str = include_str!("../../../data/rules/particles.toml");

#[derive(Debug, Clone)]
pub struct Lexicons {
    materialized_entries: BTreeMap<Box<str>, Vec<Analysis>>,
    rules: Arc<RuleSet>,
    full_pos: Option<DecodedPosLexicon>,
    enriched_predicates_loaded: bool,
    replaced_full_predicates: BTreeSet<Box<str>>,
    replaced_full_nominals: BTreeSet<Box<str>>,
}

impl Lexicons {
    pub fn embedded() -> Result<Self, DataError> {
        let (lexicon, _) = parse_lexicons(LexiconSources {
            predicates: PREDICATES,
            nominals: NOMINALS,
            modifiers: MODIFIERS,
            particles: PARTICLES,
        })?;
        let rules = parse_rule_set(RuleSources {
            endings: ENDINGS,
            alternations: ALTERNATIONS,
            contractions: CONTRACTIONS,
            derivations: DERIVATIONS,
            particles: PARTICLE_RULES,
        })?;
        let mut result = Self {
            materialized_entries: BTreeMap::new(),
            rules: Arc::new(rules),
            full_pos: None,
            enriched_predicates_loaded: false,
            replaced_full_predicates: BTreeSet::new(),
            replaced_full_nominals: BTreeSet::new(),
        };
        result.insert_core(&lexicon);
        Ok(result)
    }

    pub fn embedded_with(
        full_pos: Option<&[u8]>,
        user: Option<&UserLexicon>,
    ) -> Result<Self, DataError> {
        let mut lexicons = Self::embedded()?;
        if let Some(binary) = full_pos {
            lexicons.load_full_pos(binary)?;
        }
        if let Some(user) = user {
            lexicons.merge_user(user);
        }
        Ok(lexicons)
    }

    pub fn load_full_pos(&mut self, input: &[u8]) -> Result<(), DataError> {
        self.full_pos = Some(decode_pos_lexicon(input)?);
        Ok(())
    }

    /// Validates and merges an external predicate metadata layer.
    pub fn load_enriched_predicates(&mut self, source: &str, input: &str) -> Result<(), DataError> {
        let (records, _) = parse_predicates_tsv(source, input)?;
        validate_predicates(source, &records, &self.rules)?;
        for record in records {
            self.insert_enriched_analysis(
                record.lemma.clone().into_boxed_str(),
                predicate_analysis(&record, AnalysisSource::EnrichedLexicon),
            );
        }
        self.enriched_predicates_loaded = true;
        Ok(())
    }

    pub fn merge_user(&mut self, user: &UserLexicon) {
        for record in &user.predicates {
            if record.replace {
                self.remove_morphology(&record.entry.lemma, MorphologyKind::Predicate);
                self.replaced_full_predicates
                    .insert(record.entry.lemma.clone().into_boxed_str());
            }
            self.insert_analysis(
                record.entry.lemma.clone().into_boxed_str(),
                predicate_analysis(&record.entry, AnalysisSource::UserLexicon),
                false,
            );
        }
        for record in &user.nominals {
            if record.replace {
                self.remove_morphology(&record.entry.lemma, MorphologyKind::Nominal);
                self.replaced_full_nominals
                    .insert(record.entry.lemma.clone().into_boxed_str());
            }
            self.insert_analysis(
                record.entry.lemma.clone().into_boxed_str(),
                nominal_analysis(
                    &record.entry.lemma,
                    record.entry.pos,
                    &record.entry.overrides,
                    AnalysisSource::UserLexicon,
                ),
                false,
            );
        }
    }

    #[must_use]
    pub fn lookup(&self, surface: &str) -> Cow<'_, [Analysis]> {
        let materialized = self
            .materialized_entries
            .get(surface)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let Some(full_pos) = &self.full_pos else {
            return Cow::Borrowed(materialized);
        };
        let candidates = full_pos.lookup(surface);
        if candidates.is_empty() {
            return Cow::Borrowed(materialized);
        }

        let mut analyses = materialized
            .iter()
            .filter(|analysis| analysis.source != AnalysisSource::UserLexicon)
            .cloned()
            .collect::<Vec<_>>();
        self.append_full_pos_analyses(surface, candidates, &mut analyses);
        analyses.extend(
            materialized
                .iter()
                .filter(|analysis| analysis.source == AnalysisSource::UserLexicon)
                .cloned(),
        );
        Cow::Owned(analyses)
    }

    #[must_use]
    pub fn rules(&self) -> &RuleSet {
        &self.rules
    }

    #[must_use]
    pub fn full_pos_loaded(&self) -> bool {
        self.full_pos.is_some()
    }

    #[must_use]
    pub fn enriched_predicates_loaded(&self) -> bool {
        self.enriched_predicates_loaded
    }

    pub(crate) fn productive_predicate(&self, lemma: &str) -> Option<Analysis> {
        let rule = self
            .rules
            .derivations
            .iter()
            .filter(|rule| rule.suffix.ends_with('다'))
            .filter(|rule| {
                lemma
                    .strip_suffix(&rule.suffix)
                    .is_some_and(|base| !base.is_empty())
            })
            .max_by_key(|rule| rule.suffix.len())?;
        predicate_from_derivation(lemma, rule, AnalysisSource::ProductiveSuffix)
    }

    fn insert_core(&mut self, lexicon: &LexiconData) {
        for record in &lexicon.predicates {
            self.insert_analysis(
                record.lemma.clone().into_boxed_str(),
                predicate_analysis(record, AnalysisSource::BuiltinLexicon),
                false,
            );
        }
        for record in &lexicon.nominals {
            self.insert_analysis(
                record.lemma.clone().into_boxed_str(),
                nominal_analysis(
                    &record.lemma,
                    record.pos,
                    &record.overrides,
                    AnalysisSource::BuiltinLexicon,
                ),
                false,
            );
        }
        for record in &lexicon.modifiers {
            self.insert_analysis(
                record.lemma.clone().into_boxed_str(),
                exact_fine_analysis(
                    &record.lemma,
                    data_fine_pos(record.pos),
                    AnalysisSource::BuiltinLexicon,
                ),
                false,
            );
        }
        for record in &lexicon.particles {
            let morphology = Morphology::Particle(ParticleMorphology {
                variants: record
                    .variants
                    .iter()
                    .map(|variant| variant.clone().into_boxed_str())
                    .collect(),
                rule_id: Some(RuleId::from(record.rule_id.clone())),
            });
            for variant in &record.variants {
                self.insert_analysis(
                    variant.clone().into_boxed_str(),
                    Analysis {
                        lemma: record.lemma.clone().into_boxed_str(),
                        coarse_pos: CoarsePos::Particle,
                        fine_pos: FinePos::Particle,
                        morphology: morphology.clone(),
                        source: AnalysisSource::BuiltinLexicon,
                    },
                    false,
                );
            }
        }
    }

    fn append_full_pos_analyses(
        &self,
        lemma: &str,
        candidates: &[PosLexiconEntry],
        analyses: &mut Vec<Analysis>,
    ) {
        for entry in candidates {
            let fine_pos = data_fine_pos(entry.pos);
            let suppressed_by_user = (entry.pos.is_predicate()
                && self.replaced_full_predicates.contains(lemma))
                || (entry.pos.is_nominal() && self.replaced_full_nominals.contains(lemma));
            let suppressed_by_curated = analyses.iter().any(|analysis| {
                matches!(
                    analysis.source,
                    AnalysisSource::BuiltinLexicon | AnalysisSource::EnrichedLexicon
                ) && if entry.pos.is_predicate() {
                    analysis.coarse_pos == fine_pos.coarse() && !is_surface_only_analysis(analysis)
                } else {
                    analysis.fine_pos == fine_pos
                }
            });
            if suppressed_by_user || suppressed_by_curated {
                continue;
            }
            let analysis = self.full_pos_analysis(entry);
            if !analyses.contains(&analysis) {
                analyses.push(analysis);
            }
        }
    }

    fn full_pos_analysis(&self, entry: &PosLexiconEntry) -> Analysis {
        let fine_pos = data_fine_pos(entry.pos);
        let productive_alternation = entry.pos.is_predicate().then(|| {
            self.productive_predicate(&entry.lemma)
                .filter(|analysis| analysis.coarse_pos == fine_pos.coarse())
                .and_then(|analysis| match analysis.morphology {
                    Morphology::Predicate(predicate) => Some(predicate.alternation),
                    _ => None,
                })
        });
        default_analysis(
            &entry.lemma,
            entry.pos,
            productive_alternation
                .flatten()
                .or_else(|| predicate_shape_alternation(&entry.lemma, fine_pos.coarse())),
            AnalysisSource::FullPosLexicon,
        )
    }

    pub(crate) fn lookup_with_surface_fallback(&self, lemma: &str) -> Vec<Analysis> {
        let mut analyses = self.lookup(lemma).into_owned();
        let surface_positions = analyses
            .iter()
            .filter_map(|analysis| match &analysis.morphology {
                Morphology::Predicate(predicate)
                    if predicate.alternation == LexicalAlternation::SurfaceOnly =>
                {
                    Some(predicate.pos)
                }
                _ => None,
            })
            .collect::<BTreeSet<_>>();
        for pos in surface_positions {
            if analyses.iter().any(|analysis| {
                matches!(
                    &analysis.morphology,
                    Morphology::Predicate(predicate)
                        if predicate.pos == pos
                            && predicate.alternation != LexicalAlternation::SurfaceOnly
                )
            }) {
                continue;
            }
            let mut fallback = self
                .productive_predicate(lemma)
                .filter(|analysis| analysis.coarse_pos == pos.coarse())
                .unwrap_or_else(|| Analysis {
                    lemma: lemma.into(),
                    coarse_pos: pos.coarse(),
                    fine_pos: pos.fine(),
                    morphology: Morphology::Predicate(PredicateEntry::new(
                        lemma,
                        pos,
                        predicate_shape_alternation(lemma, pos.coarse())
                            .unwrap_or(LexicalAlternation::Regular),
                    )),
                    source: AnalysisSource::EnrichedLexicon,
                });
            fallback.source = AnalysisSource::EnrichedLexicon;
            analyses.push(fallback);
        }
        analyses
    }

    fn insert_analysis(&mut self, key: Box<str>, analysis: Analysis, skip: bool) {
        if skip {
            return;
        }
        let entries = self.materialized_entries.entry(key).or_default();
        if !entries.contains(&analysis) {
            entries.push(analysis);
        }
    }

    fn insert_enriched_analysis(&mut self, key: Box<str>, analysis: Analysis) {
        let entries = self.materialized_entries.entry(key).or_default();
        if entries
            .iter()
            .any(|existing| same_lexical_analysis(existing, &analysis))
        {
            return;
        }
        entries.push(analysis);
    }

    fn remove_morphology(&mut self, lemma: &str, kind: MorphologyKind) {
        if let Some(entries) = self.materialized_entries.get_mut(lemma) {
            entries.retain(|analysis| kind.does_not_match(&analysis.morphology));
        }
    }
}

fn same_lexical_analysis(left: &Analysis, right: &Analysis) -> bool {
    left.lemma == right.lemma
        && left.coarse_pos == right.coarse_pos
        && left.fine_pos == right.fine_pos
        && left.morphology == right.morphology
}

fn is_surface_only_analysis(analysis: &Analysis) -> bool {
    matches!(
        &analysis.morphology,
        Morphology::Predicate(predicate)
            if predicate.alternation == LexicalAlternation::SurfaceOnly
    )
}

pub(crate) fn predicate_shape_alternation(
    lemma: &str,
    pos: CoarsePos,
) -> Option<LexicalAlternation> {
    if lemma.ends_with("하다") {
        return Some(LexicalAlternation::Ha);
    }
    if pos == CoarsePos::Adjective
        && ["스럽다", "답다", "롭다"]
            .iter()
            .any(|suffix| lemma.ends_with(suffix))
    {
        return Some(LexicalAlternation::BToWo);
    }
    None
}

#[derive(Clone, Copy)]
enum MorphologyKind {
    Predicate,
    Nominal,
}

impl MorphologyKind {
    fn does_not_match(self, morphology: &Morphology) -> bool {
        !matches!(
            (self, morphology),
            (Self::Predicate, Morphology::Predicate(_)) | (Self::Nominal, Morphology::Nominal(_))
        )
    }
}

fn predicate_analysis(record: &kfind_data::PredicateRecord, source: AnalysisSource) -> Analysis {
    let pos = predicate_pos(record.pos);
    let flags = record
        .flags
        .iter()
        .fold(PredicateFlags::NONE, |flags, flag| {
            flags
                | match flag.as_str() {
                    "EU_DROP" => PredicateFlags::EU_DROP,
                    "RIEUL_DROP" => PredicateFlags::RIEUL_DROP,
                    "NO_I_EO_CONTRACTION" => PredicateFlags::NO_I_EO_CONTRACTION,
                    "NO_DECLARATIVE_CONTINUATION" => PredicateFlags::NO_DECLARATIVE_CONTINUATION,
                    _ => PredicateFlags::NONE,
                }
        });
    let overrides = record
        .overrides
        .iter()
        .map(|entry| SurfaceOverride {
            surface: entry.surface.clone().into_boxed_str(),
            core_len: entry.surface.len(),
            continuation: ContinuationState::Terminal,
            rule_id: RuleId::from(entry.rule_id.clone()),
        })
        .collect();
    let predicate = PredicateEntry {
        lemma: record.lemma.clone().into_boxed_str(),
        pos,
        alternation: data_alternation(record.alternation),
        flags,
        overrides,
    };
    Analysis {
        lemma: predicate.lemma.clone(),
        coarse_pos: pos.coarse(),
        fine_pos: pos.fine(),
        morphology: Morphology::Predicate(predicate),
        source,
    }
}

fn nominal_analysis(
    lemma: &str,
    pos: DataFinePos,
    overrides: &[kfind_data::SurfaceOverride],
    source: AnalysisSource,
) -> Analysis {
    Analysis {
        lemma: lemma.into(),
        coarse_pos: data_fine_pos(pos).coarse(),
        fine_pos: data_fine_pos(pos),
        morphology: Morphology::Nominal(NominalMorphology {
            overrides: overrides
                .iter()
                .map(|entry| NominalOverride {
                    rule_id: RuleId::from(entry.rule_id.clone()),
                    surface: entry.surface.clone().into_boxed_str(),
                })
                .collect(),
        }),
        source,
    }
}

fn exact_fine_analysis(lemma: &str, fine_pos: FinePos, source: AnalysisSource) -> Analysis {
    Analysis {
        lemma: lemma.into(),
        coarse_pos: fine_pos.coarse(),
        fine_pos,
        morphology: Morphology::Exact,
        source,
    }
}

fn default_analysis(
    lemma: &str,
    pos: DataFinePos,
    productive_alternation: Option<LexicalAlternation>,
    source: AnalysisSource,
) -> Analysis {
    if pos.is_predicate() {
        let predicate_pos = predicate_pos(pos);
        let predicate = PredicateEntry::new(
            lemma,
            predicate_pos,
            productive_alternation.unwrap_or(if pos == DataFinePos::Vcp {
                LexicalAlternation::Copula
            } else {
                LexicalAlternation::Regular
            }),
        );
        Analysis {
            lemma: lemma.into(),
            coarse_pos: predicate_pos.coarse(),
            fine_pos: predicate_pos.fine(),
            morphology: Morphology::Predicate(predicate),
            source,
        }
    } else if pos.is_nominal() {
        nominal_analysis(lemma, pos, &[], source)
    } else if pos.is_particle() {
        Analysis {
            lemma: lemma.into(),
            coarse_pos: CoarsePos::Particle,
            fine_pos: FinePos::Particle,
            morphology: Morphology::Particle(ParticleMorphology {
                variants: vec![Box::<str>::from(lemma)].into_boxed_slice(),
                rule_id: None,
            }),
            source,
        }
    } else {
        exact_fine_analysis(lemma, data_fine_pos(pos), source)
    }
}

pub(crate) fn predicate_from_derivation(
    lemma: &str,
    rule: &DerivationRule,
    source: AnalysisSource,
) -> Option<Analysis> {
    let predicate_pos = predicate_pos_if_supported(rule.result_pos)?;
    let alternation = rule
        .alternation_id
        .as_deref()
        .and_then(alternation_from_rule_id)
        .unwrap_or(LexicalAlternation::Regular);
    let predicate = PredicateEntry::new(lemma, predicate_pos, alternation);
    Some(Analysis {
        lemma: lemma.into(),
        coarse_pos: predicate_pos.coarse(),
        fine_pos: predicate_pos.fine(),
        morphology: Morphology::Predicate(predicate),
        source,
    })
}

pub(crate) fn data_fine_pos(pos: DataFinePos) -> FinePos {
    match pos {
        DataFinePos::Nng => FinePos::CommonNoun,
        DataFinePos::Nnp => FinePos::ProperNoun,
        DataFinePos::Nnb => FinePos::DependentNoun,
        DataFinePos::Nr => FinePos::Numeral,
        DataFinePos::Np => FinePos::Pronoun,
        DataFinePos::Vv => FinePos::Verb,
        DataFinePos::Va => FinePos::Adjective,
        DataFinePos::Vx => FinePos::AuxiliaryVerb,
        DataFinePos::Vcp => FinePos::Copula,
        DataFinePos::Vcn => FinePos::Adjective,
        DataFinePos::Mm => FinePos::Determiner,
        DataFinePos::Mag => FinePos::GeneralAdverb,
        DataFinePos::Maj => FinePos::ConjunctiveAdverb,
        DataFinePos::Ic => FinePos::Interjection,
        DataFinePos::Jks
        | DataFinePos::Jkc
        | DataFinePos::Jkg
        | DataFinePos::Jko
        | DataFinePos::Jkb
        | DataFinePos::Jkv
        | DataFinePos::Jkq
        | DataFinePos::Jx
        | DataFinePos::Jc => FinePos::Particle,
    }
}

fn predicate_pos(pos: DataFinePos) -> PredicatePos {
    predicate_pos_if_supported(pos).expect("predicate POS checked by data parser")
}

fn predicate_pos_if_supported(pos: DataFinePos) -> Option<PredicatePos> {
    match pos {
        DataFinePos::Vv => Some(PredicatePos::Verb),
        DataFinePos::Va => Some(PredicatePos::Adjective),
        DataFinePos::Vx => Some(PredicatePos::AuxiliaryVerb),
        DataFinePos::Vcp => Some(PredicatePos::Copula),
        DataFinePos::Vcn => Some(PredicatePos::Adjective),
        _ => None,
    }
}

fn data_alternation(value: DataAlternation) -> LexicalAlternation {
    match value {
        DataAlternation::Regular => LexicalAlternation::Regular,
        DataAlternation::DToL => LexicalAlternation::DToL,
        DataAlternation::DropS => LexicalAlternation::DropS,
        DataAlternation::BToWa => LexicalAlternation::BToWa,
        DataAlternation::BToWo => LexicalAlternation::BToWo,
        DataAlternation::DropH => LexicalAlternation::DropH,
        DataAlternation::ReuDoubleL => LexicalAlternation::ReuDoubleL,
        DataAlternation::Reo => LexicalAlternation::Reo,
        DataAlternation::Ha => LexicalAlternation::Ha,
        DataAlternation::UToEo => LexicalAlternation::UToEo,
        DataAlternation::Copula => LexicalAlternation::Copula,
        DataAlternation::Suppletive => LexicalAlternation::Suppletive,
        DataAlternation::SurfaceOnly => LexicalAlternation::SurfaceOnly,
    }
}

fn alternation_from_rule_id(id: &str) -> Option<LexicalAlternation> {
    Some(match id {
        "lexical.regular" => LexicalAlternation::Regular,
        "lexical.d-to-l" => LexicalAlternation::DToL,
        "lexical.drop-s" => LexicalAlternation::DropS,
        "lexical.b-to-wa" => LexicalAlternation::BToWa,
        "lexical.b-to-wo" => LexicalAlternation::BToWo,
        "lexical.drop-h" => LexicalAlternation::DropH,
        "lexical.reu-double-l" => LexicalAlternation::ReuDoubleL,
        "lexical.reo" => LexicalAlternation::Reo,
        "lexical.ha" => LexicalAlternation::Ha,
        "lexical.u-to-eo" => LexicalAlternation::UToEo,
        "lexical.copula" => LexicalAlternation::Copula,
        "lexical.suppletive" => LexicalAlternation::Suppletive,
        "lexical.surface-only" => LexicalAlternation::SurfaceOnly,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use kfind_data::{NominalRecord, collect_pos_entries, encode_pos_lexicon};

    use super::*;

    #[test]
    fn loading_full_pos_does_not_materialize_all_entries() {
        let mut lexicons = Lexicons::embedded().unwrap();
        let materialized_before = lexicons.materialized_entries.len();
        let full_data = LexiconData {
            nominals: vec![NominalRecord {
                lemma: "대규모사전".to_owned(),
                pos: DataFinePos::Nng,
                flags: BTreeSet::new(),
                overrides: Vec::new(),
            }],
            ..LexiconData::default()
        };
        let binary = encode_pos_lexicon(&collect_pos_entries(&full_data)).unwrap();

        lexicons.load_full_pos(&binary).unwrap();

        assert_eq!(lexicons.materialized_entries.len(), materialized_before);
        assert!(lexicons.lookup("없는표제어").is_empty());
        assert!(lexicons.lookup("대규모사전").iter().any(|analysis| {
            analysis.source == AnalysisSource::FullPosLexicon
                && analysis.fine_pos == FinePos::CommonNoun
        }));
    }

    #[test]
    fn enriched_predicates_suppress_the_same_full_pos_coarse_pos() {
        let full_data = LexiconData {
            predicates: vec![
                kfind_data::PredicateRecord {
                    lemma: "가르다".to_owned(),
                    pos: DataFinePos::Vv,
                    alternation: DataAlternation::Regular,
                    flags: BTreeSet::new(),
                    overrides: Vec::new(),
                },
                kfind_data::PredicateRecord {
                    lemma: "가르다".to_owned(),
                    pos: DataFinePos::Vx,
                    alternation: DataAlternation::Regular,
                    flags: BTreeSet::new(),
                    overrides: Vec::new(),
                },
                kfind_data::PredicateRecord {
                    lemma: "가르다".to_owned(),
                    pos: DataFinePos::Va,
                    alternation: DataAlternation::Regular,
                    flags: BTreeSet::new(),
                    overrides: Vec::new(),
                },
            ],
            ..LexiconData::default()
        };
        let binary = encode_pos_lexicon(&collect_pos_entries(&full_data)).unwrap();
        let mut lexicons = Lexicons::embedded_with(Some(&binary), None).unwrap();
        lexicons
            .load_enriched_predicates(
                "fixture.tsv",
                "lemma\tpos\talternation\tflags\toverrides\n가르다\tVV\tReuDoubleL\t\t\n",
            )
            .unwrap();

        let analyses = lexicons.lookup("가르다");
        assert!(analyses.iter().any(|analysis| {
            analysis.source == AnalysisSource::EnrichedLexicon && analysis.fine_pos == FinePos::Verb
        }));
        assert!(analyses.iter().any(|analysis| {
            analysis.source == AnalysisSource::FullPosLexicon
                && analysis.fine_pos == FinePos::Adjective
        }));
        assert!(!analyses.iter().any(|analysis| {
            analysis.source == AnalysisSource::FullPosLexicon
                && matches!(analysis.fine_pos, FinePos::Verb | FinePos::AuxiliaryVerb)
        }));
    }

    #[test]
    fn enriched_predicates_are_validated_against_runtime_rules() {
        let mut lexicons = Lexicons::embedded().unwrap();
        let error = lexicons
            .load_enriched_predicates(
                "fixture.tsv",
                "lemma\tpos\talternation\tflags\toverrides\n가르다\tVV\tReo\tEU_DROP\t\n",
            )
            .unwrap_err();

        assert_eq!(error.location.source, "fixture.tsv");
    }
}
