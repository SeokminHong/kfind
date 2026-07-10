use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};

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
    pub korean_percent: u8,
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
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorpusConfigError {
    ZeroFiles,
    InvalidKoreanPercent(u8),
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
    pub seed: u64,
}

pub fn generate_corpus_tree(
    root: &Path,
    config: CorpusConfig,
) -> Result<CorpusStats, CorpusGenerateError> {
    let config = config.validate()?;
    fs::create_dir_all(root).map_err(CorpusGenerateError::Io)?;

    let mut rng = DeterministicRng::new(config.seed);
    let mut stats = CorpusStats {
        root: root.to_path_buf(),
        bytes_written: 0,
        files_written: 0,
        korean_lines: 0,
        ascii_lines: 0,
        seed: config.seed,
    };
    for index in 0..config.file_count {
        let target_bytes = bytes_for_file(config.total_bytes, config.file_count, index);
        let path = root.join(format!("corpus-{index:05}.txt"));
        let file = File::create(path).map_err(CorpusGenerateError::Io)?;
        let mut writer = BufWriter::new(file);
        write_corpus_file(
            &mut writer,
            target_bytes,
            config.korean_percent,
            &mut rng,
            &mut stats,
        )?;
        writer.flush().map_err(CorpusGenerateError::Io)?;
        stats.files_written += 1;
    }
    Ok(stats)
}

fn write_corpus_file(
    writer: &mut impl Write,
    target_bytes: u64,
    korean_percent: u8,
    rng: &mut DeterministicRng,
    stats: &mut CorpusStats,
) -> Result<(), CorpusGenerateError> {
    let mut remaining = target_bytes;
    while remaining > 0 {
        let korean = rng.below(100) < u64::from(korean_percent);
        let choices = if korean { KOREAN_LINES } else { ASCII_LINES };
        let line = choices[rng.below(choices.len() as u64) as usize].as_bytes();
        if line.len() as u64 <= remaining {
            writer.write_all(line).map_err(CorpusGenerateError::Io)?;
            remaining -= line.len() as u64;
            if korean {
                stats.korean_lines += 1;
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

fn bytes_for_file(total: u64, files: usize, index: usize) -> u64 {
    let files = files as u64;
    let base = total / files;
    base + u64::from((index as u64) < total % files)
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
                korean_percent: 20,
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
    }

    #[test]
    fn same_seed_produces_identical_files() {
        let left = TempDir::new();
        let right = TempDir::new();
        let config = CorpusConfig {
            total_bytes: 2_048,
            file_count: 2,
            korean_percent: 80,
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
    fn invalid_configuration_is_rejected() {
        let error = CorpusConfig {
            total_bytes: 1,
            file_count: 0,
            korean_percent: 20,
            seed: 1,
        }
        .validate()
        .unwrap_err();
        assert_eq!(error, CorpusConfigError::ZeroFiles);
    }
}
