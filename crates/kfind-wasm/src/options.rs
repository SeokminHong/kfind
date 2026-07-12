use kfind::{
    BoundaryPolicy, CoarsePos, CompileOptionOverrides, CompileOptions, ExpandMode,
    NormalizationMode,
};
use serde::Deserialize;
use wasm_bindgen::{JsCast, JsError, JsValue};

const OPTION_FIELDS: [&str; 6] = [
    "boundary",
    "expand",
    "literal",
    "maxGap",
    "normalization",
    "pos",
];

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
struct JsCompileOptions {
    expand: Option<JsExpandMode>,
    boundary: Option<JsBoundaryPolicy>,
    pos: Option<JsCoarsePos>,
    normalization: Option<JsNormalizationMode>,
    max_gap: Option<usize>,
    literal: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum JsExpandMode {
    Literal,
    Inflection,
    Derivation,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum JsBoundaryPolicy {
    Smart,
    Token,
    Any,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum JsNormalizationMode {
    Nfc,
    Canonical,
    None,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum JsCoarsePos {
    Auto,
    Noun,
    Pronoun,
    Numeral,
    Verb,
    Adjective,
    Determiner,
    Adverb,
    Particle,
    Interjection,
    Literal,
}

pub fn parse_compile_options(value: JsValue) -> Result<CompileOptions, JsError> {
    let options = if value.is_undefined() || value.is_null() {
        JsCompileOptions::default()
    } else {
        validate_option_fields(&value)?;
        serde_wasm_bindgen::from_value(value)
            .map_err(|error| JsError::new(&format!("invalid compile options: {error}")))?
    };

    CompileOptions::resolve(CompileOptionOverrides {
        expand: options.expand.map(Into::into),
        boundary: options.boundary.map(Into::into),
        pos: options.pos.and_then(Into::into),
        normalization: options.normalization.map(Into::into),
        max_gap: options.max_gap,
        literal: options.literal,
        ..CompileOptionOverrides::default()
    })
    .map_err(|error| JsError::new(&format!("invalid compile options: {error}")))
}

fn validate_option_fields(value: &JsValue) -> Result<(), JsError> {
    let object: &js_sys::Object = value.unchecked_ref();
    for key in js_sys::Object::keys(object).iter() {
        let key = key
            .as_string()
            .ok_or_else(|| JsError::new("invalid compile options: option keys must be strings"))?;
        if OPTION_FIELDS.binary_search(&key.as_str()).is_err() {
            return Err(JsError::new(&format!(
                "invalid compile options: unknown field `{key}`"
            )));
        }
    }
    Ok(())
}

impl From<JsExpandMode> for ExpandMode {
    fn from(value: JsExpandMode) -> Self {
        match value {
            JsExpandMode::Literal => Self::Literal,
            JsExpandMode::Inflection => Self::Inflection,
            JsExpandMode::Derivation => Self::Derivation,
        }
    }
}

impl From<JsBoundaryPolicy> for BoundaryPolicy {
    fn from(value: JsBoundaryPolicy) -> Self {
        match value {
            JsBoundaryPolicy::Smart => Self::Smart,
            JsBoundaryPolicy::Token => Self::Token,
            JsBoundaryPolicy::Any => Self::Any,
        }
    }
}

impl From<JsNormalizationMode> for NormalizationMode {
    fn from(value: JsNormalizationMode) -> Self {
        match value {
            JsNormalizationMode::Nfc => Self::Nfc,
            JsNormalizationMode::Canonical => Self::Canonical,
            JsNormalizationMode::None => Self::None,
        }
    }
}

impl From<JsCoarsePos> for Option<CoarsePos> {
    fn from(value: JsCoarsePos) -> Self {
        Some(match value {
            JsCoarsePos::Auto => return None,
            JsCoarsePos::Noun => CoarsePos::Noun,
            JsCoarsePos::Pronoun => CoarsePos::Pronoun,
            JsCoarsePos::Numeral => CoarsePos::Numeral,
            JsCoarsePos::Verb => CoarsePos::Verb,
            JsCoarsePos::Adjective => CoarsePos::Adjective,
            JsCoarsePos::Determiner => CoarsePos::Determiner,
            JsCoarsePos::Adverb => CoarsePos::Adverb,
            JsCoarsePos::Particle => CoarsePos::Particle,
            JsCoarsePos::Interjection => CoarsePos::Interjection,
            JsCoarsePos::Literal => CoarsePos::Literal,
        })
    }
}
