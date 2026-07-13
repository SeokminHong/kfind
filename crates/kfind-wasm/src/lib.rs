//! JavaScript bindings for the kfind in-memory matcher.

mod options;
mod output;

use kfind::{Engine, Matcher as RustMatcher};
use wasm_bindgen::prelude::*;

use crate::options::parse_compile_options;
use crate::output::serialize_matches;

#[wasm_bindgen(typescript_custom_section)]
const TYPESCRIPT_TYPES: &str = r#"
export type ExpandMode = "literal" | "inflection" | "derivation";
export type BoundaryPolicy = "smart" | "token" | "any";
export type PartOfSpeech =
  | "auto"
  | "noun"
  | "pronoun"
  | "numeral"
  | "verb"
  | "adjective"
  | "determiner"
  | "adverb"
  | "particle"
  | "interjection"
  | "literal";
export type NormalizationMode = "nfc" | "canonical" | "none";

export interface CompileOptions {
  expand?: ExpandMode;
  boundary?: BoundaryPolicy;
  pos?: PartOfSpeech;
  normalization?: NormalizationMode;
  maxGap?: number;
  literal?: boolean;
}

export interface Span {
  readonly start: number;
  readonly end: number;
}

export interface MatchOrigin {
  readonly analysisIndex: number;
  readonly rulePath: readonly string[];
}

export interface MatchAtom {
  readonly core: Span;
  readonly token: Span;
  readonly origins: readonly MatchOrigin[];
}

export interface Match {
  readonly start: number;
  readonly end: number;
  readonly atoms: readonly MatchAtom[];
}

export interface Kfind {
  compile(query: string, options?: CompileOptions): Matcher;
}
"#;

/// Reusable lexicon state exposed to JavaScript.
#[wasm_bindgen]
pub struct Kfind {
    inner: Engine,
}

#[wasm_bindgen]
impl Kfind {
    #[wasm_bindgen(constructor)]
    pub fn new(component_resource: &[u8]) -> Result<Kfind, JsError> {
        Engine::new(component_resource)
            .map(|inner| Self { inner })
            .map_err(|error| JsError::new(&format!("failed to initialize kfind: {error}")))
    }

    #[wasm_bindgen(js_name = withFullPos)]
    pub fn with_full_pos(component_resource: &[u8], full_pos: &[u8]) -> Result<Kfind, JsError> {
        Engine::with_full_pos(component_resource, full_pos)
            .map(|inner| Self { inner })
            .map_err(|error| JsError::new(&format!("failed to initialize kfind: {error}")))
    }

    #[wasm_bindgen(getter, js_name = fullPosLoaded)]
    pub fn full_pos_loaded(&self) -> bool {
        self.inner.full_pos_loaded()
    }

    #[wasm_bindgen(skip_typescript)]
    pub fn compile(&self, query: &str, options: JsValue) -> Result<Matcher, JsError> {
        let options = parse_compile_options(options)?;
        self.inner
            .compile(query, &options)
            .map(|inner| Matcher { inner })
            .map_err(|error| JsError::new(&format!("failed to compile query: {error}")))
    }
}

/// A compiled query exposed to JavaScript.
#[wasm_bindgen]
pub struct Matcher {
    inner: RustMatcher,
}

#[wasm_bindgen]
impl Matcher {
    #[wasm_bindgen(js_name = findAll, unchecked_return_type = "readonly Match[]")]
    pub fn find_all(&self, text: &str) -> Result<JsValue, JsError> {
        serialize_matches(text, &self.inner.find_all(text.as_bytes()))
    }
}
