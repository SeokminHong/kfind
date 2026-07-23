use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io;
use std::path::PathBuf;

use crate::Language;

#[derive(Debug)]
pub enum InitError {
    CurrentDirectory(io::Error),
    ReadInput(io::Error),
    InputTooLarge,
    InvalidInputUtf8,
    EmptyInput,
    UnknownAgent {
        value: String,
    },
    Prompt(dialoguer::Error),
    Inspect {
        path: PathBuf,
        source: io::Error,
    },
    UnmanagedDestination(PathBuf),
    InvalidDestination {
        path: PathBuf,
    },
    CreateDirectory {
        path: PathBuf,
        source: io::Error,
    },
    Write {
        path: PathBuf,
        source: io::Error,
    },
    InspectAgentConfig {
        path: PathBuf,
        source: io::Error,
    },
    ParseAgentConfig {
        path: PathBuf,
        source: serde_json::Error,
    },
    InvalidAgentConfig {
        path: PathBuf,
        reason: &'static str,
    },
    UnmanagedAgentConfig(PathBuf),
    CreateAgentConfigDirectory {
        path: PathBuf,
        source: io::Error,
    },
    WriteAgentConfig {
        path: PathBuf,
        source: io::Error,
    },
    EncodeAgentConfig {
        path: PathBuf,
        source: serde_json::Error,
    },
    PackagedSkillMismatch(PathBuf),
    WriteOutput(io::Error),
    WriteDiagnostics(io::Error),
}

impl InitError {
    pub(crate) fn localized(&self, language: Language) -> String {
        match self {
            Self::CurrentDirectory(error) => format!(
                "{}: {error}",
                language.select(
                    "failed to read current directory",
                    "현재 디렉터리를 읽을 수 없습니다"
                )
            ),
            Self::ReadInput(error) => format!(
                "{}: {error}",
                language.select("failed to read agent input", "agent 입력을 읽을 수 없습니다")
            ),
            Self::InputTooLarge => language
                .select(
                    "agent input exceeds 4096 bytes",
                    "agent 입력이 4096 byte를 초과했습니다",
                )
                .to_owned(),
            Self::InvalidInputUtf8 => language
                .select(
                    "agent input is not valid UTF-8",
                    "agent 입력이 올바른 UTF-8이 아닙니다",
                )
                .to_owned(),
            Self::EmptyInput => language
                .select(
                    "no agents were provided; use --agent or pipe agent names",
                    "agent가 입력되지 않았습니다. --agent를 사용하거나 agent 이름을 pipe로 전달하세요",
                )
                .to_owned(),
            Self::UnknownAgent { value } => format!(
                "{} `{value}`",
                language.select("unknown agent", "알 수 없는 agent")
            ),
            Self::Prompt(error) => format!(
                "{}: {error}",
                language.select("agent selection failed", "agent 선택에 실패했습니다")
            ),
            Self::Inspect { path, source } => format!(
                "{} {}: {source}",
                language.select(
                    "failed to inspect skill destination",
                    "skill 설치 경로를 확인할 수 없습니다:"
                ),
                path.display()
            ),
            Self::UnmanagedDestination(path) => format!(
                "{}: {}",
                language.select(
                    "existing skill is not managed by kfind",
                    "기존 skill이 kfind 관리 대상이 아닙니다"
                ),
                path.display()
            ),
            Self::InvalidDestination { path } => format!(
                "{}: {}",
                language.select("invalid skill destination", "skill 설치 경로가 올바르지 않습니다"),
                path.display()
            ),
            Self::CreateDirectory { path, source } => format!(
                "{} {}: {source}",
                language.select("failed to create skill directory", "skill 디렉터리 생성 실패:"),
                path.display()
            ),
            Self::Write { path, source } => format!(
                "{} {}: {source}",
                language.select("failed to install skill", "skill 설치 실패:"),
                path.display()
            ),
            Self::InspectAgentConfig { path, source } => format!(
                "{} {}: {source}",
                language.select(
                    "failed to inspect agent configuration",
                    "agent 설정을 확인할 수 없습니다:"
                ),
                path.display()
            ),
            Self::ParseAgentConfig { path, source } => format!(
                "{} {}: {source}",
                language.select(
                    "agent configuration is not valid JSON",
                    "agent 설정이 올바른 JSON이 아닙니다:"
                ),
                path.display()
            ),
            Self::InvalidAgentConfig { path, reason } => format!(
                "{} {}: {reason}",
                language.select(
                    "agent configuration cannot be merged",
                    "agent 설정을 병합할 수 없습니다:"
                ),
                path.display()
            ),
            Self::UnmanagedAgentConfig(path) => format!(
                "{}: {}",
                language.select(
                    "existing agent configuration is not a regular file",
                    "기존 agent 설정이 일반 파일이 아닙니다"
                ),
                path.display()
            ),
            Self::CreateAgentConfigDirectory { path, source } => format!(
                "{} {}: {source}",
                language.select(
                    "failed to create agent configuration directory",
                    "agent 설정 디렉터리 생성 실패:"
                ),
                path.display()
            ),
            Self::WriteAgentConfig { path, source } => format!(
                "{} {}: {source}",
                language.select("failed to install agent hook", "agent hook 설치 실패:"),
                path.display()
            ),
            Self::EncodeAgentConfig { path, source } => format!(
                "{} {}: {source}",
                language.select(
                    "failed to encode agent configuration",
                    "agent 설정을 인코딩할 수 없습니다:"
                ),
                path.display()
            ),
            Self::PackagedSkillMismatch(path) => format!(
                "{}: {}",
                language.select(
                    "installed skill does not match the kfind binary",
                    "설치된 skill이 kfind binary와 일치하지 않습니다"
                ),
                path.display()
            ),
            Self::WriteOutput(error) => format!(
                "{}: {error}",
                language.select("failed to write skill output", "skill 출력 실패")
            ),
            Self::WriteDiagnostics(error) => format!(
                "{}: {error}",
                language.select("failed to write diagnostics", "진단 메시지 출력 실패")
            ),
        }
    }
}

impl Display for InitError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.localized(Language::English))
    }
}

impl Error for InitError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CurrentDirectory(error)
            | Self::ReadInput(error)
            | Self::WriteOutput(error)
            | Self::WriteDiagnostics(error) => Some(error),
            Self::Prompt(error) => Some(error),
            Self::Inspect { source, .. }
            | Self::CreateDirectory { source, .. }
            | Self::Write { source, .. }
            | Self::InspectAgentConfig { source, .. }
            | Self::CreateAgentConfigDirectory { source, .. }
            | Self::WriteAgentConfig { source, .. } => Some(source),
            Self::ParseAgentConfig { source, .. } | Self::EncodeAgentConfig { source, .. } => {
                Some(source)
            }
            Self::InputTooLarge
            | Self::InvalidInputUtf8
            | Self::EmptyInput
            | Self::UnknownAgent { .. }
            | Self::UnmanagedDestination(_)
            | Self::InvalidDestination { .. }
            | Self::InvalidAgentConfig { .. }
            | Self::UnmanagedAgentConfig(_)
            | Self::PackagedSkillMismatch(_) => None,
        }
    }
}
