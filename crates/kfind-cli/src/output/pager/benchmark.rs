use std::error::Error;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Instant;

use serde::Serialize;
use tempfile::tempfile;

use super::protocol::{ColumnRange, MatchLine, PagerMatch, write_match_line};
use super::viewport::Document;

#[derive(Serialize)]
struct IndexStats {
    length: usize,
    capacity: usize,
    entry_bytes: usize,
    initialized_bytes: usize,
    capacity_bytes: usize,
}

#[derive(Serialize)]
struct Timing {
    generate_seconds: f64,
    index_seconds: f64,
    layout_seconds: f64,
}

#[derive(Serialize)]
pub struct PagerMemoryReport {
    profile: &'static str,
    source_lines: usize,
    matches_per_line: usize,
    terminal_width: usize,
    file_bytes: u64,
    source_index: IndexStats,
    layout_index: IndexStats,
    total_index_initialized_bytes: usize,
    total_index_capacity_bytes: usize,
    timing: Timing,
}

pub fn run_pager_memory_benchmark(
    source_lines: usize,
    matches_per_line: usize,
    terminal_width: usize,
) -> Result<PagerMemoryReport, Box<dyn Error>> {
    if terminal_width == 0 {
        return Err("terminal width must be greater than zero".into());
    }

    let generate_started = Instant::now();
    let file = generate_input(source_lines, matches_per_line)?;
    let generate_seconds = generate_started.elapsed().as_secs_f64();
    let file_bytes = file.metadata()?.len();

    let index_started = Instant::now();
    let mut document = Document::open(file)?;
    let index_seconds = index_started.elapsed().as_secs_f64();

    let layout_started = Instant::now();
    let layout = document.layout(terminal_width)?;
    let layout_seconds = layout_started.elapsed().as_secs_f64();

    let source_index = index_stats(document.index_stats());
    let layout_index = index_stats(layout.index_stats());
    let expected_rows = if matches_per_line == 0 {
        source_lines
    } else {
        source_lines
            .checked_mul(matches_per_line)
            .ok_or("row count overflow")?
    };
    if layout_index.length != expected_rows {
        return Err(format!(
            "expected {expected_rows} layout rows, got {}",
            layout_index.length
        )
        .into());
    }

    Ok(PagerMemoryReport {
        profile: if matches_per_line == 0 {
            "plain"
        } else {
            "expanded-match"
        },
        source_lines,
        matches_per_line,
        terminal_width,
        file_bytes,
        total_index_initialized_bytes: source_index
            .initialized_bytes
            .saturating_add(layout_index.initialized_bytes),
        total_index_capacity_bytes: source_index
            .capacity_bytes
            .saturating_add(layout_index.capacity_bytes),
        source_index,
        layout_index,
        timing: Timing {
            generate_seconds,
            index_seconds,
            layout_seconds,
        },
    })
}

fn generate_input(source_lines: usize, matches_per_line: usize) -> Result<File, Box<dyn Error>> {
    let mut writer = BufWriter::new(tempfile()?);
    if matches_per_line == 0 {
        for _ in 0..source_lines {
            writer.write_all(b"sample result\n")?;
        }
    } else {
        let line = expanded_match_line(matches_per_line);
        for _ in 0..source_lines {
            write_match_line(&mut writer, &line)?;
        }
    }
    writer.flush()?;
    Ok(writer.into_inner()?)
}

fn expanded_match_line(matches_per_line: usize) -> MatchLine {
    const SLOT_WIDTH: usize = 16;
    const MATCH_START: usize = 4;
    const MATCH_WIDTH: usize = 5;

    MatchLine {
        content: "x".repeat(matches_per_line.saturating_mul(SLOT_WIDTH)),
        prefixes: vec!["sample.txt:1: ".to_owned(); matches_per_line],
        matches: (0..matches_per_line)
            .map(|index| {
                let start = index.saturating_mul(SLOT_WIDTH).saturating_add(MATCH_START);
                let span = ColumnRange::new(start, start.saturating_add(MATCH_WIDTH));
                PagerMatch {
                    span,
                    tokens: vec![span],
                }
            })
            .collect(),
        color: false,
    }
}

fn index_stats((length, capacity, entry_bytes): (usize, usize, usize)) -> IndexStats {
    IndexStats {
        length,
        capacity,
        entry_bytes,
        initialized_bytes: length.saturating_mul(entry_bytes),
        capacity_bytes: capacity.saturating_mul(entry_bytes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_plain_and_expanded_index_shapes() {
        let plain = run_pager_memory_benchmark(3, 0, 32).unwrap();
        assert_eq!(plain.source_index.length, 3);
        assert_eq!(plain.layout_index.length, 3);

        let expanded = run_pager_memory_benchmark(3, 4, 32).unwrap();
        assert_eq!(expanded.source_index.length, 3);
        assert_eq!(expanded.layout_index.length, 12);
    }
}
