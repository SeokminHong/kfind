use super::*;

#[test]
fn product_owned_details_have_an_english_form() {
    assert_eq!(
        translate_detail("표제어가 유효한 UTF-8이 아닙니다", Language::English),
        "lemma is not valid UTF-8"
    );
    assert_eq!(
        translate_detail(
            "lexical.d-to-l 규칙에 선언되지 않은 predicate flag입니다",
            Language::English
        ),
        "predicate flag is not declared by rule lexical.d-to-l"
    );
    for (korean, english) in [
        (
            "magic 또는 format version이 올바르지 않습니다",
            "invalid magic bytes or format version",
        ),
        ("varint가 중간에 끝났습니다", "truncated varint"),
        (
            "선택 규칙은 정확히 두 이형태를 가져야 합니다",
            "a selection rule must have exactly two variants",
        ),
    ] {
        assert_eq!(translate_detail(korean, Language::English), english);
    }
}
