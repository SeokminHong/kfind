use crate::cli::Cli;
use crate::query::{CompiledQuery, Seed};
use std::io::{self, Write};
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Hit {
    pub start: usize,
    pub end: usize,
    pub seed: Seed,
}

pub fn matching_hits(cq: &CompiledQuery, bytes: &[u8]) -> Vec<Hit> {
    let hits = scan_bytes(cq, bytes);
    if cq.phrase {
        phrase_hits(cq, hits)
    } else {
        hits
    }
}
fn scan_bytes(cq: &CompiledQuery, bytes: &[u8]) -> Vec<Hit> {
    let text = String::from_utf8_lossy(bytes);
    let mut hits = Vec::new();
    for seed in &cq.seeds {
        let mut off = 0;
        while off <= text.len() {
            let Some(i) = text[off..].find(&seed.surface) else {
                break;
            };
            let start = off + i;
            let end = start + seed.surface.len();
            if text.is_char_boundary(start) && text.is_char_boundary(end) {
                hits.push(Hit {
                    start,
                    end,
                    seed: seed.clone(),
                });
            }
            off = start + seed.surface.len().max(1);
        }
    }
    hits.sort_by_key(|h| (h.start, std::cmp::Reverse(h.end - h.start)));
    let mut dedup = Vec::new();
    for h in hits {
        if dedup.last().is_none_or(|p: &Hit| p.start != h.start) {
            dedup.push(h);
        }
    }
    dedup
}
fn phrase_hits(cq: &CompiledQuery, hits: Vec<Hit>) -> Vec<Hit> {
    let mut out = Vec::new();
    for h in hits.iter().filter(|h| h.seed.atom_index == 0) {
        let mut prev = h;
        let mut ok = true;
        for atom in 1..cq.atoms.len() {
            if let Some(n) = hits.iter().find(|x| {
                x.seed.atom_index == atom
                    && x.start >= prev.end
                    && x.start - prev.end <= cq.max_gap * 3
                    && (!cq.adjacent
                        || is_adjacent_gap(&String::from_utf8_lossy(&[]), x.start - prev.end))
            }) {
                prev = n
            } else {
                ok = false;
                break;
            }
        }
        if ok {
            out.push(h.clone())
        }
    }
    out
}
fn is_adjacent_gap(_gap: &str, byte_len: usize) -> bool {
    byte_len <= 12
}

pub fn print_matches(
    w: &mut impl Write,
    cli: &Cli,
    path: &Path,
    bytes: &[u8],
    hits: &[Hit],
) -> io::Result<()> {
    for h in hits {
        let (line, col, text) = line_info(bytes, h.start);
        if cli.json {
            writeln!(
                w,
                "{{\"path\":\"{}\",\"line\":{},\"column\":{},\"text\":\"{}\",\"matches\":[{{\"surface\":\"{}\",\"generated_from\":\"{}\",\"atom_type\":\"{}\",\"rule\":\"{}\",\"note\":\"homonym-disambiguation-disabled\"}}]}}",
                esc(&path.display().to_string()),
                line,
                col,
                esc(&text),
                esc(&h.seed.surface),
                esc(&h.seed.generated_from),
                h.seed.atom_type.as_str(),
                esc(&h.seed.rule_id)
            )?
        } else {
            writeln!(w, "{}:{line}:{col}: {text}", path.display())?;
            if cli.explain_match {
                writeln!(
                    w,
                    "  surface: {}\n  generated_from: {}\n  atom_type: {}\n  rule: {}",
                    h.seed.surface,
                    h.seed.generated_from,
                    h.seed.atom_type.as_str(),
                    h.seed.rule_id
                )?;
            }
        }
    }
    Ok(())
}
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
fn line_info(bytes: &[u8], pos: usize) -> (usize, usize, String) {
    let safe_pos = pos.min(bytes.len());
    let start = bytes[..safe_pos]
        .iter()
        .rposition(|&b| b == b'\n')
        .map(|i| i + 1)
        .unwrap_or(0);
    let end = bytes[safe_pos..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|i| safe_pos + i)
        .unwrap_or(bytes.len());
    let line = bytes[..safe_pos].iter().filter(|&&b| b == b'\n').count() + 1;
    let col = String::from_utf8_lossy(&bytes[start..safe_pos])
        .chars()
        .count()
        + 1;
    (
        line,
        col,
        String::from_utf8_lossy(&bytes[start..end]).into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    use crate::query::compile_query;
    #[test]
    fn handles_invalid_utf8_without_panic() {
        let c = Cli::parse_from(["걷다"]).unwrap();
        let q = compile_query(&c);
        let h = matching_hits(&q, b"\xff\xfe \xea\xb1\xb8\xec\x96\xb4");
        assert!(!h.is_empty());
    }
    #[test]
    fn json_escapes_control_chars() {
        assert_eq!(esc("a\nb\""), "a\\nb\\\"");
    }
}
