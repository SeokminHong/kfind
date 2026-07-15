//! Build-time validation and compilation for kfind data files.

mod binary;
mod component;
mod error;
mod fixture;
mod lexicon;
mod mecab;
mod morphology;
mod rules;
mod tsv;
mod validation;

pub use binary::{
    ApprovedPosLexicon, DecodedPosLexicon, PosLexiconEntry, PosLexiconStats, collect_pos_entries,
    decode_pos_lexicon, encode_pos_lexicon,
};
pub use component::{
    COMPONENT_RESOURCE_SOURCE_DIGEST, ComponentAnalysis, ComponentResource, ComponentResourceStats,
    MorphologyGraphAnalysis, MorphologyGraphComponent, MorphologyGraphExpressionKind,
    MorphologyGraphPosClass, MorphologyGraphProjectionStats, MorphologyGraphResource,
    MorphologyGraphResourceStats, MorphologyGraphStringId, decode_component_resource,
    decode_morphology_graph_resource, encode_component_resource, encode_morphology_graph_resource,
    validate_morphology_graph_projection,
};
pub use error::{DataError, DataErrorKind, DataWarning, SourceLocation};
pub use fixture::{ExpectedMatch, FixturePos, MorphologyCase, parse_morphology_cases_tsv};
pub use lexicon::{
    DICTIONARY_CONJUGATION_RULE_ID, DICTIONARY_RELATED_ADVERB_RULE_ID, DataAlternation,
    DataFinePos, LexiconData, LexiconSources, ModifierRecord, NominalRecord, ParticleRecord,
    PredicateRecord, SurfaceOverride, UserLexicon, UserNominalRecord, UserPredicateRecord,
    is_dictionary_surface_rule, parse_lexicons, parse_modifiers_tsv, parse_nominals_tsv,
    parse_particles_tsv, parse_predicates_tsv, parse_user_lexicon_toml,
};
pub use mecab::{
    MecabConnectionMatrix, MecabExtraction, MecabMorphologyEntry, MecabMorphologyExtraction,
    MecabSourceMorphologyEntry, MecabSourceMorphologyExtraction, extract_mecab_ko_dic,
    extract_mecab_morphology, extract_mecab_source_morphology, parse_mecab_connection_matrix,
};
pub use morphology::{
    DecodedMorphologyResource, MorphologyAnalysis, MorphologyExpressionAlignment,
    MorphologyExpressionAlignmentKind, MorphologyExpressionComponent, MorphologyResourceStats,
    align_morphology_expression, decode_morphology_resource, encode_morphology_resource,
    morphology_pos_transitions, parse_sha256,
};
pub use rules::{
    AlternationRule, ContractionRule, DerivationRule, EndingCategory, EndingInitial, EndingRule,
    ParticleSelection, ParticleTransitionRule, RuleSet, RuleSources, parse_rule_set,
};
pub use validation::{ValidatedData, load_data_dir, validate_data, validate_predicates};
