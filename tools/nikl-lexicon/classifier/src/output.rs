use std::collections::BTreeMap;
use std::error::Error;
use std::fs;
use std::path::Path;

use kfind_data::DerivationRule;

use crate::classify::is_productive_duplicate;
use crate::model::{CandidateKey, Classification, CoreEntries, Evidence, SourceRecord, core_key};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CandidateStatus {
    Promoted,
    MixedRegular,
    CoreDuplicate,
    ProductiveDuplicate,
    RegularControl,
    Review,
}

impl CandidateStatus {
    const VALUES: [Self; 6] = [
        Self::Promoted,
        Self::MixedRegular,
        Self::CoreDuplicate,
        Self::ProductiveDuplicate,
        Self::RegularControl,
        Self::Review,
    ];

    const fn name(self) -> &'static str {
        match self {
            Self::Promoted => "promoted",
            Self::MixedRegular => "mixed-regular",
            Self::CoreDuplicate => "core-duplicate",
            Self::ProductiveDuplicate => "productive-duplicate",
            Self::RegularControl => "regular-control",
            Self::Review => "review",
        }
    }

    const fn stats_key(self) -> &'static str {
        match self {
            Self::Promoted => "promoted_count",
            Self::MixedRegular => "mixed_regular_count",
            Self::CoreDuplicate => "core_duplicate_count",
            Self::ProductiveDuplicate => "productive_duplicate_count",
            Self::RegularControl => "regular_control_count",
            Self::Review => "review_count",
        }
    }
}

fn candidate_status(
    candidate: &CandidateKey,
    evidence: &Evidence,
    candidates: &BTreeMap<CandidateKey, Evidence>,
    core: &CoreEntries,
    derivations: &[DerivationRule],
) -> CandidateStatus {
    if core.contains(&core_key(candidate)) {
        CandidateStatus::CoreDuplicate
    } else if !candidate.classification.is_enriched()
        && is_mixed_regular(candidate, evidence, candidates, core, derivations)
    {
        CandidateStatus::MixedRegular
    } else if !candidate.classification.is_enriched() {
        CandidateStatus::RegularControl
    } else if is_productive_duplicate(candidate, derivations) {
        CandidateStatus::ProductiveDuplicate
    } else if evidence.has_required_sources() {
        CandidateStatus::Promoted
    } else {
        CandidateStatus::Review
    }
}

fn is_mixed_regular(
    candidate: &CandidateKey,
    evidence: &Evidence,
    candidates: &BTreeMap<CandidateKey, Evidence>,
    core: &CoreEntries,
    derivations: &[DerivationRule],
) -> bool {
    evidence.has_required_sources()
        && candidates.iter().any(|(sibling, sibling_evidence)| {
            sibling.lemma == candidate.lemma
                && sibling.pos == candidate.pos
                && sibling.classification.is_enriched()
                && (core.contains(&core_key(sibling))
                    || (sibling_evidence.has_required_sources()
                        && !is_productive_duplicate(sibling, derivations)))
        })
}

pub fn write_predicates(
    path: &Path,
    candidates: &BTreeMap<CandidateKey, Evidence>,
    core: &CoreEntries,
    derivations: &[DerivationRule],
) -> Result<(), Box<dyn Error>> {
    let mut output = String::from("lemma\tpos\talternation\tflags\toverrides\n");
    for (candidate, evidence) in candidates {
        if !matches!(
            candidate_status(candidate, evidence, candidates, core, derivations),
            CandidateStatus::Promoted | CandidateStatus::MixedRegular
        ) {
            continue;
        }
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t\n",
            candidate.lemma,
            candidate.pos,
            candidate.classification.alternation(),
            candidate.classification.flags()
        ));
    }
    fs::write(path, output)?;
    Ok(())
}

pub fn write_report(
    path: &Path,
    candidates: &BTreeMap<CandidateKey, Evidence>,
    core: &CoreEntries,
    derivations: &[DerivationRule],
) -> Result<(), Box<dyn Error>> {
    let mut output = String::from(
        "lemma\tpos\talternation\tflags\tkrdict_ids\tstdict_ids\topendict_ids\tstatus\n",
    );
    for (candidate, evidence) in candidates {
        let status = candidate_status(candidate, evidence, candidates, core, derivations);
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            candidate.lemma,
            candidate.pos,
            candidate.classification.alternation(),
            candidate.classification.flags(),
            evidence.ids("krdict"),
            evidence.ids("stdict"),
            evidence.ids("opendict"),
            status.name(),
        ));
    }
    fs::write(path, output)?;
    Ok(())
}

pub fn write_stats(
    path: &Path,
    records: &[SourceRecord],
    candidates: &BTreeMap<CandidateKey, Evidence>,
    core: &CoreEntries,
    derivations: &[DerivationRule],
) -> Result<(), Box<dyn Error>> {
    let count_status = |classification: Option<Classification>, status: CandidateStatus| {
        candidates
            .iter()
            .filter(|(candidate, evidence)| {
                classification.is_none_or(|value| candidate.classification == value)
                    && candidate_status(candidate, evidence, candidates, core, derivations)
                        == status
            })
            .count()
    };
    let mut output = format!(
        "schema_version = 2\ngenerator = \"nikl-lexicon-classifier@0.2.0\"\nrecord_count = {}\ncandidate_count = {}\n",
        records.len(),
        candidates.len(),
    );
    for status in CandidateStatus::VALUES {
        output.push_str(&format!(
            "{} = {}\n",
            status.stats_key(),
            count_status(None, status)
        ));
    }
    for classification in Classification::VALUES {
        let candidate_count = candidates
            .keys()
            .filter(|candidate| candidate.classification == classification)
            .count();
        output.push_str(&format!(
            "\n[[classification]]\nname = \"{}\"\ncandidate_count = {candidate_count}\n",
            classification.name()
        ));
        for status in CandidateStatus::VALUES {
            output.push_str(&format!(
                "{} = {}\n",
                status.stats_key(),
                count_status(Some(classification), status)
            ));
        }
    }
    for source in ["krdict", "stdict", "opendict"] {
        let record_count = records
            .iter()
            .filter(|record| record.source == source)
            .count();
        let candidate_count = candidates
            .values()
            .filter(|evidence| evidence.source_ids.contains_key(source))
            .count();
        output.push_str(&format!(
            "\n[[source]]\nname = \"{source}\"\nrecord_count = {record_count}\ncandidate_count = {candidate_count}\n"
        ));
    }
    fs::write(path, output)?;
    Ok(())
}
