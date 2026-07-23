use std::collections::HashSet;
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use dialoguer::MultiSelect;

use crate::output::write_safe_text;
use crate::{AgentArg, Args, Language};

mod error;
mod hook_config;

pub use error::InitError;
pub(crate) const SKILL_CONTENT: &str = include_str!("../../../skills/kfind/SKILL.md");
const MANAGED_MARKER: &str = "<!-- managed by kfind init -->";
const MAX_STDIN_BYTES: u64 = 4 * 1024;
static NEXT_TEMP_FILE: AtomicUsize = AtomicUsize::new(0);

#[derive(Clone, Debug)]
enum SkillSource {
    Embedded,
    #[cfg(unix)]
    Homebrew(PathBuf),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InstallAction {
    Install,
    Update,
    Unchanged,
}

pub fn run_init_with_io<R, W, E>(
    args: &Args,
    language: Language,
    mut stdin: R,
    stdout: &mut W,
    stderr: &mut E,
    interactive_terminal: bool,
) -> Result<(), InitError>
where
    R: Read,
    W: Write,
    E: Write,
{
    let root = env::current_dir().map_err(InitError::CurrentDirectory)?;
    let source = resolve_skill_source()?;
    run_init_in_directory(
        args,
        language,
        &mut stdin,
        stdout,
        stderr,
        interactive_terminal,
        &root,
        source,
    )
}

#[allow(clippy::too_many_arguments)]
fn run_init_in_directory<R, W, E>(
    args: &Args,
    language: Language,
    stdin: &mut R,
    stdout: &mut W,
    stderr: &mut E,
    interactive_terminal: bool,
    root: &Path,
    source: SkillSource,
) -> Result<(), InitError>
where
    R: Read,
    W: Write,
    E: Write,
{
    let agents = selected_agents(args, stdin, language, interactive_terminal)?;
    if agents.is_empty() {
        write_status(
            stderr,
            language.select("No agents selected.", "선택한 agent가 없습니다."),
        )?;
        return Ok(());
    }

    let mut installations = Vec::new();
    let mut hook_installations = Vec::new();
    for agent in agents.iter().copied().filter(|agent| agent.installs_file()) {
        let path = agent.skill_path(root);
        let action = inspect_destination(&path, &source)?;
        installations.push((agent, path, action));
        if let Some(installation) = hook_config::prepare_hook_installation(root, agent)? {
            hook_installations.push(installation);
        }
    }

    for (agent, path, action) in &installations {
        if *action != InstallAction::Unchanged {
            install_skill(path, &source)?;
        }
        write_install_status(stderr, language, *agent, path, *action)?;
    }
    for installation in &hook_installations {
        installation.write()?;
        write_hook_install_status(
            stderr,
            language,
            installation.agent,
            &installation.path,
            installation.action,
        )?;
    }

    if agents.contains(&AgentArg::Custom) {
        stdout
            .write_all(SKILL_CONTENT.as_bytes())
            .map_err(InitError::WriteOutput)?;
        stdout.flush().map_err(InitError::WriteOutput)?;
    }

    Ok(())
}

fn selected_agents<R: Read>(
    args: &Args,
    stdin: &mut R,
    language: Language,
    interactive_terminal: bool,
) -> Result<Vec<AgentArg>, InitError> {
    if !args.agent.is_empty() {
        return Ok(deduplicate(&args.agent));
    }
    if interactive_terminal {
        return select_interactively(language);
    }
    read_agents(stdin)
}

fn select_interactively(language: Language) -> Result<Vec<AgentArg>, InitError> {
    let agents = AgentArg::ALL;
    let labels = agents.map(AgentArg::display_name);
    let selected = MultiSelect::new()
        .with_prompt(language.select(
            "Select kfind integration targets",
            "kfind 통합을 설치할 agent를 선택하세요",
        ))
        .items(labels)
        .interact_opt()
        .map_err(InitError::Prompt)?
        .unwrap_or_default();
    Ok(selected.into_iter().map(|index| agents[index]).collect())
}

fn read_agents(reader: &mut impl Read) -> Result<Vec<AgentArg>, InitError> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_STDIN_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(InitError::ReadInput)?;
    if bytes.len() as u64 > MAX_STDIN_BYTES {
        return Err(InitError::InputTooLarge);
    }
    let input = std::str::from_utf8(&bytes).map_err(|_| InitError::InvalidInputUtf8)?;
    let mut agents = Vec::new();
    for value in input.split_whitespace() {
        agents.push(
            AgentArg::from_name(value).ok_or_else(|| InitError::UnknownAgent {
                value: value.to_owned(),
            })?,
        );
    }
    if agents.is_empty() {
        return Err(InitError::EmptyInput);
    }
    Ok(deduplicate(&agents))
}

