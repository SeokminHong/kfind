use std::ops::Range;

use kfind_query::BoundaryPolicy;
use unicode_general_category::{GeneralCategory, get_general_category};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BoundaryVerifier {
    pub policy: BoundaryPolicy,
    pub require_smart_left: bool,
}

impl BoundaryVerifier {
    #[must_use]
    pub fn accepts(self, haystack: &[u8], core: Range<usize>, token: Range<usize>) -> bool {
        let require_left = self.policy == BoundaryPolicy::Token || self.require_smart_left;
        accepts_requirements(
            haystack,
            core,
            token,
            require_left,
            self.policy != BoundaryPolicy::Any,
        )
    }
}

pub(crate) fn accepts_requirements(
    haystack: &[u8],
    core: Range<usize>,
    token: Range<usize>,
    require_left: bool,
    require_right: bool,
) -> bool {
    valid_ranges(haystack.len(), &core, &token)
        && (!require_left || is_boundary_before(haystack, token.start))
        && (!require_right || is_boundary_after(haystack, token.end))
}

#[must_use]
pub fn is_token_character(character: char) -> bool {
    character == '_'
        || character.is_ascii_alphanumeric()
        || matches!(character, '\u{ac00}'..='\u{d7a3}')
        || (!character.is_ascii() && character.is_alphanumeric())
        || (!character.is_ascii()
            && matches!(
                get_general_category(character),
                GeneralCategory::NonspacingMark
                    | GeneralCategory::SpacingMark
                    | GeneralCategory::EnclosingMark
            ))
}

fn valid_ranges(length: usize, core: &Range<usize>, token: &Range<usize>) -> bool {
    token.start <= core.start
        && core.start <= core.end
        && core.end <= token.end
        && token.start <= token.end
        && token.end <= length
}

fn is_boundary_before(haystack: &[u8], at: usize) -> bool {
    previous_character(haystack, at).is_none_or(|character| !is_token_character(character))
}

fn is_boundary_after(haystack: &[u8], at: usize) -> bool {
    next_character(haystack, at).is_none_or(|character| !is_token_character(character))
}

pub(crate) fn surrounding_token_span(haystack: &[u8], span: Range<usize>) -> Range<usize> {
    let mut start = span.start;
    while let Some((previous_start, character)) = previous_character_with_start(haystack, start) {
        if !is_token_character(character) {
            break;
        }
        start = previous_start;
    }

    let mut end = span.end;
    while let Some((next_end, character)) = next_character_with_end(haystack, end) {
        if !is_token_character(character) {
            break;
        }
        end = next_end;
    }
    start..end
}

pub(crate) fn bounded_surrounding_token_span(
    haystack: &[u8],
    span: Range<usize>,
    max_bytes: usize,
) -> Result<Range<usize>, usize> {
    if span.len() > max_bytes {
        return Err(span.len());
    }
    let mut start = span.start;
    while let Some((previous_start, character)) = previous_character_with_start(haystack, start) {
        if !is_token_character(character) {
            break;
        }
        let minimum = span.end - previous_start;
        if minimum > max_bytes {
            return Err(minimum);
        }
        start = previous_start;
    }

    let mut end = span.end;
    while let Some((next_end, character)) = next_character_with_end(haystack, end) {
        if !is_token_character(character) {
            break;
        }
        let minimum = next_end - start;
        if minimum > max_bytes {
            return Err(minimum);
        }
        end = next_end;
    }
    Ok(start..end)
}

fn previous_character(haystack: &[u8], at: usize) -> Option<char> {
    previous_character_with_start(haystack, at).map(|(_, character)| character)
}

fn previous_character_with_start(haystack: &[u8], at: usize) -> Option<(usize, char)> {
    if at == 0 || at > haystack.len() {
        return None;
    }
    let mut start = at - 1;
    while start > at.saturating_sub(4) && is_utf8_continuation(haystack[start]) {
        start -= 1;
    }
    let text = std::str::from_utf8(haystack.get(start..at)?).ok()?;
    let mut characters = text.chars();
    let character = characters.next()?;
    characters.next().is_none().then_some((start, character))
}

fn next_character(haystack: &[u8], at: usize) -> Option<char> {
    next_character_with_end(haystack, at).map(|(_, character)| character)
}

fn next_character_with_end(haystack: &[u8], at: usize) -> Option<(usize, char)> {
    let width = utf8_width(*haystack.get(at)?)?;
    let end = at.checked_add(width).filter(|&end| end <= haystack.len())?;
    let text = std::str::from_utf8(haystack.get(at..end)?).ok()?;
    let mut characters = text.chars();
    let character = characters.next()?;
    characters.next().is_none().then_some((end, character))
}

fn utf8_width(first: u8) -> Option<usize> {
    match first {
        0x00..=0x7f => Some(1),
        0xc2..=0xdf => Some(2),
        0xe0..=0xef => Some(3),
        0xf0..=0xf4 => Some(4),
        _ => None,
    }
}

