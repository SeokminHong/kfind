use kfind_morph::CoarsePos;

/// Default maximum number of Unicode scalars allowed between phrase atoms.
pub const DEFAULT_MAX_GAP: usize = 24;

/// A query after lexical parsing and option compatibility checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryAst {
    pub atoms: Vec<QueryAtom>,
    pub composition: QueryComposition,
    pub phrase: PhrasePolicy,
}

impl QueryAst {
    #[must_use]
    pub fn is_phrase(&self) -> bool {
        self.composition == QueryComposition::Phrase && self.atoms.len() > 1
    }
}

/// How independently parsed query atoms are combined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryComposition {
    Phrase,
    Disjunction,
}

/// One ordered query atom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryAtom {
    pub raw: Box<str>,
    pub forced_pos: Option<CoarsePos>,
    pub quoted_literal: bool,
}

/// Rules used when joining independently verified atom spans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhrasePolicy {
    /// Unicode scalar count between the previous token end and next token start.
    pub max_gap: usize,
}

impl Default for PhrasePolicy {
    fn default() -> Self {
        Self {
            max_gap: DEFAULT_MAX_GAP,
        }
    }
}