fn deduplicate(agents: &[AgentArg]) -> Vec<AgentArg> {
    let mut seen = HashSet::new();
    agents
        .iter()
        .copied()
        .filter(|agent| seen.insert(*agent))
        .collect()
}

fn inspect_destination(path: &Path, source: &SkillSource) -> Result<InstallAction, InitError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(InstallAction::Install),
        Err(source) => {
            return Err(InitError::Inspect {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    if metadata.file_type().is_symlink() {
        let target = fs::read_link(path).map_err(|source| InitError::Inspect {
            path: path.to_path_buf(),
            source,
        })?;
        if source.matches_link(&target) {
            return Ok(InstallAction::Unchanged);
        }
        if is_managed_homebrew_link(&target) {
            return Ok(InstallAction::Update);
        }
        return Err(InitError::UnmanagedDestination(path.to_path_buf()));
    }
    if !metadata.is_file() {
        return Err(InitError::UnmanagedDestination(path.to_path_buf()));
    }

    let contents = fs::read_to_string(path).map_err(|source| InitError::Inspect {
        path: path.to_path_buf(),
        source,
    })?;
    if !contents.contains(MANAGED_MARKER) {
        return Err(InitError::UnmanagedDestination(path.to_path_buf()));
    }
    if matches!(source, SkillSource::Embedded) && contents == SKILL_CONTENT {
        Ok(InstallAction::Unchanged)
    } else {
        Ok(InstallAction::Update)
    }
}

fn install_skill(path: &Path, source: &SkillSource) -> Result<(), InitError> {
    let parent = path.parent().ok_or_else(|| InitError::InvalidDestination {
        path: path.to_path_buf(),
    })?;
    fs::create_dir_all(parent).map_err(|source| InitError::CreateDirectory {
        path: parent.to_path_buf(),
        source,
    })?;
    let temporary = temporary_path(parent);
    let result = match source {
        SkillSource::Embedded => write_embedded_skill(&temporary),
        #[cfg(unix)]
        SkillSource::Homebrew(target) => std::os::unix::fs::symlink(target, &temporary),
    };
    result.map_err(|source| InitError::Write {
        path: temporary.clone(),
        source,
    })?;
    if let Err(source) = fs::rename(&temporary, path) {
        let _ = fs::remove_file(&temporary);
        return Err(InitError::Write {
            path: path.to_path_buf(),
            source,
        });
    }
    Ok(())
}

fn write_embedded_skill(path: &Path) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(SKILL_CONTENT.as_bytes())?;
    file.sync_all()
}

fn temporary_path(parent: &Path) -> PathBuf {
    let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
    parent.join(format!(".SKILL.md.kfind-{}-{sequence}", std::process::id()))
}

fn resolve_skill_source() -> Result<SkillSource, InitError> {
    #[cfg(unix)]
    if let Some(path) = homebrew_skill_path()? {
        return Ok(SkillSource::Homebrew(path));
    }
    Ok(SkillSource::Embedded)
}

#[cfg(unix)]
fn homebrew_skill_path() -> Result<Option<PathBuf>, InitError> {
    let executable = match env::current_exe() {
        Ok(executable) => executable,
        Err(_) => return Ok(None),
    };
    let mut candidates = Vec::new();
    if let Some(prefix) = executable
        .ancestors()
        .find(|path| path.file_name().is_some_and(|name| name == "Cellar"))
        .and_then(Path::parent)
    {
        candidates.push(prefix.join("opt/kfind/share/kfind/skills/kfind/SKILL.md"));
    }
    if let Some(binary_prefix) = executable.parent().and_then(Path::parent) {
        candidates.push(binary_prefix.join("opt/kfind/share/kfind/skills/kfind/SKILL.md"));
        if binary_prefix
            .file_name()
            .is_some_and(|name| name == "kfind")
            && binary_prefix
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|name| name == "opt")
        {
            candidates.push(binary_prefix.join("share/kfind/skills/kfind/SKILL.md"));
        }
    }
    let Some(path) = candidates.into_iter().find(|path| path.is_file()) else {
        return Ok(None);
    };
    let contents = fs::read_to_string(&path).map_err(|source| InitError::Inspect {
        path: path.clone(),
        source,
    })?;
    if contents != SKILL_CONTENT {
        return Err(InitError::PackagedSkillMismatch(path));
    }
    Ok(Some(path))
}

