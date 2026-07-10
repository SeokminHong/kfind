//! Anchor matching and local morphology verification.

mod anchor;
mod boundary;
mod morph;

pub use anchor::{AnchorBuildError, AnchorBuildLimits, AnchorEngine, AnchorHit, AnchorHits};
pub use boundary::{BoundaryVerifier, is_token_character};
pub use morph::{MorphMatcher, MorphMatcherBuildError};
