use std::{collections::HashMap, error::Error, fmt, ops::Range};

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, FindOverlappingIter, MatchKind};
use memchr::memmem::Finder;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnchorHit {
    pub anchor_index: usize,
    pub span: Range<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnchorBuildLimits {
    pub max_anchors: usize,
    pub max_memory_bytes: usize,
}

impl Default for AnchorBuildLimits {
    fn default() -> Self {
        Self {
            max_anchors: 4_096,
            max_memory_bytes: 64 * 1024 * 1024,
        }
    }
}

#[derive(Debug)]
pub enum AnchorEngine {
    One {
        finder: Box<Finder<'static>>,
        anchor_len: usize,
    },
    Many(AhoCorasick),
}

impl AnchorEngine {
    pub fn new(anchors: &[Box<[u8]>]) -> Result<Self, AnchorBuildError> {
        Self::new_with_limits(anchors, AnchorBuildLimits::default())
    }

    pub fn new_with_limits(
        anchors: &[Box<[u8]>],
        limits: AnchorBuildLimits,
    ) -> Result<Self, AnchorBuildError> {
        validate_anchors(anchors, limits.max_anchors)?;

        if anchors.len() == 1 {
            let engine = Self::One {
                finder: Box::new(Finder::new(&anchors[0]).into_owned()),
                anchor_len: anchors[0].len(),
            };
            return enforce_memory_limit(engine, limits.max_memory_bytes);
        }

        let matcher = AhoCorasickBuilder::new()
            .match_kind(MatchKind::Standard)
            .build(anchors)
            .map_err(|error| AnchorBuildError::Build(error.to_string()))?;
        enforce_memory_limit(Self::Many(matcher), limits.max_memory_bytes)
    }

    pub fn hits<'engine, 'haystack>(
        &'engine self,
        haystack: &'haystack [u8],
        at: usize,
    ) -> AnchorHits<'engine, 'haystack> {
        if at > haystack.len() {
            return AnchorHits::Empty;
        }

        match self {
            Self::One { finder, anchor_len } => AnchorHits::One {
                finder,
                haystack,
                cursor: at,
                anchor_len: *anchor_len,
            },
            Self::Many(matcher) => AnchorHits::Many {
                offset: at,
                matches: matcher.find_overlapping_iter(&haystack[at..]),
            },
        }
    }

    #[must_use]
    pub fn memory_usage(&self) -> usize {
        match self {
            Self::One { anchor_len, .. } => *anchor_len,
            Self::Many(matcher) => matcher.memory_usage(),
        }
    }

    #[cfg(test)]
    fn find_overlapping(&self, haystack: &[u8], at: usize) -> Vec<AnchorHit> {
        self.hits(haystack, at).collect()
    }
}

pub enum AnchorHits<'engine, 'haystack> {
    Empty,
    One {
        finder: &'engine Finder<'static>,
        haystack: &'haystack [u8],
        cursor: usize,
        anchor_len: usize,
    },
    Many {
        offset: usize,
        matches: FindOverlappingIter<'engine, 'haystack>,
    },
}

impl Iterator for AnchorHits<'_, '_> {
    type Item = AnchorHit;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty => None,
            Self::One {
                finder,
                haystack,
                cursor,
                anchor_len,
            } => {
                let relative_start = finder.find(&haystack[*cursor..])?;
                let start = *cursor + relative_start;
                *cursor = start + 1;
                Some(AnchorHit {
                    anchor_index: 0,
                    span: start..start + *anchor_len,
                })
            }
            Self::Many { offset, matches } => matches.next().map(|matched| AnchorHit {
                anchor_index: matched.pattern().as_usize(),
                span: *offset + matched.start()..*offset + matched.end(),
            }),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum AnchorBuildError {
    EmptySet,
    EmptyAnchor(usize),
    TooManyAnchors { actual: usize, limit: usize },
    MemoryLimit { estimated: usize, limit: usize },
    DuplicateAnchor { first: usize, duplicate: usize },
    Build(String),
}

impl fmt::Display for AnchorBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptySet => formatter.write_str("an anchor engine requires at least one anchor"),
            Self::EmptyAnchor(index) => write!(formatter, "anchor {index} is empty"),
            Self::TooManyAnchors { actual, limit } => {
                write!(formatter, "anchor count {actual} exceeds limit {limit}")
            }
            Self::MemoryLimit { estimated, limit } => write!(
                formatter,
                "anchor matcher requires approximately {estimated} bytes; limit is {limit}"
            ),
            Self::DuplicateAnchor { first, duplicate } => write!(
                formatter,
                "anchor {duplicate} duplicates anchor {first}; merge branch origins before matching"
            ),
            Self::Build(message) => write!(formatter, "failed to build anchor matcher: {message}"),
        }
    }
}

