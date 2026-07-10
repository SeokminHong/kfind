//! Parallel file traversal, decoding, and result collection.

mod input;
mod walker;

pub use input::{
    FileSearchResult, InputEncoding, InputOptions, InputSearchError, SearchLine, SearchLineKind,
    SearchRecord, search_path, search_reader,
};
pub use walker::{WalkConfigError, WalkOptions, build_walker, resolve_search_paths};
