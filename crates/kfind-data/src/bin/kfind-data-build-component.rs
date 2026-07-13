use std::error::Error;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use kfind_data::{
    decode_component_resource, encode_component_resource, extract_mecab_source_morphology,
    parse_mecab_connection_matrix, parse_sha256,
};

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() < 6 {
        return Err(
            "usage: kfind-data-build-component OUTPUT SOURCE_SHA256 MATRIX CHAR_DEF UNK_DEF CSV..."
                .into(),
        );
    }
    let output = PathBuf::from(&arguments[0]);
    let source_sha256 = arguments[1]
        .to_str()
        .ok_or("SOURCE_SHA256 must be valid UTF-8")?;
    let source_digest = parse_sha256(source_sha256)?;
    let matrix_path = Path::new(&arguments[2]);
    let matrix = parse_mecab_connection_matrix(
        &matrix_path.display().to_string(),
        BufReader::new(File::open(matrix_path)?),
    )?;
    let char_def = fs::read(&arguments[3])?;
    let unk_def = fs::read(&arguments[4])?;
    let mut entries = Vec::new();
    for csv in &arguments[5..] {
        let path = Path::new(csv);
        let extraction = extract_mecab_source_morphology(
            &path.display().to_string(),
            BufReader::new(File::open(path)?),
        )?;
        entries.extend_from_slice(extraction.entries());
    }
    entries.sort_unstable();
    entries.dedup();
    let resource =
        encode_component_resource(source_digest, &entries, &matrix, &char_def, &unk_def)?;
    let decoded =
        decode_component_resource(&output.display().to_string(), resource, &source_digest)?;
    let stats = decoded.stats().clone();
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, decoded.into_bytes())?;
    println!("path={}", output.display());
    println!("surfaces={}", stats.surface_count);
    println!("analyses={}", stats.analysis_count);
    println!("right_contexts={}", stats.right_contexts);
    println!("left_contexts={}", stats.left_contexts);
    Ok(())
}
