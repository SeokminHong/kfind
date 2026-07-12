//! Anchor matching and local morphology verification.

mod anchor;
mod boundary;
mod morph;
mod window;

pub use anchor::{AnchorBuildError, AnchorBuildLimits, AnchorEngine, AnchorHit, AnchorHits};
pub use boundary::{BoundaryVerifier, is_token_character};
pub use morph::{MorphMatcher, MorphMatcherBuildError, VerificationCounters};
pub use window::{
    AnalysisWindow, AnalysisWindowError, AnalysisWindowLimits, DEFAULT_ANALYSIS_WINDOW_LIMITS,
};