impl Error for AnchorBuildError {}

fn validate_anchors(anchors: &[Box<[u8]>], max_anchors: usize) -> Result<(), AnchorBuildError> {
    if anchors.is_empty() {
        return Err(AnchorBuildError::EmptySet);
    }
    if anchors.len() > max_anchors {
        return Err(AnchorBuildError::TooManyAnchors {
            actual: anchors.len(),
            limit: max_anchors,
        });
    }

    let mut seen = HashMap::<&[u8], usize>::with_capacity(anchors.len());
    for (index, anchor) in anchors.iter().enumerate() {
        if anchor.is_empty() {
            return Err(AnchorBuildError::EmptyAnchor(index));
        }
        if let Some(first) = seen.insert(anchor, index) {
            return Err(AnchorBuildError::DuplicateAnchor {
                first,
                duplicate: index,
            });
        }
    }
    Ok(())
}

fn enforce_memory_limit(
    engine: AnchorEngine,
    max_memory_bytes: usize,
) -> Result<AnchorEngine, AnchorBuildError> {
    let estimated = engine.memory_usage();
    if estimated > max_memory_bytes {
        return Err(AnchorBuildError::MemoryLimit {
            estimated,
            limit: max_memory_bytes,
        });
    }
    Ok(engine)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchors(values: &[&str]) -> Vec<Box<[u8]>> {
        values.iter().map(|value| value.as_bytes().into()).collect()
    }

    #[test]
    fn one_anchor_reports_overlapping_hits() {
        let matcher = AnchorEngine::new(&anchors(&["aba"])).unwrap();

        assert_eq!(
            matcher.find_overlapping(b"ababa", 0),
            [
                AnchorHit {
                    anchor_index: 0,
                    span: 0..3,
                },
                AnchorHit {
                    anchor_index: 0,
                    span: 2..5,
                },
            ]
        );
    }

    #[test]
    fn many_anchors_preserve_pattern_indices() {
        let matcher = AnchorEngine::new(&anchors(&["걸어", "걸었"])).unwrap();
        let haystack = "걸어서 걸었다".as_bytes();

        assert_eq!(
            matcher.find_overlapping(haystack, 0),
            [
                AnchorHit {
                    anchor_index: 0,
                    span: 0..6,
                },
                AnchorHit {
                    anchor_index: 1,
                    span: 10..16,
                },
            ]
        );
    }

    #[test]
    fn search_start_is_an_absolute_byte_offset() {
        let matcher = AnchorEngine::new(&anchors(&["가"])).unwrap();

        assert_eq!(
            matcher.find_overlapping("가 가".as_bytes(), 3),
            [AnchorHit {
                anchor_index: 0,
                span: 4..7,
            }]
        );
    }

    #[test]
    fn duplicate_anchors_are_rejected() {
        let error = AnchorEngine::new(&anchors(&["걷", "걷"])).unwrap_err();

        assert_eq!(
            error,
            AnchorBuildError::DuplicateAnchor {
                first: 0,
                duplicate: 1,
            }
        );
    }

    #[test]
    fn limits_are_checked_without_truncating() {
        let values = anchors(&["걷", "걸"]);
        let count_error = AnchorEngine::new_with_limits(
            &values,
            AnchorBuildLimits {
                max_anchors: 1,
                max_memory_bytes: usize::MAX,
            },
        )
        .unwrap_err();
        assert_eq!(
            count_error,
            AnchorBuildError::TooManyAnchors {
                actual: 2,
                limit: 1,
            }
        );

        let memory_error = AnchorEngine::new_with_limits(
            &anchors(&["걷"]),
            AnchorBuildLimits {
                max_anchors: 1,
                max_memory_bytes: 1,
            },
        )
        .unwrap_err();
        assert_eq!(
            memory_error,
            AnchorBuildError::MemoryLimit {
                estimated: "걷".len(),
                limit: 1,
            }
        );
    }
}
