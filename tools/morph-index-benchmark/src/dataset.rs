use std::collections::BTreeMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::{Context, Result, ensure};
use kfind_data::{DataFinePos, MecabMorphologyEntry, extract_mecab_morphology};
use serde::Serialize;

const WORKLOAD_SIZE: usize = 10_000;

#[derive(Debug)]
pub struct Dataset {
    keys: Vec<(String, u32)>,
    payload: Vec<u8>,
    pos_counts: [u32; 23],
    rows_read: usize,
    skipped_unsupported_pos: usize,
    duplicate_entries: usize,
}

#[derive(Debug, Serialize)]
pub struct BuildReport<'a> {
    pub schema_version: u32,
    pub source_sha256: &'a str,
    pub rows_read: usize,
    pub skipped_unsupported_pos: usize,
    pub duplicate_entries: usize,
    pub surface_count: u32,
    pub analysis_count: u32,
    pub entries_by_pos: BTreeMap<&'static str, u32>,
}

#[derive(Debug, serde::Deserialize, Serialize)]
pub struct Workload {
    pub exact: Vec<String>,
    pub prefix: Vec<String>,
}

impl Dataset {
    pub fn load(paths: &[PathBuf]) -> Result<Self> {
        ensure!(!paths.is_empty(), "at least one MeCab CSV is required");
        let mut entries = Vec::new();
        let mut rows_read = 0;
        let mut skipped_unsupported_pos = 0;
        let mut extraction_duplicates = 0;
        for path in paths {
            let file =
                File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
            let extraction =
                extract_mecab_morphology(&path.display().to_string(), BufReader::new(file))?;
            rows_read += extraction.rows_read;
            skipped_unsupported_pos += extraction.skipped_unsupported_pos;
            extraction_duplicates += extraction.duplicate_entries;
            entries.extend_from_slice(extraction.entries());
        }
        entries.sort_unstable();
        let before_merge = entries.len();
        entries.dedup();
        let duplicate_entries = extraction_duplicates + before_merge - entries.len();

        let mut grouped = BTreeMap::<String, Vec<MecabMorphologyEntry>>::new();
        let mut pos_counts = [0_u32; 23];
        for entry in entries {
            pos_counts[usize::from(entry.pos.code())] += 1;
            grouped
                .entry(entry.surface.clone())
                .or_default()
                .push(entry);
        }
        let mut keys = Vec::with_capacity(grouped.len());
        let mut groups = Vec::with_capacity(grouped.len());
        for (group_id, (surface, analyses)) in grouped.into_iter().enumerate() {
            keys.push((surface, u32::try_from(group_id)?));
            groups.push(analyses);
        }
        let payload = encode_payload(&groups)?;
        Ok(Self {
            keys,
            payload,
            pos_counts,
            rows_read,
            skipped_unsupported_pos,
            duplicate_entries,
        })
    }

    pub fn keys(&self) -> &[(String, u32)] {
        &self.keys
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    pub fn pos_counts(&self) -> [u32; 23] {
        self.pos_counts
    }

    pub fn surface_count(&self) -> u32 {
        u32::try_from(self.keys.len()).expect("surface count was checked while encoding")
    }

    pub fn analysis_count(&self) -> u32 {
        self.pos_counts.iter().sum()
    }

    pub fn workload(&self) -> Workload {
        let step = self.keys.len().div_ceil(WORKLOAD_SIZE).max(1);
        let sampled = self.keys.iter().step_by(step).take(WORKLOAD_SIZE);
        let mut exact = Vec::new();
        let mut prefix = Vec::new();
        for (surface, group_id) in sampled {
            if group_id % 2 == 0 {
                exact.push(surface.clone());
            } else {
                exact.push(format!("{surface}없는표면"));
            }
            prefix.push(format!("{surface}에서"));
        }
        Workload { exact, prefix }
    }

    pub fn report<'a>(&self, source_sha256: &'a str) -> BuildReport<'a> {
        let entries_by_pos = self
            .pos_counts
            .iter()
            .enumerate()
            .filter(|(_, count)| **count > 0)
            .map(|(code, count)| {
                (
                    DataFinePos::from_code(code as u8)
                        .expect("POS count index is valid")
                        .as_str(),
                    *count,
                )
            })
            .collect();
        BuildReport {
            schema_version: 1,
            source_sha256,
            rows_read: self.rows_read,
            skipped_unsupported_pos: self.skipped_unsupported_pos,
            duplicate_entries: self.duplicate_entries,
            surface_count: self.surface_count(),
            analysis_count: self.analysis_count(),
            entries_by_pos,
        }
    }
}

pub fn encode_payload(groups: &[Vec<MecabMorphologyEntry>]) -> Result<Vec<u8>> {
    let surface_count = u32::try_from(groups.len())?;
    let analysis_count = u32::try_from(groups.iter().map(Vec::len).sum::<usize>())?;
    let mut output = Vec::with_capacity(8 + (groups.len() + 1) * 4 + analysis_count as usize * 12);
    output.extend_from_slice(&surface_count.to_le_bytes());
    output.extend_from_slice(&analysis_count.to_le_bytes());
    let mut offset = 0_u32;
    output.extend_from_slice(&offset.to_le_bytes());
    for group in groups {
        offset = offset
            .checked_add(u32::try_from(group.len())?)
            .ok_or_else(|| {
                anyhow::anyhow!("analysis count overflow while encoding payload offsets")
            })?;
        output.extend_from_slice(&offset.to_le_bytes());
    }
    for group in groups {
        for entry in group {
            output.push(entry.pos.code());
            output.extend_from_slice(&[0; 3]);
            output.extend_from_slice(&entry.left_id.to_le_bytes());
            output.extend_from_slice(&entry.right_id.to_le_bytes());
            output.extend_from_slice(&entry.word_cost.to_le_bytes());
        }
    }
    Ok(output)
}

#[cfg(test)]
pub fn analysis(code: u8) -> MecabMorphologyEntry {
    MecabMorphologyEntry {
        surface: "가".to_owned(),
        pos: DataFinePos::from_code(code).unwrap(),
        left_id: 1,
        right_id: 2,
        word_cost: 3,
    }
}
