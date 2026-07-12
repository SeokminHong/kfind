use std::fs::{self, File};
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result, ensure};
use memmap2::{Mmap, MmapOptions};
use serde::Serialize;

use crate::artifact::{IndexKind, validate_container};
use crate::dataset::Workload;
use crate::index::IndexView;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StorageMode {
    Resident,
    Mmap,
}

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

enum ArtifactBytes {
    Resident(Vec<u8>),
    Mapped(Mmap),
}

impl AsRef<[u8]> for ArtifactBytes {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Resident(bytes) => bytes,
            Self::Mapped(bytes) => bytes,
        }
    }
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
    let bytes = match storage {
        StorageMode::Resident => ArtifactBytes::Resident(
            fs::read(artifact_path)
                .with_context(|| format!("failed to read {}", artifact_path.display()))?,
        ),
        StorageMode::Mmap => ArtifactBytes::Mapped(map_read_only(artifact_path)?),
    };
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

fn map_read_only(path: &Path) -> Result<Mmap> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    // The benchmark owns immutable build artifacts and never opens them for writing while mapped.
    let mapping = unsafe { MmapOptions::new().map(&file) }
        .with_context(|| format!("failed to map {}", path.display()))?;
    Ok(mapping)
}

fn peak_rss_bytes() -> Result<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    // getrusage initializes the complete rusage structure on success.
    let result = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    ensure!(
        result == 0,
        "getrusage failed: {}",
        std::io::Error::last_os_error()
    );
    let usage = unsafe { usage.assume_init() };
    #[cfg(target_os = "macos")]
    let bytes = u64::try_from(usage.ru_maxrss)?;
    #[cfg(not(target_os = "macos"))]
    let bytes = u64::try_from(usage.ru_maxrss)?.saturating_mul(1024);
    Ok(bytes)
}
