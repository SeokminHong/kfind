use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;

use kfind_data::decode_pos_lexicon;
use kfind_pos_layout_prototype::encode;
use sha2::{Digest, Sha256};

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let input = required_path(&mut arguments, "schema 1 input")?;
    let output = required_path(&mut arguments, "prototype output")?;
    if arguments.next().is_some() {
        return Err(usage_error("인수가 너무 많습니다").into());
    }

    let source = fs::read(&input)?;
    let decoded = decode_pos_lexicon(&source)?;
    let encoded = encode(decoded.entries())?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, &encoded)?;
    println!("path={}", output.display());
    println!("bytes={}", encoded.len());
    println!("sha256={}", hex(&Sha256::digest(&encoded)));
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
        format!("{message}\nusage: kfind-pos-layout-prototype <schema1.bin> <prototype.bin>"),
    )
}

fn hex(input: &[u8]) -> String {
    input.iter().map(|byte| format!("{byte:02x}")).collect()
}
