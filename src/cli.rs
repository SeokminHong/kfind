use crate::query::{Interpretation, MatchMode};
use std::env;
use std::path::PathBuf;

const MAX_QUERY_CHARS: usize = 512;

#[derive(Debug, Clone)]
pub struct Cli {
    pub query: String,
    pub paths: Vec<PathBuf>,
    pub interpretation: Interpretation,
    pub mode: MatchMode,
    pub derive: bool,
    pub max_gap: usize,
    pub adjacent: bool,
    pub explain_query: bool,
    pub explain_match: bool,
    pub json: bool,
    pub count: bool,
    pub files_with_matches: bool,
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
    pub hidden: bool,
    pub no_ignore: bool,
    pub threads: usize,
    pub mmap: MmapMode,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum MmapMode {
    Auto,
    On,
    Off,
}

impl Cli {
    pub fn parse_env() -> Result<Self, String> {
        Self::parse_from(env::args().skip(1))
    }

    pub fn parse_from<I, S>(args: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut args = args.into_iter().map(Into::into).peekable();
        let first = args.next().ok_or_else(usage)?;
        if first == "--help" || first == "-h" {
            println!("{}", usage());
            std::process::exit(0);
        }
        if first.starts_with('-') {
            return Err(format!("missing query before option: {first}\n{}", usage()));
        }
        validate_query(&first)?;
        let mut cli = Cli {
            query: first,
            paths: Vec::new(),
            interpretation: Interpretation::Auto,
            mode: MatchMode::Normal,
            derive: false,
            max_gap: 24,
            adjacent: false,
            explain_query: false,
            explain_match: false,
            json: false,
            count: false,
            files_with_matches: false,
            includes: Vec::new(),
            excludes: Vec::new(),
            hidden: false,
            no_ignore: false,
            threads: 1,
            mmap: MmapMode::Auto,
        };
        while let Some(a) = args.next() {
            match split_opt(&a) {
                Some(("as", v)) => cli.interpretation = parse_interp(value(v, &mut args, "--as")?)?,
                Some(("mode", v)) => cli.mode = parse_mode(value(v, &mut args, "--mode")?)?,
                Some(("max-gap", v)) => {
                    cli.max_gap =
                        parse_bounded_usize(value(v, &mut args, "--max-gap")?, 4096, "--max-gap")?
                }
                Some(("include", v)) => {
                    let val = value(v, &mut args, "--include")?;
                    cli.includes.push(nonempty(&val, "--include")?.to_string())
                }
                Some(("exclude", v)) => {
                    let val = value(v, &mut args, "--exclude")?;
                    cli.excludes.push(nonempty(&val, "--exclude")?.to_string())
                }
                Some(("threads", v)) => {
                    cli.threads =
                        parse_bounded_usize(value(v, &mut args, "--threads")?, 1024, "--threads")?
                            .max(1)
                }
                Some(("mmap", v)) => cli.mmap = parse_mmap(value(v, &mut args, "--mmap")?)?,
                Some(("derive", None)) => cli.derive = true,
                Some(("adjacent", None)) => cli.adjacent = true,
                Some(("explain-query", None)) => cli.explain_query = true,
                Some(("explain-match", None)) => cli.explain_match = true,
                Some(("json", None)) => cli.json = true,
                Some(("count", None)) => cli.count = true,
                Some(("files-with-matches", None)) => cli.files_with_matches = true,
                Some(("hidden", None)) => cli.hidden = true,
                Some(("no-ignore", None)) => cli.no_ignore = true,
                Some((name, Some(_))) => return Err(format!("unknown option: --{name}")),
                Some((name, None)) => return Err(format!("unknown option: --{name}")),
                None if a.starts_with('-') => return Err(format!("unknown option: {a}")),
                None => cli.paths.push(PathBuf::from(a)),
            }
        }
        if cli.paths.is_empty() {
            cli.paths.push(PathBuf::from("."));
        }
        Ok(cli)
    }
}

fn split_opt(a: &str) -> Option<(&str, Option<&str>)> {
    a.strip_prefix("--")
        .map(|x| x.split_once('=').map_or((x, None), |(k, v)| (k, Some(v))))
}
fn value<I: Iterator<Item = String>>(
    v: Option<&str>,
    args: &mut I,
    name: &str,
) -> Result<String, String> {
    v.map(str::to_string)
        .or_else(|| args.next())
        .ok_or_else(|| format!("{name} needs a value"))
}
fn nonempty<'a>(s: &'a str, name: &str) -> Result<&'a str, String> {
    if s.is_empty() {
        Err(format!("{name} must not be empty"))
    } else {
        Ok(s)
    }
}
fn parse_bounded_usize(s: String, max: usize, name: &str) -> Result<usize, String> {
    let n: usize = s.parse().map_err(|_| format!("invalid {name}"))?;
    if n > max {
        Err(format!("{name} is too large; max {max}"))
    } else {
        Ok(n)
    }
}
fn parse_mmap(s: String) -> Result<MmapMode, String> {
    Ok(match s.as_str() {
        "auto" => MmapMode::Auto,
        "on" => MmapMode::On,
        "off" => MmapMode::Off,
        _ => return Err("invalid --mmap".into()),
    })
}
fn parse_interp(s: String) -> Result<Interpretation, String> {
    Ok(match s.as_str() {
        "auto" => Interpretation::Auto,
        "literal" => Interpretation::Literal,
        "noun" => Interpretation::Noun,
        "predicate" => Interpretation::Predicate,
        "modifier" => Interpretation::Modifier,
        "particle" => Interpretation::Particle,
        _ => return Err("invalid --as".into()),
    })
}
fn parse_mode(s: String) -> Result<MatchMode, String> {
    Ok(match s.as_str() {
        "strict" => MatchMode::Strict,
        "normal" => MatchMode::Normal,
        "loose" => MatchMode::Loose,
        _ => return Err("invalid --mode".into()),
    })
}
fn validate_query(q: &str) -> Result<(), String> {
    if q.trim().is_empty() {
        return Err("query must not be empty".into());
    }
    if q.chars().count() > MAX_QUERY_CHARS {
        return Err(format!(
            "query is too long; max {MAX_QUERY_CHARS} characters"
        ));
    }
    if q.chars()
        .any(|c| c == '\0' || c.is_control() && !c.is_whitespace())
    {
        return Err("query contains unsupported control characters".into());
    }
    Ok(())
}
pub fn usage() -> String {
    "Usage: kfind <QUERY> [PATH|GLOB ...] [--as auto|literal|noun|predicate|modifier|particle] [--mode strict|normal|loose] [--include GLOB] [--exclude GLOB]".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_multi_paths_and_equals() {
        let c = Cli::parse_from([
            "걷다",
            "src/*.rs",
            "tests",
            "--as=predicate",
            "--max-gap",
            "16",
            "--include",
            "*.rs",
        ])
        .unwrap();
        assert_eq!(c.paths.len(), 2);
        assert_eq!(c.interpretation, Interpretation::Predicate);
        assert_eq!(c.max_gap, 16);
    }
    #[test]
    fn rejects_control_and_huge_gap() {
        assert!(Cli::parse_from(["bad\0"]).is_err());
        assert!(Cli::parse_from(["걷다", "--max-gap=999999"]).is_err());
    }
}
