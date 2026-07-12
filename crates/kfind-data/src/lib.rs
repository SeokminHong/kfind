//! Build-time validation and compilation for kfind data files.

mod binary;
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
pub use error::{DataError, DataErrorKind, DataWarning, SourceLocation};
pub use fixture::{ExpectedMatch, FixturePos, MorphologyCase, parse_morphology_cases_tsv};
pub use lexicon::{
    DataAlternation, DataFinePos, LexiconData, LexiconSources, ModifierRecord, NominalRecord,
    ParticleRecord, PredicateRecord, SurfaceOverride, UserLexicon, UserNominalRecord,
    UserPredicateRecord, parse_lexicons, parse_modifiers_tsv, parse_nominals_tsv,
    parse_particles_tsv, parse_predicates_tsv, parse_user_lexicon_toml,
};
pub use mecab::{
    MecabConnectionMatrix, MecabExtraction, MecabMorphologyEntry, MecabMorphologyExtraction,
    MecabSourceMorphologyEntry, MecabSourceMorphologyExtraction, extract_mecab_ko_dic,
    extract_mecab_morphology, extract_mecab_source_morphology, parse_mecab_connection_matrix,
};
pub use morphology::{
    DecodedMorphologyResource, MorphologyAnalysis, MorphologyResourceStats,
    decode_morphology_resource, encode_morphology_resource, parse_sha256,
};
pub use rules::{
    AlternationRule, ContractionRule, DerivationRule, EndingCategory, EndingInitial, EndingRule,
    ParticleSelection, ParticleTransitionRule, RuleSet, RuleSources, parse_rule_set,
};
pub use validation::{ValidatedData, load_data_dir, validate_data};
