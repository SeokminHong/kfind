use std::collections::BTreeMap;

use unicode_normalization::is_nfc;

use super::payload::{StringLayout, encode_strings};
use super::{DataError, build_error, resource_error};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum NominalRole {
    Spatial = 1,
    Person = 2,
    Scheduled = 4,
    Temporal = 8,
    DoObject = 16,
    DisplaySurface = 32,
    HouseholdArticle = 64,
    Food = 128,
    BodySecretion = 256,
    OrthographicUnit = 512,
    Emptyable = 1024,
    ContentUnit = 2048,
    AudiblePhenomenon = 4096,
    OwnershipAsset = 8192,
    Animal = 16384,
}

const KNOWN_ROLES: u16 = (NominalRole::Spatial as u16)
    | (NominalRole::Person as u16)
    | (NominalRole::Scheduled as u16)
    | (NominalRole::Temporal as u16)
    | (NominalRole::DoObject as u16)
    | (NominalRole::DisplaySurface as u16)
    | (NominalRole::HouseholdArticle as u16)
    | (NominalRole::Food as u16)
    | (NominalRole::BodySecretion as u16)
    | (NominalRole::OrthographicUnit as u16)
    | (NominalRole::Emptyable as u16)
    | (NominalRole::ContentUnit as u16)
    | (NominalRole::AudiblePhenomenon as u16)
    | (NominalRole::OwnershipAsset as u16)
    | (NominalRole::Animal as u16);
const MASK_OFFSET: u32 = b'A' as u32;

pub(super) fn encode(roles: &[(&str, NominalRole)]) -> Result<Vec<u8>, DataError> {
    let mut grouped = BTreeMap::new();
    for &(noun, role) in roles {
        if noun.is_empty() || noun.contains('\t') || !is_nfc(noun) {
            return Err(build_error("invalid nominal role surface"));
        }
        *grouped.entry(noun).or_insert(0_u16) |= role as u16;
    }
    let records: Vec<_> = grouped
        .into_iter()
        .map(|(noun, mask)| {
            let encoded = char::from_u32(MASK_OFFSET + u32::from(mask))
                .expect("nominal role mask fits one scalar");
            format!("{noun}\t{encoded}")
        })
        .collect();
    encode_strings(records.iter())
}

#[derive(Debug)]
pub(super) struct NominalRoleLayout(StringLayout);

impl NominalRoleLayout {
    pub fn parse(source: &str, bytes: &[u8], legacy: bool) -> Result<Self, DataError> {
        let layout = StringLayout::parse(source, bytes)?;
        let mut previous = "";
        for id in 0..layout.count {
            let (noun, _) = layout
                .get(bytes, id)
                .and_then(parse_record)
                .filter(|(noun, mask)| *noun > previous && is_nfc(noun) && (!legacy || *mask < 32))
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
                std::cmp::Ordering::Equal => return mask & role as u16 != 0,
            }
        }
        false
    }
}

fn parse_record(record: &str) -> Option<(&str, u16)> {
    let (noun, encoded) = record.split_once('\t')?;
    let mut scalars = encoded.chars();
    let scalar = u32::from(scalars.next()?);
    if scalars.next().is_some() {
        return None;
    }
    let mask = u16::try_from(scalar.checked_sub(MASK_OFFSET)?).ok()?;
    (!noun.is_empty() && mask != 0 && mask & !KNOWN_ROLES == 0).then_some((noun, mask))
}
