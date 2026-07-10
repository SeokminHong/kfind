//! Parallel file traversal, decoding, and result collection.

mod executor;
mod input;
mod walker;

#[cfg(test)]
mod executor_tests;

pub use executor::{
    ExecutionOptions, ResultOrder, SearchConfig, SearchEvent, SearchIssue, SearchIssueKind,
    SearchRunError, SearchSummary, execute_search, execute_search_with_stdin,
};
pub use input::{
    FileSearchResult, InputEncoding, InputOptions, InputSearchError, InputSearcher, SearchLine,
    SearchLineKind, SearchRecord, search_path, search_reader,
};
pub use walker::{WalkConfigError, WalkOptions, build_walker, resolve_search_paths};
