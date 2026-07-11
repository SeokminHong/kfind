use super::*;

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
