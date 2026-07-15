//! Query parsing, lexical analysis, and search-plan compilation.

mod analysis;
mod ast;
mod compile;
mod error;
mod lexer;
mod lexicons;
mod options;
mod phrase;
mod plan;

pub use analysis::{
    Analysis, AnalysisSource, AnalyzeError, LexiconQueryAnalyzer, Morphology, NominalMorphology,
    NominalOverride, ParticleMorphology, QueryAnalyzer,
};
pub use ast::{DEFAULT_MAX_GAP, PhrasePolicy, QueryAst, QueryAtom};
pub use compile::{compile_query, registered_lexical_context_prefix_len};
pub use error::{
    CompileError, CompileErrorKind, PhraseJoinError, QueryError, QueryErrorKind, SourceSpan,
};
pub use lexer::parse_query;
pub use lexicons::Lexicons;
pub use options::{
    BoundaryPolicy, CompileOptionError, CompileOptionOverrides, CompileOptions,
    DEFAULT_MAX_ANALYSES_PER_ATOM, DEFAULT_MAX_ATOMS, DEFAULT_MAX_BRANCHES,
    DEFAULT_MAX_CONTINUATION_DEPTH, DEFAULT_MAX_MATCHER_BYTES, DEFAULT_MAX_QUERY_SCALARS,
    ExpandMode, NormalizationMode, PlanLimits,
};
pub use phrase::{PhraseMatch, join_phrase_spans};
pub use plan::{
    AtomPlan, BoundaryProof, BranchEnvironment, BranchVerifier, ContextRequirement, CoreMapping,
    Origin, QueryDiagnostic, QueryPlan, SurfaceBranch, VerifiedSpan,
};
