use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, BufReader};
use std::path::PathBuf;

use kfind_data::{MecabExtraction, encode_pos_lexicon, extract_mecab_ko_dic};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let output_path = required_path(&mut arguments, "output binary")?;
    let csv_paths = arguments.map(PathBuf::from).collect::<Vec<_>>();
    if csv_paths.is_empty() {
        return Err(usage_error("하나 이상의 mecab-ko-dic CSV가 필요합니다").into());
    }

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
    let pos_lexicon = extraction.into_pos_lexicon();
    let encoded = encode_pos_lexicon(&pos_lexicon)?;
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

fn usage_error(message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("{message}\nusage: kfind-data-extract-mecab <output.bin> <mecab.csv>..."),
    )
}
