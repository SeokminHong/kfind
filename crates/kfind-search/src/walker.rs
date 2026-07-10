use std::{env, error::Error, fmt, path::PathBuf};

use ignore::{WalkBuilder, WalkParallel, overrides::OverrideBuilder, types::TypesBuilder};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct WalkOptions {
    pub hidden: bool,
    pub no_ignore: bool,
    pub follow_links: bool,
    pub threads: Option<usize>,
    pub globs: Vec<String>,
    pub selected_types: Vec<String>,
    pub type_definitions: Vec<String>,
    pub current_dir: Option<PathBuf>,
}

#[must_use]
pub fn resolve_search_paths(paths: &[PathBuf], stdin_is_terminal: bool) -> Vec<PathBuf> {
    if !paths.is_empty() {
        return paths.to_vec();
    }
    if stdin_is_terminal {
        vec![PathBuf::from(".")]
    } else {
        vec![PathBuf::from("-")]
    }
}

pub fn build_walker(
    paths: &[PathBuf],
    options: &WalkOptions,
) -> Result<WalkParallel, WalkConfigError> {
    let (first, remaining) = paths.split_first().ok_or(WalkConfigError::NoPaths)?;
    let mut builder = WalkBuilder::new(first);
    for path in remaining {
        builder.add(path);
    }

    builder
        .hidden(!options.hidden)
        .follow_links(options.follow_links);
    if options.no_ignore {
        builder
            .parents(false)
            .ignore(false)
            .git_global(false)
            .git_ignore(false)
            .git_exclude(false);
    }
    if let Some(threads) = options.threads {
        builder.threads(threads);
    }

    if !options.globs.is_empty() {
        let current_dir = match &options.current_dir {
            Some(path) => path.clone(),
            None => env::current_dir().map_err(WalkConfigError::CurrentDir)?,
        };
        let mut overrides = OverrideBuilder::new(current_dir);
        for glob in &options.globs {
            overrides
                .add(glob)
                .map_err(|error| WalkConfigError::Glob(error.to_string()))?;
        }
        builder.overrides(
            overrides
                .build()
                .map_err(|error| WalkConfigError::Glob(error.to_string()))?,
        );
    }

    if !options.selected_types.is_empty() || !options.type_definitions.is_empty() {
        let mut types = TypesBuilder::new();
        types.add_defaults();
        for definition in &options.type_definitions {
            types
                .add_def(definition)
                .map_err(|error| WalkConfigError::FileType(error.to_string()))?;
        }
        for selected in &options.selected_types {
            types.select(selected);
        }
        builder.types(
            types
                .build()
                .map_err(|error| WalkConfigError::FileType(error.to_string()))?,
        );
    }

    Ok(builder.build_parallel())
}

#[derive(Debug)]
pub enum WalkConfigError {
    NoPaths,
    CurrentDir(std::io::Error),
    Glob(String),
    FileType(String),
}

impl fmt::Display for WalkConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoPaths => formatter.write_str("at least one search path is required"),
            Self::CurrentDir(error) => {
                write!(formatter, "failed to read current directory: {error}")
            }
            Self::Glob(error) => write!(formatter, "invalid glob: {error}"),
            Self::FileType(error) => write!(formatter, "invalid file type: {error}"),
        }
    }
}