fn is_utf8_continuation(byte: u8) -> bool {
    byte & 0b1100_0000 == 0b1000_0000
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn verifier(policy: BoundaryPolicy, require_smart_left: bool) -> BoundaryVerifier {
        BoundaryVerifier {
            policy,
            require_smart_left,
        }
    }

    #[test]
    fn smart_nominal_boundaries_reject_compound_substrings() {
        let text = "사용자권한은 권한이다";
        let first_start = "사용자".len();
        let first_core = first_start.."사용자권한은".find('은').unwrap();
        let first_token = first_start.."사용자권한은".len();
        let second_start = text.rfind("권한").unwrap();

        assert!(!verifier(BoundaryPolicy::Smart, true).accepts(
            text.as_bytes(),
            first_core,
            first_token,
        ));
        assert!(verifier(BoundaryPolicy::Smart, true).accepts(
            text.as_bytes(),
            second_start..second_start + "권한".len(),
            second_start..text.len(),
        ));
    }

    #[test]
    fn any_policy_allows_identifier_substrings() {
        let text = "user_권한_value";
        let start = text.find("권한").unwrap();
        let span = start..start + "권한".len();

        assert!(verifier(BoundaryPolicy::Any, false).accepts(text.as_bytes(), span.clone(), span,));
    }

    #[test]
    fn token_policy_treats_combining_marks_and_underscore_as_token_characters() {
        for text in ["_권한", "\u{0301}권한", "A권한"] {
            let start = text.find("권한").unwrap();
            let span = start..start + "권한".len();
            assert!(!verifier(BoundaryPolicy::Token, false).accepts(
                text.as_bytes(),
                span.clone(),
                span,
            ));
        }
    }

    #[test]
    fn malformed_neighboring_bytes_form_a_safe_boundary() {
        let haystack = [0xff, 0xea, 0xb6, 0x8c, 0xed, 0x95, 0x9c, 0xff];
        let span = 1..7;

        assert!(verifier(BoundaryPolicy::Token, false).accepts(&haystack, span.clone(), span,));
    }

    #[test]
    fn invalid_ranges_are_never_accepted() {
        let reversed = Range { start: 2, end: 1 };
        assert!(!verifier(BoundaryPolicy::Any, false).accepts(b"abc", reversed, 0..3));
        assert!(!verifier(BoundaryPolicy::Any, false).accepts(b"abc", 0..4, 0..4));
    }

    #[test]
    fn surrounding_token_span_expands_to_orthographic_boundaries() {
        let text = "앞 매일_일 뒤";
        let start = text.find('일').unwrap();
        let span = start..start + '일'.len_utf8();

        assert_eq!(
            &text[surrounding_token_span(text.as_bytes(), span)],
            "매일_일"
        );
    }

    #[test]
    fn bounded_token_span_stops_after_proving_the_limit_is_exceeded() {
        let text = "가나다라마바사";
        let start = text.find('라').unwrap();
        let span = start..start + '라'.len_utf8();

        assert_eq!(
            bounded_surrounding_token_span(text.as_bytes(), span.clone(), text.len()),
            Ok(0..text.len())
        );
        assert_eq!(
            bounded_surrounding_token_span(text.as_bytes(), span, 8),
            Err(9)
        );
    }

    fn reference_is_token_character(character: char) -> bool {
        character == '_'
            || character.is_alphanumeric()
            || matches!(
                get_general_category(character),
                GeneralCategory::NonspacingMark
                    | GeneralCategory::SpacingMark
                    | GeneralCategory::EnclosingMark
            )
    }

    fn reference_previous_character_with_start(
        haystack: &[u8],
        at: usize,
    ) -> Option<(usize, char)> {
        if at == 0 || at > haystack.len() {
            return None;
        }
        let earliest = at.saturating_sub(4);
        (earliest..at).rev().find_map(|start| {
            let text = std::str::from_utf8(&haystack[start..at]).ok()?;
            let mut characters = text.chars();
            let character = characters.next()?;
            characters.next().is_none().then_some((start, character))
        })
    }

    fn reference_next_character_with_end(haystack: &[u8], at: usize) -> Option<(usize, char)> {
        if at >= haystack.len() {
            return None;
        }
        let latest = (at + 4).min(haystack.len());
        ((at + 1)..=latest).find_map(|end| {
            let text = std::str::from_utf8(&haystack[at..end]).ok()?;
            let mut characters = text.chars();
            let character = characters.next()?;
            characters.next().is_none().then_some((end, character))
        })
    }

    proptest! {
        #[test]
        fn token_character_fast_paths_preserve_the_unicode_contract(character in any::<char>()) {
            prop_assert_eq!(
                is_token_character(character),
                reference_is_token_character(character)
            );
        }

        #[test]
        fn bounded_scalar_decoders_match_the_reference_on_arbitrary_bytes(
            haystack in prop::collection::vec(any::<u8>(), 0..512),
            at in 0usize..600,
        ) {
            prop_assert_eq!(
                previous_character_with_start(&haystack, at),
                reference_previous_character_with_start(&haystack, at)
            );
            prop_assert_eq!(
                next_character_with_end(&haystack, at),
                reference_next_character_with_end(&haystack, at)
            );
        }
    }
}
