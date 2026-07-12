use std::collections::{BTreeMap, BTreeSet};

use kfind_data::DecodedMorphologyResource;

use super::LocalLatticeError;

#[derive(Clone, Debug)]
pub(super) struct UnknownAnalysis {
    pub pos: String,
    pub left_id: u16,
    pub right_id: u16,
    pub word_cost: i32,
}

#[derive(Clone, Debug)]
struct CharacterClass {
    invoke: bool,
    group: bool,
    length: usize,
    analyses: Vec<UnknownAnalysis>,
}

#[derive(Clone, Debug)]
struct CharacterRange {
    start: char,
    end: char,
    classes: Vec<String>,
}

#[derive(Clone, Debug)]
pub(super) struct UnknownDictionary {
    classes: BTreeMap<String, CharacterClass>,
    ranges: Vec<CharacterRange>,
}

impl UnknownDictionary {
    pub fn parse(resource: &DecodedMorphologyResource<'_>) -> Result<Self, LocalLatticeError> {
        let char_def = std::str::from_utf8(resource.char_def())
            .map_err(|_| LocalLatticeError::InvalidUnknownModel)?;
        let mut classes = BTreeMap::new();
        let mut ranges = Vec::new();
        for line in char_def
            .lines()
            .map(strip_comment)
            .filter(|line| !line.is_empty())
        {
            let fields = line.split_whitespace().collect::<Vec<_>>();
            if fields[0].starts_with("0x") {
                if fields.len() < 2 {
                    return Err(LocalLatticeError::InvalidUnknownModel);
                }
                let (start, end) = parse_range(fields[0])?;
                ranges.push(CharacterRange {
                    start,
                    end,
                    classes: fields[1..]
                        .iter()
                        .map(|value| (*value).to_owned())
                        .collect(),
                });
            } else {
                if fields.len() != 4 || classes.contains_key(fields[0]) {
                    return Err(LocalLatticeError::InvalidUnknownModel);
                }
                classes.insert(
                    fields[0].to_owned(),
                    CharacterClass {
                        invoke: parse_flag(fields[1])?,
                        group: parse_flag(fields[2])?,
                        length: parse_field(fields[3])?,
                        analyses: Vec::new(),
                    },
                );
            }
        }
        if !classes.contains_key("DEFAULT")
            || ranges
                .iter()
                .flat_map(|range| &range.classes)
                .any(|name| !classes.contains_key(name))
        {
            return Err(LocalLatticeError::InvalidUnknownModel);
        }

        let unk_def = std::str::from_utf8(resource.unk_def())
            .map_err(|_| LocalLatticeError::InvalidUnknownModel)?;
        for line in unk_def
            .lines()
            .map(strip_comment)
            .filter(|line| !line.is_empty())
        {
            let fields = line.split(',').collect::<Vec<_>>();
            if fields.len() < 5 {
                return Err(LocalLatticeError::InvalidUnknownModel);
            }
            let analysis = UnknownAnalysis {
                left_id: parse_field(fields[1])?,
                right_id: parse_field(fields[2])?,
                word_cost: parse_field(fields[3])?,
                pos: fields[4].to_owned(),
            };
            if analysis.left_id >= resource.stats().left_contexts
                || analysis.right_id >= resource.stats().right_contexts
            {
                return Err(LocalLatticeError::InvalidUnknownModel);
            }
            classes
                .get_mut(fields[0])
                .ok_or(LocalLatticeError::InvalidUnknownModel)?
                .analyses
                .push(analysis);
        }
        if classes.values().any(|class| class.analyses.is_empty()) {
            return Err(LocalLatticeError::InvalidUnknownModel);
        }
        Ok(Self { classes, ranges })
    }

    pub fn nodes_at(
        &self,
        text: &str,
        start: usize,
        has_dictionary: bool,
    ) -> Vec<(usize, &UnknownAnalysis)> {
        let Some(character) = text[start..].chars().next() else {
            return Vec::new();
        };
        let mut output = Vec::new();
        for name in self.classes_for(character) {
            let class = &self.classes[name];
            if has_dictionary && !class.invoke {
                continue;
            }
            let mut ends = text[start..]
                .char_indices()
                .take_while(|(_, character)| self.classes_for(*character).contains(&name))
                .map(|(offset, character)| start + offset + character.len_utf8())
                .collect::<Vec<_>>();
            let mut selected = BTreeSet::new();
            selected.extend(ends.iter().copied().take(class.length));
            if class.group {
                if let Some(end) = ends.pop() {
                    selected.insert(end);
                }
            }
            for end in selected {
                for analysis in &class.analyses {
                    output.push((end, analysis));
                }
            }
        }
        output
    }

    fn classes_for(&self, character: char) -> Vec<&str> {
        let matches = self
            .ranges
            .iter()
            .filter(|range| range.start <= character && character <= range.end)
            .flat_map(|range| range.classes.iter().map(String::as_str))
            .collect::<BTreeSet<_>>();
        if matches.is_empty() {
            vec!["DEFAULT"]
        } else {
            matches.into_iter().collect()
        }
    }
}

fn parse_range(value: &str) -> Result<(char, char), LocalLatticeError> {
    let (start, end) = value
        .split_once("..")
        .map_or((value, value), |(start, end)| (start, end));
    let start = parse_character(start)?;
    let end = parse_character(end)?;
    if start > end {
        return Err(LocalLatticeError::InvalidUnknownModel);
    }
    Ok((start, end))
}

fn parse_character(value: &str) -> Result<char, LocalLatticeError> {
    u32::from_str_radix(
        value
            .strip_prefix("0x")
            .ok_or(LocalLatticeError::InvalidUnknownModel)?,
        16,
    )
    .ok()
    .and_then(char::from_u32)
    .ok_or(LocalLatticeError::InvalidUnknownModel)
}

fn parse_flag(value: &str) -> Result<bool, LocalLatticeError> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(LocalLatticeError::InvalidUnknownModel),
    }
}

fn parse_field<T: std::str::FromStr>(value: &str) -> Result<T, LocalLatticeError> {
    value
        .parse()
        .map_err(|_| LocalLatticeError::InvalidUnknownModel)
}

fn strip_comment(line: &str) -> &str {
    line.split_once('#')
        .map_or(line, |(content, _)| content)
        .trim()
}
