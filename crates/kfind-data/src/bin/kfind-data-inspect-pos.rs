use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;

use kfind_data::decode_pos_lexicon;

fn main() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args_os().skip(1);
    let input = arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| usage_error("input binary 경로가 필요합니다"))?;
    if arguments.next().is_some() {
        return Err(usage_error("인수가 너무 많습니다").into());
    }
    let decoded = decode_pos_lexicon(&fs::read(input)?)?;
    print!("{}", toml::to_string(&decoded.stats())?);
    Ok(())
}

fn usage_error(message: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("{message}\nusage: kfind-data-inspect-pos <lexicon.bin>"),
    )
}
