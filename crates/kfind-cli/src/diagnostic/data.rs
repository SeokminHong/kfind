use std::borrow::Cow;
use std::fmt::{self, Formatter};

use kfind_data::{DataError, DataErrorKind};

use crate::Language;

pub(super) fn write_error(
    error: &DataError,
    language: Language,
    formatter: &mut Formatter<'_>,
) -> fmt::Result {
    write!(formatter, "{}: ", error.location)?;
    match error.kind.as_ref() {
        DataErrorKind::Io(message) => write!(
            formatter,
            "{}: {message}",
            language.select("I/O error", "I/O 오류")
        ),
        DataErrorKind::InvalidHeader { expected, actual } => write!(
            formatter,
            "{}: {} `{expected}`, {} `{actual}`",
            language.select("invalid TSV header", "TSV 헤더가 올바르지 않습니다"),
            language.select("expected", "기대값"),
            language.select("got", "실제값")
        ),
        DataErrorKind::InvalidFieldCount { expected, actual } => write!(
            formatter,
            "{}: {} {expected}, {} {actual}",
            language.select("invalid field count", "필드 수가 올바르지 않습니다"),
            language.select("expected", "기대값"),
            language.select("got", "실제값")
        ),
        DataErrorKind::InvalidValue {
            field,
            value,
            reason,
        } => write!(
            formatter,
            "{} `{value}` {} `{field}`: {}",
            language.select("invalid value", "올바르지 않은 값"),
            language.select("for", "필드"),
            translate_detail(reason, language)
        ),
        DataErrorKind::NonNfc { field, value } => write!(
            formatter,
            "{} `{value}` {} `{field}` {}",
            language.select("value", "값"),
            language.select("for", "필드"),
            language.select("is not NFC", "이 NFC가 아닙니다")
        ),
        DataErrorKind::InvalidPredicateLemma(lemma) => write!(
            formatter,
            "{} `{lemma}` {}",
            language.select("predicate lemma", "용언 표제어"),
            language.select(
                "must be a non-empty -다 form",
                "는 비어 있지 않은 `다` 기본형이어야 합니다"
            )
        ),
        DataErrorKind::DuplicateRuleId(id) => write!(
            formatter,
            "{} `{id}`",
            language.select("duplicate rule ID", "중복된 규칙 ID")
        ),
        DataErrorKind::UnknownRuleId(id) => write!(
            formatter,
            "{} `{id}`",
            language.select("unknown rule ID", "존재하지 않는 규칙 ID")
        ),
        DataErrorKind::OverrideConflict {
            lemma,
            rule_id,
            first,
            second,
        } => write!(
            formatter,
            "{} `{lemma}` / `{rule_id}`: `{first}` / `{second}`",
            language.select("conflicting overrides for", "서로 충돌하는 override"),
            lemma = translate_placeholder(lemma, language)
        ),
        DataErrorKind::Toml(message) => write!(
            formatter,
            "{}: {message}",
            language.select("TOML schema error", "TOML 스키마 오류")
        ),
        DataErrorKind::Binary(message) => write!(
            formatter,
            "{}: {}",
            language.select("POS lexicon binary error", "POS lexicon binary 오류"),
            translate_detail(message, language)
        ),
        DataErrorKind::MissingFixtureCoverage(feature) => write!(
            formatter,
            "{} `{feature}`",
            language.select(
                "no morphology fixture covers lexicon class",
                "morphology fixture가 없는 사전 class"
            )
        ),
    }
}

