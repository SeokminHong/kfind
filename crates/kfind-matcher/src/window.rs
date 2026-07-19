use std::borrow::Cow;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;

use unicode_normalization::{UnicodeNormalization, is_nfc};

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
    normalized_to_raw: Option<Vec<(usize, usize)>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AnalysisWindowRef<'a> {
    raw_span: Range<usize>,
    normalized: Cow<'a, str>,
    normalized_to_raw: Option<Vec<(usize, usize)>>,
}

impl AnalysisWindow {
    pub fn extract(
        haystack: &[u8],
        target: Range<usize>,
        limits: AnalysisWindowLimits,
    ) -> Result<Self, AnalysisWindowError> {
        Ok(AnalysisWindowRef::extract(haystack, target, limits)?.into_owned())
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
        original_span(
            &self.raw_span,
            &self.normalized,
            self.normalized_to_raw.as_deref(),
            normalized,
        )
    }

    #[must_use]
    pub fn normalized_span(&self, original: Range<usize>) -> Option<Range<usize>> {
        normalized_span(
            &self.raw_span,
            &self.normalized,
            self.normalized_to_raw.as_deref(),
            original,
        )
    }
}

impl<'a> AnalysisWindowRef<'a> {
    pub(crate) fn extract(
        haystack: &'a [u8],
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
        let raw_is_nfc = is_nfc(raw);
        let normalized = if raw_is_nfc {
            Cow::Borrowed(raw)
        } else {
            Cow::Owned(raw.nfc().collect::<String>())
        };
        let scalar_count = normalized.chars().count();
        if scalar_count > limits.max_normalized_scalars {
            return Err(AnalysisWindowError::NormalizedScalars {
                actual: scalar_count,
                limit: limits.max_normalized_scalars,
            });
        }
        let normalized_to_raw = if raw_is_nfc {
            None
        } else {
            Some(stable_normalized_boundaries(raw, &normalized))
        };
        Ok(Self {
            raw_span,
            normalized,
            normalized_to_raw,
        })
    }

    pub(crate) fn raw_span(&self) -> Range<usize> {
        self.raw_span.clone()
    }

    pub(crate) fn normalized(&self) -> &str {
        &self.normalized
    }

    pub(crate) fn normalized_span(&self, original: Range<usize>) -> Option<Range<usize>> {
        normalized_span(
            &self.raw_span,
            &self.normalized,
            self.normalized_to_raw.as_deref(),
            original,
        )
    }

    fn into_owned(self) -> AnalysisWindow {
        AnalysisWindow {
            raw_span: self.raw_span,
            normalized: self.normalized.into_owned(),
            normalized_to_raw: self.normalized_to_raw,
        }
    }
}

fn original_span(
    raw_span: &Range<usize>,
    normalized_text: &str,
    boundaries: Option<&[(usize, usize)]>,
    normalized: Range<usize>,
) -> Option<Range<usize>> {
    if normalized.start > normalized.end || normalized.end > normalized_text.len() {
        return None;
    }
    let start = raw_boundary(normalized_text, boundaries, normalized.start)?;
    let end = raw_boundary(normalized_text, boundaries, normalized.end)?;
    Some(raw_span.start.checked_add(start)?..raw_span.start.checked_add(end)?)
}

fn normalized_span(
    raw_span: &Range<usize>,
    normalized_text: &str,
    boundaries: Option<&[(usize, usize)]>,
    original: Range<usize>,
) -> Option<Range<usize>> {
    if original.start < raw_span.start
        || original.start > original.end
        || original.end > raw_span.end
    {
        return None;
    }
    let relative_start = original.start.checked_sub(raw_span.start)?;
    let relative_end = original.end.checked_sub(raw_span.start)?;
    let start = normalized_boundary(normalized_text, boundaries, relative_start)?;
    let end = normalized_boundary(normalized_text, boundaries, relative_end)?;
    Some(start..end)
}

fn raw_boundary(
    normalized_text: &str,
    boundaries: Option<&[(usize, usize)]>,
    normalized: usize,
) -> Option<usize> {
    let Some(boundaries) = boundaries else {
        return normalized_text
            .is_char_boundary(normalized)
            .then_some(normalized);
    };
    boundaries
        .binary_search_by_key(&normalized, |(offset, _)| *offset)
        .ok()
        .map(|index| boundaries[index].1)
}

fn normalized_boundary(
    normalized_text: &str,
    boundaries: Option<&[(usize, usize)]>,
    raw: usize,
) -> Option<usize> {
    let Some(boundaries) = boundaries else {
        return normalized_text.is_char_boundary(raw).then_some(raw);
    };
    boundaries
        .binary_search_by_key(&raw, |(_, offset)| *offset)
        .ok()
        .map(|index| boundaries[index].0)
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
    fn internal_window_borrows_nfc_and_owns_only_normalized_text() {
        let nfc = "앞 매일 뒤";
        let nfc_start = nfc.find('매').unwrap();
        let nfc_window = AnalysisWindowRef::extract(
            nfc.as_bytes(),
            nfc_start..nfc_start + "매일".len(),
            DEFAULT_ANALYSIS_WINDOW_LIMITS,
        )
        .unwrap();
        assert!(matches!(nfc_window.normalized, Cow::Borrowed("매일")));
        assert_eq!(nfc_window.normalized_to_raw, None);

        let nfd = "앞 매일 뒤";
        let nfd_start = nfd.find('ᄆ').unwrap();
        let nfd_end = nfd.find(" 뒤").unwrap();
        let nfd_window = AnalysisWindowRef::extract(
            nfd.as_bytes(),
            nfd_start..nfd_end,
            DEFAULT_ANALYSIS_WINDOW_LIMITS,
        )
        .unwrap();
        assert!(matches!(nfd_window.normalized, Cow::Owned(ref text) if text == "매일"));
        assert!(nfd_window.normalized_to_raw.is_some());
    }

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
        assert_eq!(window.normalized_to_raw, None);
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
                normalized_to_raw: Some(normalized_to_raw),
            };

            for &(normalized, raw) in window.normalized_to_raw.as_ref().unwrap() {
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
