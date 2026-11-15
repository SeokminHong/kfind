use kfind_morph::hangul::{
    JONG_BIEUP, JONG_DIGEUT, JONG_HIEUH, JONG_NONE, JONG_SIOT, JUNG_A, JUNG_AE, JUNG_EO, JUNG_EU,
    JUNG_I, JUNG_O, JUNG_OE, JUNG_U, JUNG_WA, JUNG_WAE, JUNG_WEO, JUNG_YA, JUNG_YAE, JUNG_YE,
    JUNG_YEO, add_final, decompose_syllable, drop_last_final, has_rieul_final, replace_last_final,
    replace_last_vowel,
};
use kfind_morph::predicate::has_vowel_connective_surface;
use kfind_morph::{CoarsePos, FinePos, LexicalAlternation, PredicateEntry, PredicatePos};

use crate::{Analysis, AnalysisSource, Lexicons, Morphology};

pub(super) fn change_of_state(lexicons: &Lexicons, lemma: &str) -> Option<Analysis> {
    let connective = lemma.strip_suffix("지다").filter(|form| !form.is_empty())?;
    let has_host =
        possible_vowel_stems(connective)
            .into_iter()
            .any(|mut stem| {
                stem.push('다');
                lexicons.lookup_with_surface_fallback(&stem).iter().any(|analysis| {
            matches!(
                &analysis.morphology,
                Morphology::Predicate(predicate)
                    if matches!(predicate.pos, PredicatePos::Verb | PredicatePos::Adjective)
                        && has_vowel_connective_surface(predicate, connective)
            )
        })
            });
    has_host.then(|| Analysis {
        lemma: lemma.into(),
        coarse_pos: CoarsePos::Verb,
        fine_pos: FinePos::Verb,
        morphology: Morphology::Predicate(PredicateEntry::new(
            lemma,
            PredicatePos::Verb,
            LexicalAlternation::Regular,
        )),
        source: AnalysisSource::ProductiveSuffix,
    })
}

fn possible_vowel_stems(connective: &str) -> Vec<String> {
    let mut stems = vec![connective.to_owned()];
    if let Some(stem) = connective
        .strip_suffix(['아', '어', '여'])
        .filter(|stem| !stem.is_empty())
    {
        stems.push(stem.to_owned());
        stems.extend(add_final(stem, JONG_SIOT));
        if stem.ends_with(has_rieul_final) {
            stems.extend(replace_last_final(stem, JONG_DIGEUT));
        }
    }
    if let Some(stem) = connective.strip_suffix(['와', '워']) {
        stems.extend(add_final(stem, JONG_BIEUP));
    }
    if let Some(stem) = connective.strip_suffix('놔') {
        stems.push(format!("{stem}놓"));
    }
    if let Some(stem) = connective
        .strip_suffix(['라', '러'])
        .filter(|stem| stem.ends_with(has_rieul_final))
    {
        stems.push(format!("{stem}르"));
        if let Some(base) = drop_last_final(stem) {
            stems.push(format!("{base}르"));
        }
    }
    if let Some(stem) = connective
        .strip_suffix('러')
        .filter(|stem| stem.ends_with('르'))
    {
        stems.push(stem.to_owned());
    }
    if let Some(last) = connective.chars().next_back().and_then(decompose_syllable)
        && last.jongseong == JONG_NONE
    {
        let vowels: &[u8] = match last.jungseong {
            JUNG_WA => &[JUNG_O],
            JUNG_WAE => &[JUNG_OE],
            JUNG_WEO => &[JUNG_U],
            JUNG_YEO => &[JUNG_I],
            JUNG_A => &[JUNG_EU],
            JUNG_EO => &[JUNG_EU, JUNG_U],
            JUNG_AE => &[JUNG_A],
            _ => &[],
        };
        stems.extend(
            vowels
                .iter()
                .filter_map(|&vowel| replace_last_vowel(connective, vowel)),
        );
        let h_vowels: &[u8] = match last.jungseong {
            JUNG_AE => &[JUNG_A, JUNG_EO, JUNG_AE],
            JUNG_YAE => &[JUNG_YA],
            JUNG_YE => &[JUNG_YEO],
            _ => &[],
        };
        stems.extend(
            h_vowels.iter().filter_map(|&vowel| {
                add_final(&replace_last_vowel(connective, vowel)?, JONG_HIEUH)
            }),
        );
    }
    stems
}
