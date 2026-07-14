use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

use kfind_morph::{
    LexicalAlternation, PredicateEntry, PredicateFlags, PredicatePos, generate_predicate_branches,
    has_rieul_final,
};

const REQUIRED_SOURCES: [&str; 2] = ["krdict", "stdict"];

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Classification {
    ReuDoubleL,
    Reo,
    RegularEuDrop,
}

impl Classification {
    const VALUES: [Self; 3] = [Self::ReuDoubleL, Self::Reo, Self::RegularEuDrop];

    const fn alternation(self) -> &'static str {
        match self {
            Self::ReuDoubleL => "ReuDoubleL",
            Self::Reo => "Reo",
            Self::RegularEuDrop => "Regular",
        }
    }

    const fn flags(self) -> &'static str {
        match self {
            Self::RegularEuDrop => "EU_DROP",
            Self::ReuDoubleL | Self::Reo => "",
        }
    }

    const fn rule_id(self) -> &'static str {
        match self {
            Self::ReuDoubleL => "lexical.reu-double-l",
            Self::Reo => "lexical.reo",
            Self::RegularEuDrop => "contraction.eu-drop",
        }
    }

    const fn lexical_alternation(self) -> LexicalAlternation {
        match self {
            Self::ReuDoubleL => LexicalAlternation::ReuDoubleL,
            Self::Reo => LexicalAlternation::Reo,
            Self::RegularEuDrop => LexicalAlternation::Regular,
        }
    }

    const fn is_enriched(self) -> bool {
        !matches!(self, Self::RegularEuDrop)
    }
}

#[derive(Clone, Debug)]
struct SourceRecord {
    source: String,
    source_id: String,
    lemma: String,
    pos: String,
    lexical_status: String,
    conjugations: BTreeSet<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CandidateKey {
    lemma: String,
    pos: String,
    classification: Classification,
}

#[derive(Clone, Debug, Default)]
struct Evidence {
    source_ids: BTreeMap<String, BTreeSet<String>>,
}

impl Evidence {
    fn add(&mut self, source: &str, source_id: &str) {
        self.source_ids
            .entry(source.to_owned())
            .or_default()
            .insert(source_id.to_owned());
    }

    fn has_required_sources(&self) -> bool {
        REQUIRED_SOURCES
            .iter()
            .all(|source| self.source_ids.contains_key(*source))
    }

    fn ids(&self, source: &str) -> String {
        self.source_ids
            .get(source)
            .map(|values| values.iter().cloned().collect::<Vec<_>>().join("|"))
            .unwrap_or_default()
    }
}

#[derive(Debug)]
struct UsageError(String);

impl Display for UsageError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for UsageError {}

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1).map(PathBuf::from);
    let input = required_argument(&mut arguments, "normalized records TSV")?;
    let core = required_argument(&mut arguments, "core predicates TSV")?;
    let output_directory = required_argument(&mut arguments, "output directory")?;
    if arguments.next().is_some() {
        return Err(Box::new(UsageError(usage())));
    }

    let records = parse_source_records(&fs::read_to_string(input)?)?;
    let core_entries = parse_core_entries(&fs::read_to_string(core)?)?;
    let candidates = collect_candidates(&records);
    fs::create_dir_all(&output_directory)?;
    write_predicates(
        &output_directory.join("predicates.tsv"),
        &candidates,
        &core_entries,
    )?;
    write_report(
        &output_directory.join("REPORT.tsv"),
        &candidates,
        &core_entries,
    )?;
    write_stats(
        &output_directory.join("STATS.toml"),
        &records,
        &candidates,
        &core_entries,
    )?;
    Ok(())
}

fn required_argument(
    arguments: &mut impl Iterator<Item = PathBuf>,
    name: &str,
) -> Result<PathBuf, UsageError> {
    arguments
        .next()
        .ok_or_else(|| UsageError(format!("missing {name}\n{}", usage())))
}

fn usage() -> String {
    "usage: nikl-lexicon-classifier <records.tsv> <core-predicates.tsv> <output-directory>"
        .to_owned()
}

