use std::fs;
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, ensure};
use serde::Serialize;

use crate::artifact::{IndexKind, validate_container};
use crate::dataset::Workload;
use crate::index::IndexView;
use crate::storage::{ArtifactBytes, StorageMode, peak_rss_bytes};

#[derive(Debug, Serialize)]
pub struct ProbeReport {
    pub schema_version: u32,
    pub kind: IndexKind,
    pub storage: StorageMode,
    pub artifact_bytes: usize,
    pub surface_count: u32,
    pub analysis_count: u32,
    pub initialization_ms: f64,
    pub exact_queries: usize,
    pub exact_ns_per_query: f64,
    pub prefix_queries: usize,
    pub prefix_ns_per_query: f64,
    pub prefix_matches: usize,
    pub peak_rss_bytes: u64,
    pub checksum: u64,
}

pub fn probe(
    artifact_path: &Path,
    query_path: &Path,
    expected_source_digest: &[u8; 32],
    storage: StorageMode,
    iterations: usize,
) -> Result<ProbeReport> {
    ensure!(iterations > 0, "iterations must be greater than zero");
    let workload: Workload = serde_json::from_slice(
        &fs::read(query_path)
            .with_context(|| format!("failed to read {}", query_path.display()))?,
    )?;
    ensure!(!workload.exact.is_empty(), "exact workload is empty");
    ensure!(!workload.prefix.is_empty(), "prefix workload is empty");

    let initialization_started = Instant::now();
    let bytes = ArtifactBytes::load(artifact_path, storage)?;
    let view = validate_container(bytes.as_ref(), expected_source_digest)?;
    let index = IndexView::new(view.kind, view.index)?;
    let initialization_ms = initialization_started.elapsed().as_secs_f64() * 1_000.0;

    let exact_started = Instant::now();
    let mut checksum = 0_u64;
    for _ in 0..iterations {
        for query in &workload.exact {
            checksum = checksum.wrapping_add(u64::from(index.exact(query.as_bytes()).unwrap_or(0)));
        }
    }
    let exact_count = workload.exact.len() * iterations;
    let exact_ns_per_query = exact_started.elapsed().as_nanos() as f64 / exact_count as f64;

    let prefix_started = Instant::now();
    let mut prefix_matches = 0_usize;
    for _ in 0..iterations {
        for query in &workload.prefix {
            index.common_prefixes(query.as_bytes(), |value, length| {
                checksum = checksum.wrapping_add(u64::from(value) + length as u64);
                prefix_matches += 1;
            });
        }
    }
    let prefix_count = workload.prefix.len() * iterations;
    let prefix_ns_per_query = prefix_started.elapsed().as_nanos() as f64 / prefix_count as f64;
    black_box(checksum);

    Ok(ProbeReport {
        schema_version: 1,
        kind: view.kind,
        storage,
        artifact_bytes: bytes.as_ref().len(),
        surface_count: view.surface_count,
        analysis_count: view.analysis_count,
        initialization_ms,
        exact_queries: exact_count,
        exact_ns_per_query,
        prefix_queries: prefix_count,
        prefix_ns_per_query,
        prefix_matches,
        peak_rss_bytes: peak_rss_bytes()?,
        checksum,
    })
}
