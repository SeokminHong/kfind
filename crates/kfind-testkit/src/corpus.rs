use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use unicode_normalization::UnicodeNormalization;

const KOREAN_LINES: &[&str] = &[
    "길을 걸어 갔다. 권한을 검증했습니다.\n",
    "사용자들에게 새 문서를 보여 주었다.\n",
    "예쁜 꽃이 피었지만 하늘은 파랬다.\n",
    "설명을 들으면 문제를 해결할 수 있다.\n",
];

const ASCII_LINES: &[&str] = &[
    "fn validate_access(user: &User) -> bool { user.is_active() }\n",
    "The quick brown fox jumps over the lazy dog.\n",
    "const MAX_RETRIES: usize = 3; // deterministic corpus line\n",
    "path/to/module.rs: search fixtures stay reproducible.\n",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CorpusConfig {
    pub total_bytes: u64,
    pub file_count: usize,
    pub small_file_count: usize,
    pub small_file_bytes: u64,
    pub korean_percent: u8,
    pub nfd_percent: u8,
    pub seed: u64,
}

impl CorpusConfig {
    pub fn validate(self) -> Result<Self, CorpusConfigError> {
        if self.file_count == 0 {
            return Err(CorpusConfigError::ZeroFiles);
        }
        if self.korean_percent > 100 {
            return Err(CorpusConfigError::InvalidKoreanPercent(self.korean_percent));
        }
        if self.nfd_percent > 100 {
            return Err(CorpusConfigError::InvalidNfdPercent(self.nfd_percent));
        }
        if self.small_file_count > self.file_count {
            return Err(CorpusConfigError::TooManySmallFiles {
                small: self.small_file_count,
                total: self.file_count,
            });
        }
        let small_bytes = (self.small_file_count as u64)
            .checked_mul(self.small_file_bytes)
            .ok_or(CorpusConfigError::FileSizeOverflow)?;
        if small_bytes > self.total_bytes {
            return Err(CorpusConfigError::SmallFilesExceedTotal);
        }
        let large_count = self.file_count - self.small_file_count;
        if large_count == 0 && small_bytes != self.total_bytes {
            return Err(CorpusConfigError::UnassignedBytes);
        }
        if self.small_file_count > 0 && large_count > 0 {
            let minimum_large_bytes = (large_count as u64)
                .checked_mul(self.small_file_bytes)
                .ok_or(CorpusConfigError::FileSizeOverflow)?;
            if self.total_bytes - small_bytes < minimum_large_bytes {
                return Err(CorpusConfigError::LargeFilesSmallerThanSmallFiles);
            }
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorpusConfigError {
    ZeroFiles,
    InvalidKoreanPercent(u8),
    InvalidNfdPercent(u8),
    TooManySmallFiles { small: usize, total: usize },
    FileSizeOverflow,
    SmallFilesExceedTotal,
    UnassignedBytes,
    LargeFilesSmallerThanSmallFiles,
}

impl std::fmt::Display for CorpusConfigError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroFiles => formatter.write_str("file_count must be greater than zero"),
            Self::InvalidKoreanPercent(percent) => {
                write!(
                    formatter,
                    "korean_percent must be between 0 and 100, got {percent}"
                )
            }
            Self::InvalidNfdPercent(percent) => {
                write!(
                    formatter,
                    "nfd_percent must be between 0 and 100, got {percent}"
                )
            }
            Self::TooManySmallFiles { small, total } => write!(
                formatter,
                "small_file_count ({small}) must not exceed file_count ({total})"
            ),
            Self::FileSizeOverflow => formatter.write_str("corpus file sizes overflow u64"),
            Self::SmallFilesExceedTotal => {
                formatter.write_str("small files exceed total corpus bytes")
            }
            Self::UnassignedBytes => formatter.write_str(
                "all files are small files but their sizes do not equal total corpus bytes",
            ),
            Self::LargeFilesSmallerThanSmallFiles => formatter
                .write_str("large files must be at least as large as the configured small files"),
        }
    }
}

impl std::error::Error for CorpusConfigError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CorpusStats {
    pub root: PathBuf,
    pub bytes_written: u64,
    pub files_written: usize,
    pub korean_lines: u64,
    pub ascii_lines: u64,
    pub nfc_korean_lines: u64,
    pub nfd_korean_lines: u64,
    pub small_files_written: usize,
    pub large_files_written: usize,
    pub seed: u64,
}

pub fn generate_corpus_tree(
    root: &Path,
    config: CorpusConfig,
) -> Result<CorpusStats, CorpusGenerateError> {
    let config = config.validate()?;
    fs::create_dir_all(root).map_err(CorpusGenerateError::Io)?;
    remove_generated_files(root)?;

    let mut rng = DeterministicRng::new(config.seed);
    let mut stats = CorpusStats {
        root: root.to_path_buf(),
        bytes_written: 0,
        files_written: 0,
        korean_lines: 0,
        ascii_lines: 0,
        nfc_korean_lines: 0,
        nfd_korean_lines: 0,
        small_files_written: 0,
        large_files_written: 0,
        seed: config.seed,
    };
    for index in 0..config.file_count {
        let target_bytes = bytes_for_file(config, index);
        let path = root.join(format!("corpus-{index:05}.txt"));
        let file = File::create(path).map_err(CorpusGenerateError::Io)?;
        let mut writer = BufWriter::new(file);
        write_corpus_file(
            &mut writer,
            target_bytes,
            config.korean_percent,
            config.nfd_percent,
            &mut rng,
            &mut stats,
        )?;
        writer.flush().map_err(CorpusGenerateError::Io)?;
        stats.files_written += 1;
        if index < config.small_file_count {
            stats.small_files_written += 1;
        } else {
            stats.large_files_written += 1;
        }
    }
    Ok(stats)
}

fn remove_generated_files(root: &Path) -> Result<(), CorpusGenerateError> {
    for entry in fs::read_dir(root).map_err(CorpusGenerateError::Io)? {
        let entry = entry.map_err(CorpusGenerateError::Io)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(index) = name
            .strip_prefix("corpus-")
            .and_then(|name| name.strip_suffix(".txt"))
        else {
            continue;
        };
        if index.len() == 5 && index.bytes().all(|byte| byte.is_ascii_digit()) {
            fs::remove_file(entry.path()).map_err(CorpusGenerateError::Io)?;
        }
    }
    Ok(())
}

fn write_corpus_file(
    writer: &mut impl Write,
    target_bytes: u64,
    korean_percent: u8,
    nfd_percent: u8,
    rng: &mut DeterministicRng,
    stats: &mut CorpusStats,
) -> Result<(), CorpusGenerateError> {
    let mut remaining = target_bytes;
    while remaining > 0 {
        let korean = rng.below(100) < u64::from(korean_percent);
        let nfd = korean && rng.below(100) < u64::from(nfd_percent);
        let line = select_line(korean, nfd, rng);
        if line.len() as u64 <= remaining {
            writer.write_all(line).map_err(CorpusGenerateError::Io)?;
            remaining -= line.len() as u64;
            if korean {
                stats.korean_lines += 1;
                if nfd {
                    stats.nfd_korean_lines += 1;
                } else {
                    stats.nfc_korean_lines += 1;
                }
            } else {
                stats.ascii_lines += 1;
            }
            continue;
        }

        write_ascii_padding(writer, remaining)?;
        remaining = 0;
    }
    stats.bytes_written += target_bytes;
    Ok(())
}

fn select_line(korean: bool, nfd: bool, rng: &mut DeterministicRng) -> &'static [u8] {
    if !korean {
        return ASCII_LINES[rng.below(ASCII_LINES.len() as u64) as usize].as_bytes();
    }
    let index = rng.below(KOREAN_LINES.len() as u64) as usize;
    if nfd {
        return nfd_korean_lines()[index].as_bytes();
    }
    KOREAN_LINES[index].as_bytes()
}

