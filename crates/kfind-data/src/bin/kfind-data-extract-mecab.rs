use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::PathBuf;

use kfind_data::{
    DataError, MecabExtraction, PosLexiconEntry, encode_pos_lexicon, extract_mecab_ko_dic,
    parse_predicates_tsv,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let approved_path = required_path(&mut arguments, "gold predicate TSV")?;
    let output_path = required_path(&mut arguments, "output binary")?;
    let csv_paths = arguments.map(PathBuf::from).collect::<Vec<_>>();
    if csv_paths.is_empty() {
        return Err(usage_error("하나 이상의 mecab-ko-dic CSV가 필요합니다").into());
    }

    let approved_source = std::fs::read_to_string(&approved_path)?;
    let approved = parse_gold_predicates(&approved_path.display().to_string(), &approved_source)?;
    let mut combined: Option<MecabExtraction> = None;
    for path in csv_paths {
        let file = File::open(&path)?;
        let source = path.display().to_string();
        let extraction = extract_mecab_ko_dic(&source, BufReader::new(file))?;
        combined = Some(match combined {
            Some(previous) => previous.merge(extraction),
            None => extraction,
        });
    }

    let extraction = combined.expect("CSV path를 하나 이상 검사했습니다");
    let approved_lexicon = extraction.approve_predicates(&approved);
    let encoded = encode_pos_lexicon(approved_lexicon.pos_lexicon())?;
    std::fs::write(output_path, encoded)?;
    Ok(())
}

fn required_path(
    arguments: &mut impl Iterator<Item = std::ffi::OsString>,
    label: &str,
) -> Result<PathBuf, io::Error> {
    arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| usage_error(&format!("{label} 경로가 필요합니다")))
}

fn parse_gold_predicates(
    source: &str,
    input: &str,
) -> Result<BTreeSet<PosLexiconEntry>, DataError> {
    let (records, _) = parse_predicates_tsv(source, input)?;
    Ok(records
        .into_iter()
        .map(|record| PosLexiconEntry {
            lemma: record.lemma,
            pos: record.pos,
        })
        .collect())
}

fn usage_error(message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "{message}\nusage: kfind-data-extract-mecab <gold-predicates.tsv> <output.bin> <mecab.csv>..."
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_predicate_tsv_is_the_gold_approval_input() {
        let entries = parse_gold_predicates(
            "predicates.tsv",
            "lemma\tpos\talternation\tflags\toverrides\n걷다\tVV\tDToL\t\t\n",
        )
        .unwrap();
        assert!(entries.contains(&PosLexiconEntry {
            lemma: "걷다".to_owned(),
            pos: kfind_data::DataFinePos::Vv,
        }));

        assert!(parse_gold_predicates("predicates.tsv", "lemma\tpos\n걷다\tVV\n").is_err());
    }
}
