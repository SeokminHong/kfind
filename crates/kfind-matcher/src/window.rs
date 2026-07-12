use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;

use unicode_normalization::UnicodeNormalization;

use crate::boundary::bounded_surrounding_token_span;

pub const DEFAULT_ANALYSIS_WINDOW_LIMITS: AnalysisWindowLimits = AnalysisWindowLimits {
    max_raw_bytes: 256,
    max_normalized_scalars: 64,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AnalysisWindowLimits {
    pub max_raw_bytes: usize,
    pub max_normalized_scalars: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AnalysisWindow {
    raw_span: Range<usize>,
    normalized: String,
    normalized_to_raw: Vec<(usize, usize)>,
}

impl AnalysisWindow {
    pub fn extract(
        haystack: &[u8],
        target: Range<usize>,
        limits: AnalysisWindowLimits,
    ) -> Result<Self, AnalysisWindowError> {
        if target.start >= target.end || target.end > haystack.len() {
            return Err(AnalysisWindowError::InvalidTarget);
        }
        std::str::from_utf8(&haystack[target.clone()])
            .map_err(|_| AnalysisWindowError::InvalidUtf8)?;
        let raw_span = bounded_surrounding_token_span(haystack, target, limits.max_raw_bytes)
            .map_err(|minimum| AnalysisWindowError::RawBytes {
                minimum,
                limit: limits.max_raw_bytes,
            })?;
        let raw = std::str::from_utf8(&haystack[raw_span.clone()])
            .map_err(|_| AnalysisWindowError::InvalidUtf8)?;
        let normalized = raw.nfc().collect::<String>();
        let scalar_count = normalized.chars().count();
        if scalar_count > limits.max_normalized_scalars {
            return Err(AnalysisWindowError::NormalizedScalars {
                actual: scalar_count,
                limit: limits.max_normalized_scalars,
            });
        }
        let normalized_to_raw = stable_normalized_boundaries(raw, &normalized);
        Ok(Self {
            raw_span,
            normalized,
            normalized_to_raw,
        })
    }

    #[must_use]
    pub fn raw_span(&self) -> Range<usize> {
        self.raw_span.clone()
    }

    #[must_use]
    pub fn normalized(&self) -> &str {
        &self.normalized
    }

    #[must_use]
    pub fn original_span(&self, normalized: Range<usize>) -> Option<Range<usize>> {
        if normalized.start > normalized.end || normalized.end > self.normalized.len() {
            return None;
        }
        let start = self.raw_boundary(normalized.start)?;
        let end = self.raw_boundary(normalized.end)?;
        Some(self.raw_span.start.checked_add(start)?..self.raw_span.start.checked_add(end)?)
    }

    #[must_use]
    pub fn normalized_span(&self, original: Range<usize>) -> Option<Range<usize>> {
        if original.start < self.raw_span.start
            || original.start > original.end
            || original.end > self.raw_span.end
        {
            return None;
        }
        let relative_start = original.start.checked_sub(self.raw_span.start)?;
        let relative_end = original.end.checked_sub(self.raw_span.start)?;
        let start = self.normalized_boundary(relative_start)?;
        let end = self.normalized_boundary(relative_end)?;
        Some(start..end)
    }

    fn raw_boundary(&self, normalized: usize) -> Option<usize> {
        self.normalized_to_raw
            .binary_search_by_key(&normalized, |(offset, _)| *offset)
            .ok()
            .map(|index| self.normalized_to_raw[index].1)
    }

    fn normalized_boundary(&self, raw: usize) -> Option<usize> {
        self.normalized_to_raw
            .binary_search_by_key(&raw, |(_, offset)| *offset)
            .ok()
            .map(|index| self.normalized_to_raw[index].0)
    }
}

fn stable_normalized_boundaries(raw: &str, normalized: &str) -> Vec<(usize, usize)> {
    let mut boundaries = Vec::with_capacity(raw.chars().count().saturating_add(1));
    boundaries.push((0, 0));
    for raw_end in raw
        .char_indices()
        .map(|(offset, _)| offset)
        .skip(1)
        .chain(std::iter::once(raw.len()))
    {
        let prefix = raw[..raw_end].nfc().collect::<String>();
        if normalized.starts_with(&prefix) {
            boundaries.push((prefix.len(), raw_end));
        }
    }
    boundaries
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnalysisWindowError {
    InvalidTarget,
    InvalidUtf8,
    RawBytes { minimum: usize, limit: usize },
    NormalizedScalars { actual: usize, limit: usize },
}

impl Display for AnalysisWindowError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTarget => {
                formatter.write_str("analysis target is empty or outside the input")
            }
            Self::InvalidUtf8 => formatter.write_str("analysis window is not valid UTF-8"),
            Self::RawBytes { minimum, limit } => {
                write!(
                    formatter,
                    "analysis window has at least {minimum} bytes; limit is {limit}"
                )
            }
            Self::NormalizedScalars { actual, limit } => write!(
                formatter,
                "analysis window has {actual} normalized scalars; limit is {limit}"
            ),
        }
    }
}

