use kfind_morph::FinePos;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LexicalContextRule {
    RepeatedToken,
}

#[derive(Clone, Copy)]
struct Registration {
    surface: &'static str,
    fine_pos: FinePos,
    rule: LexicalContextRule,
}

const REGISTRATIONS: &[Registration] = &[Registration {
    surface: "매일",
    fine_pos: FinePos::GeneralAdverb,
    rule: LexicalContextRule::RepeatedToken,
}];

pub(super) fn lexical_context_rule(surface: &str, fine_pos: FinePos) -> Option<LexicalContextRule> {
    REGISTRATIONS
        .iter()
        .find(|registration| registration.surface == surface && registration.fine_pos == fine_pos)
        .map(|registration| registration.rule)
}

pub fn registered_lexical_context_prefix_len(token: &str) -> Option<usize> {
    REGISTRATIONS
        .iter()
        .filter(|registration| token.starts_with(registration.surface))
        .map(|registration| registration.surface.len())
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_context_is_registered_by_surface_and_fine_pos() {
        assert_eq!(
            lexical_context_rule("매일", FinePos::GeneralAdverb),
            Some(LexicalContextRule::RepeatedToken)
        );
        assert_eq!(lexical_context_rule("매일", FinePos::CommonNoun), None);
        assert_eq!(lexical_context_rule("빨리", FinePos::GeneralAdverb), None);
    }

    #[test]
    fn registered_surface_is_shared_with_component_fallback() {
        assert_eq!(registered_lexical_context_prefix_len("매일"), Some(6));
        assert_eq!(registered_lexical_context_prefix_len("매일을"), Some(6));
        assert_eq!(registered_lexical_context_prefix_len("날마다"), None);
    }
}
