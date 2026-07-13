use std::fs::{self, File};
use std::path::Path;

use anyhow::{Context, Result, ensure};
use memmap2::{Mmap, MmapOptions};
use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StorageMode {
    Resident,
    Mmap,
}

pub enum ArtifactBytes {
    Resident(Vec<u8>),
    Mapped(Mmap),
}

impl ArtifactBytes {
    pub fn load(path: &Path, storage: StorageMode) -> Result<Self> {
        match storage {
            StorageMode::Resident => {
                Ok(Self::Resident(fs::read(path).with_context(|| {
                    format!("failed to read {}", path.display())
                })?))
            }
            StorageMode::Mmap => Ok(Self::Mapped(map_read_only(path)?)),
        }
    }
}

impl AsRef<[u8]> for ArtifactBytes {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Resident(bytes) => bytes,
            Self::Mapped(bytes) => bytes,
        }
    }
}

pub fn peak_rss_bytes() -> Result<u64> {
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

fn map_read_only(path: &Path) -> Result<Mmap> {
    let file = File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    // The benchmark owns immutable build artifacts and never opens them for writing while mapped.
    let mapping = unsafe { MmapOptions::new().map(&file) }
        .with_context(|| format!("failed to map {}", path.display()))?;
    Ok(mapping)
}
