use std::collections::{BTreeMap, BTreeSet};

use kfind_data::{
    DICTIONARY_ADVERBIAL_I_RULE_ID, DICTIONARY_CONJUGATION_RULE_ID,
    DICTIONARY_RELATED_ADVERB_RULE_ID, DICTIONARY_VOICE_DERIVATION_RULE_ID, DataAlternation,
    DerivationRule, PredicateRecord,
};
use kfind_morph::{
    LexicalAlternation, PredicateEntry, PredicateFlags, PredicatePos, generate_predicate_branches,
    verify_predicate_continuation,
};

use crate::model::{CandidateKey, CoreEntries, Evidence, SourceRecord};
use crate::output::{CandidateStatus, candidate_status};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SurfaceKind {
    Conjugation,
    AttestedAdverbial,
    RelatedAdverb,
}

impl SurfaceKind {
    pub const fn rule_id(self) -> &'static str {
        match self {
            Self::Conjugation => DICTIONARY_CONJUGATION_RULE_ID,
            Self::AttestedAdverbial => DICTIONARY_ADVERBIAL_I_RULE_ID,
            Self::RelatedAdverb => DICTIONARY_RELATED_ADVERB_RULE_ID,
        }
    }

    pub const fn status(self) -> &'static str {
        match self {
            Self::Conjugation => "dictionary-conjugation",
            Self::AttestedAdverbial => "dictionary-adverbial-i",
            Self::RelatedAdverb => "dictionary-related-adverb",
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SurfaceKey {
    pub lemma: String,
    pub pos: String,
    pub surface: String,
    pub kind: SurfaceKind,
}

#[derive(Clone, Debug)]
pub struct SurfaceCandidate {
    pub key: SurfaceKey,
    pub evidence: Evidence,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PredicateDerivationKey {
    pub lemma: String,
    pub pos: String,
    pub target_lemma: String,
}

#[derive(Clone, Debug)]
pub struct PredicateDerivationCandidate {
    pub key: PredicateDerivationKey,
    pub evidence: Evidence,
}

impl PredicateDerivationCandidate {
    pub const fn rule_id(&self) -> &'static str {
        DICTIONARY_VOICE_DERIVATION_RULE_ID
    }
}

#[derive(Clone, Debug, Default)]
pub struct SurfaceCollection {
    pub candidates: Vec<SurfaceCandidate>,
    pub predicate_derivations: Vec<PredicateDerivationCandidate>,
    pub agreed_conjugation_count: usize,
    pub generated_conjugation_count: usize,
    pub agreed_adverbial_i_count: usize,
    pub related_adverb_count: usize,
    pub surface_related_adverb_count: usize,
    pub related_voice_derivation_count: usize,
}

pub fn collect_surfaces(
    records: &[SourceRecord],
    core_records: &[PredicateRecord],
    candidates: &BTreeMap<CandidateKey, Evidence>,
    core: &CoreEntries,
    derivations: &[DerivationRule],
) -> SurfaceCollection {
    let mut conjugations = BTreeMap::<SurfaceKey, Evidence>::new();
    let mut attested_adverbials = BTreeMap::<SurfaceKey, Evidence>::new();
    let mut related_adverbs = BTreeMap::<SurfaceKey, Evidence>::new();
    let general_predicates = records
        .iter()
        .filter(|record| record.lexical_status == "일반어")
        .fold(
            BTreeMap::<(String, String, String), BTreeSet<String>>::new(),
            |mut output, record| {
                output
                    .entry((
                        record.source.clone(),
                        record.lemma.clone(),
                        record.pos.clone(),
                    ))
                    .or_default()
                    .insert(record.source_id.clone());
                output
            },
        );
    let mut predicate_derivations = BTreeMap::<PredicateDerivationKey, Evidence>::new();
    for record in records {
        if record.lexical_status != "일반어" || predicate_pos(&record.pos).is_none() {
            continue;
        }
        if matches!(record.source.as_str(), "krdict" | "stdict") {
            for surface in &record.conjugations {
                conjugations
                    .entry(SurfaceKey {
                        lemma: record.lemma.clone(),
                        pos: record.pos.clone(),
                        surface: surface.clone(),
                        kind: SurfaceKind::Conjugation,
                    })
                    .or_default()
                    .add(&record.source, &record.source_id);
            }
        }
        if record.source == "krdict" {
            for surface in &record.related_adverbs {
                related_adverbs
                    .entry(SurfaceKey {
                        lemma: record.lemma.clone(),
                        pos: record.pos.clone(),
                        surface: surface.clone(),
                        kind: SurfaceKind::RelatedAdverb,
                    })
                    .or_default()
                    .add(&record.source, &record.source_id);
            }
            for (target_lemma, target_ids) in &record.related_voice_derivations {
                let evidence = predicate_derivations
                    .entry(PredicateDerivationKey {
                        lemma: record.lemma.clone(),
                        pos: record.pos.clone(),
                        target_lemma: target_lemma.clone(),
                    })
                    .or_default();
                for target_id in target_ids {
                    evidence.add(&record.source, &format!("{}>{target_id}", record.source_id));
                }
                for source in ["stdict", "opendict"] {
                    let source_ids = general_predicates.get(&(
                        source.to_owned(),
                        record.lemma.clone(),
                        record.pos.clone(),
                    ));
                    let target_ids = general_predicates.get(&(
                        source.to_owned(),
                        target_lemma.clone(),
                        record.pos.clone(),
                    ));
                    if let (Some(source_ids), Some(target_ids)) = (source_ids, target_ids) {
                        for source_id in source_ids {
                            evidence.add(source, &format!("source:{source_id}"));
                        }
                        for target_id in target_ids {
                            evidence.add(source, &format!("target:{target_id}"));
                        }
                    }
                }
            }
        }
        for (surface, target_ids) in &record.attested_adverbials {
            let evidence = attested_adverbials
                .entry(SurfaceKey {
                    lemma: record.lemma.clone(),
                    pos: record.pos.clone(),
                    surface: surface.clone(),
                    kind: SurfaceKind::AttestedAdverbial,
                })
                .or_default();
            for target_id in target_ids {
                evidence.add(&record.source, &format!("{}>{target_id}", record.source_id));
            }
        }
    }

    conjugations.retain(|_, evidence| evidence.has_required_sources());
    attested_adverbials.retain(|_, evidence| evidence.has_required_sources());
    predicate_derivations.retain(|_, evidence| evidence.has_required_sources());
    for key in attested_adverbials.keys() {
        related_adverbs.remove(&SurfaceKey {
            lemma: key.lemma.clone(),
            pos: key.pos.clone(),
            surface: key.surface.clone(),
            kind: SurfaceKind::RelatedAdverb,
        });
    }
    let dictionary_keys = conjugations
        .keys()
        .map(|key| (key.lemma.clone(), key.pos.clone()))
        .collect::<BTreeSet<_>>();
    let analyses = runtime_predicates(
        core_records,
        candidates,
        core,
        derivations,
        &dictionary_keys,
    );
    let mut generated_conjugation_count = 0;
    let mut output = Vec::new();
    for (key, evidence) in &conjugations {
        let generated = analyses
            .get(&(key.lemma.clone(), key.pos.clone()))
            .is_some_and(|entries| entries.iter().any(|entry| generates(entry, &key.surface)));
        if generated {
            generated_conjugation_count += 1;
        } else {
            output.push(SurfaceCandidate {
                key: key.clone(),
                evidence: evidence.clone(),
            });
        }
    }
    output.extend(
        attested_adverbials
            .iter()
            .map(|(key, evidence)| SurfaceCandidate {
                key: key.clone(),
                evidence: evidence.clone(),
            }),
    );
    let surface_related_adverb_count = related_adverbs.len();
    output.extend(
        related_adverbs
            .into_iter()
            .map(|(key, evidence)| SurfaceCandidate { key, evidence }),
    );
    output.sort_by(|left, right| left.key.cmp(&right.key));
    let predicate_derivations = predicate_derivations
        .into_iter()
        .map(|(key, evidence)| PredicateDerivationCandidate { key, evidence })
        .collect();
    SurfaceCollection {
        candidates: output,
        predicate_derivations,
        agreed_conjugation_count: conjugations.len(),
        generated_conjugation_count,
        agreed_adverbial_i_count: attested_adverbials.len(),
        related_adverb_count: records
            .iter()
            .filter(|record| record.source == "krdict")
            .map(|record| record.related_adverbs.len())
            .sum(),
        surface_related_adverb_count,
        related_voice_derivation_count: records
            .iter()
            .filter(|record| record.source == "krdict")
            .map(|record| record.related_voice_derivations.len())
            .sum(),
    }
}

fn runtime_predicates(
    core_records: &[PredicateRecord],
    candidates: &BTreeMap<CandidateKey, Evidence>,
    core: &CoreEntries,
    derivations: &[DerivationRule],
    dictionary_keys: &BTreeSet<(String, String)>,
) -> BTreeMap<(String, String), Vec<PredicateEntry>> {
    let mut entries = BTreeMap::<(String, String), Vec<PredicateEntry>>::new();
    for record in core_records {
        let Some(predicate) = predicate_from_record(record) else {
            continue;
        };
        entries
            .entry((record.lemma.clone(), record.pos.as_str().to_owned()))
            .or_default()
            .push(predicate);
    }
    for (candidate, evidence) in candidates {
        if matches!(
            candidate_status(candidate, evidence, candidates, core, derivations),
            CandidateStatus::Promoted | CandidateStatus::MixedRegular
        ) {
            let Some(pos) = predicate_pos(&candidate.pos) else {
                continue;
            };
            let mut entry = PredicateEntry::new(
                candidate.lemma.as_str(),
                pos,
                candidate.classification.lexical_alternation(),
            );
            entry.flags = candidate.classification.predicate_flags();
            entries
                .entry((candidate.lemma.clone(), candidate.pos.clone()))
                .or_default()
                .push(entry);
        }
    }

    for (lemma, pos_name) in dictionary_keys {
        if entries.contains_key(&(lemma.clone(), pos_name.clone())) {
            continue;
        }
        let Some(pos) = predicate_pos(pos_name) else {
            continue;
        };
        let alternation = productive_alternation(lemma, pos_name, derivations)
            .unwrap_or(LexicalAlternation::Regular);
        entries.insert(
            (lemma.clone(), pos_name.clone()),
            vec![PredicateEntry::new(lemma.as_str(), pos, alternation)],
        );
    }
    entries
}

fn productive_alternation(
    lemma: &str,
    pos: &str,
    derivations: &[DerivationRule],
) -> Option<LexicalAlternation> {
    if lemma.ends_with("하다") {
        return Some(LexicalAlternation::Ha);
    }
    if pos == "VA"
        && ["스럽다", "답다", "롭다"]
            .iter()
            .any(|suffix| lemma.ends_with(suffix))
    {
        return Some(LexicalAlternation::BToWo);
    }
    derivations
        .iter()
        .filter(|rule| rule.result_pos.as_str() == pos)
        .filter(|rule| {
            lemma
                .strip_suffix(&rule.suffix)
                .is_some_and(|base| !base.is_empty())
        })
        .max_by_key(|rule| rule.suffix.len())
        .and_then(|rule| rule.alternation_id.as_deref())
        .and_then(alternation_from_rule_id)
}

fn predicate_from_record(record: &PredicateRecord) -> Option<PredicateEntry> {
    let mut entry = PredicateEntry::new(
        record.lemma.as_str(),
        predicate_pos(record.pos.as_str())?,
        data_alternation(record.alternation),
    );
    entry.flags = record
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
    Some(entry)
}

fn generates(entry: &PredicateEntry, surface: &str) -> bool {
    generate_predicate_branches(entry)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|branch| {
            let following = surface.strip_prefix(branch.anchor.as_ref())?;
            verify_predicate_continuation(
                branch.continuation,
                branch.pos,
                &branch.anchor,
                following,
            )
        })
        .any(|matched| matched.token_end == surface.len())
}

