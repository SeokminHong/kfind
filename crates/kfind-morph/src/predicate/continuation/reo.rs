use super::{Suffix, suffix};

macro_rules! reo_endings {
    ($prefix:literal $(, $prior:literal)*) => {
        &[
            suffix(concat!($prefix, "러니"), &[$($prior,)* "ending.retrospective-reoni"]),
            suffix(concat!($prefix, "러니라"), &[$($prior,)* "ending.retrospective-reo-final"]),
            suffix(concat!($prefix, "러니이까"), &[$($prior,)* "ending.retrospective-reo-final"]),
            suffix(concat!($prefix, "러니이다"), &[$($prior,)* "ending.retrospective-reo-final"]),
            suffix(concat!($prefix, "러라"), &[$($prior,)* "ending.retrospective-reo-final"]),
            suffix(concat!($prefix, "러이까"), &[$($prior,)* "ending.retrospective-reo-final"]),
            suffix(concat!($prefix, "러이다"), &[$($prior,)* "ending.retrospective-reo-final"]),
            suffix(concat!($prefix, "런가"), &[$($prior,)* "ending.retrospective-reo-final"]),
            suffix(concat!($prefix, "런들"), &[$($prior,)* "ending.retrospective-reondeul"]),
        ]
    };
}

pub(super) const COPULA_REO_ENDINGS: &[Suffix] = reo_endings!("");
pub(super) const HONORIFIC_REO_ENDINGS: &[Suffix] = reo_endings!("시", "ending.honorific");
pub(super) const RETROSPECTIVE_REO_ENDINGS: &[Suffix] =
    reo_endings!("더", "ending.retrospective-prefinal");
pub(super) const HONORIFIC_RETROSPECTIVE_REO: &[Suffix] =
    reo_endings!("시더", "ending.honorific", "ending.retrospective-prefinal");
pub(super) const PROSPECTIVE_REO: &[Suffix] = reo_endings!("리", "ending.prospective-prefinal");
pub(super) const HONORIFIC_PROSPECTIVE_REO: &[Suffix] =
    reo_endings!("시리", "ending.honorific", "ending.prospective-prefinal");
pub(super) const PREFINAL_PROSPECTIVE_REO: &[Suffix] =
    reo_endings!("으리", "ending.prospective-prefinal");

pub(in crate::predicate) fn is_copula_reo_ending(surface: &str) -> bool {
    surface.starts_with(['러', '런'])
        && COPULA_REO_ENDINGS
            .iter()
            .any(|ending| ending.surface == surface)
}

#[cfg(test)]
mod tests {
    use crate::{ContinuationState, PredicatePos, verify_predicate_continuation};

    #[test]
    fn requires_a_copula_or_licensed_prefinal_and_a_token_end() {
        assert!(
            verify_predicate_continuation(
                ContinuationState::StemEnding,
                PredicatePos::Verb,
                "먹",
                "러이다"
            )
            .is_none()
        );
        assert!(
            verify_predicate_continuation(
                ContinuationState::StemEnding,
                PredicatePos::Copula,
                "이",
                "러이다말"
            )
            .is_none()
        );
        assert!(
            verify_predicate_continuation(
                ContinuationState::StemEnding,
                PredicatePos::Copula,
                "이",
                "러이다."
            )
            .is_some()
        );
    }
}
