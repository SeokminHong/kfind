use std::fs::{self, File};
use std::hint::black_box;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result, ensure};
use clap::ValueEnum;
use kfind_data::{
    DecodedMorphologyResource, MecabSourceMorphologyEntry, decode_morphology_resource,
    encode_morphology_resource, extract_mecab_source_morphology, parse_mecab_connection_matrix,
};
use morph_index_benchmark::component_artifact::{
    CompactComponentAnalysis, CompactComponentResource, decode_compact_component_resource,
    encode_compact_component_resource,
};
use serde::Serialize;

use crate::dataset::Workload;
use crate::storage::{ArtifactBytes, StorageMode, peak_rss_bytes};

const WORKLOAD_SIZE: usize = 10_000;
pub const FULL_ARTIFACT_NAME: &str = "morphology-full.kfm";
pub const COMPACT_ARTIFACT_NAME: &str = "morphology-component-compact.kfc";
pub const WORKLOAD_NAME: &str = "component-queries.json";

#[derive(Clone, Copy, Debug, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ComponentFormat {
    Full,
    Compact,
}

#[derive(Debug, Serialize)]
pub struct ComponentBuildReport<'a> {
    pub schema_version: u32,
    pub source_sha256: &'a str,
    pub surface_count: u32,
    pub analysis_count: u32,
    pub full_artifact_bytes: usize,
    pub compact_artifact_bytes: usize,
    pub compact_to_full_percent: f64,
    pub exact_equivalence: LookupTotals,
    pub prefix_equivalence: LookupTotals,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct LookupTotals {
    pub queries: usize,
    pub matches: usize,
    pub analyses: usize,
    pub checksum: u64,
}

#[derive(Debug, Serialize)]
pub struct ComponentProbeReport {
    pub schema_version: u32,
    pub format: ComponentFormat,
    pub storage: StorageMode,
    pub artifact_bytes: usize,
    pub surface_count: u32,
    pub analysis_count: u32,
    pub initialization_ms: f64,
    pub exact: LookupMeasurement,
    pub prefix: LookupMeasurement,
    pub peak_rss_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct LookupMeasurement {
    pub queries: usize,
    pub ns_per_query: f64,
    pub matches: usize,
    pub analyses: usize,
    pub checksum: u64,
}

pub struct ComponentBuildInput<'a> {
    pub source_sha256: &'a str,
    pub source_digest: [u8; 32],
    pub output: &'a Path,
    pub matrix: &'a Path,
    pub char_def: &'a Path,
    pub unk_def: &'a Path,
    pub csv: &'a [PathBuf],
}

