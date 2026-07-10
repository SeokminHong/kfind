//! Arithmetic operations on precomposed Hangul syllables.

const HANGUL_BASE: u32 = 0xac00;
const CHOSEONG_COUNT: u32 = 19;
const JUNGSEONG_COUNT: u32 = 21;
const JONGSEONG_COUNT: u32 = 28;
const SYLLABLE_COUNT: u32 = CHOSEONG_COUNT * JUNGSEONG_COUNT * JONGSEONG_COUNT;

pub const JUNG_A: u8 = 0;
pub const JUNG_AE: u8 = 1;
pub const JUNG_YA: u8 = 2;
pub const JUNG_YAE: u8 = 3;
pub const JUNG_EO: u8 = 4;
pub const JUNG_E: u8 = 5;
pub const JUNG_YEO: u8 = 6;
pub const JUNG_YE: u8 = 7;
pub const JUNG_O: u8 = 8;
pub const JUNG_WA: u8 = 9;
pub const JUNG_WAE: u8 = 10;
pub const JUNG_OE: u8 = 11;
pub const JUNG_U: u8 = 13;
pub const JUNG_WEO: u8 = 14;
pub const JUNG_EU: u8 = 18;
pub const JUNG_I: u8 = 20;

pub const JONG_NONE: u8 = 0;
pub const JONG_NIEUN: u8 = 4;
pub const JONG_DIGEUT: u8 = 7;
pub const JONG_RIEUL: u8 = 8;
pub const JONG_BIEUP: u8 = 17;
pub const JONG_SIOT: u8 = 19;
pub const JONG_SSANGSIOT: u8 = 20;
pub const JONG_HIEUH: u8 = 27;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Syllable {
    pub choseong: u8,
    pub jungseong: u8,
    pub jongseong: u8,
}

#[must_use]
pub fn decompose_syllable(character: char) -> Option<Syllable> {
    let codepoint = u32::from(character);
    let offset = codepoint.checked_sub(HANGUL_BASE)?;
    if offset >= SYLLABLE_COUNT {
        return None;
    }

    let choseong = offset / (JUNGSEONG_COUNT * JONGSEONG_COUNT);
    let jungseong = (offset % (JUNGSEONG_COUNT * JONGSEONG_COUNT)) / JONGSEONG_COUNT;
    let jongseong = offset % JONGSEONG_COUNT;

    Some(Syllable {
        choseong: u8::try_from(choseong).ok()?,
        jungseong: u8::try_from(jungseong).ok()?,
        jongseong: u8::try_from(jongseong).ok()?,
    })
}

#[must_use]
pub fn compose_syllable(syllable: Syllable) -> Option<char> {
    if u32::from(syllable.choseong) >= CHOSEONG_COUNT
        || u32::from(syllable.jungseong) >= JUNGSEONG_COUNT
        || u32::from(syllable.jongseong) >= JONGSEONG_COUNT
    {
        return None;
    }

    let offset = u32::from(syllable.choseong) * JUNGSEONG_COUNT * JONGSEONG_COUNT
        + u32::from(syllable.jungseong) * JONGSEONG_COUNT
        + u32::from(syllable.jongseong);
    char::from_u32(HANGUL_BASE + offset)
}

#[must_use]
pub fn replace_final(character: char, jongseong: u8) -> Option<char> {
    let mut syllable = decompose_syllable(character)?;
    if u32::from(jongseong) >= JONGSEONG_COUNT {
        return None;
    }
    syllable.jongseong = jongseong;
    compose_syllable(syllable)
}

#[must_use]
pub fn drop_final(character: char) -> Option<char> {
    let syllable = decompose_syllable(character)?;
    if syllable.jongseong == JONG_NONE {
        return None;
    }
    replace_final(character, JONG_NONE)
}

#[must_use]
pub fn replace_last_final(value: &str, jongseong: u8) -> Option<String> {
    replace_last_syllable(value, |last| replace_final(last, jongseong))
}

#[must_use]
pub fn drop_last_final(value: &str) -> Option<String> {
    replace_last_syllable(value, drop_final)
}

#[must_use]
pub fn add_final(value: &str, jongseong: u8) -> Option<String> {
    if jongseong == JONG_NONE || u32::from(jongseong) >= JONGSEONG_COUNT {
        return None;
    }
    let last = value.chars().next_back()?;
    if decompose_syllable(last)?.jongseong != JONG_NONE {
        return None;
    }
    replace_last_final(value, jongseong)
}

#[must_use]
pub fn replace_last_vowel(value: &str, jungseong: u8) -> Option<String> {
    if u32::from(jungseong) >= JUNGSEONG_COUNT {
        return None;
    }
    replace_last_syllable(value, |last| {
        let mut syllable = decompose_syllable(last)?;
        syllable.jungseong = jungseong;
        compose_syllable(syllable)
    })
}

#[must_use]
pub fn has_final(character: char) -> bool {
    decompose_syllable(character).is_some_and(|syllable| syllable.jongseong != JONG_NONE)
}

#[must_use]
pub fn has_rieul_final(character: char) -> bool {
    decompose_syllable(character).is_some_and(|syllable| syllable.jongseong == JONG_RIEUL)
}

fn replace_last_syllable(
    value: &str,
    replace: impl FnOnce(char) -> Option<char>,
) -> Option<String> {
    let (last_index, last) = value.char_indices().next_back()?;
    let replacement = replace(last)?;
    let mut result = String::with_capacity(value.len());
    result.push_str(&value[..last_index]);
    result.push(replacement);
    Some(result)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn rejects_non_hangul_and_invalid_indices() {
        assert_eq!(decompose_syllable('A'), None);
        assert_eq!(
            compose_syllable(Syllable {
                choseong: 19,
                jungseong: 0,
                jongseong: 0,
            }),
            None
        );
        assert_eq!(replace_final('가', 28), None);
    }

    #[test]
    fn changes_the_last_precomposed_syllable_only() {
        assert_eq!(replace_last_final("가", JONG_NIEUN).as_deref(), Some("간"));
        assert_eq!(drop_last_final("길").as_deref(), Some("기"));
        assert_eq!(add_final("먹", JONG_NIEUN), None);
        assert_eq!(replace_last_vowel("예쁘", JUNG_EO).as_deref(), Some("예뻐"));
        assert!(has_final('먹'));
        assert!(has_rieul_final('길'));
    }

    proptest! {
        #[test]
        fn compose_then_decompose_roundtrips(
            choseong in 0_u8..19,
            jungseong in 0_u8..21,
            jongseong in 0_u8..28,
        ) {
            let syllable = Syllable { choseong, jungseong, jongseong };
            let composed = compose_syllable(syllable).expect("valid syllable indices");
            prop_assert_eq!(decompose_syllable(composed), Some(syllable));
        }
    }
}
