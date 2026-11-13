use crate::ContinuationState;

use super::{
    JIMAN, JIMANEUN, JIYO, JYO, MAE, PredicateContinuationMatch, Suffix, auxiliary, suffix,
};

const AUXILIARY: &str = "ending.auxiliary-boda";
const STEM_ENDINGS: &[Suffix] = &[
    JIYO,
    JYO,
    suffix("다", &["ending.final-da"]),
    suffix("고요", &["ending.connective-go", "ending.polite-yo"]),
    suffix("네요", &["ending.final-ne", "ending.polite-yo"]),
    suffix("네", &["ending.final-ne"]),
    JIMAN,
    JIMANEUN,
    suffix("므로", &["ending.connective-meuro"]),
    MAE,
    suffix("기에", &["ending.connective-gie"]),
    suffix("는구나", &["ending.final-neun-exclamative"]),
    suffix("는군", &["ending.final-neun-exclamative"]),
];

pub(super) fn verify(anchor_len: usize, following: &str) -> Option<PredicateContinuationMatch> {
    use ContinuationState::{
        AOrEo, BieupFinal, CompletedNieunEnding, Declarative, Future, Interrogative, NieunEnding,
        Past, RieulFinal, Terminal,
    };
    let (form, state, rule) = match following.chars().next()? {
        '본' if following.starts_with("본다") => {
            ("본다", Declarative, Some("ending.declarative"))
        }
        '본' => ("본", CompletedNieunEnding, Some("ending.past-adnominal")),
        '볼' => ("볼", RieulFinal, Some("ending.future-adnominal")),
        '봄' if following.starts_with("봄세") => {
            ("봄세", Terminal, Some("ending.volitional-mse"))
        }
        '봄' => ("봄", Terminal, Some("ending.nominalizer")),
        '봅' if following.starts_with("봅시다") => {
            ("봅시다", Terminal, Some("ending.propositive"))
        }
        '봅' => ("봅", BieupFinal, None),
        '봐' => ("봐", AOrEo, Some("contraction.o-a")),
        '봤' => ("봤", Past, Some("ending.past")),
        '보' if following.starts_with("보았") => ("보았", Past, Some("ending.past")),
        '보' if following.starts_with("보겠") => ("보겠", Future, Some("ending.future")),
        '보' if following.starts_with("보아") => ("보아", AOrEo, Some("ending.aoeo")),
        '보' if following.starts_with("보느냐") => {
            ("보느냐", Interrogative, Some("ending.interrogative-neunya"))
        }
        '보' if following.starts_with("보는")
            && !following.starts_with("보는구나")
            && !following.starts_with("보는군") =>
        {
            ("보는", NieunEnding, Some("ending.present-adnominal"))
        }
        '보' if following.starts_with("보기") => {
            ("보기", Terminal, Some("ending.nominalizer-gi"))
        }
        '보' => {
            return auxiliary::verify_stem(
                anchor_len,
                "보",
                &following["보".len()..],
                AUXILIARY,
                STEM_ENDINGS,
            );
        }
        _ => return None,
    };
    auxiliary::verify_form(anchor_len, following, form, state, rule, AUXILIARY)
}