pub fn build_component_resources(input: ComponentBuildInput<'_>) -> Result<()> {
    ensure!(!input.csv.is_empty(), "at least one MeCab CSV is required");
    let matrix = parse_mecab_connection_matrix(
        &input.matrix.display().to_string(),
        BufReader::new(File::open(input.matrix)?),
    )?;
    let char_def = fs::read(input.char_def)
        .with_context(|| format!("failed to read {}", input.char_def.display()))?;
    let unk_def = fs::read(input.unk_def)
        .with_context(|| format!("failed to read {}", input.unk_def.display()))?;
    let entries = load_entries(input.csv)?;
    let full =
        encode_morphology_resource(input.source_digest, &entries, &matrix, &char_def, &unk_def)?;
    let compact = encode_compact_component_resource(
        input.source_digest,
        &entries,
        &matrix,
        &char_def,
        &unk_def,
    )?;
    let full_view =
        decode_morphology_resource("full benchmark resource", &full, &input.source_digest)?;
    let compact_view = decode_compact_component_resource(&compact, &input.source_digest)?;
    ensure_equivalent_metadata(&full_view, &compact_view)?;
    let workload = build_workload(&entries);
    let full_resource = ComponentResource::Full(full_view);
    let compact_resource = ComponentResource::Compact(compact_view);
    let full_exact = measure_workload(&full_resource, &workload.exact, true, 1).0;
    let compact_exact = measure_workload(&compact_resource, &workload.exact, true, 1).0;
    ensure!(
        full_exact == compact_exact,
        "component exact lookup differs from full resource"
    );
    let full_prefix = measure_workload(&full_resource, &workload.prefix, false, 1).0;
    let compact_prefix = measure_workload(&compact_resource, &workload.prefix, false, 1).0;
    ensure!(
        full_prefix == compact_prefix,
        "component prefix lookup differs from full resource"
    );

    fs::create_dir_all(input.output)
        .with_context(|| format!("failed to create {}", input.output.display()))?;
    fs::write(input.output.join(FULL_ARTIFACT_NAME), &full)?;
    fs::write(input.output.join(COMPACT_ARTIFACT_NAME), &compact)?;
    fs::write(
        input.output.join(WORKLOAD_NAME),
        serde_json::to_vec_pretty(&workload)?,
    )?;
    let report = ComponentBuildReport {
        schema_version: 1,
        source_sha256: input.source_sha256,
        surface_count: full_resource.surface_count(),
        analysis_count: full_resource.analysis_count(),
        full_artifact_bytes: full.len(),
        compact_artifact_bytes: compact.len(),
        compact_to_full_percent: compact.len() as f64 / full.len() as f64 * 100.0,
        exact_equivalence: full_exact,
        prefix_equivalence: full_prefix,
    };
    fs::write(
        input.output.join("component-build-report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

pub fn probe_component_resource(
    format: ComponentFormat,
    artifact_path: &Path,
    query_path: &Path,
    expected_source_digest: &[u8; 32],
    storage: StorageMode,
    iterations: usize,
) -> Result<ComponentProbeReport> {
    ensure!(iterations > 0, "iterations must be greater than zero");
    let workload: Workload = serde_json::from_slice(
        &fs::read(query_path)
            .with_context(|| format!("failed to read {}", query_path.display()))?,
    )?;
    ensure!(
        !workload.exact.is_empty(),
        "component exact workload is empty"
    );
    ensure!(
        !workload.prefix.is_empty(),
        "component prefix workload is empty"
    );

    let initialization_started = Instant::now();
    let bytes = ArtifactBytes::load(artifact_path, storage)?;
    let resource = match format {
        ComponentFormat::Full => ComponentResource::Full(decode_morphology_resource(
            &artifact_path.display().to_string(),
            bytes.as_ref(),
            expected_source_digest,
        )?),
        ComponentFormat::Compact => ComponentResource::Compact(decode_compact_component_resource(
            bytes.as_ref(),
            expected_source_digest,
        )?),
    };
    let initialization_ms = initialization_started.elapsed().as_secs_f64() * 1_000.0;
    let (exact, exact_elapsed) = measure_workload(&resource, &workload.exact, true, iterations);
    let (prefix, prefix_elapsed) = measure_workload(&resource, &workload.prefix, false, iterations);
    black_box((exact.checksum, prefix.checksum));
    Ok(ComponentProbeReport {
        schema_version: 1,
        format,
        storage,
        artifact_bytes: bytes.as_ref().len(),
        surface_count: resource.surface_count(),
        analysis_count: resource.analysis_count(),
        initialization_ms,
        exact: LookupMeasurement {
            queries: exact.queries,
            ns_per_query: exact_elapsed.as_nanos() as f64 / exact.queries as f64,
            matches: exact.matches,
            analyses: exact.analyses,
            checksum: exact.checksum,
        },
        prefix: LookupMeasurement {
            queries: prefix.queries,
            ns_per_query: prefix_elapsed.as_nanos() as f64 / prefix.queries as f64,
            matches: prefix.matches,
            analyses: prefix.analyses,
            checksum: prefix.checksum,
        },
        peak_rss_bytes: peak_rss_bytes()?,
    })
}

fn load_entries(paths: &[PathBuf]) -> Result<Vec<MecabSourceMorphologyEntry>> {
    let mut entries = Vec::new();
    for path in paths {
        let extraction = extract_mecab_source_morphology(
            &path.display().to_string(),
            BufReader::new(File::open(path)?),
        )?;
        entries.extend_from_slice(extraction.entries());
    }
    entries.sort_unstable();
    entries.dedup();
    Ok(entries)
}

fn build_workload(entries: &[MecabSourceMorphologyEntry]) -> Workload {
    let mut surfaces = entries
        .iter()
        .map(|entry| entry.surface.as_str())
        .collect::<Vec<_>>();
    surfaces.sort_unstable();
    surfaces.dedup();
    let step = surfaces.len().div_ceil(WORKLOAD_SIZE).max(1);
    let mut exact = Vec::new();
    let mut prefix = Vec::new();
    for (index, surface) in surfaces
        .into_iter()
        .step_by(step)
        .take(WORKLOAD_SIZE)
        .enumerate()
    {
        exact.push(if index % 2 == 0 {
            surface.to_owned()
        } else {
            format!("{surface}없는표면")
        });
        prefix.push(format!("{surface}에서"));
    }
    Workload { exact, prefix }
}

fn ensure_equivalent_metadata(
    full: &DecodedMorphologyResource<'_>,
    compact: &CompactComponentResource<'_>,
) -> Result<()> {
    let full_stats = full.stats();
    let compact_stats = compact.stats();
    ensure!(
        full_stats.surface_count == compact_stats.surface_count
            && full_stats.analysis_count == compact_stats.analysis_count
            && full_stats.pos_counts == compact_stats.pos_counts
            && full_stats.right_contexts == compact_stats.right_contexts
            && full_stats.left_contexts == compact_stats.left_contexts,
        "compact component metadata differs from full resource"
    );
    ensure!(
        full.char_def() == compact.char_def() && full.unk_def() == compact.unk_def(),
        "compact unknown definitions differ from full resource"
    );
    for right_id in 0..full_stats.right_contexts {
        for left_id in 0..full_stats.left_contexts {
            ensure!(
                full.connection_cost(right_id, left_id)
                    == compact.connection_cost(right_id, left_id),
                "compact connection matrix differs from full resource"
            );
        }
    }
    Ok(())
}

enum ComponentResource<'a> {
    Full(DecodedMorphologyResource<'a>),
    Compact(CompactComponentResource<'a>),
}

impl ComponentResource<'_> {
    fn surface_count(&self) -> u32 {
        match self {
            Self::Full(resource) => resource.stats().surface_count,
            Self::Compact(resource) => resource.stats().surface_count,
        }
    }

    fn analysis_count(&self) -> u32 {
        match self {
            Self::Full(resource) => resource.stats().analysis_count,
            Self::Compact(resource) => resource.stats().analysis_count,
        }
    }

    fn lookup(&self, input: &[u8], exact: bool) -> LookupTotals {
        let mut totals = LookupTotals::default();
        match self {
            Self::Full(resource) => resource.common_prefixes(input, |length, analyses| {
                if !exact || length == input.len() {
                    totals.record(
                        length,
                        analyses.iter().map(|analysis| {
                            (
                                analysis.pos,
                                analysis.left_id,
                                analysis.right_id,
                                analysis.word_cost,
                            )
                        }),
                    );
                }
            }),
            Self::Compact(resource) => resource.common_prefixes(input, |length, analyses| {
                if !exact || length == input.len() {
                    totals.record(length, analyses.iter().map(compact_fields));
                }
            }),
        }
        totals
    }
}

impl LookupTotals {
    fn record<'a>(
        &mut self,
        length: usize,
        analyses: impl Iterator<Item = (&'a str, u16, u16, i32)>,
    ) {
        self.matches += 1;
        self.checksum = mix(self.checksum, u64::try_from(length).unwrap_or(u64::MAX));
        for (pos, left_id, right_id, word_cost) in analyses {
            self.analyses += 1;
            self.checksum = mix_analysis(self.checksum, pos, left_id, right_id, word_cost);
        }
    }
}

fn compact_fields<'a>(analysis: &CompactComponentAnalysis<'a>) -> (&'a str, u16, u16, i32) {
    (
        analysis.pos,
        analysis.left_id,
        analysis.right_id,
        analysis.word_cost,
    )
}

fn measure_workload(
    resource: &ComponentResource<'_>,
    queries: &[String],
    exact: bool,
    iterations: usize,
) -> (LookupTotals, std::time::Duration) {
    let started = Instant::now();
    let mut totals = LookupTotals::default();
    for _ in 0..iterations {
        for query in queries {
            let result = resource.lookup(query.as_bytes(), exact);
            totals.queries += 1;
            totals.matches += result.matches;
            totals.analyses += result.analyses;
            totals.checksum = mix(totals.checksum, result.checksum);
        }
    }
    (totals, started.elapsed())
}

fn mix_analysis(mut checksum: u64, pos: &str, left_id: u16, right_id: u16, word_cost: i32) -> u64 {
    for byte in pos.as_bytes() {
        checksum = mix(checksum, u64::from(*byte));
    }
    checksum = mix(checksum, u64::from(left_id));
    checksum = mix(checksum, u64::from(right_id));
    mix(checksum, u64::from(word_cost as u32))
}

fn mix(checksum: u64, value: u64) -> u64 {
    checksum.rotate_left(7) ^ value.wrapping_mul(0x9E37_79B1_85EB_CA87)
}
