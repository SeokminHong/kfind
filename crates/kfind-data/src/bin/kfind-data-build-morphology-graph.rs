use std::error::Error;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

use kfind_data::{
    MorphologyGraphExpressionKind, decode_morphology_graph_resource, decode_morphology_resource,
    encode_morphology_graph_resource, encode_morphology_resource, extract_mecab_source_morphology,
    parse_mecab_connection_matrix, parse_sha256, validate_morphology_graph_projection,
};

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    if arguments.len() < 6 {
        return Err(
            "usage: kfind-data-build-morphology-graph OUTPUT SOURCE_SHA256 MATRIX CHAR_DEF UNK_DEF CSV..."
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
        encode_morphology_graph_resource(source_digest, &entries, &matrix, &char_def, &unk_def)?;
    let decoded =
        decode_morphology_graph_resource(&output.display().to_string(), resource, &source_digest)?;
    let full_resource =
        encode_morphology_resource(source_digest, &entries, &matrix, &char_def, &unk_def)?;
    let full = decode_morphology_resource("projection-full", &full_resource, &source_digest)?;
    let projection = validate_morphology_graph_projection("projection", &full, &decoded)?;
    let stats = decoded.stats().clone();
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(&output, decoded.into_bytes())?;
    println!("path={}", output.display());
    println!("schema={}", stats.schema_version);
    println!("surfaces={}", stats.surface_count);
    println!("analyses={}", stats.analysis_count);
    println!("components={}", stats.component_count);
    println!(
        "expressions_absent={}",
        expression_count(
            &stats.expression_counts,
            MorphologyGraphExpressionKind::Absent
        )
    );
    println!(
        "expressions_span_aligned={}",
        expression_count(
            &stats.expression_counts,
            MorphologyGraphExpressionKind::SpanAligned,
        )
    );
    println!(
        "expressions_fused={}",
        expression_count(
            &stats.expression_counts,
            MorphologyGraphExpressionKind::Fused
        )
    );
    println!(
        "expressions_unaligned={}",
        expression_count(
            &stats.expression_counts,
            MorphologyGraphExpressionKind::Unaligned,
        )
    );
    println!(
        "expressions_invalid={}",
        expression_count(
            &stats.expression_counts,
            MorphologyGraphExpressionKind::Invalid
        )
    );
    println!("right_contexts={}", stats.right_contexts);
    println!("left_contexts={}", stats.left_contexts);
    println!("projection_surfaces={}", projection.surface_count);
    println!("projection_analyses={}", projection.analysis_count);
    println!("projection_components={}", projection.component_count);
    println!("projection_matrix_costs={}", projection.matrix_cost_count);
    Ok(())
}

fn expression_count(
    counts: &std::collections::BTreeMap<MorphologyGraphExpressionKind, u32>,
    kind: MorphologyGraphExpressionKind,
) -> u32 {
    counts.get(&kind).copied().unwrap_or_default()
}
