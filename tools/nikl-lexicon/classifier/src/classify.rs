use std::collections::{BTreeMap, BTreeSet};

use kfind_data::{DataFinePos, DerivationRule};
use kfind_morph::{
    PredicateEntry, PredicatePos, decompose_syllable, generate_predicate_branches, has_rieul_final,
};

use crate::model::{CandidateKey, Classification, Evidence, SourceRecord};

pub fn collect_candidates(records: &[SourceRecord]) -> BTreeMap<CandidateKey, Evidence> {
    let mut candidates = BTreeMap::<CandidateKey, Evidence>::new();
    let mut diagnostics =
        BTreeMap::<(String, String), Vec<(Classification, BTreeSet<String>)>>::new();
    for record in records {
        if !record_is_eligible(record) {
            continue;
        }
        let key = (record.lemma.clone(), record.pos.clone());
        let record_diagnostics = diagnostics
            .entry(key)
            .or_insert_with(|| classification_diagnostics(&record.lemma, &record.pos));
        let matched = record_diagnostics
            .iter()
            .filter_map(|(classification, anchors)| {
                anchors
                    .iter()
                    .any(|anchor| record.conjugations.contains(anchor))
                    .then_some(*classification)
            })
            .collect::<Vec<_>>();
        if matched.len() != 1 {
            continue;
        }
        let candidate = CandidateKey {
            lemma: record.lemma.clone(),
            pos: record.pos.clone(),
            classification: matched[0],
        };
        candidates
            .entry(candidate)
            .or_default()
            .add(&record.source, &record.source_id);
    }
    candidates
}

fn record_is_eligible(record: &SourceRecord) -> bool {
    record.lexical_status == "일반어"
        && !record.conjugations.is_empty()
        && predicate_pos(&record.pos).is_some()
}

fn classification_diagnostics(lemma: &str, pos: &str) -> Vec<(Classification, BTreeSet<String>)> {
    Classification::VALUES
        .into_iter()
        .filter(|classification| classification_supports_lemma(lemma, *classification))
        .filter_map(|classification| {
            let anchors = diagnostic_anchors(lemma, pos, classification);
            (!anchors.is_empty()).then_some((classification, anchors))
        })
        .collect()
}

fn classification_supports_lemma(lemma: &str, classification: Classification) -> bool {
    match classification {
        Classification::DToL | Classification::RegularD => has_same_final(lemma, '닫'),
        Classification::DropS | Classification::RegularS => has_same_final(lemma, '짓'),
        Classification::BToWa | Classification::BToWo | Classification::RegularB => {
            has_same_final(lemma, '눕')
        }
        Classification::DropH | Classification::RegularH => has_same_final(lemma, '좋'),
        Classification::ReuDoubleL => lemma.ends_with("르다") && reu_shape_is_diagnostic(lemma),
        Classification::Reo | Classification::RegularEuDrop => lemma.ends_with("르다"),
        Classification::UToEo => has_same_vowel_and_final(lemma, '푸'),
    }
}

fn stem_last(lemma: &str) -> Option<char> {
    lemma.strip_suffix('다')?.chars().next_back()
}

fn has_same_final(lemma: &str, exemplar: char) -> bool {
    let actual = stem_last(lemma).and_then(decompose_syllable);
    let expected = decompose_syllable(exemplar);
    actual
        .zip(expected)
        .is_some_and(|(actual, expected)| actual.jongseong == expected.jongseong)
}

fn has_same_vowel_and_final(lemma: &str, exemplar: char) -> bool {
    let actual = stem_last(lemma).and_then(decompose_syllable);
    let expected = decompose_syllable(exemplar);
    actual.zip(expected).is_some_and(|(actual, expected)| {
        actual.jungseong == expected.jungseong && actual.jongseong == expected.jongseong
    })
}

fn reu_shape_is_diagnostic(lemma: &str) -> bool {
    lemma
        .strip_suffix("르다")
        .and_then(|prefix| prefix.chars().next_back())
        .is_some_and(|syllable| !has_rieul_final(syllable))
}

fn predicate_pos(pos: &str) -> Option<PredicatePos> {
    match pos {
        "VV" => Some(PredicatePos::Verb),
        "VA" => Some(PredicatePos::Adjective),
        _ => None,
    }
}