fn nfd_korean_lines() -> &'static [String] {
    static LINES: OnceLock<Vec<String>> = OnceLock::new();
    LINES.get_or_init(|| {
        KOREAN_LINES
            .iter()
            .map(|line| line.nfd().collect::<String>())
            .collect()
    })
}

fn write_ascii_padding(
    writer: &mut impl Write,
    mut remaining: u64,
) -> Result<(), CorpusGenerateError> {
    const PADDING: &[u8] = b"// kfind corpus padding\n";
    while remaining > 0 {
        let length = usize::try_from(remaining.min(PADDING.len() as u64))
            .expect("padding length is bounded by a small constant");
        writer
            .write_all(&PADDING[..length])
            .map_err(CorpusGenerateError::Io)?;
        remaining -= length as u64;
    }
    Ok(())
}

fn bytes_for_file(config: CorpusConfig, index: usize) -> u64 {
    if index < config.small_file_count {
        return config.small_file_bytes;
    }
    let small_total = config.small_file_count as u64 * config.small_file_bytes;
    let large_total = config.total_bytes - small_total;
    let large_count = (config.file_count - config.small_file_count) as u64;
    let large_index = (index - config.small_file_count) as u64;
    let base = large_total / large_count;
    base + u64::from(large_index < large_total % large_count)
}

