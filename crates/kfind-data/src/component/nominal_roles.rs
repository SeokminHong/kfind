use std::collections::BTreeMap;

use unicode_normalization::is_nfc;

use super::payload::{StringLayout, encode_strings};
use super::{DataError, build_error, resource_error};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum NominalRole {
    Spatial = 1,
    Person = 2,
    Scheduled = 4,
    Temporal = 8,
    DoObject = 16,
}

const KNOWN_ROLES: u8 = (NominalRole::Spatial as u8)
    | (NominalRole::Person as u8)
    | (NominalRole::Scheduled as u8)
    | (NominalRole::Temporal as u8)
    | (NominalRole::DoObject as u8);
const MASK_OFFSET: u8 = b'A';

pub(super) fn encode(roles: &[(&str, NominalRole)]) -> Result<Vec<u8>, DataError> {
    let mut grouped = BTreeMap::new();
    for &(noun, role) in roles {
        if noun.is_empty() || noun.contains('\t') || !is_nfc(noun) {
            return Err(build_error("invalid nominal role surface"));
        }
        *grouped.entry(noun).or_insert(0_u8) |= role as u8;
    }
    let records: Vec<_> = grouped
        .into_iter()
        .map(|(noun, mask)| format!("{noun}\t{}", char::from(MASK_OFFSET + mask)))
        .collect();
    encode_strings(records.iter())
}

#[derive(Debug)]
pub(super) struct NominalRoleLayout(StringLayout);

impl NominalRoleLayout {
    pub fn parse(source: &str, bytes: &[u8]) -> Result<Self, DataError> {
        let layout = StringLayout::parse(source, bytes)?;
        let mut previous = "";
        for id in 0..layout.count {
            let (noun, _) = layout
                .get(bytes, id)
                .and_then(parse_record)
                .filter(|(noun, _)| *noun > previous && is_nfc(noun))
                .ok_or_else(|| resource_error(source, "invalid nominal role record"))?;
            previous = noun;
        }
        Ok(Self(layout))
    }

    pub fn contains(&self, bytes: &[u8], noun: &str, role: NominalRole) -> bool {
        let mut left = 0;
        let mut right = self.0.count;
        while left < right {
            let middle = left + (right - left) / 2;
            let Some((entry, mask)) = self.0.get(bytes, middle).and_then(parse_record) else {
                return false;
            };
            match entry.cmp(noun) {
                std::cmp::Ordering::Less => left = middle + 1,
                std::cmp::Ordering::Greater => right = middle,
                std::cmp::Ordering::Equal => return mask & role as u8 != 0,
            }
        }
        false
    }
}

fn parse_record(record: &str) -> Option<(&str, u8)> {
    let (noun, encoded) = record.split_once('\t')?;
    let &[byte] = encoded.as_bytes() else {
        return None;
    };
    let mask = byte.checked_sub(MASK_OFFSET)?;
    (!noun.is_empty() && mask != 0 && mask & !KNOWN_ROLES == 0).then_some((noun, mask))
}