fn diagnostic_anchors(lemma: &str, pos: &str, classification: Classification) -> BTreeSet<String> {
    let Some(pos) = predicate_pos(pos) else {
        return BTreeSet::new();
    };
    let mut anchors = generated_anchors(lemma, pos, classification, true);
    for competitor in classification.competitors() {
        let competitor_anchors = generated_anchors(lemma, pos, *competitor, false);
        anchors.retain(|anchor| !competitor_anchors.contains(anchor));
    }
    anchors
}

fn generated_anchors(
    lemma: &str,
    pos: PredicatePos,
    classification: Classification,
    diagnostic_only: bool,
) -> BTreeSet<String> {
    let mut entry = PredicateEntry::new(lemma, pos, classification.lexical_alternation());
    entry.flags = classification.predicate_flags();
    generate_predicate_branches(&entry)
        .unwrap_or_default()
        .into_iter()
        .filter(|branch| {
            !diagnostic_only
                || classification.diagnostic_rule_id().is_none()
                || branch
                    .rule_path
                    .iter()
                    .any(|rule| Some(rule.as_str()) == classification.diagnostic_rule_id())
        })
        .map(|branch| branch.anchor.into_string())
        .collect()
}

pub fn is_productive_duplicate(candidate: &CandidateKey, rules: &[DerivationRule]) -> bool {
    let Some(pos) = DataFinePos::parse(&candidate.pos) else {
        return false;
    };
    rules.iter().any(|rule| {
        rule.result_pos == pos
            && rule.alternation_id.as_deref()
                == Some(candidate.classification.alternation_rule_id())
            && candidate
                .lemma
                .strip_suffix(&rule.suffix)
                .is_some_and(|base| !base.is_empty())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(lemma: &str, pos: &str, conjugations: &[&str]) -> SourceRecord {
        SourceRecord {
            source: "krdict".to_owned(),
            source_id: "fixture".to_owned(),
            lemma: lemma.to_owned(),
            pos: pos.to_owned(),
            lexical_status: "일반어".to_owned(),
            conjugations: conjugations
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            related_adverbs: BTreeSet::new(),
            attested_adverbials: BTreeMap::new(),
        }
    }

    fn classifications(record: &SourceRecord) -> Vec<Classification> {
        classification_diagnostics(&record.lemma, &record.pos)
            .into_iter()
            .filter_map(|(classification, anchors)| {
                anchors
                    .iter()
                    .any(|anchor| record.conjugations.contains(anchor))
                    .then_some(classification)
            })
            .collect()
    }

    #[test]
    fn distinguishes_consonant_irregulars_and_regular_controls() {
        for (lemma, pos, surface, expected) in [
            ("깨닫다", "VV", "깨달아", Classification::DToL),
            ("믿다", "VV", "믿어", Classification::RegularD),
            ("짓다", "VV", "지어", Classification::DropS),
            ("벗다", "VV", "벗어", Classification::RegularS),
            ("곱다", "VA", "고와", Classification::BToWa),
            ("가깝다", "VA", "가까워", Classification::BToWo),
            ("잡다", "VV", "잡아", Classification::RegularB),
            ("노랗다", "VA", "노래", Classification::DropH),
            ("좋다", "VA", "좋아", Classification::RegularH),
        ] {
            assert_eq!(
                classifications(&record(lemma, pos, &[surface])),
                vec![expected]
            );
        }
    }

    #[test]
    fn distinguishes_reu_reo_regular_eu_drop_and_u() {
        assert_eq!(
            classifications(&record("다르다", "VA", &["달라"])),
            vec![Classification::ReuDoubleL]
        );
        assert_eq!(
            classifications(&record("푸르다", "VA", &["푸르러"])),
            vec![Classification::Reo]
        );
        assert_eq!(
            classifications(&record("치르다", "VV", &["치러"])),
            vec![Classification::RegularEuDrop]
        );
        assert_eq!(
            classifications(&record("푸다", "VV", &["퍼"])),
            vec![Classification::UToEo]
        );
    }

    #[test]
    fn preserves_distinct_ireuda_source_records() {
        assert_eq!(
            classifications(&record("이르다", "VV", &["이르러"])),
            vec![Classification::Reo]
        );
        assert_eq!(
            classifications(&record("이르다", "VV", &["일러"])),
            vec![Classification::ReuDoubleL]
        );
    }

    #[test]
    fn rejects_nonstandard_records() {
        let mut redirected = record("다르다", "VV", &["달라"]);
        redirected.lexical_status = "redirect".to_owned();
        assert!(!record_is_eligible(&redirected));
    }
}
