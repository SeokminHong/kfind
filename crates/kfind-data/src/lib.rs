//! Build-time validation and compilation for kfind data files.

mod binary;
mod error;
mod fixture;
mod lexicon;
mod mecab;
mod rules;
mod tsv;
mod validation;

pub use binary::{
    ApprovedPosLexicon, DecodedPosLexicon, PosLexiconEntry, collect_pos_entries,
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
pub use mecab::{GoldApprovedMecabLexicon, MecabExtraction, extract_mecab_ko_dic};
pub use rules::{
    AlternationRule, ContractionRule, DerivationRule, EndingCategory, EndingInitial, EndingRule,
    ParticleSelection, ParticleTransitionRule, RuleSet, RuleSources, parse_rule_set,
};
pub use validation::{ValidatedData, load_data_dir, validate_data};
