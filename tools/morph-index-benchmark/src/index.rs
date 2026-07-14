use anyhow::{Context, Result, ensure};
use fst::Map;
use fst::raw::Output;
use yada::DoubleArray;
use yada::builder::DoubleArrayBuilder;

use crate::artifact::IndexKind;

pub fn build(kind: IndexKind, keys: &[(String, u32)]) -> Result<Vec<u8>> {
    ensure!(!keys.is_empty(), "cannot build an empty morphology index");
    match kind {
        IndexKind::DoubleArray => {
            let keyset = keys
                .iter()
                .map(|(key, value)| (key.as_bytes(), *value))
                .collect::<Vec<_>>();
            DoubleArrayBuilder::build(&keyset)
                .ok_or_else(|| anyhow::anyhow!("failed to build packed Double-Array trie"))
        }
        IndexKind::Fst => {
            let map = Map::from_iter(
                keys.iter()
                    .map(|(key, value)| (key.as_bytes(), u64::from(*value))),
            )
            .context("failed to build FST map")?;
            Ok(map.into_fst().into_inner())
        }
    }
}

pub fn validate(kind: IndexKind, bytes: &[u8], surface_count: u32) -> Result<()> {
    ensure!(
        !bytes.is_empty(),
        "artifact corruption: empty index section"
    );
    match kind {
        IndexKind::DoubleArray => {
            ensure!(
                bytes.len().is_multiple_of(4),
                "artifact corruption: invalid Double-Array length"
            );
        }
        IndexKind::Fst => {
            let map = Map::new(bytes).context("artifact corruption: invalid FST header")?;
            map.as_fst()
                .verify()
                .context("artifact corruption: invalid FST structure")?;
            ensure!(
                map.len() == usize::try_from(surface_count)?,
                "artifact corruption: FST key count mismatch"
            );
        }
    }
    Ok(())
}

pub enum IndexView<'a> {
    DoubleArray(DoubleArray<&'a [u8]>),
    Fst(Map<&'a [u8]>),
}

impl<'a> IndexView<'a> {
    pub fn new(kind: IndexKind, bytes: &'a [u8]) -> Result<Self> {
        Ok(match kind {
            IndexKind::DoubleArray => Self::DoubleArray(DoubleArray::new(bytes)),
            IndexKind::Fst => Self::Fst(Map::new(bytes)?),
        })
    }

    pub fn exact(&self, key: &[u8]) -> Option<u32> {
        match self {
            Self::DoubleArray(index) => index.exact_match_search(key),
            Self::Fst(index) => index.get(key).and_then(|value| u32::try_from(value).ok()),
        }
    }

    pub fn common_prefixes(&self, input: &[u8], mut emit: impl FnMut(u32, usize)) {
        match self {
            Self::DoubleArray(index) => {
                for (value, length) in index.common_prefix_search(input) {
                    emit(value, length);
                }
            }
            Self::Fst(index) => fst_common_prefixes(index, input, emit),
        }
    }
}

fn fst_common_prefixes(index: &Map<&[u8]>, input: &[u8], mut emit: impl FnMut(u32, usize)) {
    let fst = index.as_fst();
    let mut node = fst.root();
    let mut output = Output::zero();
    for (offset, byte) in input.iter().copied().enumerate() {
        let Some(transition_index) = node.find_input(byte) else {
            break;
        };
        let transition = node.transition(transition_index);
        output = output.cat(transition.out);
        node = fst.node(transition.addr);
        if node.is_final() {
            let value = output.cat(node.final_output()).value();
            if let Ok(value) = u32::try_from(value) {
                emit(value, offset + 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidates_return_identical_exact_and_prefix_results() {
        let keys = vec![
            ("가".to_owned(), 0),
            ("가다".to_owned(), 1),
            ("가방".to_owned(), 2),
        ];
        for kind in [IndexKind::DoubleArray, IndexKind::Fst] {
            let bytes = build(kind, &keys).unwrap();
            validate(kind, &bytes, 3).unwrap();
            let index = IndexView::new(kind, &bytes).unwrap();
            assert_eq!(index.exact("가다".as_bytes()), Some(1));
            assert_eq!(index.exact("없다".as_bytes()), None);
            let mut prefixes = Vec::new();
            index.common_prefixes("가다가".as_bytes(), |value, length| {
                prefixes.push((value, length));
            });
            assert_eq!(prefixes, vec![(0, "가".len()), (1, "가다".len())]);
        }
    }
}