fn is_managed_homebrew_link(target: &Path) -> bool {
    target.ends_with(Path::new("opt/kfind/share/kfind/skills/kfind/SKILL.md"))
}

fn write_install_status(
    writer: &mut impl Write,
    language: Language,
    agent: AgentArg,
    path: &Path,
    action: InstallAction,
) -> Result<(), InitError> {
    let verb = match action {
        InstallAction::Install => language.select("Installed", "설치했습니다"),
        InstallAction::Update => language.select("Updated", "갱신했습니다"),
        InstallAction::Unchanged => language.select("Unchanged", "변경 없음"),
    };
    let message = match language {
        Language::English => format!("{verb} {} skill: {}", agent.display_name(), path.display()),
        Language::Korean => format!("{} skill {verb}: {}", agent.display_name(), path.display()),
    };
    write_status(writer, &message)
}

fn write_hook_install_status(
    writer: &mut impl Write,
    language: Language,
    agent: AgentArg,
    path: &Path,
    action: InstallAction,
) -> Result<(), InitError> {
    let verb = match action {
        InstallAction::Install => language.select("Installed", "설치했습니다"),
        InstallAction::Update => language.select("Updated", "갱신했습니다"),
        InstallAction::Unchanged => language.select("Unchanged", "변경 없음"),
    };
    let message = match language {
        Language::English => format!("{verb} {} hook: {}", agent.display_name(), path.display()),
        Language::Korean => format!("{} hook {verb}: {}", agent.display_name(), path.display()),
    };
    write_status(writer, &message)
}

fn write_status(writer: &mut impl Write, message: &str) -> Result<(), InitError> {
    write_safe_text(writer, message).map_err(InitError::WriteDiagnostics)?;
    writer.write_all(b"\n").map_err(InitError::WriteDiagnostics)
}

impl AgentArg {
    const ALL: [Self; 4] = [Self::ClaudeCode, Self::Codex, Self::Gemini, Self::Custom];

    const fn display_name(self) -> &'static str {
        match self {
            Self::ClaudeCode => "Claude Code",
            Self::Codex => "Codex",
            Self::Gemini => "Gemini CLI",
            Self::Custom => "Custom output (stdout)",
        }
    }

    const fn installs_file(self) -> bool {
        !matches!(self, Self::Custom)
    }

    fn from_name(value: &str) -> Option<Self> {
        match value {
            "claude-code" => Some(Self::ClaudeCode),
            "codex" => Some(Self::Codex),
            "gemini" => Some(Self::Gemini),
            "custom" => Some(Self::Custom),
            _ => None,
        }
    }

    fn skill_path(self, root: &Path) -> PathBuf {
        let relative = match self {
            Self::ClaudeCode => ".claude/skills/kfind/SKILL.md",
            Self::Codex => ".agents/skills/kfind/SKILL.md",
            Self::Gemini => ".gemini/skills/kfind/SKILL.md",
            Self::Custom => unreachable!("custom output does not install a file"),
        };
        root.join(relative)
    }
}

impl SkillSource {
    fn matches_link(&self, target: &Path) -> bool {
        #[cfg(not(unix))]
        let _ = target;
        match self {
            Self::Embedded => false,
            #[cfg(unix)]
            Self::Homebrew(expected) => target == expected,
        }
    }
}

#[cfg(test)]
mod tests;
