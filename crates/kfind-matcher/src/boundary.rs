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
        || character.is_alphanumeric()
        || matches!(
            get_general_category(character),
            GeneralCategory::NonspacingMark
                | GeneralCategory::SpacingMark
                | GeneralCategory::EnclosingMark
        )
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

fn previous_character(haystack: &[u8], at: usize) -> Option<char> {
    if at == 0 || at > haystack.len() {
        return None;
    }
    let earliest = at.saturating_sub(4);
    (earliest..at).rev().find_map(|start| {
        let text = std::str::from_utf8(&haystack[start..at]).ok()?;
        let mut characters = text.chars();
        let character = characters.next()?;
        characters.next().is_none().then_some(character)
    })
}

fn next_character(haystack: &[u8], at: usize) -> Option<char> {
    if at >= haystack.len() {
        return None;
    }
    let latest = (at + 4).min(haystack.len());
    ((at + 1)..=latest).find_map(|end| {
        let text = std::str::from_utf8(&haystack[at..end]).ok()?;
        let mut characters = text.chars();
        let character = characters.next()?;
        characters.next().is_none().then_some(character)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
