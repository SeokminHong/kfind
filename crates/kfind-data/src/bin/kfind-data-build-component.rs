use std::error::Error;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use kfind_data::{
    NominalRole, decode_component_resource, encode_component_resource_with_nominal_roles,
    extract_mecab_source_morphology, parse_sha256,
};

const ROLE_SOURCES: &[(NominalRole, &str)] = &[
    (NominalRole::Spatial, "krdict-spatial-destinations.tsv"),
    (NominalRole::Person, "krdict-person-nouns.tsv"),
    (NominalRole::Scheduled, "krdict-scheduled-nouns.tsv"),
    (NominalRole::Temporal, "krdict-temporal-nouns.tsv"),
    (NominalRole::DoObject, "krdict-ha-objects.tsv"),
];

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() < 4 {
        return Err(
            "usage: kfind-data-build-component OUTPUT SOURCE_SHA256 ROLE_SOURCE_DIRECTORY CSV..."
                .into(),
        );
    }
    let output = PathBuf::from(&arguments[0]);
    let source_sha256 = arguments[1]
        .to_str()
        .ok_or("SOURCE_SHA256 must be valid UTF-8")?;
    let source_digest = parse_sha256(source_sha256)?;
    let role_source_dir = Path::new(&arguments[2]);
    let sources = ROLE_SOURCES
        .iter()
        .map(|&(role, file)| {
            fs::read_to_string(role_source_dir.join(file)).map(|source| (role, source))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut roles = Vec::new();
    for (role, source) in &sources {
        for line in source.lines().filter(|line| !line.starts_with('#')) {
            let (noun, _) = line
                .split_once('\t')
                .ok_or("invalid nominal role source row")?;
            roles.push((noun, *role));
        }
    }
    let frame_nouns = fs::read_to_string(role_source_dir.join("krdict-frame-nouns.tsv"))?;
    for line in frame_nouns.lines().filter(|line| !line.starts_with('#')) {
        let mut columns = line.split('\t');
        let (Some(role), Some(noun), Some(_)) = (columns.next(), columns.next(), columns.next())
        else {
            return Err("invalid frame noun source row".into());
        };
        let role = match role {
            "display-surface" => NominalRole::DisplaySurface,
            "household-article" => NominalRole::HouseholdArticle,
            "food" => NominalRole::Food,
            "body-secretion" => NominalRole::BodySecretion,
            "orthographic-unit" => NominalRole::OrthographicUnit,
            "emptyable" => NominalRole::Emptyable,
            "content-unit" => NominalRole::ContentUnit,
            "audible-phenomenon" => NominalRole::AudiblePhenomenon,
            _ => return Err("unknown frame noun role".into()),
        };
        roles.push((noun, role));
    }
    let mut entries = Vec::new();
    for csv in &arguments[3..] {
        let path = Path::new(csv);
        let extraction = extract_mecab_source_morphology(
            &path.display().to_string(),
            BufReader::new(File::open(path)?),
        )?;
        entries.extend_from_slice(extraction.entries());
    }
    entries.sort_unstable();
    entries.dedup();
    let resource = encode_component_resource_with_nominal_roles(source_digest, &entries, &roles)?;
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
