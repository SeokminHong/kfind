use std::collections::BTreeMap;
use std::sync::Arc;

use kfind_data::{
    DataAlternation, DataError, DataFinePos, DerivationRule, LexiconData, LexiconSources,
    PosLexiconEntry, RuleSet, RuleSources, UserLexicon, decode_pos_lexicon, parse_lexicons,
    parse_rule_set,
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
    entries: BTreeMap<Box<str>, Vec<Analysis>>,
    rules: Arc<RuleSet>,
    full_pos_loaded: bool,
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
            entries: BTreeMap::new(),
            rules: Arc::new(rules),
            full_pos_loaded: false,
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
        let decoded = decode_pos_lexicon(input)?;
        for entry in decoded.entries() {
            self.insert_full_pos(entry);
        }
        self.full_pos_loaded = true;
        Ok(())
    }

    pub fn merge_user(&mut self, user: &UserLexicon) {
        for record in &user.predicates {
            if record.replace {
                self.remove_morphology(&record.entry.lemma, MorphologyKind::Predicate);
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
    pub fn lookup(&self, surface: &str) -> &[Analysis] {
        self.entries.get(surface).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn rules(&self) -> &RuleSet {
        &self.rules
    }

    #[must_use]
    pub const fn full_pos_loaded(&self) -> bool {
        self.full_pos_loaded
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

    fn insert_full_pos(&mut self, entry: &PosLexiconEntry) {
        let fine_pos = data_fine_pos(entry.pos);
        let analysis = default_analysis(&entry.lemma, entry.pos, AnalysisSource::BuiltinLexicon);
        self.insert_analysis(
            entry.lemma.clone().into_boxed_str(),
            analysis,
            self.lookup(&entry.lemma)
                .iter()
                .any(|existing| existing.fine_pos == fine_pos),
        );
    }

    fn insert_analysis(&mut self, key: Box<str>, analysis: Analysis, skip: bool) {
        if skip {
            return;
        }
        let entries = self.entries.entry(key).or_default();
        if !entries.contains(&analysis) {
            entries.push(analysis);
        }
    }

    fn remove_morphology(&mut self, lemma: &str, kind: MorphologyKind) {
        if let Some(entries) = self.entries.get_mut(lemma) {
            entries.retain(|analysis| kind.does_not_match(&analysis.morphology));
        }
    }
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

fn default_analysis(lemma: &str, pos: DataFinePos, source: AnalysisSource) -> Analysis {
    if pos.is_predicate() {
        let predicate_pos = predicate_pos(pos);
        let predicate = PredicateEntry::new(
            lemma,
            predicate_pos,
            if pos == DataFinePos::Vcp {
                LexicalAlternation::Copula
            } else {
                LexicalAlternation::Regular
            },
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
        _ => return None,
    })
}