fn translate_detail(message: &str, language: Language) -> Cow<'_, str> {
    if language == Language::Korean {
        return Cow::Borrowed(message);
    }
    let translated = match message {
        "predicate POS가 아닙니다" => "not a predicate POS",
        "nominal POS가 아닙니다" => "not a nominal POS",
        "modifier POS가 아닙니다" => "not a modifier POS",
        "particle POS가 아닙니다" => "not a particle POS",
        "알려진 lexical alternation이 아닙니다" => "unknown lexical alternation",
        "하나 이상의 표면형이 필요합니다" => "at least one surface form is required",
        "지원하는 세부 품사가 아닙니다" => "unsupported fine-grained part of speech",
        "대문자 ASCII identifier여야 합니다" => "must be an uppercase ASCII identifier",
        "`rule.id=surface` 형식이어야 합니다" => "must use the `rule.id=surface` format",
        "surface가 비어 있습니다" | "비어 있습니다" => "must not be empty",
        "alternation 규칙에 선언되지 않은 flag입니다" => {
            "flag is not declared by the alternation rule"
        }
        "사용자 사전 predicate POS가 아닙니다" => "not a user-lexicon predicate POS",
        "사용자 사전 nominal POS가 아닙니다" => "not a user-lexicon nominal POS",
        "소문자 ASCII namespace 형식이어야 합니다" => {
            "must be a lowercase ASCII namespace"
        }
        "1..=4 범위여야 합니다" => "must be in the range 1..=4",
        "지원 버전은 1입니다" => "supported version is 1",
        "하나 이상의 next 전이가 필요합니다" => {
            "at least one next transition is required"
        }
        "nonterminal 규칙에는 하나 이상의 next 전이가 필요합니다" => {
            "a nonterminal rule requires at least one next transition"
        }
        "alternation kind는 하나의 규범 규칙만 가져야 합니다" => {
            "an alternation kind must have exactly one canonical rule"
        }
        "lexical.* 규칙이어야 합니다" => "must be a lexical.* rule",
        "required feature와 겹칩니다" => "overlaps a required feature",
        "suffix와 source_pos가 필요합니다" => "suffix and source_pos are required",
        "비어 있거나 중복된 표면형입니다" => "surface form is empty or duplicated",
        "선택 규칙은 정확히 두 이형태를 가져야 합니다" => {
            "a selection rule must have exactly two variants"
        }
        "알려진 고유 morphology feature여야 합니다" => {
            "must be a known, unique morphology feature"
        }
        "morphology fixture 스키마에 맞지 않습니다" => {
            "does not match the morphology fixture schema"
        }
        "mecab-ko-dic CSV 스키마에 맞지 않습니다" => {
            "does not match the mecab-ko-dic CSV schema"
        }
        "순환 전이는 허용하지 않습니다" => "cyclic transitions are not allowed",
        "max_continuation_depth를 초과합니다" => "exceeds max_continuation_depth",
        "entry 수가 u32 범위를 초과합니다" => "entry count exceeds the u32 range",
        "entry 수 상한을 초과합니다" => "entry count exceeds the limit",
        "binary 파일 크기 상한을 초과합니다" => "binary file size exceeds the limit",
        "magic 또는 format version이 올바르지 않습니다" => {
            "invalid magic bytes or format version"
        }
        "entry 수가 binary 크기와 일치하지 않습니다" => {
            "entry count does not match the binary size"
        }
        "entry 저장 공간을 할당할 수 없습니다" => "failed to allocate entry storage",
        "prefix 길이가 이전 표제어의 문자 경계가 아닙니다" => {
            "prefix length is not a character boundary in the previous lemma"
        }
        "suffix가 binary 범위를 벗어납니다" => "suffix exceeds the binary bounds",
        "entry가 엄격한 정렬 순서가 아닙니다" => "entries are not strictly sorted",
        "decoded lemma byte 수가 overflow했습니다" => "decoded lemma byte count overflowed",
        "decoded lemma byte 수 상한을 초과합니다" => {
            "decoded lemma byte count exceeds the limit"
        }
        "varint가 중간에 끝났습니다" => "truncated varint",
        "varint가 u32 범위를 초과합니다" => "varint exceeds the u32 range",
        "varint가 너무 깁니다" => "varint is too long",
        "u32가 중간에 끝났습니다" => "truncated u32",
        "u32를 읽을 수 없습니다" => "failed to read u32",
        "표제어가 비어 있습니다" => "lemma is empty",
        "표제어가 너무 깁니다" => "lemma is too long",
        "표제어 길이가 overflow했습니다" => "lemma length overflowed",
        "표제어 저장 공간을 할당할 수 없습니다" => {
            "failed to allocate lemma storage"
        }
        "알 수 없는 POS code입니다" => "unknown POS code",
        "표제어가 유효한 UTF-8이 아닙니다" => "lemma is not valid UTF-8",
        "마지막 entry 뒤에 불필요한 바이트가 있습니다" => {
            "unexpected bytes follow the last entry"
        }
        _ => return translate_dynamic_detail(message),
    };
    Cow::Borrowed(translated)
}

fn translate_dynamic_detail(message: &str) -> Cow<'_, str> {
    if let Some(rule) = message.strip_suffix(" 규칙에 선언되지 않은 predicate flag입니다")
    {
        return Cow::Owned(format!("predicate flag is not declared by rule {rule}"));
    }
    if let Some(rule) = message.strip_suffix(" 규칙의 forms와 정확히 일치해야 합니다")
    {
        return Cow::Owned(format!("must exactly match the forms of rule {rule}"));
    }
    Cow::Borrowed(message)
}

fn translate_placeholder(value: &str, language: Language) -> Cow<'_, str> {
    if language == Language::English {
        match value {
            "현재 행" => return Cow::Borrowed("current row"),
            "사용자 사전 항목" => return Cow::Borrowed("user lexicon entry"),
            _ => {}
        }
    }
    Cow::Borrowed(value)
}

#[cfg(test)]
mod tests;
