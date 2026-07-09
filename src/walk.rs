use crate::cli::Cli;
use std::fs;
use std::path::{Path, PathBuf};

pub fn walk_paths(cli: &Cli) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for raw in &cli.paths {
        if has_glob(raw) {
            expand_glob(raw, cli, &mut out);
        } else {
            collect(raw, cli, &mut out);
        }
    }
    out.sort();
    out.dedup();
    out
}
fn has_glob(p: &Path) -> bool {
    p.to_string_lossy()
        .chars()
        .any(|c| matches!(c, '*' | '?' | '['))
}
fn expand_glob(pattern: &Path, cli: &Cli, out: &mut Vec<PathBuf>) {
    let pat = pattern.to_string_lossy();
    let root = glob_root(&pat);
    collect_matching(&root, &pat, cli, out);
}
fn glob_root(pat: &str) -> PathBuf {
    let mut root = PathBuf::new();
    for part in Path::new(pat).components() {
        let s = part.as_os_str().to_string_lossy();
        if s.chars().any(|c| matches!(c, '*' | '?' | '[')) {
            break;
        }
        root.push(part.as_os_str());
    }
    if root.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        root
    }
}
fn collect_matching(p: &Path, pat: &str, cli: &Cli, out: &mut Vec<PathBuf>) {
    if let Ok(md) = fs::metadata(p) {
        if md.is_file() {
            let s = p.to_string_lossy();
            if glob_match(pat, &s) && allowed(p, cli) {
                out.push(p.into());
            }
        } else if md.is_dir() {
            if skip_hidden_dir(p, cli) {
                return;
            }
            if let Ok(rd) = fs::read_dir(p) {
                for e in rd.flatten() {
                    collect_matching(&e.path(), pat, cli, out);
                }
            }
        }
    }
}
fn collect(p: &Path, cli: &Cli, out: &mut Vec<PathBuf>) {
    if let Ok(md) = fs::metadata(p) {
        if md.is_file() {
            if allowed(p, cli) {
                out.push(p.into())
            }
        } else if md.is_dir() {
            if skip_hidden_dir(p, cli) {
                return;
            }
            if let Ok(rd) = fs::read_dir(p) {
                for e in rd.flatten() {
                    collect(&e.path(), cli, out);
                }
            }
        }
    }
}
fn skip_hidden_dir(p: &Path, cli: &Cli) -> bool {
    !cli.hidden
        && p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.') && n != ".")
}
fn allowed(p: &Path, cli: &Cli) -> bool {
    let s = p.to_string_lossy();
    (cli.includes.is_empty() || cli.includes.iter().any(|g| glob_match(g, &s)))
        && !cli.excludes.iter().any(|g| glob_match(g, &s))
}
fn glob_match(pat: &str, text: &str) -> bool {
    wildcard(pat.as_bytes(), text.as_bytes())
        || Path::new(text)
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| wildcard(pat.as_bytes(), n.as_bytes()))
}
fn wildcard(p: &[u8], t: &[u8]) -> bool {
    let (mut pi, mut ti, mut star, mut match_i) = (0, 0, None, 0);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == b'?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == b'*' {
            star = Some(pi);
            match_i = ti;
            pi += 1;
        } else if let Some(si) = star {
            pi = si + 1;
            match_i += 1;
            ti = match_i;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == b'*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wildcard_matches_file_names() {
        assert!(glob_match("*.rs", "src/main.rs"));
        assert!(glob_match("src/*.rs", "src/main.rs"));
        assert!(!glob_match("*.md", "src/main.rs"));
    }
}
