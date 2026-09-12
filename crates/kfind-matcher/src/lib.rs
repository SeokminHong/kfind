//! Anchor matching and local morphology verification.

mod anchor;
mod boundary;
mod diagnostics;
mod morph;
mod window;

pub use anchor::{AnchorBuildError, AnchorBuildLimits, AnchorEngine, AnchorHit, AnchorHits};
pub use boundary::{BoundaryVerifier, is_token_character};
pub use diagnostics::SearchDiagnostics;
pub use morph::{
    LocalAnalysisCandidate, MatchLimitExceeded, MorphMatcher, MorphMatcherBuildError,
    VerificationCounters,
};
pub use window::{
    AnalysisWindow, AnalysisWindowError, AnalysisWindowLimits, DEFAULT_ANALYSIS_WINDOW_LIMITS,
};
