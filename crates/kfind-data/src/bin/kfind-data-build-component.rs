use std::error::Error;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use kfind_data::{
    decode_component_resource, encode_component_resource, extract_mecab_source_morphology,
    parse_sha256,
};

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() < 3 {
        return Err("usage: kfind-data-build-component OUTPUT SOURCE_SHA256 CSV...".into());
    }
    let output = PathBuf::from(&arguments[0]);
    let source_sha256 = arguments[1]
        .to_str()
        .ok_or("SOURCE_SHA256 must be valid UTF-8")?;
    let source_digest = parse_sha256(source_sha256)?;
    let mut entries = Vec::new();
    for csv in &arguments[2..] {
        let path = Path::new(csv);
        let extraction = extract_mecab_source_morphology(
            &path.display().to_string(),
            BufReader::new(File::open(path)?),
        )?;
        entries.extend_from_slice(extraction.entries());
    }
    entries.sort_unstable();
    entries.dedup();
    let resource = encode_component_resource(source_digest, &entries)?;
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
    println!("components={}", stats.component_count);
    Ok(())
}
