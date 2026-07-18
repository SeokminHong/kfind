use super::*;
use crate::InitError;

#[test]
fn compile_option_errors_are_localized() {
    let error = CliError::Options(CompileOptionError::LiteralPosConflict {
        pos: CoarsePos::Verb,
    });
    let english = error.localized(Language::English).to_string();
    assert!(english.contains("conflicts"));
    assert!(english.contains("--pos verb"));
    assert!(
        error
            .localized(Language::Korean)
            .to_string()
            .contains("함께 사용할 수 없습니다")
    );
}

#[test]
fn top_level_errors_escape_terminal_control_characters() {
    let error = CliError::MissingData("bad\u{1b}[31m\nlexicon.bin".into());
    let mut output = Vec::new();

    write_cli_error(&mut output, &error, Language::English).unwrap();

    assert!(!output.contains(&0x1b));
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains(r"bad\u{001B}[31m\nlexicon.bin"));
    assert!(output.ends_with('\n'));
}

#[test]
fn resource_size_errors_are_localized() {
    let error = CliError::ResourceTooLarge {
        path: "lexicon.bin".into(),
        limit: 128 * 1024 * 1024,
    };

    assert!(
        error
            .localized(Language::English)
            .to_string()
            .contains("resource exceeds the size limit")
    );
    assert!(
        error
            .localized(Language::Korean)
            .to_string()
            .contains("resource가 크기 상한을 초과했습니다")
    );
}

#[test]
fn init_errors_escape_terminal_control_characters() {
    let error = CliError::Init(InitError::UnknownAgent {
        value: "bad\u{1b}[31m\nagent".to_owned(),
    });
    let mut output = Vec::new();

    write_cli_error(&mut output, &error, Language::English).unwrap();

    assert!(!output.contains(&0x1b));
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains(r"bad\u{001B}[31m\nagent"));
}