impl Error for AnalysisWindowError {}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn extracts_the_surrounding_eojeol_and_maps_nfc_offsets() {
        let text = "앞 학생일 뒤";
        let target_start = text.find('일').unwrap();
        let window = AnalysisWindow::extract(
            text.as_bytes(),
            target_start..target_start + '일'.len_utf8(),
            DEFAULT_ANALYSIS_WINDOW_LIMITS,
        )
        .unwrap();

        assert_eq!(&text[window.raw_span()], "학생일");
        assert_eq!(window.normalized(), "학생일");
        assert_eq!(
            window.original_span("학생".len().."학생일".len()),
            Some(target_start..target_start + '일'.len_utf8())
        );
    }

    #[test]
    fn maps_decomposed_hangul_only_at_stable_normalized_boundaries() {
        let text = "앞 매일 뒤";
        let raw_start = text.find('ᄆ').unwrap();
        let raw_end = text.find(" 뒤").unwrap();
        let target_start = text.find('ᄋ').unwrap();
        let window = AnalysisWindow::extract(
            text.as_bytes(),
            target_start..raw_end,
            DEFAULT_ANALYSIS_WINDOW_LIMITS,
        )
        .unwrap();

        assert_eq!(window.normalized(), "매일");
        assert_eq!(
            window.original_span(0.."매일".len()),
            Some(raw_start..raw_end)
        );
        assert_eq!(
            window.original_span("매".len().."매일".len()),
            Some(target_start..raw_end)
        );
        assert_eq!(
            window.normalized_span(target_start..raw_end),
            Some("매".len().."매일".len())
        );
        assert_eq!(window.original_span(0..1), None);
    }

    #[test]
    fn rejects_windows_that_exceed_either_limit() {
        let text = "가나다";
        let target = 0..'가'.len_utf8();
        assert!(matches!(
            AnalysisWindow::extract(
                text.as_bytes(),
                target.clone(),
                AnalysisWindowLimits {
                    max_raw_bytes: 8,
                    max_normalized_scalars: 3,
                },
            ),
            Err(AnalysisWindowError::RawBytes { .. })
        ));
        assert!(matches!(
            AnalysisWindow::extract(
                text.as_bytes(),
                target,
                AnalysisWindowLimits {
                    max_raw_bytes: text.len(),
                    max_normalized_scalars: 2,
                },
            ),
            Err(AnalysisWindowError::NormalizedScalars { .. })
        ));
    }

    #[test]
    fn rejects_empty_and_non_utf8_targets_before_expansion() {
        assert_eq!(
            AnalysisWindow::extract(b"abc", 1..1, DEFAULT_ANALYSIS_WINDOW_LIMITS),
            Err(AnalysisWindowError::InvalidTarget)
        );
        assert_eq!(
            AnalysisWindow::extract(&[0xff], 0..1, DEFAULT_ANALYSIS_WINDOW_LIMITS),
            Err(AnalysisWindowError::InvalidUtf8)
        );
    }

    proptest! {
        #[test]
        fn stable_nfc_boundaries_round_trip(
            characters in prop::collection::vec(any::<char>(), 0..64)
        ) {
            let raw = characters.into_iter().collect::<String>();
            let normalized = raw.nfc().collect::<String>();
            let normalized_to_raw = stable_normalized_boundaries(&raw, &normalized);
            let raw_start = 11;
            let window = AnalysisWindow {
                raw_span: raw_start..raw_start + raw.len(),
                normalized,
                normalized_to_raw,
            };

            for &(normalized, raw) in &window.normalized_to_raw {
                prop_assert_eq!(
                    window.original_span(normalized..normalized),
                    Some(raw_start + raw..raw_start + raw)
                );
                prop_assert_eq!(
                    window.normalized_span(raw_start + raw..raw_start + raw),
                    Some(normalized..normalized)
                );
            }
        }
    }
}
