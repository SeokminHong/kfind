use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use kfind_data::{
    MecabSourceMorphologyEntry, MorphologyExpressionAlignmentKind, NominalRole,
    align_morphology_expression, decode_component_resource,
    encode_component_resource_with_nominal_roles, extract_mecab_source_morphology, parse_sha256,
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
            "ownership-asset" => NominalRole::OwnershipAsset,
            "animal" => NominalRole::Animal,
            _ => return Err("unknown frame noun role".into()),
        };
        roles.push((noun, role));
    }
    let suffix_source =
        fs::read_to_string(role_source_dir.join("stdict-nominal-suffix-components.tsv"))?;
    let suffixes = read_nominal_suffixes(&suffix_source)?;
    let mut entries = Vec::new();
    for csv in &arguments[3..] {
        let path = Path::new(csv);
        let extraction = extract_mecab_source_morphology(
            &path.display().to_string(),
            BufReader::new(File::open(path)?),
        )?;
        entries.extend(extraction.entries().iter().cloned().map(|mut entry| {
            if let Some(&suffix) = suffixes.get(entry.surface.as_str()) {
                apply_nominal_suffix(&mut entry, suffix);
            }
            entry
        }));
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

fn read_nominal_suffixes(source: &str) -> Result<HashMap<&str, &str>, Box<dyn Error>> {
    let mut lines = source.lines();
    if lines.next() != Some("surface\tsuffix\tstdict_ids") {
        return Err("invalid nominal suffix source header".into());
    }
    let mut suffixes = HashMap::new();
    for line in lines {
        let mut columns = line.split('\t');
        let (Some(word), Some(suffix), Some(references), None) = (
            columns.next(),
            columns.next(),
            columns.next(),
            columns.next(),
        ) else {
            return Err("invalid nominal suffix source row".into());
        };
        if suffix.is_empty()
            || word.strip_suffix(suffix).is_none_or(str::is_empty)
            || !references
                .split('|')
                .all(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()))
            || suffixes.insert(word, suffix).is_some()
        {
            return Err("invalid nominal suffix source row".into());
        }
    }
    Ok(suffixes)
}

fn apply_nominal_suffix(entry: &mut MecabSourceMorphologyEntry, suffix: &str) {
    if entry.pos != "NNG" || entry.analysis_type != "Compound" {
        return;
    }
    let alignment = align_morphology_expression(&entry.surface, &entry.expression);
    if alignment.kind != MorphologyExpressionAlignmentKind::SpanAligned
        || !matches!(alignment.components.as_slice(), [_, .., last]
            if last.surface == suffix && last.pos == "NNG")
    {
        return;
    }
    let Some((head, tail)) = entry.expression.rsplit_once('+') else {
        return;
    };
    let Some((surface, feature)) = tail.split_once("/NNG/") else {
        return;
    };
    entry.expression = format!("{head}+{surface}/XSN/{feature}");
    if entry.end_pos == "NNG" {
        entry.end_pos = "XSN".to_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_only_the_attested_aligned_final_component() {
        let noun = MecabSourceMorphologyEntry {
            surface: "근로자".to_owned(),
            pos: "NNG".to_owned(),
            left_id: 0,
            right_id: 0,
            word_cost: 0,
            analysis_type: "Compound".to_owned(),
            start_pos: "NNG".to_owned(),
            end_pos: "NNG".to_owned(),
            expression: "근로/NNG/*+자/NNG/*".to_owned(),
        };
        let mut corrected = noun.clone();
        apply_nominal_suffix(&mut corrected, "자");
        assert_eq!(
            corrected,
            MecabSourceMorphologyEntry {
                end_pos: "XSN".to_owned(),
                expression: "근로/NNG/*+자/XSN/*".to_owned(),
                ..noun.clone()
            }
        );

        for mut entry in [
            MecabSourceMorphologyEntry {
                pos: "NNP".to_owned(),
                ..noun.clone()
            },
            MecabSourceMorphologyEntry {
                expression: "노동/NNG/*+자/NNG/*".to_owned(),
                ..noun.clone()
            },
            MecabSourceMorphologyEntry {
                expression: "근로/NNG/*+자/NNB/*".to_owned(),
                ..noun
            },
        ] {
            let original = entry.clone();
            apply_nominal_suffix(&mut entry, "자");
            assert_eq!(entry, original);
        }
    }
}