fn parse_source_records(input: &str) -> Result<Vec<SourceRecord>, UsageError> {
    let mut lines = input.lines();
    let header = lines.next().unwrap_or_default();
    if header != "source\tsource_id\traw_homonym\tlemma\tpos\tlexical_status\tconjugations" {
        return Err(UsageError(
            "unexpected normalized records header".to_owned(),
        ));
    }
    lines
        .enumerate()
        .map(|(index, line)| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 7 {
                return Err(UsageError(format!(
                    "normalized records line {} has {} fields",
                    index + 2,
                    fields.len()
                )));
            }
            Ok(SourceRecord {
                source: fields[0].to_owned(),
                source_id: fields[1].to_owned(),
                lemma: fields[3].to_owned(),
                pos: fields[4].to_owned(),
                lexical_status: fields[5].to_owned(),
                conjugations: fields[6]
                    .split('|')
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .collect(),
            })
        })
        .collect()
}

fn parse_core_entries(
    input: &str,
) -> Result<BTreeSet<(String, String, String, String)>, UsageError> {
    let mut lines = input.lines();
    if lines.next().unwrap_or_default() != "lemma\tpos\talternation\tflags\toverrides" {
        return Err(UsageError("unexpected core predicates header".to_owned()));
    }
    lines
        .enumerate()
        .map(|(index, line)| {
            let fields = line.split('\t').collect::<Vec<_>>();
            if fields.len() != 5 {
                return Err(UsageError(format!(
                    "core predicates line {} has {} fields",
                    index + 2,
                    fields.len()
                )));
            }
            Ok((
                fields[0].to_owned(),
                fields[1].to_owned(),
                fields[2].to_owned(),
                fields[3].to_owned(),
            ))
        })
        .collect()
}

fn collect_candidates(records: &[SourceRecord]) -> BTreeMap<CandidateKey, Evidence> {
    let mut candidates = BTreeMap::<CandidateKey, Evidence>::new();
    for record in records {
        let classifications = classify_record(record);
        if classifications.len() != 1 {
            continue;
        }
        let key = CandidateKey {
            lemma: record.lemma.clone(),
            pos: record.pos.clone(),
            classification: classifications[0],
        };
        candidates
            .entry(key)
            .or_default()
            .add(&record.source, &record.source_id);
    }
    candidates
}

fn classify_record(record: &SourceRecord) -> Vec<Classification> {
    if record.lexical_status != "일반어"
        || !record.lemma.ends_with("르다")
        || record.conjugations.is_empty()
    {
        return Vec::new();
    }
    let Some(pos) = predicate_pos(&record.pos) else {
        return Vec::new();
    };
    Classification::VALUES
        .into_iter()
        .filter(|classification| {
            *classification != Classification::ReuDoubleL || reu_shape_is_diagnostic(&record.lemma)
        })
        .filter(|classification| {
            diagnostic_anchors(&record.lemma, pos, *classification)
                .iter()
                .any(|anchor| record.conjugations.contains(anchor))
        })
        .collect()
}

fn predicate_pos(pos: &str) -> Option<PredicatePos> {
    match pos {
        "VV" => Some(PredicatePos::Verb),
        "VA" => Some(PredicatePos::Adjective),
        _ => None,
    }
}

fn reu_shape_is_diagnostic(lemma: &str) -> bool {
    lemma
        .strip_suffix("르다")
        .and_then(|prefix| prefix.chars().next_back())
        .is_some_and(|syllable| !has_rieul_final(syllable))
}

fn diagnostic_anchors(
    lemma: &str,
    pos: PredicatePos,
    classification: Classification,
) -> BTreeSet<String> {
    let mut entry = PredicateEntry::new(lemma, pos, classification.lexical_alternation());
    if classification == Classification::RegularEuDrop {
        entry.flags = PredicateFlags::EU_DROP;
    }
    generate_predicate_branches(&entry)
        .unwrap_or_default()
        .into_iter()
        .filter(|branch| {
            branch
                .rule_path
                .iter()
                .any(|rule| rule.as_str() == classification.rule_id())
        })
        .map(|branch| branch.anchor.into_string())
        .collect()
}

