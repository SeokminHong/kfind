//! Parallel file traversal, decoding, and result collection.

mod walker;

pub use walker::{WalkConfigError, WalkOptions, build_walker};