#[derive(Debug)]
pub enum CorpusGenerateError {
    Config(CorpusConfigError),
    Io(io::Error),
}

impl std::fmt::Display for CorpusGenerateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Config(error) => error.fmt(formatter),
            Self::Io(error) => write!(formatter, "failed to generate corpus: {error}"),
        }
    }
}

impl std::error::Error for CorpusGenerateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Config(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

impl From<CorpusConfigError> for CorpusGenerateError {
    fn from(error: CorpusConfigError) -> Self {
        Self::Config(error)
    }
}

struct DeterministicRng(u64);

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn below(&mut self, upper: u64) -> u64 {
        debug_assert!(upper > 0);
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % upper
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::*;

    static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "kfind-corpus-test-{}-{sequence}",
                std::process::id()
            ));
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn generated_tree_has_exact_size_and_valid_utf8() {
        let temp = TempDir::new();
        let stats = generate_corpus_tree(
            &temp.0,
            CorpusConfig {
                total_bytes: 10_003,
                file_count: 7,
                small_file_count: 2,
                small_file_bytes: 256,
                korean_percent: 20,
                nfd_percent: 50,
                seed: 42,
            },
        )
        .unwrap();

        assert_eq!(stats.bytes_written, 10_003);
        assert_eq!(stats.files_written, 7);
        let files = fs::read_dir(&temp.0)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(files.len(), 7);
        let actual_size = files
            .iter()
            .map(|entry| {
                let bytes = fs::read(entry.path()).unwrap();
                std::str::from_utf8(&bytes).unwrap();
                bytes.len() as u64
            })
            .sum::<u64>();
        assert_eq!(actual_size, 10_003);
        assert!(stats.korean_lines > 0);
        assert!(stats.ascii_lines > 0);
        assert!(stats.nfc_korean_lines > 0);
        assert!(stats.nfd_korean_lines > 0);
        assert_eq!(stats.small_files_written, 2);
        assert_eq!(stats.large_files_written, 5);
        assert_eq!(
            fs::metadata(temp.0.join("corpus-00000.txt")).unwrap().len(),
            256
        );
        assert!(fs::metadata(temp.0.join("corpus-00002.txt")).unwrap().len() > 256);
    }

    #[test]
    fn same_seed_produces_identical_files() {
        let left = TempDir::new();
        let right = TempDir::new();
        let config = CorpusConfig {
            total_bytes: 2_048,
            file_count: 2,
            small_file_count: 0,
            small_file_bytes: 0,
            korean_percent: 80,
            nfd_percent: 50,
            seed: 7,
        };

        generate_corpus_tree(&left.0, config).unwrap();
        generate_corpus_tree(&right.0, config).unwrap();

        for index in 0..config.file_count {
            let name = format!("corpus-{index:05}.txt");
            assert_eq!(
                fs::read(left.0.join(&name)).unwrap(),
                fs::read(right.0.join(name)).unwrap()
            );
        }
    }

    #[test]
    fn regeneration_removes_stale_generated_files_only() {
        let temp = TempDir::new();
        let mut config = CorpusConfig {
            total_bytes: 3_000,
            file_count: 3,
            small_file_count: 0,
            small_file_bytes: 0,
            korean_percent: 20,
            nfd_percent: 0,
            seed: 1,
        };
        generate_corpus_tree(&temp.0, config).unwrap();
        fs::write(temp.0.join("keep.txt"), "user file").unwrap();

        config.total_bytes = 2_000;
        config.file_count = 2;
        generate_corpus_tree(&temp.0, config).unwrap();

        assert!(!temp.0.join("corpus-00002.txt").exists());
        assert_eq!(
            fs::read_to_string(temp.0.join("keep.txt")).unwrap(),
            "user file"
        );
    }

    #[test]
    fn invalid_configuration_is_rejected() {
        let error = CorpusConfig {
            total_bytes: 1,
            file_count: 0,
            small_file_count: 0,
            small_file_bytes: 0,
            korean_percent: 20,
            nfd_percent: 0,
            seed: 1,
        }
        .validate()
        .unwrap_err();
        assert_eq!(error, CorpusConfigError::ZeroFiles);
    }

    #[test]
    fn invalid_mixed_file_layout_is_rejected() {
        let error = CorpusConfig {
            total_bytes: 100,
            file_count: 2,
            small_file_count: 1,
            small_file_bytes: 75,
            korean_percent: 20,
            nfd_percent: 50,
            seed: 1,
        }
        .validate()
        .unwrap_err();
        assert_eq!(error, CorpusConfigError::LargeFilesSmallerThanSmallFiles);
    }
}