fn write_predicates(
    path: &Path,
    candidates: &BTreeMap<CandidateKey, Evidence>,
    core: &BTreeSet<(String, String, String, String)>,
) -> Result<(), Box<dyn Error>> {
    let mut output = String::from("lemma\tpos\talternation\tflags\toverrides\n");
    for (candidate, evidence) in candidates {
        if !candidate.classification.is_enriched()
            || !evidence.has_required_sources()
            || core.contains(&core_key(candidate))
        {
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

fn write_report(
    path: &Path,
    candidates: &BTreeMap<CandidateKey, Evidence>,
    core: &BTreeSet<(String, String, String, String)>,
) -> Result<(), Box<dyn Error>> {
    let mut output = String::from(
        "lemma\tpos\talternation\tflags\tkrdict_ids\tstdict_ids\topendict_ids\tstatus\n",
    );
    for (candidate, evidence) in candidates {
        let status = if !candidate.classification.is_enriched() {
            "regular-control"
        } else if core.contains(&core_key(candidate)) {
            "core-duplicate"
        } else if evidence.has_required_sources() {
            "promoted"
        } else {
            "review"
        };
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{status}\n",
            candidate.lemma,
            candidate.pos,
            candidate.classification.alternation(),
            candidate.classification.flags(),
            evidence.ids("krdict"),
            evidence.ids("stdict"),
            evidence.ids("opendict"),
        ));
    }
    fs::write(path, output)?;
    Ok(())
}

fn write_stats(
    path: &Path,
    records: &[SourceRecord],
    candidates: &BTreeMap<CandidateKey, Evidence>,
    core: &BTreeSet<(String, String, String, String)>,
) -> Result<(), Box<dyn Error>> {
    let promoted = candidates
        .iter()
        .filter(|(candidate, evidence)| {
            candidate.classification.is_enriched()
                && evidence.has_required_sources()
                && !core.contains(&core_key(candidate))
        })
        .count();
    let core_duplicates = candidates
        .iter()
        .filter(|(candidate, evidence)| {
            candidate.classification.is_enriched()
                && evidence.has_required_sources()
                && core.contains(&core_key(candidate))
        })
        .count();
    let regular_controls = candidates
        .keys()
        .filter(|candidate| !candidate.classification.is_enriched())
        .count();
    let mut output = format!(
        "schema_version = 1\ngenerator = \"nikl-lexicon-classifier@0.1.0\"\nrecord_count = {}\ncandidate_count = {}\npromoted_count = {promoted}\ncore_duplicate_count = {core_duplicates}\nregular_control_count = {regular_controls}\n",
        records.len(),
        candidates.len(),
    );
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

fn core_key(candidate: &CandidateKey) -> (String, String, String, String) {
    (
        candidate.lemma.clone(),
        candidate.pos.clone(),
        candidate.classification.alternation().to_owned(),
        candidate.classification.flags().to_owned(),
    )
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
        }
    }

    #[test]
    fn distinguishes_reu_reo_and_regular_eu_drop() {
        assert_eq!(
            classify_record(&record("다르다", "VA", &["달라"])),
            vec![Classification::ReuDoubleL]
        );
        assert_eq!(
            classify_record(&record("푸르다", "VA", &["푸르러"])),
            vec![Classification::Reo]
        );
        assert_eq!(
            classify_record(&record("들르다", "VV", &["들러"])),
            vec![Classification::RegularEuDrop]
        );
        assert_eq!(
            classify_record(&record("치르다", "VV", &["치러"])),
            vec![Classification::RegularEuDrop]
        );
    }

    #[test]
    fn preserves_distinct_ireuda_source_records() {
        assert_eq!(
            classify_record(&record("이르다", "VV", &["이르러"])),
            vec![Classification::Reo]
        );
        assert_eq!(
            classify_record(&record("이르다", "VV", &["일러"])),
            vec![Classification::ReuDoubleL]
        );
    }

    #[test]
    fn rejects_nonstandard_records() {
        let mut redirected = record("다르다", "VV", &["달라"]);
        redirected.lexical_status = "redirect".to_owned();
        assert!(classify_record(&redirected).is_empty());
    }
}
