use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap_complete::{Shell, generate};

use crate::Language;
use crate::parse::localized_command;

const PROGRAM_NAME: &str = "kfind";
const MAN_WORKFLOWS: &str = r#".SH WORKFLOWS
.SS Interactive use
Part-of-speech tagging is optional for interactive searches. The default auto
POS and smart boundary mode favor precise results and use the installed full
POS lexicon when available.
.PP
.EX
kfind 걷다 src
kfind 사용자 src docs
.EE
.SS Agent automation
Agents should specify the part of speech for every morphology atom and use
unrestricted boundaries, the embedded lexicon, and JSON Lines output.
.PP
.EX
kfind --embedded --boundary any --pos verb --json 걷다 src docs
kfind --embedded --boundary any --json 'n:사용자 v:검증하다' src
.EE
The agent must inspect context and discard false positives. Narrow the path or
glob, or retry with smart boundaries when the candidate set is too large.
"#;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DistributionAssets {
    pub man_page: PathBuf,
    pub bash_completion: PathBuf,
    pub zsh_completion: PathBuf,
    pub fish_completion: PathBuf,
}

#[derive(Debug)]
pub enum AssetGenerationError {
    CreateDirectory { path: PathBuf, source: io::Error },
    RenderManPage(io::Error),
    WriteFile { path: PathBuf, source: io::Error },
}

impl fmt::Display for AssetGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CreateDirectory { path, source } => {
                write!(formatter, "failed to create {}: {source}", path.display())
            }
            Self::RenderManPage(source) => write!(formatter, "failed to render man page: {source}"),
            Self::WriteFile { path, source } => {
                write!(formatter, "failed to write {}: {source}", path.display())
            }
        }
    }
}

impl Error for AssetGenerationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CreateDirectory { source, .. }
            | Self::WriteFile { source, .. }
            | Self::RenderManPage(source) => Some(source),
        }
    }
}

pub fn generate_distribution_assets(
    output_root: impl AsRef<Path>,
) -> Result<DistributionAssets, AssetGenerationError> {
    let output_root = output_root.as_ref();
    let man_directory = output_root.join("man/man1");
    let completion_directory = output_root.join("completions");
    create_directory(&man_directory)?;
    create_directory(&completion_directory)?;

    let assets = DistributionAssets {
        man_page: man_directory.join("kfind.1"),
        bash_completion: completion_directory.join("kfind.bash"),
        zsh_completion: completion_directory.join("_kfind"),
        fish_completion: completion_directory.join("kfind.fish"),
    };

    let mut man_page = Vec::new();
    clap_mangen::Man::new(localized_command(Language::English))
        .render(&mut man_page)
        .map_err(AssetGenerationError::RenderManPage)?;
    man_page.extend_from_slice(MAN_WORKFLOWS.as_bytes());
    write_file(&assets.man_page, &man_page)?;
    write_file(&assets.bash_completion, &render_completion(Shell::Bash))?;
    write_file(&assets.zsh_completion, &render_completion(Shell::Zsh))?;
    write_file(&assets.fish_completion, &render_completion(Shell::Fish))?;

    Ok(assets)
}

fn create_directory(path: &Path) -> Result<(), AssetGenerationError> {
    fs::create_dir_all(path).map_err(|source| AssetGenerationError::CreateDirectory {
        path: path.to_path_buf(),
        source,
    })
}

fn write_file(path: &Path, contents: &[u8]) -> Result<(), AssetGenerationError> {
    fs::write(path, contents).map_err(|source| AssetGenerationError::WriteFile {
        path: path.to_path_buf(),
        source,
    })
}

fn render_completion(shell: Shell) -> Vec<u8> {
    let mut command = localized_command(Language::English);
    let mut completion = Vec::new();
    generate(shell, &mut command, PROGRAM_NAME, &mut completion);
    completion
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_deterministic_homebrew_assets() {
        let temporary_directory = tempfile::tempdir().unwrap();
        let assets = generate_distribution_assets(temporary_directory.path()).unwrap();
        let first_contents = read_assets(&assets);

        assert_eq!(
            assets.man_page,
            temporary_directory.path().join("man/man1/kfind.1")
        );
        assert_eq!(
            assets.bash_completion,
            temporary_directory.path().join("completions/kfind.bash")
        );
        assert_eq!(
            assets.zsh_completion,
            temporary_directory.path().join("completions/_kfind")
        );
        assert_eq!(
            assets.fish_completion,
            temporary_directory.path().join("completions/kfind.fish")
        );

        assert!(first_contents[0].contains("kfind"));
        assert!(first_contents[0].contains("Fast Korean lemma"));
        assert!(first_contents[0].contains(".SH WORKFLOWS"));
        assert!(first_contents[0].contains(".SS Interactive use"));
        assert!(first_contents[0].contains(".SS Agent automation"));
        assert!(first_contents[0].contains("--embedded --boundary any --pos verb --json"));
        assert!(first_contents[1].contains("_kfind"));
        assert!(first_contents[2].contains("#compdef kfind"));
        assert!(first_contents[3].contains("complete -c kfind"));

        let regenerated_assets = generate_distribution_assets(temporary_directory.path()).unwrap();
        assert_eq!(first_contents, read_assets(&regenerated_assets));
    }

    fn read_assets(assets: &DistributionAssets) -> [String; 4] {
        [
            fs::read_to_string(&assets.man_page).unwrap(),
            fs::read_to_string(&assets.bash_completion).unwrap(),
            fs::read_to_string(&assets.zsh_completion).unwrap(),
            fs::read_to_string(&assets.fish_completion).unwrap(),
        ]
    }
}