fn predicate_pos(pos: &str) -> Option<PredicatePos> {
    match pos {
        "VV" => Some(PredicatePos::Verb),
        "VA" => Some(PredicatePos::Adjective),
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
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(source: &str, source_id: &str, attested: bool) -> SourceRecord {
        SourceRecord {
            source: source.to_owned(),
            source_id: source_id.to_owned(),
            lemma: "같다".to_owned(),
            pos: "VA".to_owned(),
            lexical_status: "일반어".to_owned(),
            conjugations: BTreeSet::new(),
            related_adverbs: (source == "krdict")
                .then(|| "같이".to_owned())
                .into_iter()
                .collect(),
            related_voice_derivations: BTreeMap::new(),
            attested_adverbials: attested
                .then(|| {
                    (
                        "같이".to_owned(),
                        [format!("{source_id}-adverb")].into_iter().collect(),
                    )
                })
                .into_iter()
                .collect(),
        }
    }

    fn voice_record(
        source: &str,
        source_id: &str,
        lemma: &str,
        target: Option<(&str, &str)>,
    ) -> SourceRecord {
        SourceRecord {
            source: source.to_owned(),
            source_id: source_id.to_owned(),
            lemma: lemma.to_owned(),
            pos: "VV".to_owned(),
            lexical_status: "일반어".to_owned(),
            conjugations: BTreeSet::new(),
            related_adverbs: BTreeSet::new(),
            related_voice_derivations: target
                .map(|(target_lemma, target_id)| {
                    (
                        target_lemma.to_owned(),
                        [target_id.to_owned()].into_iter().collect(),
                    )
                })
                .into_iter()
                .collect(),
            attested_adverbials: BTreeMap::new(),
        }
    }

    #[test]
    fn promotes_only_cross_dictionary_adverbials_and_prefers_them_to_relations() {
        let records = vec![record("krdict", "k1", true), record("stdict", "s1", true)];
        let surfaces = collect_surfaces(&records, &[], &BTreeMap::new(), &CoreEntries::new(), &[]);

        assert_eq!(surfaces.candidates.len(), 1);
        assert_eq!(
            surfaces.candidates[0].key.kind,
            SurfaceKind::AttestedAdverbial
        );
        assert_eq!(surfaces.agreed_adverbial_i_count, 1);
        assert_eq!(surfaces.related_adverb_count, 1);
        assert_eq!(surfaces.surface_related_adverb_count, 0);

        let single_source = collect_surfaces(
            &[record("krdict", "k1", true)],
            &[],
            &BTreeMap::new(),
            &CoreEntries::new(),
            &[],
        );
        assert_eq!(single_source.candidates.len(), 1);
        assert_eq!(
            single_source.candidates[0].key.kind,
            SurfaceKind::RelatedAdverb
        );
        assert_eq!(single_source.agreed_adverbial_i_count, 0);
    }

    #[test]
    fn promotes_voice_derivation_only_when_both_lemmas_exist_in_required_sources() {
        let records = vec![
            voice_record("krdict", "k1", "밀다", Some(("밀리다", "k2"))),
            voice_record("krdict", "k2", "밀리다", None),
            voice_record("stdict", "s1", "밀다", None),
            voice_record("stdict", "s2", "밀리다", None),
        ];
        let surfaces = collect_surfaces(&records, &[], &BTreeMap::new(), &CoreEntries::new(), &[]);

        assert_eq!(surfaces.predicate_derivations.len(), 1);
        assert_eq!(surfaces.predicate_derivations[0].key.lemma, "밀다");
        assert_eq!(surfaces.predicate_derivations[0].key.target_lemma, "밀리다");

        let without_stdict_target = collect_surfaces(
            &records[..3],
            &[],
            &BTreeMap::new(),
            &CoreEntries::new(),
            &[],
        );
        assert!(without_stdict_target.predicate_derivations.is_empty());
    }
}