impl Error for WalkConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CurrentDir(error) => Some(error),
            Self::NoPaths | Self::Glob(_) | Self::FileType(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::Path,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use ignore::WalkState;

    use super::*;

    static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

    struct TempTree(PathBuf);

    impl TempTree {
        fn new() -> Self {
            let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
            let path = env::temp_dir().join(format!(
                "kfind-walker-test-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn write(&self, relative: &str, contents: &str) -> PathBuf {
            let path = self.0.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, contents).unwrap();
            path
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn collect_files(walker: WalkParallel) -> Vec<PathBuf> {
        let files = Arc::new(Mutex::new(Vec::new()));
        walker.run(|| {
            let files = Arc::clone(&files);
            Box::new(move |entry| {
                if let Ok(entry) = entry {
                    if entry.file_type().is_some_and(|kind| kind.is_file()) {
                        files.lock().unwrap().push(entry.into_path());
                    }
                }
                WalkState::Continue
            })
        });
        let mut files = Arc::try_unwrap(files).unwrap().into_inner().unwrap();
        files.sort();
        files
    }

    fn relative_files(root: &Path, files: Vec<PathBuf>) -> Vec<PathBuf> {
        files
            .into_iter()
            .map(|path| path.strip_prefix(root).unwrap().to_path_buf())
            .collect()
    }

    fn unwrap_build_error(result: Result<WalkParallel, WalkConfigError>) -> WalkConfigError {
        match result {
            Ok(_) => panic!("expected walker configuration to fail"),
            Err(error) => error,
        }
    }

    #[test]
    fn empty_paths_are_rejected() {
        let error = unwrap_build_error(build_walker(&[], &WalkOptions::default()));

        assert_eq!(error.to_string(), "at least one search path is required");
    }

    #[test]
    fn omitted_paths_select_stdin_or_current_directory() {
        assert_eq!(resolve_search_paths(&[], false), [PathBuf::from("-")]);
        assert_eq!(resolve_search_paths(&[], true), [PathBuf::from(".")]);

        let explicit = [PathBuf::from("src"), PathBuf::from("-")];
        assert_eq!(resolve_search_paths(&explicit, true), explicit);
    }

    #[test]
    fn malformed_globs_are_rejected_before_walking() {
        let error = unwrap_build_error(build_walker(
            &[PathBuf::from(".")],
            &WalkOptions {
                globs: vec!["[".to_owned()],
                ..WalkOptions::default()
            },
        ));

        assert!(error.to_string().starts_with("invalid glob:"));
    }

    #[test]
    fn unknown_file_types_are_rejected_before_walking() {
        let error = unwrap_build_error(build_walker(
            &[PathBuf::from(".")],
            &WalkOptions {
                selected_types: vec!["does-not-exist".to_owned()],
                ..WalkOptions::default()
            },
        ));

        assert!(error.to_string().starts_with("invalid file type:"));
    }

    #[test]
    fn default_walk_respects_hidden_and_ignore_rules() {
        let tree = TempTree::new();
        tree.write(".git/HEAD", "ref: refs/heads/main\n");
        tree.write(".gitignore", "ignored.rs\n");
        tree.write("visible.rs", "visible");
        tree.write("ignored.rs", "ignored");
        tree.write(".hidden.rs", "hidden");
        tree.write("nested/readme.md", "docs");

        let files = relative_files(
            &tree.0,
            collect_files(
                build_walker(std::slice::from_ref(&tree.0), &WalkOptions::default()).unwrap(),
            ),
        );

        assert_eq!(
            files,
            [
                PathBuf::from("nested/readme.md"),
                PathBuf::from("visible.rs")
            ]
        );
    }

    #[test]
    fn hidden_and_no_ignore_options_are_independent() {
        let tree = TempTree::new();
        tree.write(".gitignore", "ignored.rs\n");
        tree.write("ignored.rs", "ignored");
        tree.write(".hidden.rs", "hidden");

        let files = relative_files(
            &tree.0,
            collect_files(
                build_walker(
                    std::slice::from_ref(&tree.0),
                    &WalkOptions {
                        hidden: true,
                        no_ignore: true,
                        ..WalkOptions::default()
                    },
                )
                .unwrap(),
            ),
        );

        assert!(files.contains(&PathBuf::from(".hidden.rs")));
        assert!(files.contains(&PathBuf::from("ignored.rs")));
    }

    #[test]
    fn explicit_ignored_files_and_multiple_roots_are_walked() {
        let tree = TempTree::new();
        tree.write(".gitignore", "first.rs\n");
        let first = tree.write("first.rs", "first");
        let second = tree.write("nested/second.rs", "second");

        let files = collect_files(
            build_walker(&[first.clone(), second.clone()], &WalkOptions::default()).unwrap(),
        );

        assert_eq!(files, [first, second]);
    }

    #[test]
    fn glob_and_custom_type_filters_apply_to_real_paths() {
        let tree = TempTree::new();
        tree.write("source.rs", "source");
        tree.write("guide.mdx", "guide");
        tree.write("notes.txt", "notes");

        let files = relative_files(
            &tree.0,
            collect_files(
                build_walker(
                    std::slice::from_ref(&tree.0),
                    &WalkOptions {
                        globs: vec!["*.{mdx,txt}".to_owned(), "!notes.txt".to_owned()],
                        type_definitions: vec!["docs:*.{mdx,txt}".to_owned()],
                        selected_types: vec!["docs".to_owned()],
                        current_dir: Some(tree.0.clone()),
                        ..WalkOptions::default()
                    },
                )
                .unwrap(),
            ),
        );

        assert_eq!(files, [PathBuf::from("guide.mdx")]);
    }
}
