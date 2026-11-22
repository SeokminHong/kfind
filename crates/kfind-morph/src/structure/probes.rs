use crate::structure::ConstraintResolver;
use crate::structure::evidence::TokenEvidence;
use crate::structure::graph::EdgeGraph;
use crate::structure::lexical::{
    complete_connective_path, complete_dependent_noun_particle_suffix, complete_ha_predicate_path,
    complete_inflected_ending, complete_nominal_host, complete_nominal_particle_host,
    complete_predicate_ending_path, complete_suffix, derived_inflected_path,
    exact_analysis_starts_with_pos, has_adnominal_result_complement, has_complete_object_phrase,
    has_exact_fine_pos, has_exact_sequence, numeric_unit_path, numeric_unit_path_at,
    predicate_inflected_path, predicate_pos_matches, structural_predicate_pos_matches,
};
use crate::structure::paths::predicate::{
    compound_predicate_pos, has_complete_attached_auxiliary_path, is_attached_auxiliary_whole_path,
};
use crate::structure::roman_numeral::{is_roman_digit, is_roman_numeral};
use crate::{
    LexicalAlternation, PredicatePos, PredicatePosSet, verify_copula_surface_after_nominal,
};
use kfind_data::{ComponentPos as StructuralPos, DataFinePos, NominalRole};
use std::ops::Range;

const NON_ACCUSATIVE_PREDICATE_ANCHORS: &[&str] =
    include!("../../../../data/rules/krdict-non-accusative-predicate-anchors.rs");
const PRODUCTIVE_NOMINAL_HEADS: &[&str] =
    include!("../../../../data/rules/nikl-productive-nominal-heads.rs");
const BOUND_NOMINAL_PREFIX_USAGES: &[(&str, &[&str])] =
    include!("../../../../data/rules/nikl-bound-nominal-prefix-usages.rs");
const STDICT_BOUND_DAE_PREFIX_USAGES: &[&str] =
    include!("../../../../data/rules/stdict-bound-dae-prefix-usages.rs");

include!("../../../../data/rules/krdict-nominal-frames.rs");
include!("../../../../data/rules/krdict-auxiliary-frames.rs");

// Keep one copy of the large table in the linked image.
#[inline(never)]
fn stem_excludes_accusative(stem: &str) -> bool {
    NON_ACCUSATIVE_PREDICATE_ANCHORS
        .binary_search(&stem)
        .is_ok()
}

struct AuxiliaryInflections {
    future: Vec<crate::SurfaceBranchSpec>,
    present_or_past: Vec<crate::SurfaceBranchSpec>,
    interrogative: Vec<crate::SurfaceBranchSpec>,
}

#[derive(Clone, Copy)]
enum VowelAuxiliaryHost {
    Verb,
    Adjective,
    VerbOrAdjective,
}

impl VowelAuxiliaryHost {
    fn accepts(self, pos: PredicatePos) -> bool {
        match self {
            Self::Verb => pos.is_action(),
            Self::Adjective => pos.execution() == PredicatePos::Adjective,
            Self::VerbOrAdjective => pos != PredicatePos::Copula,
        }
    }
}

struct VowelAuxiliaryInflection {
    lemma: &'static str,
    host: VowelAuxiliaryHost,
    form: crate::SurfaceBranchSpec,
}

fn vowel_auxiliary_inflections() -> &'static [VowelAuxiliaryInflection] {
    static FORMS: std::sync::OnceLock<Vec<VowelAuxiliaryInflection>> = std::sync::OnceLock::new();
    FORMS.get_or_init(|| {
        let mut forms: Vec<_> = VOWEL_AUXILIARIES
            .iter()
            .flat_map(|&(lemma, alternation, host)| {
                let entry =
                    crate::PredicateEntry::new(lemma, PredicatePos::AuxiliaryVerb, alternation);
                crate::generate_predicate_branches(&entry)
                    .expect("valid dictionary auxiliary")
                    .into_iter()
                    .map(move |form| VowelAuxiliaryInflection { lemma, host, form })
            })
            .collect();
        forms.sort_unstable_by(|left, right| left.form.anchor.cmp(&right.form.anchor));
        forms
    })
}

fn auxiliary_inflections() -> &'static AuxiliaryInflections {
    static INFLECTIONS: std::sync::OnceLock<AuxiliaryInflections> = std::sync::OnceLock::new();
    INFLECTIONS.get_or_init(|| {
        let branches = |lemma: &str, pos, alternation| {
            let entry = crate::PredicateEntry::new(lemma, pos, alternation);
            crate::generate_predicate_branches(&entry).expect("valid dictionary auxiliary")
        };
        let mut future = Vec::new();
        let mut present_or_past = Vec::new();
        for &(lemma, pos, alternation, takes_future, takes_present_or_past) in ADNOMINAL_AUXILIARIES
        {
            let forms = branches(lemma, pos, alternation);
            if takes_future {
                future.extend(forms.iter().cloned());
            }
            if takes_present_or_past {
                present_or_past.extend(forms);
            }
        }
        AuxiliaryInflections {
            future,
            present_or_past,
            interrogative: INTERROGATIVE_AUXILIARIES
                .iter()
                .flat_map(|&(lemma, pos, alternation)| branches(lemma, pos, alternation))
                .collect(),
        }
    })
}

pub(crate) fn matches_interrogative_auxiliary_inflection(text: &str) -> bool {
    matches_predicate_inflection(text, &auxiliary_inflections().interrogative)
}

fn matches_predicate_inflection(text: &str, forms: &[crate::SurfaceBranchSpec]) -> bool {
    forms
        .iter()
        .any(|form| matches_predicate_branch(text, form))
}

fn purpose_host_alternations() -> &'static [(LexicalAlternation, crate::SurfaceBranchSpec)] {
    static FORMS: std::sync::OnceLock<Vec<(LexicalAlternation, crate::SurfaceBranchSpec)>> =
        std::sync::OnceLock::new();
    FORMS.get_or_init(|| {
        let (records, _) = kfind_data::parse_predicates_tsv(
            "data/lexicon/predicates.tsv",
            include_str!("../../../../data/lexicon/predicates.tsv"),
        )
        .expect("valid embedded predicate lexicon");
        records
            .iter()
            .filter_map(crate::PredicateEntry::from_lexicon_record)
            .filter(|entry| {
                matches!(
                    entry.alternation,
                    LexicalAlternation::DToL | LexicalAlternation::DropS | LexicalAlternation::Reo
                )
            })
            .flat_map(|entry| {
                crate::generate_predicate_branches(&entry)
                    .expect("valid dictionary alternation")
                    .into_iter()
                    .map(move |form| (entry.alternation, form))
            })
            .collect()
    })
}

fn go_auxiliary_inflections() -> &'static [crate::SurfaceBranchSpec] {
    static FORMS: std::sync::OnceLock<Vec<crate::SurfaceBranchSpec>> = std::sync::OnceLock::new();
    FORMS.get_or_init(|| {
        PROGRESSIVE_OR_DESIDERATIVE_AUXILIARIES
            .iter()
            .flat_map(|&(lemma, pos)| {
                let entry = crate::PredicateEntry::new(lemma, pos, LexicalAlternation::Regular);
                crate::generate_predicate_branches(&entry).expect("valid dictionary auxiliary")
            })
            .collect()
    })
}

pub fn matches_negative_auxiliary_inflection(text: &str, pos: PredicatePos) -> bool {
    static FORMS: std::sync::OnceLock<Vec<crate::SurfaceBranchSpec>> = std::sync::OnceLock::new();
    let Some(initial) = text.chars().next() else {
        return false;
    };
    if NEGATIVE_AUXILIARIES
        .iter()
        .any(|&(lemma, auxiliary_pos, _)| {
            auxiliary_pos == pos && lemma.strip_suffix('다') == Some(text)
        })
    {
        return false;
    }
    let forms = FORMS.get_or_init(|| {
        let (records, _) = kfind_data::parse_predicates_tsv(
            "data/lexicon/predicates.tsv",
            include_str!("../../../../data/lexicon/predicates.tsv"),
        )
        .expect("valid embedded predicate lexicon");
        let mut forms: Vec<_> = NEGATIVE_AUXILIARIES
            .iter()
            .flat_map(|&(lemma, pos, alternation)| {
                let entry = records
                    .iter()
                    .filter(|record| record.lemma == lemma)
                    .filter_map(crate::PredicateEntry::from_lexicon_record)
                    .find(|entry| entry.pos == pos)
                    .unwrap_or_else(|| crate::PredicateEntry::new(lemma, pos, alternation));
                crate::generate_predicate_branches(&entry).expect("valid dictionary auxiliary")
            })
            .collect();
        forms.sort_unstable_by(|left, right| left.anchor.cmp(&right.anchor));
        forms
    });
    let start = forms.partition_point(|form| form.anchor.chars().next() < Some(initial));
    forms[start..]
        .iter()
        .take_while(|form| form.anchor.starts_with(initial))
        .any(|form| {
            form.pos == pos
                && matches_predicate_branch_with_ending(text, form, |ending| {
                    ending.is_none_or(|rule| {
                        !matches!(
                            rule.as_str(),
                            "ending.honorific" | "ending.past" | "ending.future"
                        )
                    })
                })
        })
}

fn matches_predicate_branch(text: &str, form: &crate::SurfaceBranchSpec) -> bool {
    matches_predicate_branch_with_ending(text, form, |_| true)
}

fn matches_predicate_branch_with_ending(
    text: &str,
    form: &crate::SurfaceBranchSpec,
    accepts: impl FnOnce(Option<&crate::RuleId>) -> bool,
) -> bool {
    text.strip_prefix(form.anchor.as_ref()).is_some_and(|tail| {
        if tail.is_empty() {
            return (!form.rule_path.is_empty() || form.anchor.ends_with('다'))
                && accepts(form.rule_path.last());
        }
        crate::verify_predicate_continuation(form.continuation, form.pos, &form.anchor, tail)
            .is_some_and(|matched| {
                matched.token_end == text.len()
                    && accepts(matched.rule_path.last().or_else(|| form.rule_path.last()))
            })
    })
}

const DURATION_NOUNS: &[&str] = &[
    "하루",
    "이틀",
    "사흘",
    "나흘",
    "닷새",
    "엿새",
    "이레",
    "여드레",
    "아흐레",
    "열흘",
    "보름",
    "반달",
    "반년",
];

const DURATION_UNITS: &[&str] = &[
    "밀리초",
    "주일",
    "개월",
    "시간",
    "세기",
    "년",
    "해",
    "달",
    "주",
    "일",
    "날",
    "분",
    "초",
];

// Numeric senses attested by the pinned Korean Basic Dictionary and licensed
// in digit-led unit constructions.
const DICTIONARY_NUMERIC_UNITS: &[&str] = &[
    "가닥",
    "가래",
    "게임",
    "계단",
    "곡조",
    "굽이",
    "기가",
    "까풀",
    "꺼풀",
    "꼬치",
    "꿰미",
    "끼니",
    "날",
    "낱",
    "년대",
    "년도",
    "대접",
    "도막",
    "돌림",
    "동강",
    "드럼",
    "떨기",
    "뙈기",
    "메가",
    "박스",
    "발자국",
    "보시기",
    "봉",
    "분기",
    "사단",
    "사리",
    "삼태기",
    "섬지기",
    "세기",
    "스푼",
    "쌈지",
    "알갱이",
    "연대",
    "음절",
    "인분",
    "자락",
    "젓갈",
    "제곱킬로미터",
    "종",
    "종지",
    "죽",
    "차시",
    "책",
    "컵",
    "타래",
    "탄",
    "패",
    "페이지",
    "평방미터",
    "평방킬로미터",
    "표",
    "학점",
    "항",
    "행",
    "호봉",
];

const SHORT_NOMINAL_DERIVATIONS: &str =
    include_str!("../../../../data/rules/stdict-short-nominal-derivations.tsv");
include!("../../../../data/rules/stdict-attested-nominal-suffix-usages.rs");
include!("../../../../data/rules/stdict-bound-si-hosts.rs");

impl ConstraintResolver {
    #[must_use]
    pub fn has_complete_connective_path(&self, text: &str) -> bool {
        complete_connective_path(&self.resource, text)
    }

    /// Verifies the predicate heads in the prohibition construction `-어서는 안 되다`.
    #[must_use]
    pub fn has_prohibitive_negation_frame(&self, previous: &str, next: &str) -> bool {
        let Some(stem) = previous
            .strip_suffix("서는")
            .filter(|stem| !stem.is_empty())
        else {
            return false;
        };
        let connective = &previous[..stem.len() + '서'.len_utf8()];
        if !self.has_complete_connective_path(connective) {
            return false;
        }
        static FORMS: std::sync::LazyLock<Vec<crate::SurfaceBranchSpec>> =
            std::sync::LazyLock::new(|| {
                let entry = crate::PredicateEntry::new(
                    "되다",
                    PredicatePos::Verb,
                    LexicalAlternation::Regular,
                );
                crate::generate_predicate_branches(&entry).expect("valid permission predicate")
            });
        matches_predicate_inflection(next, &FORMS)
    }

    #[must_use]
    pub fn has_bound_si_suffix(&self, host: &str, following: &str, nominal_end: bool) -> bool {
        STDICT_BOUND_SI_HOSTS
            .binary_search_by_key(&host, |&(word, _, _)| word)
            .is_ok_and(|index| {
                let (_, unambiguous, hada) = STDICT_BOUND_SI_HOSTS[index];
                (unambiguous
                    && (nominal_end || verify_copula_surface_after_nominal('시', following)))
                    || (hada && complete_ha_predicate_path(&self.resource, following))
            })
    }
    #[must_use]
    pub fn has_attested_nominal_suffix(suffix: &str) -> bool {
        STDICT_ATTACHED_NOMINAL_USAGES
            .binary_search_by_key(&suffix, |&(registered, _)| registered)
            .is_ok()
    }

    #[must_use]
    pub fn has_attested_nominal_suffix_usage(word: &str, suffix: &str) -> bool {
        STDICT_ATTACHED_NOMINAL_USAGES
            .binary_search(&(suffix, word))
            .is_ok()
    }

    #[must_use]
    pub fn has_attested_nominal_prefix(prefix: &str) -> bool {
        BOUND_NOMINAL_PREFIX_USAGES
            .binary_search_by_key(&prefix, |&(form, _)| form)
            .is_ok()
    }

    #[must_use]
    pub fn has_bound_nominal_prefix(&self, prefix: &str, text: &str) -> bool {
        let Ok(index) =
            BOUND_NOMINAL_PREFIX_USAGES.binary_search_by_key(&prefix, |&(form, _)| form)
        else {
            return false;
        };
        let accepts_tail = |noun: &str, tail: &str| {
            tail.is_empty()
                || complete_suffix(&self.resource, tail, StructuralPos::is_particle)
                || noun
                    .chars()
                    .next_back()
                    .is_some_and(|last| verify_copula_surface_after_nominal(last, tail))
        };
        if BOUND_NOMINAL_PREFIX_USAGES[index].1.iter().any(|&noun| {
            text.strip_prefix(noun)
                .is_some_and(|tail| accepts_tail(noun, tail))
        }) {
            return true;
        }
        if prefix != "대" {
            return false;
        }
        let mut attested = STDICT_BOUND_DAE_PREFIX_USAGES;
        for (start, character) in text.char_indices().skip(1) {
            let (noun, tail) = text.split_at(start + character.len_utf8());
            attested = &attested[attested.partition_point(|&word| word < noun)..];
            let Some(first) = attested.first() else {
                return false;
            };
            if !first.starts_with(noun) {
                return false;
            }
            if *first == noun && accepts_tail(noun, tail) {
                return true;
            }
        }
        false
    }

    #[must_use]
    pub fn has_source_nominal_prefix(&self, text: &str) -> bool {
        has_exact_sequence(&self.resource, text, &[StructuralPos::XPN])
    }

    #[must_use]
    pub fn has_aligned_nominal_prefix(&self, prefix: &str, text: &str) -> bool {
        if prefix.is_empty() || !text.starts_with(prefix) {
            return false;
        }
        let mut matched = false;
        self.resource
            .common_prefix_analysis_refs(text.as_bytes(), |end, analysis| {
                if matched
                    || !matches!(
                        analysis.positions(),
                        [StructuralPos::XPN, StructuralPos::XR]
                            | [StructuralPos::XPN, StructuralPos::XR, StructuralPos::NNG]
                    )
                    || !analysis
                        .components()
                        .next()
                        .is_some_and(|first| first.span == (0..prefix.len()) && first.pos == "XPN")
                {
                    return;
                }
                let (nominal, tail) = text.split_at(end);
                matched = tail.is_empty()
                    || super::lexical::has_grammatical_particle_chain(nominal, tail)
                    || nominal
                        .chars()
                        .next_back()
                        .is_some_and(|last| verify_copula_surface_after_nominal(last, tail));
            });
        matched
    }

    #[must_use]
    pub fn is_productive_nominal_head(head: &str) -> bool {
        PRODUCTIVE_NOMINAL_HEADS.contains(&head)
    }

    #[must_use]
    pub fn has_productive_nominal_head_prefix(&self, prefix: &str) -> bool {
        let independent_noun =
            |word| has_exact_fine_pos(&self.resource, word, |pos| pos == DataFinePos::Nng);
        if independent_noun(prefix) {
            return true;
        }
        let Some((offset, last)) = prefix.char_indices().next_back() else {
            return false;
        };
        if !crate::hangul::decompose_syllable(last)
            .is_some_and(|syllable| syllable.jongseong == crate::hangul::JONG_SIOT)
        {
            return false;
        }
        let mut original = String::with_capacity(prefix.len());
        original.push_str(&prefix[..offset]);
        original.extend(crate::drop_final(last));
        independent_noun(&original)
    }

    #[must_use]
    pub fn has_unambiguous_final_nominal_component(&self, word: &str, suffix: &str) -> bool {
        let Some(prefix) = word
            .strip_suffix(suffix)
            .filter(|prefix| !prefix.is_empty())
        else {
            return false;
        };
        let mut atomic = false;
        let mut aligned = false;
        self.resource
            .common_prefix_analysis_refs(word.as_bytes(), |length, analysis| {
                if length != word.len() {
                    return;
                }
                if !matches!(analysis.positions(), [pos] if pos.is_nominal()) {
                    return;
                }
                let last = analysis.components().last();
                atomic |= last.is_none();
                aligned |= last.is_some_and(|component| {
                    component.pos == "NNG" && component.span == (prefix.len()..length)
                });
            });
        aligned && !atomic
    }

    #[must_use]
    pub fn has_attested_short_nominal_derivation(
        &self,
        root: &str,
        suffix: &str,
        particle: Option<&str>,
    ) -> bool {
        SHORT_NOMINAL_DERIVATIONS.lines().skip(1).any(|line| {
            let mut fields = line.split('\t');
            fields.next() == Some(root)
                && fields.next() == Some(suffix)
                && particle.is_none_or(|particle| {
                    fields
                        .next_back()
                        .is_some_and(|prefix| prefix == "*" || particle.starts_with(prefix))
                })
        }) && has_exact_sequence(&self.resource, root, &[StructuralPos::NNG])
            && has_exact_sequence(&self.resource, suffix, &[StructuralPos::XSN])
    }

    #[must_use]
    pub fn is_dictionary_numeric_unit(unit: &str) -> bool {
        DICTIONARY_NUMERIC_UNITS.binary_search(&unit).is_ok()
    }

    #[must_use]
    pub fn has_exact_numeric_unit(&self, unit: &str) -> bool {
        if Self::is_dictionary_numeric_unit(unit) {
            return true;
        }
        let mut matched = false;
        self.resource
            .common_prefix_positions(unit.as_bytes(), |length, positions| {
                matched |= length == unit.len()
                    && matches!(positions, [StructuralPos::NNB | StructuralPos::NNBC]);
            });
        matched
    }

    #[must_use]
    pub fn has_prefixed_nominal_derivation_copula(
        &self,
        prefix: &str,
        noun: &str,
        suffix: &str,
    ) -> bool {
        if !has_exact_sequence(&self.resource, prefix, &[StructuralPos::XPN])
            || !has_exact_sequence(&self.resource, noun, &[StructuralPos::NNG])
            || !suffix.char_indices().skip(1).any(|(split, _)| {
                let (derivation, copula) = suffix.split_at(split);
                derivation
                    .chars()
                    .next_back()
                    .is_some_and(|preceding| verify_copula_surface_after_nominal(preceding, copula))
                    && complete_suffix(&self.resource, derivation, |pos| pos == StructuralPos::XSN)
            })
        {
            return false;
        }
        let mut host = String::with_capacity(prefix.len() + noun.len() + suffix.len());
        host.push_str(prefix);
        host.push_str(noun);
        if self.has_exact_nominal_surface(&host) {
            return false;
        }
        host.push_str(suffix);
        !self.has_exact_nominal_surface(&host) && !self.has_whole_modifier(&host)
    }

    #[must_use]
    pub fn has_dependent_noun_subject_path(&self, previous: &str, text: &str) -> bool {
        super::lexical::has_exact_sequence(
            &self.resource,
            text,
            &[StructuralPos::NNB, StructuralPos::JKS],
        ) && (super::lexical::complete_predicate_adnominal_path(&self.resource, previous)
            || super::lexical::complete_derived_adnominal_path(&self.resource, previous)
            || previous
                .strip_suffix('인')
                .is_some_and(|host| self.has_complete_nominal_surface(host)))
    }

    #[must_use]
    pub fn has_numeric_unit_before_dependent_noun(&self, text: &str, unit: &str) -> bool {
        numeric_unit_path_at(&self.resource, text, 0)
            .is_some_and(|path| path.dependent_tail.is_some() && &text[path.unit] == unit)
    }

    #[must_use]
    pub fn has_complete_dictionary_numeric_unit_path(&self, text: &str) -> bool {
        numeric_unit_path_at(&self.resource, text, 0).is_some_and(|path| {
            path.dependent_tail.is_none() && Self::is_dictionary_numeric_unit(&text[path.unit])
        })
    }

    #[must_use]
    pub fn has_complete_attached_counted_unit(&self, text: &str) -> bool {
        numeric_unit_path(&self.resource, text).is_some_and(|path| {
            path.dependent_tail.is_none() && self.has_exact_numeric_unit(&text[path.unit])
        })
    }

    #[must_use]
    pub fn calendar_chain_last_unit(text: &str, unit: Range<usize>) -> Option<Range<usize>> {
        super::lexical::calendar_chain_last_unit(text, Some(unit))
    }

    #[must_use]
    pub fn has_numeric_unit_surface(&self, text: &str) -> bool {
        let digits = text.bytes().take_while(u8::is_ascii_digit).count();
        let prefix_len = if digits == 0 {
            let roman_end = text
                .char_indices()
                .find_map(|(offset, ch)| (!is_roman_digit(ch)).then_some(offset))
                .unwrap_or(text.len());
            if !is_roman_numeral(&text[..roman_end]) {
                return false;
            }
            roman_end
        } else {
            digits
        };
        if prefix_len == text.len() {
            return false;
        }
        Self::is_dictionary_numeric_unit(&text[prefix_len..])
            || (digits != 0
                && text
                    .as_bytes()
                    .get(prefix_len + 3)
                    .is_some_and(u8::is_ascii_digit)
                && super::lexical::calendar_chain_last_unit(text, None)
                    .is_some_and(|unit| unit.end == text.len()))
            || numeric_unit_path_at(&self.resource, text, prefix_len).is_some_and(|path| {
                path.dependent_tail.as_ref().unwrap_or(&path.unit).end == text.len()
                    && exact_analysis_starts_with_pos(&self.resource, &text[path.unit], |pos| {
                        matches!(pos, StructuralPos::NNB | StructuralPos::NNBC)
                    })
            })
    }

    #[must_use]
    pub fn is_roman_unit_prefix(text: &str) -> bool {
        is_roman_numeral(text)
    }

    #[must_use]
    pub fn has_object_marked_nominal(&self, text: &str) -> bool {
        has_complete_object_phrase(&self.resource, text)
    }

    #[must_use]
    pub fn has_subject_marked_nominal(&self, text: &str) -> bool {
        has_exact_sequence(
            &self.resource,
            text,
            &[StructuralPos::NP, StructuralPos::JKS],
        ) || text.strip_suffix(['이', '가']).is_some_and(|host| {
            self.has_complete_nominal_surface(host)
                && has_exact_sequence(&self.resource, &text[host.len()..], &[StructuralPos::JKS])
        })
    }

    #[must_use]
    pub fn has_subject_or_topic_marked_proper_name(&self, text: &str) -> bool {
        let Some((host, position)) = text
            .strip_suffix(['이', '가'])
            .map(|host| (host, StructuralPos::JKS))
            .or_else(|| {
                text.strip_suffix(['은', '는'])
                    .map(|host| (host, StructuralPos::JX))
            })
        else {
            return false;
        };
        has_exact_sequence(&self.resource, host, &[StructuralPos::NNP])
            && has_exact_sequence(&self.resource, &text[host.len()..], &[position])
    }

    #[must_use]
    pub fn has_adnominal_result_complement(&self, text: &str) -> bool {
        has_adnominal_result_complement(&self.resource, text)
    }

    #[must_use]
    pub fn has_exact_conjunctive_adverb(&self, text: &str) -> bool {
        let mut matched = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                matched |= length == text.len() && positions == [StructuralPos::MAJ];
            });
        matched
    }

    #[must_use]
    pub fn has_alternative_connective(&self, text: &str) -> bool {
        let Some(stem) = text.strip_suffix("거나") else {
            return false;
        };
        has_exact_sequence(&self.resource, "거나", &[StructuralPos::EC])
            && (exact_analysis_starts_with_pos(
                &self.resource,
                stem,
                StructuralPos::is_predicate_tag,
            ) || stem.strip_suffix('이').is_some_and(|noun| {
                has_exact_sequence(&self.resource, "이", &[StructuralPos::VCP])
                    && self.has_complete_nominal_surface(noun)
            }))
    }

    #[must_use]
    pub fn predicate_choice_stem<'a>(&self, text: &'a str) -> Option<&'a str> {
        ["든지", "든가", "든"].into_iter().find_map(|suffix| {
            let stem = text.strip_suffix(suffix)?;
            (has_exact_sequence(&self.resource, suffix, &[StructuralPos::EC])
                && predicate_inflected_path(
                    &self.resource,
                    text,
                    StructuralPos::is_predicate_tag,
                    |pos| pos == StructuralPos::EC,
                ))
            .then_some(stem)
        })
    }

    #[must_use]
    pub fn has_whole_modifier(&self, text: &str) -> bool {
        let mut matched = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length == text.len() {
                    matched |= positions.first().is_some_and(|pos| {
                        matches!(
                            *pos,
                            StructuralPos::MM | StructuralPos::MAG | StructuralPos::MAJ
                        )
                    });
                }
            });
        matched
    }

    #[must_use]
    pub fn has_whole_determiner(&self, text: &str) -> bool {
        has_exact_sequence(&self.resource, text, &[StructuralPos::MM])
    }

    #[must_use]
    pub fn has_exact_adverb_surface(&self, text: &str) -> bool {
        has_exact_fine_pos(&self.resource, text, |pos| pos == DataFinePos::Mag)
    }

    #[must_use]
    pub fn has_adverb_topic_contraction(&self, adverb: &str, contracted: &str) -> bool {
        (has_exact_sequence(&self.resource, adverb, &[StructuralPos::MAG])
            || has_exact_sequence(&self.resource, adverb, &[StructuralPos::MAJ]))
            && has_exact_sequence(
                &self.resource,
                contracted,
                &[StructuralPos::MAG, StructuralPos::JX],
            )
    }

    #[must_use]
    pub fn has_pronoun_topic_contraction(&self, text: &str) -> bool {
        has_exact_sequence(
            &self.resource,
            text,
            &[StructuralPos::NP, StructuralPos::JX],
        )
    }

    #[must_use]
    pub fn has_pronoun_object_contraction(&self, text: &str) -> bool {
        has_exact_sequence(
            &self.resource,
            text,
            &[StructuralPos::NP, StructuralPos::JKO],
        )
    }

    #[must_use]
    pub fn has_boi_predicate(&self, text: &str) -> bool {
        static FORMS: std::sync::OnceLock<Vec<crate::SurfaceBranchSpec>> =
            std::sync::OnceLock::new();
        let forms = FORMS.get_or_init(|| {
            let entry = crate::PredicateEntry::new(
                "보이다",
                PredicatePos::Verb,
                LexicalAlternation::Regular,
            );
            crate::generate_predicate_branches(&entry).expect("valid dictionary predicate")
        });
        matches_predicate_inflection(text, forms)
            && (self.has_exact_predicate_ending_path(text, PredicatePos::Verb)
                || complete_predicate_ending_path(&self.resource, text))
    }

    #[must_use]
    pub fn has_request_auxiliary_host(&self, text: &str) -> bool {
        has_exact_sequence(
            &self.resource,
            text,
            &[StructuralPos::VV, StructuralPos::EC],
        ) || text.strip_suffix(['아', '어', '여']).is_some_and(|stem| {
            has_exact_sequence(&self.resource, stem, &[StructuralPos::VV])
                && has_exact_sequence(&self.resource, &text[stem.len()..], &[StructuralPos::EC])
        }) || text.strip_suffix('해').is_some_and(|root| {
            !root.is_empty()
                && self.has_complete_nominal_surface(root)
                && has_exact_sequence(
                    &self.resource,
                    "해",
                    &[StructuralPos::XSV, StructuralPos::EC],
                )
        })
    }

    #[must_use]
    pub fn has_vowel_connective_host(&self, text: &str) -> bool {
        has_exact_sequence(
            &self.resource,
            text,
            &[StructuralPos::VV, StructuralPos::EC],
        ) && !has_exact_sequence(&self.resource, text, &[StructuralPos::VV])
    }

    #[must_use]
    pub fn supports_predicate_ending_path(
        &self,
        text: &str,
        anchor_len: usize,
        pos: PredicatePos,
        node_limit: usize,
    ) -> bool {
        self.supports_predicate_ending_path_with_terminal(
            text, anchor_len, pos, node_limit, None, false,
        )
    }

    #[must_use]
    pub fn has_exact_predicate_ending_path(&self, text: &str, pos: PredicatePos) -> bool {
        self.has_exact_predicate_ending_path_matching(
            text,
            |actual| structural_predicate_pos_matches(actual, pos),
            |_| true,
        )
    }

    #[must_use]
    pub fn has_exact_predicate_nominalization(&self, text: &str) -> bool {
        self.has_exact_predicate_ending_path_matching(
            text,
            |pos| {
                matches!(
                    pos,
                    StructuralPos::VV | StructuralPos::VA | StructuralPos::VX
                )
            },
            |ending| ending == StructuralPos::ETN,
        )
    }

    // 한국어기초사전 전(前), 17301의 '-기 전' 결합.
    #[must_use]
    pub fn has_temporal_before_frame(&self, previous: &str) -> bool {
        has_exact_sequence(&self.resource, "전", &[StructuralPos::NNG])
            && !self.has_exact_nominal_surface(previous)
            && self.has_ki_nominalization(previous)
    }

    #[must_use]
    pub fn has_ki_nominalization(&self, previous: &str) -> bool {
        let nominalizer = |pos| pos == StructuralPos::ETN;
        previous.ends_with('기')
            && (predicate_inflected_path(
                &self.resource,
                previous,
                |pos| {
                    matches!(
                        pos,
                        StructuralPos::VV
                            | StructuralPos::VA
                            | StructuralPos::VX
                            | StructuralPos::VCN
                    )
                },
                nominalizer,
            ) || derived_inflected_path(&self.resource, previous, |_| true, nominalizer)
                || previous.char_indices().skip(1).any(|(split, _)| {
                    let (host, copula) = previous.split_at(split);
                    predicate_inflected_path(
                        &self.resource,
                        copula,
                        |pos| pos == StructuralPos::VCP,
                        nominalizer,
                    ) && complete_nominal_host(&self.resource, host)
                }))
    }

    #[must_use]
    pub fn has_adverbial_copula_predicate(&self, text: &str) -> bool {
        text.char_indices().skip(1).any(|(split, initial)| {
            matches!(initial, '이' | '였' | '예' | '여')
                && text[..split].chars().next_back().is_some_and(|host_end| {
                    verify_copula_surface_after_nominal(host_end, &text[split..])
                })
                && self.has_exact_adverb_surface(&text[..split])
        })
    }

    #[must_use]
    pub fn has_negative_copula_after_focus(&self, text: &str) -> bool {
        has_exact_sequence(&self.resource, "가", &[StructuralPos::JX])
            && (self.has_exact_predicate_ending_path_matching(
                text,
                |pos| pos == StructuralPos::VCN,
                |_| true,
            ) || text
                .strip_prefix("아니")
                .filter(|suffix| !suffix.is_empty())
                .is_some_and(|suffix| {
                    has_exact_sequence(&self.resource, "아니", &[StructuralPos::VCN])
                        && complete_suffix(&self.resource, suffix, StructuralPos::is_ending)
                }))
    }

    #[must_use]
    pub fn has_exact_predicate_connective_path(
        &self,
        text: &str,
        source_positions: PredicatePosSet,
    ) -> bool {
        self.has_exact_predicate_ending_path_matching(
            text,
            |actual| {
                source_positions
                    .iter()
                    .any(|pos| structural_predicate_pos_matches(actual, pos))
            },
            |last| last == StructuralPos::EC,
        )
    }

    #[must_use]
    pub fn has_purpose_connective_host(&self, text: &str) -> bool {
        let Some(stem) = text.strip_suffix('러') else {
            return false;
        };
        let bare_stem = |stem: &str| has_exact_sequence(&self.resource, stem, &[StructuralPos::VV]);
        let with_eu = stem.strip_suffix('으').is_some_and(|base| {
            base.chars()
                .next_back()
                .is_some_and(|last| crate::has_final(last) && !crate::has_rieul_final(last))
                && bare_stem(base)
        });
        let last = stem.chars().next_back();
        let without_eu = last
            .is_some_and(|last| !crate::has_final(last) || crate::has_rieul_final(last))
            && bare_stem(stem);
        // The vowel connective can also surface in this position, as in 눌러 and 이르러.
        let vowel_connective_possible = last.is_some_and(crate::has_rieul_final)
            && self.has_vowel_connective_host(text)
            || last == Some('르')
                && purpose_host_alternations()
                    .iter()
                    .any(|(alternation, form)| {
                        *alternation == LexicalAlternation::Reo
                            && form.anchor.as_ref() == text
                            && form
                                .rule_path
                                .iter()
                                .any(|rule| rule.as_str() == "lexical.reo")
                    });
        (with_eu
            || without_eu
            || stem.ends_with('으')
                && purpose_host_alternations()
                    .iter()
                    .any(|(alternation, form)| {
                        *alternation != LexicalAlternation::Reo
                            && matches_predicate_branch_with_ending(text, form, |rule| {
                                rule.is_some_and(|rule| rule.as_str() == "ending.purpose-reo")
                            })
                    })
            || derived_inflected_path(
                &self.resource,
                text,
                |pos| pos == StructuralPos::XSV,
                |pos| pos == StructuralPos::EC,
            ))
            && !vowel_connective_possible
            && !self.has_exact_nominal_surface(text)
    }

    #[must_use]
    pub fn has_motion_go_inflection(text: &str) -> bool {
        static FORMS: std::sync::OnceLock<Vec<crate::SurfaceBranchSpec>> =
            std::sync::OnceLock::new();
        let forms = FORMS.get_or_init(|| {
            let entry =
                crate::PredicateEntry::new("가다", PredicatePos::Verb, LexicalAlternation::Regular);
            crate::generate_predicate_branches(&entry).expect("valid motion predicate")
        });
        matches_predicate_inflection(text, forms)
    }

    /// Verifies the purchase verb 사다 (KRDict 49419).
    #[must_use]
    pub fn has_purchase_inflection(text: &str) -> bool {
        static FORMS: std::sync::LazyLock<Vec<crate::SurfaceBranchSpec>> =
            std::sync::LazyLock::new(|| {
                let entry = crate::PredicateEntry::new(
                    "사다",
                    PredicatePos::Verb,
                    LexicalAlternation::Regular,
                );
                crate::generate_predicate_branches(&entry).expect("valid purchase predicate")
            });
        matches_predicate_inflection(text, &FORMS)
    }

    /// Verifies the independent possession predicate 가지다 (KRDict 65234).
    #[must_use]
    pub fn has_possession_inflection(text: &str) -> bool {
        static FORMS: std::sync::LazyLock<Vec<crate::SurfaceBranchSpec>> =
            std::sync::LazyLock::new(|| {
                let entry = crate::PredicateEntry::new(
                    "가지다",
                    PredicatePos::Verb,
                    LexicalAlternation::Regular,
                );
                crate::generate_predicate_branches(&entry).expect("valid possession predicate")
            });
        matches_predicate_inflection(text, &FORMS)
    }

    #[must_use]
    pub fn has_negative_connective_host(&self, text: &str) -> bool {
        let Some(stem) = text.strip_suffix('지') else {
            return false;
        };
        let positions = PredicatePosSet::one(PredicatePos::Verb)
            | PredicatePosSet::one(PredicatePos::Adjective)
            | PredicatePosSet::one(PredicatePos::AuxiliaryVerb)
            | PredicatePosSet::one(PredicatePos::AuxiliaryAdjective);
        self.has_exact_predicate_connective_path(text, positions)
            || (!stem.is_empty()
                && (self.has_exact_non_copula_predicate_surface(stem)
                    || stem
                        .strip_suffix('하')
                        .is_some_and(|host| self.has_complete_nominal_surface(host))))
            || derived_inflected_path(
                &self.resource,
                text,
                |_| true,
                |pos| pos == StructuralPos::EC,
            )
    }

    #[must_use]
    pub fn has_negative_resultative_host(&self, text: &str) -> bool {
        let Some(host) = text.strip_suffix("지지") else {
            return false;
        };
        !self.has_exact_nominal_surface(text)
            && [PredicatePos::Verb, PredicatePos::Adjective]
                .into_iter()
                .any(|pos| {
                    self.has_exact_predicate_connective_path(host, PredicatePosSet::one(pos))
                        && self.has_attached_resultative_inflection("지지", pos)
                })
    }

    fn has_exact_predicate_ending_path_matching(
        &self,
        text: &str,
        accepts_predicate: impl Fn(StructuralPos) -> bool,
        accepts_last_ending: impl Fn(StructuralPos) -> bool,
    ) -> bool {
        let mut matched = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length != text.len() {
                    return;
                }
                let Some((first, endings)) = positions.split_first() else {
                    return;
                };
                matched |= accepts_predicate(*first)
                    && endings.last().copied().is_some_and(&accepts_last_ending)
                    && endings.iter().all(|position| position.is_ending());
            });
        matched
    }

    #[must_use]
    pub fn has_exact_pronoun_copula_ending_path(&self, text: &str) -> bool {
        let mut matched = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length != text.len() {
                    return;
                }
                matched |= positions.len() >= 3
                    && positions[0] == StructuralPos::NP
                    && positions[1] == StructuralPos::VCP
                    && positions[2..].iter().all(|pos| pos.is_ending())
                    && positions
                        .last()
                        .is_some_and(|pos| matches!(*pos, StructuralPos::EC | StructuralPos::EF));
            });
        matched
    }

    #[must_use]
    pub fn has_exact_dependent_copula_ending_path(&self, text: &str) -> bool {
        has_exact_sequence(
            &self.resource,
            text,
            &[StructuralPos::NNB, StructuralPos::VCP, StructuralPos::EF],
        )
    }

    #[must_use]
    pub fn has_exact_lost_span_copula_ending_path(&self, text: &str) -> bool {
        let mut matched = false;
        let mut whole_adverb = false;
        self.resource
            .common_prefix_analysis_refs(text.as_bytes(), |length, analysis| {
                if length != text.len() {
                    return;
                }
                let positions = analysis.positions();
                whole_adverb |= matches!(positions, [StructuralPos::MAG | StructuralPos::MAJ]);
                let Some(copula_index) =
                    positions.iter().position(|pos| *pos == StructuralPos::VCP)
                else {
                    return;
                };
                matched |= copula_index > 0
                    && positions[..copula_index].iter().all(|pos| pos.is_nominal())
                    && positions[copula_index + 1..]
                        .iter()
                        .all(|pos| pos.is_ending())
                    && positions
                        .last()
                        .is_some_and(|pos| matches!(*pos, StructuralPos::EC | StructuralPos::EF))
                    && !analysis
                        .components()
                        .any(|component| component.pos == "VCP");
            });
        matched && !whole_adverb
    }

    fn supports_predicate_ending_path_with_terminal(
        &self,
        text: &str,
        anchor_len: usize,
        pos: PredicatePos,
        node_limit: usize,
        required_terminal: Option<StructuralPos>,
        allow_complete_anchor: bool,
    ) -> bool {
        if anchor_len == 0
            || if allow_complete_anchor {
                anchor_len > text.len()
            } else {
                anchor_len >= text.len()
            }
            || !text.is_char_boundary(anchor_len)
        {
            return false;
        }
        let mut visited = vec![[[false; 2]; 2]; text.len() + 1];
        let mut pending = Vec::new();
        let mut nodes = 0;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length != anchor_len {
                    return;
                }
                nodes += 1;
                let Some((first, endings)) = positions.split_first() else {
                    return;
                };
                if structural_predicate_pos_matches(*first, pos)
                    && endings.iter().all(|ending| ending.is_ending())
                {
                    let has_ending = !endings.is_empty();
                    let terminal_matches = endings
                        .last()
                        .is_some_and(|ending| required_terminal.is_none_or(|tag| *ending == tag));
                    if !visited[length][usize::from(has_ending)][usize::from(terminal_matches)] {
                        visited[length][usize::from(has_ending)][usize::from(terminal_matches)] =
                            true;
                        pending.push((length, has_ending, terminal_matches));
                    }
                }
            });
        while let Some((start, has_ending, terminal_matches)) = pending.pop() {
            if nodes > node_limit {
                return false;
            }
            if start == text.len()
                && has_ending
                && required_terminal.is_none_or(|_| terminal_matches)
            {
                return true;
            }
            self.resource
                .common_prefix_positions(&text.as_bytes()[start..], |length, endings| {
                    if length == 0 || start + length > text.len() {
                        return;
                    }
                    nodes += 1;
                    if !endings.is_empty() && endings.iter().all(|ending| ending.is_ending()) {
                        let end = start + length;
                        let terminal_matches = endings.last().is_some_and(|ending| {
                            required_terminal.is_none_or(|tag| *ending == tag)
                        });
                        if !visited[end][1][usize::from(terminal_matches)] {
                            visited[end][1][usize::from(terminal_matches)] = true;
                            pending.push((end, true, terminal_matches));
                        }
                    }
                });
        }
        false
    }

    #[must_use]
    pub fn supports_adnominal_dependent_noun_particle_path(
        &self,
        text: &str,
        anchor_len: usize,
        adnominal_len: usize,
        pos: PredicatePos,
        node_limit: usize,
    ) -> bool {
        if anchor_len == 0
            || anchor_len > adnominal_len
            || adnominal_len >= text.len()
            || !text.is_char_boundary(anchor_len)
            || !text.is_char_boundary(adnominal_len)
        {
            return false;
        }
        self.supports_predicate_ending_path_with_terminal(
            &text[..adnominal_len],
            anchor_len,
            pos,
            node_limit,
            Some(StructuralPos::ETM),
            true,
        ) && complete_dependent_noun_particle_suffix(
            &self.resource,
            &text[adnominal_len..],
            node_limit,
        )
    }

    #[must_use]
    pub fn supports_predicate_ending_particle_path(
        &self,
        text: &str,
        anchor_len: usize,
        ending_len: usize,
        pos: PredicatePos,
        node_limit: usize,
    ) -> bool {
        if anchor_len == 0
            || anchor_len >= ending_len
            || ending_len >= text.len()
            || !text.is_char_boundary(anchor_len)
            || !text.is_char_boundary(ending_len)
        {
            return false;
        }
        self.supports_predicate_ending_path(&text[..ending_len], anchor_len, pos, node_limit)
            && complete_suffix(
                &self.resource,
                &text[ending_len..],
                StructuralPos::is_particle,
            )
    }

    #[must_use]
    pub fn supports_ending_suffix_path(&self, text: &str, start: usize, node_limit: usize) -> bool {
        if start >= text.len() || !text.is_char_boundary(start) {
            return false;
        }
        let mut visited = vec![false; text.len() + 1];
        let mut pending = vec![start];
        let mut nodes = 0;
        while let Some(position) = pending.pop() {
            if nodes > node_limit {
                return false;
            }
            if position == text.len() {
                return true;
            }
            self.resource.common_prefix_positions(
                &text.as_bytes()[position..],
                |length, positions| {
                    if length == 0 || position + length > text.len() {
                        return;
                    }
                    nodes += 1;
                    if positions.iter().all(|pos| pos.is_ending()) {
                        let end = position + length;
                        if !visited[end] {
                            visited[end] = true;
                            pending.push(end);
                        }
                    }
                },
            );
        }
        false
    }

    #[must_use]
    pub fn has_exact_ending_surface(&self, text: &str) -> bool {
        let mut supported = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length == text.len()
                    && !positions.is_empty()
                    && positions.iter().all(|pos| pos.is_ending())
                {
                    supported = true;
                }
            });
        supported
    }

    #[must_use]
    pub fn has_exact_nominalizing_ending_surface(&self, text: &str) -> bool {
        let mut supported = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length == text.len()
                    && !positions.is_empty()
                    && positions.iter().all(|pos| pos.is_ending())
                    && positions.last() == Some(&StructuralPos::ETN)
                {
                    supported = true;
                }
            });
        supported
    }

    #[must_use]
    pub fn auxiliary_splits(&self, text: &str) -> Vec<usize> {
        let mut splits = Vec::new();
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                let Some((first, endings)) = positions.split_first() else {
                    return;
                };
                if *first != StructuralPos::VX || !endings.iter().all(|pos| pos.is_ending()) {
                    return;
                }
                if length == text.len() || endings.is_empty() {
                    splits.push(length);
                }
            });
        splits.sort_unstable();
        splits.dedup();
        splits
    }

    #[must_use]
    pub fn supports_auxiliary_sequence(&self, text: &str, node_limit: usize) -> bool {
        self.supports_auxiliary_sequence_matching(text, node_limit, |_| true, |_| true)
    }

    #[must_use]
    pub fn has_vowel_auxiliary_inflection(&self, text: &str, host_pos: PredicatePos) -> bool {
        self.has_matching_vowel_auxiliary_inflection(text, host_pos, |_| true, |_| true)
    }

    /// Verifies the attached result-state auxiliary 있다 (KRDict 62595).
    #[must_use]
    pub fn has_result_state_auxiliary_frame(&self, host: &str, auxiliary: &str) -> bool {
        use crate::hangul::{
            JONG_NONE, JUNG_A, JUNG_AE, JUNG_E, JUNG_EO, JUNG_WA, JUNG_WAE, JUNG_WEO, JUNG_YE,
            JUNG_YEO, decompose_syllable,
        };
        let is_vowel_connective = host
            .chars()
            .next_back()
            .and_then(decompose_syllable)
            .is_some_and(|syllable| {
                syllable.jongseong == JONG_NONE
                    && matches!(
                        syllable.jungseong,
                        JUNG_A
                            | JUNG_AE
                            | JUNG_E
                            | JUNG_EO
                            | JUNG_WA
                            | JUNG_WAE
                            | JUNG_WEO
                            | JUNG_YE
                            | JUNG_YEO
                    )
            });
        let connective = |pos| pos == StructuralPos::EC;
        is_vowel_connective
            && self.has_matching_vowel_auxiliary_inflection(
                auxiliary,
                PredicatePos::Verb,
                |lemma| lemma == "있다",
                |_| true,
            )
            && (self.has_exact_predicate_connective_path(
                host,
                PredicatePosSet::one(PredicatePos::Verb),
            ) || (host.ends_with(['아', '어', '여'])
                && predicate_inflected_path(
                    &self.resource,
                    host,
                    |pos| pos == StructuralPos::VV,
                    connective,
                ))
                || (host.chars().count() <= 2
                    && derived_inflected_path(
                        &self.resource,
                        host,
                        |pos| pos == StructuralPos::XSV,
                        connective,
                    )))
    }

    #[must_use]
    pub fn has_attached_resultative_inflection(&self, text: &str, host_pos: PredicatePos) -> bool {
        self.has_matching_vowel_auxiliary_inflection(
            text,
            host_pos,
            |lemma| lemma == ATTACHED_RESULTATIVE_LEMMA,
            |_| true,
        )
    }

    #[must_use]
    pub fn has_progressive_or_desiderative_ending(
        &self,
        connective: &str,
        auxiliary: &str,
    ) -> bool {
        if !self.has_matching_vowel_auxiliary_inflection(
            connective,
            PredicatePos::Verb,
            |_| true,
            |ending| ending.is_some_and(|rule| rule.as_str() == "ending.connective-go"),
        ) {
            return false;
        }
        matches_predicate_inflection(auxiliary, go_auxiliary_inflections())
            && self.supports_auxiliary_sequence_matching(
                auxiliary,
                crate::DEFAULT_LATTICE_NODE_LIMIT,
                |_| true,
                |positions| positions.last() == Some(&StructuralPos::EF),
            )
    }

    #[must_use]
    pub fn has_progressive_auxiliary_inflection(&self, text: &str) -> bool {
        go_auxiliary_inflections().iter().any(|form| {
            form.pos == PredicatePos::AuxiliaryVerb && matches_predicate_branch(text, form)
        })
    }

    fn has_matching_vowel_auxiliary_inflection(
        &self,
        text: &str,
        host_pos: PredicatePos,
        accepts_lemma: impl Fn(&str) -> bool,
        accepts_ending: impl Fn(Option<&crate::RuleId>) -> bool,
    ) -> bool {
        let Some(initial) = text.chars().next() else {
            return false;
        };
        let forms = vowel_auxiliary_inflections();
        let start = forms.partition_point(|entry| entry.form.anchor.chars().next() < Some(initial));
        forms[start..]
            .iter()
            .take_while(|entry| entry.form.anchor.starts_with(initial))
            .any(|VowelAuxiliaryInflection { lemma, host, form }| {
                accepts_lemma(lemma)
                    && host.accepts(host_pos)
                    && (form.anchor.as_ref() != text
                        || !matches!(
                            form.continuation,
                            crate::ContinuationState::Past | crate::ContinuationState::Future
                        ))
                    && matches_predicate_branch_with_ending(text, form, &accepts_ending)
            })
    }

    #[must_use]
    pub fn has_adnominal_auxiliary_tail(
        &self,
        text: &str,
        future: bool,
        present_or_past: bool,
    ) -> bool {
        if !ADNOMINAL_AUXILIARIES
            .iter()
            .any(|(lemma, _, _, _, _)| text.chars().next() == lemma.chars().next())
        {
            return false;
        }
        let forms = auxiliary_inflections();
        (future && matches_predicate_inflection(text, &forms.future))
            || (present_or_past && matches_predicate_inflection(text, &forms.present_or_past))
    }

    #[must_use]
    pub fn has_interrogative_auxiliary_surface(&self, text: &str) -> bool {
        matches_interrogative_auxiliary_inflection(text)
    }

    #[must_use]
    pub fn supports_auxiliary_with_fused_prefinal(&self, text: &str, node_limit: usize) -> bool {
        self.supports_auxiliary_sequence_matching(
            text,
            node_limit,
            |endings| endings.first() == Some(&StructuralPos::EP),
            |_| true,
        )
    }

    fn supports_auxiliary_sequence_matching(
        &self,
        text: &str,
        node_limit: usize,
        accepts_auxiliary_endings: impl Fn(&[StructuralPos]) -> bool,
        accepts_last_segment: impl Fn(&[StructuralPos]) -> bool,
    ) -> bool {
        if text.is_empty() {
            return false;
        }
        let mut visited = vec![false; text.len() + 1];
        let mut pending = vec![0];
        let mut nodes = 0;
        while let Some(start) = pending.pop() {
            if nodes > node_limit {
                return false;
            }
            if start == text.len() && start > 0 {
                return true;
            }
            self.resource.common_prefix_positions(
                &text.as_bytes()[start..],
                |length, positions| {
                    if length == 0 || start + length > text.len() {
                        return;
                    }
                    nodes += 1;
                    let allowed = if start == 0 {
                        positions.first() == Some(&StructuralPos::VX)
                            && positions[1..].iter().all(|pos| pos.is_ending())
                            && accepts_auxiliary_endings(&positions[1..])
                    } else {
                        positions.iter().all(|pos| pos.is_ending())
                    };
                    if allowed {
                        let end = start + length;
                        if !visited[end] && (end != text.len() || accepts_last_segment(positions)) {
                            visited[end] = true;
                            pending.push(end);
                        }
                    }
                },
            );
        }
        false
    }

    #[must_use]
    pub fn whole_predicate_conflicts(
        &self,
        text: &str,
        anchor_len: usize,
        pos: PredicatePos,
    ) -> bool {
        self.whole_predicate_conflicts_at(text, 0..anchor_len, pos)
    }

    #[must_use]
    pub fn whole_predicate_conflicts_at(
        &self,
        text: &str,
        anchor: Range<usize>,
        pos: PredicatePos,
    ) -> bool {
        self.whole_predicate_conflicts_matching(text, anchor, pos, |_| true)
    }

    #[must_use]
    pub fn whole_predicate_conflicts_without_auxiliary(
        &self,
        text: &str,
        anchor_len: usize,
        pos: PredicatePos,
    ) -> bool {
        self.whole_predicate_conflicts_matching(text, 0..anchor_len, pos, |positions| {
            !is_attached_auxiliary_whole_path(positions)
        })
    }

    fn whole_predicate_conflicts_matching(
        &self,
        text: &str,
        anchor: Range<usize>,
        pos: PredicatePos,
        competes: impl Fn(&[StructuralPos]) -> bool,
    ) -> bool {
        if anchor.is_empty()
            || anchor.end > text.len()
            || !text.is_char_boundary(anchor.start)
            || !text.is_char_boundary(anchor.end)
        {
            return false;
        }
        let mut whole_predicate = false;
        let mut aligned_query_stem = false;
        self.resource
            .common_prefix_analysis_refs(text.as_bytes(), |length, analysis| {
                if length != text.len() {
                    return;
                }
                let Some(first) = analysis.positions().first() else {
                    return;
                };
                if !first.is_predicate() {
                    return;
                }
                whole_predicate |= competes(analysis.positions());
                aligned_query_stem |= analysis.components().any(|component| {
                    component.span == anchor && predicate_pos_matches(component.pos, pos)
                });
            });
        whole_predicate && !aligned_query_stem
    }

    #[must_use]
    pub fn has_source_aligned_compound_predicate_component(
        &self,
        text: &str,
        anchor: Range<usize>,
        source_positions: PredicatePosSet,
        node_limit: usize,
    ) -> bool {
        if anchor.is_empty()
            || anchor.start == 0
            || anchor.end > text.len()
            || !text.is_char_boundary(anchor.start)
            || !text.is_char_boundary(anchor.end)
        {
            return false;
        }
        TokenEvidence::collect(&self.resource, text, node_limit, false, false, false).is_ok_and(
            |evidence| {
                !evidence.has_whole_modifier()
                    && evidence
                        .compound_predicate_components
                        .iter()
                        .filter(|component| {
                            component.start == anchor.start && component.end == anchor.end
                        })
                        .any(|component| {
                            source_positions
                                .iter()
                                .any(|pos| predicate_pos_matches(component.pos.as_str(), pos))
                        })
            },
        )
    }

    #[must_use]
    pub fn has_attached_auxiliary_whole_path(&self, text: &str) -> bool {
        let mut supported = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length == text.len() {
                    supported |= is_attached_auxiliary_whole_path(positions);
                }
            });
        supported
    }

    #[must_use]
    pub fn has_unambiguous_predicate_surface(
        &self,
        text: &str,
        source_positions: PredicatePosSet,
    ) -> bool {
        let mut found = false;
        let mut conflicting = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length != text.len() {
                    return;
                }
                let compatible = positions.split_first().is_some_and(|(first, endings)| {
                    source_positions
                        .iter()
                        .any(|pos| structural_predicate_pos_matches(*first, pos))
                        && endings.iter().all(|pos| pos.is_ending())
                });
                let predicate_only = positions.split_first().is_some_and(|(first, endings)| {
                    first.is_predicate() && endings.iter().all(|pos| pos.is_ending())
                });
                found |= compatible;
                conflicting |= !predicate_only;
            });
        found && !conflicting
    }

    #[must_use]
    pub fn has_derived_predicate_connective_surface(
        &self,
        text: &str,
        source_positions: PredicatePosSet,
    ) -> bool {
        let mut found = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, root| {
                if length >= text.len() || !matches!(root, [StructuralPos::NNG | StructuralPos::XR])
                {
                    return;
                }
                let tail = &text[length..];
                found |= (source_positions.contains(PredicatePos::Verb)
                    && has_exact_sequence(
                        &self.resource,
                        tail,
                        &[StructuralPos::XSV, StructuralPos::EC],
                    ))
                    || (source_positions.contains(PredicatePos::Adjective)
                        && has_exact_sequence(
                            &self.resource,
                            tail,
                            &[StructuralPos::XSA, StructuralPos::EC],
                        ));
            });
        found
    }

    #[must_use]
    pub fn has_conflicting_derived_ha_suffix(
        &self,
        text: &str,
        anchor: Range<usize>,
        source_positions: PredicatePosSet,
    ) -> bool {
        let Some((host, suffix)) = text.split_at_checked(anchor.start) else {
            return false;
        };
        if !self.has_inflected_derivational_suffix(suffix, ['하', '해'], false) {
            return false;
        }
        let mut derived_root = false;
        let mut aligned_compound_predicate = false;
        self.resource
            .common_prefix_analysis_refs(text.as_bytes(), |length, analysis| {
                if length == anchor.start {
                    derived_root |= matches!(
                        analysis.positions(),
                        [StructuralPos::NNG | StructuralPos::XR]
                    );
                }
                if length != text.len() {
                    return;
                }
                aligned_compound_predicate |= compound_predicate_pos(analysis.positions())
                    .is_some_and(|pos| {
                        source_positions
                            .iter()
                            .any(|source| predicate_pos_matches(pos.as_str(), source))
                            && analysis.components().any(|component| {
                                component.span == anchor && component.pos == pos.as_str()
                            })
                    });
            });
        derived_root
            && !aligned_compound_predicate
            && !(host.ends_with("하여") && complete_connective_path(&self.resource, host))
    }

    /// Validates the productive passive suffix -되- after an action noun (KRDict 74902).
    #[must_use]
    pub fn has_conflicting_passive_suffix(&self, text: &str, anchor: Range<usize>) -> bool {
        let Some((host, suffix)) = text.split_at_checked(anchor.start) else {
            return false;
        };
        if !self.is_attested_do_object(host)
            || !self.has_inflected_derivational_suffix(suffix, ['되', '돼'], false)
        {
            return false;
        }
        let mut aligned_verb = false;
        self.resource
            .common_prefix_analysis_refs(text.as_bytes(), |length, analysis| {
                aligned_verb |= length == text.len()
                    && compound_predicate_pos(analysis.positions()) == Some(DataFinePos::Vv)
                    && analysis
                        .components()
                        .any(|component| component.span == anchor && component.pos == "VV");
            });
        !aligned_verb
    }

    #[must_use]
    pub fn has_derived_ha_predicate_crossing(&self, text: &str, prefix_end: usize) -> bool {
        let mut found = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |root_end, root| {
                if found
                    || root_end <= prefix_end
                    || root_end == text.len()
                    || !matches!(root, [StructuralPos::NNG | StructuralPos::XR])
                {
                    return;
                }
                found =
                    self.has_inflected_derivational_suffix(&text[root_end..], ['하', '해'], true);
            });
        found
    }

    fn has_inflected_derivational_suffix(
        &self,
        tail: &str,
        stem_forms: [char; 2],
        allow_joined_head: bool,
    ) -> bool {
        if !tail.chars().next().is_some_and(|initial| {
            crate::hangul::replace_final(initial, crate::hangul::JONG_NONE)
                .is_some_and(|base| stem_forms.contains(&base))
        }) {
            return false;
        }
        let mut found = false;
        self.resource
            .common_prefix_positions(tail.as_bytes(), |end, positions| {
                let [StructuralPos::XSV | StructuralPos::XSA, endings @ ..] = positions else {
                    return;
                };
                if endings.is_empty()
                    && !stem_forms
                        .iter()
                        .any(|stem| end == stem.len_utf8() && tail.starts_with(*stem))
                {
                    return;
                }
                if found {
                    return;
                }
                let remaining = &tail[end..];
                found = complete_inflected_ending(&self.resource, endings, remaining, |pos| {
                    matches!(
                        pos,
                        StructuralPos::EF
                            | StructuralPos::EC
                            | StructuralPos::ETM
                            | StructuralPos::ETN
                    )
                }) || remaining.char_indices().any(|(split, _)| {
                    let (ending, particles) = remaining.split_at(split);
                    complete_inflected_ending(&self.resource, endings, ending, |pos| {
                        pos == StructuralPos::ETN
                    }) && complete_suffix(&self.resource, particles, StructuralPos::is_particle)
                }) || (allow_joined_head
                    && remaining.char_indices().any(|(split, _)| {
                        let (predicate, noun) = tail.split_at(end + split);
                        predicate.chars().next_back().is_some_and(|last| {
                            crate::hangul::decompose_syllable(last).is_some_and(|syllable| {
                                matches!(
                                    syllable.jongseong,
                                    crate::hangul::JONG_NIEUN | crate::hangul::JONG_RIEUL
                                )
                            })
                        }) && complete_inflected_ending(
                            &self.resource,
                            endings,
                            &remaining[..split],
                            |pos| pos == StructuralPos::ETM,
                        ) && self.has_nominal_with_optional_particles(noun)
                    }));
            });
        found
    }

    #[must_use]
    pub fn has_extended_predicate_prefix_path(
        &self,
        text: &str,
        core_len: usize,
        source_positions: PredicatePosSet,
        node_limit: usize,
    ) -> bool {
        if core_len >= text.len() || !text.is_char_boundary(core_len) {
            return false;
        }
        let mut prefix_ends = Vec::new();
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length > core_len
                    && length < text.len()
                    && positions.first().is_some_and(|actual| {
                        positions.len() == 1
                            && source_positions
                                .iter()
                                .any(|pos| structural_predicate_pos_matches(*actual, pos))
                    })
                {
                    prefix_ends.push(length);
                }
            });
        prefix_ends
            .into_iter()
            .any(|end| self.supports_ending_suffix_path(text, end, node_limit))
    }

    #[must_use]
    pub fn has_unambiguous_attached_auxiliary_whole_path(&self, text: &str) -> bool {
        let mut found = false;
        let mut conflicting = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                if length != text.len() {
                    return;
                }
                let compatible = is_attached_auxiliary_whole_path(positions);
                found |= compatible;
                conflicting |= !compatible;
            });
        found && !conflicting
    }

    #[must_use]
    pub fn has_complete_attached_auxiliary_path(&self, text: &str, node_limit: usize) -> bool {
        EdgeGraph::collect(&self.resource, text, node_limit)
            .is_ok_and(|graph| has_complete_attached_auxiliary_path(text.len(), &graph))
    }

    #[must_use]
    pub fn has_nominalized_attached_auxiliary_path(&self, text: &str, node_limit: usize) -> bool {
        EdgeGraph::collect(&self.resource, text, node_limit).is_ok_and(|graph| {
            crate::structure::paths::predicate::has_attached_auxiliary_path_ending_in(
                text.len(),
                &graph,
                |pos| pos == StructuralPos::ETN,
            )
        })
    }

    #[must_use]
    pub fn has_complete_nominal_surface(&self, text: &str) -> bool {
        !text.is_empty() && complete_nominal_host(&self.resource, text)
    }

    #[must_use]
    pub fn has_exact_nominal_surface(&self, text: &str) -> bool {
        has_exact_fine_pos(&self.resource, text, DataFinePos::is_nominal)
    }

    #[must_use]
    pub fn has_exact_dependent_noun_surface(&self, text: &str) -> bool {
        has_exact_sequence(&self.resource, text, &[StructuralPos::NNB])
    }

    #[must_use]
    pub fn has_polite_nominal_reply(&self, previous: &str, host: &str, pos: DataFinePos) -> bool {
        pos.is_nominal()
            && self.has_whole_determiner(previous)
            && !has_exact_fine_pos(&self.resource, previous, |pos| {
                matches!(pos, DataFinePos::Mag | DataFinePos::Maj)
            })
            && has_exact_fine_pos(&self.resource, host, |candidate| candidate == pos)
            && has_exact_sequence(&self.resource, "요", &[StructuralPos::JX])
    }

    #[must_use]
    pub fn has_adnominal_dependent_noun(&self, previous: &str, text: &str) -> bool {
        has_exact_sequence(&self.resource, text, &[StructuralPos::NNB])
            && (super::lexical::complete_predicate_adnominal_path(&self.resource, previous)
                || super::lexical::complete_derived_adnominal_path(&self.resource, previous))
    }

    #[must_use]
    pub fn has_repeated_predicate_in_possibility_clause(
        &self,
        connective: &str,
        modifier: &str,
        noun: &str,
    ) -> bool {
        let Some(stem) = connective
            .strip_suffix('지')
            .filter(|stem| !stem.is_empty())
        else {
            return false;
        };
        noun.strip_prefix('수').is_some_and(|tail| {
            (modifier.strip_suffix('을') == Some(stem)
                || (modifier == stem && stem.ends_with(crate::hangul::has_rieul_final))
                || crate::hangul::add_final(stem, crate::hangul::JONG_RIEUL).as_deref()
                    == Some(modifier))
                && self.has_adnominal_dependent_noun(modifier, "수")
                && (tail.is_empty()
                    || complete_suffix(&self.resource, tail, |pos| {
                        matches!(pos, StructuralPos::JKS | StructuralPos::JX)
                    }))
        })
    }

    #[must_use]
    pub fn has_existential_final_predicate(&self, text: &str) -> bool {
        ["있", "없"].into_iter().any(|stem| {
            text.strip_prefix(stem).is_some_and(|tail| {
                exact_analysis_starts_with_pos(&self.resource, stem, |pos| {
                    matches!(
                        pos,
                        StructuralPos::VA | StructuralPos::VV | StructuralPos::VX
                    )
                }) && !tail.is_empty()
                    && tail.char_indices().any(|(split, _)| {
                        has_exact_sequence(&self.resource, &tail[split..], &[StructuralPos::EF])
                            && (split == 0
                                || complete_suffix(&self.resource, &tail[..split], |pos| {
                                    pos == StructuralPos::EP
                                }))
                    })
            })
        })
    }

    #[must_use]
    pub fn has_quotative_adnominal_dependent_noun(&self, previous: &str, text: &str) -> bool {
        !self.has_object_marked_nominal(previous)
            && self.has_adnominal_dependent_noun(previous, text)
    }

    #[must_use]
    pub fn has_dependent_noun_particle_contraction(
        &self,
        previous: &str,
        text: &str,
        object: bool,
        node_limit: usize,
    ) -> bool {
        let particle = if object {
            StructuralPos::JKO
        } else {
            StructuralPos::JX
        };
        has_exact_sequence(&self.resource, text, &[StructuralPos::NNB, particle])
            && (super::lexical::complete_predicate_adnominal_path(&self.resource, previous)
                || super::lexical::complete_derived_adnominal_path(&self.resource, previous)
                || EdgeGraph::collect(&self.resource, previous, node_limit).is_ok_and(|graph| {
                    crate::structure::paths::predicate::has_attached_auxiliary_path_ending_in(
                        previous.len(),
                        &graph,
                        |pos| pos == StructuralPos::ETM,
                    )
                }))
    }

    #[must_use]
    pub fn has_compound_noun_surface(&self, text: &str) -> bool {
        if text.chars().take(4).count() < 4 {
            return false;
        }
        let mut reachable = vec![false; text.len() + 1];
        reachable[0] = true;
        for (start, _) in text.char_indices() {
            if !reachable[start] {
                continue;
            }
            self.resource.common_prefix_positions(
                &text.as_bytes()[start..],
                |length, positions| {
                    let end = start + length;
                    if length == text.len()
                        || !matches!(positions, [StructuralPos::NNG | StructuralPos::NNP])
                    {
                        return;
                    }
                    if let Some(noun) = text.get(start..end)
                        && noun.chars().nth(1).is_some()
                        && !self.has_exact_action_predicate_surface(noun)
                    {
                        reachable[end] = true;
                    }
                },
            );
        }
        reachable[text.len()]
    }

    #[must_use]
    pub fn has_final_copula_after_nominal(&self, text: &str) -> bool {
        text.char_indices().skip(1).any(|(split, initial)| {
            (initial == '이' || crate::drop_final(initial) == Some('이'))
                && (&text[split..] == "이다"
                    || self.has_exact_predicate_ending_path_matching(
                        &text[split..],
                        |pos| pos == StructuralPos::VCP,
                        |ending| ending == StructuralPos::EF,
                    ))
                && self.has_complete_nominal_surface(&text[..split])
        })
    }

    #[must_use]
    pub fn has_comparative_nominal_surface(&self, text: &str) -> bool {
        COMPARATIVE_NOMINALS.iter().any(|&noun| {
            text.strip_prefix(noun).is_some_and(|tail| {
                has_exact_sequence(&self.resource, noun, &[StructuralPos::NNG])
                    && noun
                        .chars()
                        .next_back()
                        .is_some_and(|last| verify_copula_surface_after_nominal(last, tail))
            })
        })
    }

    #[must_use]
    pub fn has_comparative_nominal_after_quantity(&self, text: &str, shorter_host: &str) -> bool {
        COMPARATIVE_NOMINALS.iter().any(|&noun| {
            noun.len() > shorter_host.len()
                && noun.starts_with(shorter_host)
                && text.strip_prefix(noun).is_some_and(|tail| {
                    has_exact_sequence(&self.resource, noun, &[StructuralPos::NNG])
                        && complete_suffix(&self.resource, tail, StructuralPos::is_particle)
                })
        })
    }

    #[must_use]
    pub fn has_causal_nominal_surface(&self, text: &str) -> bool {
        CAUSAL_NOMINALS.iter().any(|&(noun, particle)| {
            text.strip_prefix(noun) == Some(particle)
                && has_exact_fine_pos(&self.resource, noun, DataFinePos::is_nominal)
                && has_exact_sequence(&self.resource, particle, &[StructuralPos::JKB])
        })
    }

    #[must_use]
    pub fn has_nominal_oblique(&self, text: &str) -> bool {
        text.char_indices().skip(1).any(|(split, _)| {
            has_exact_sequence(&self.resource, &text[split..], &[StructuralPos::JKB])
                && self.has_complete_nominal_surface(&text[..split])
        })
    }

    #[must_use]
    pub fn has_spatial_destination(&self, text: &str) -> bool {
        text.strip_suffix("으로")
            .or_else(|| text.strip_suffix('로'))
            .is_some_and(|noun| self.is_spatial_nominal(noun) && self.has_nominal_oblique(text))
    }

    #[must_use]
    pub fn is_spatial_nominal(&self, noun: &str) -> bool {
        self.resource.has_nominal_role(noun, NominalRole::Spatial)
    }

    #[must_use]
    pub fn is_person_nominal(&self, noun: &str) -> bool {
        self.resource.has_nominal_role(noun, NominalRole::Person)
    }

    #[must_use]
    pub fn is_scheduled_nominal(&self, noun: &str) -> bool {
        self.resource.has_nominal_role(noun, NominalRole::Scheduled)
    }

    /// Verifies the independent verb 늦다 (KRDict 61181).
    #[must_use]
    pub fn has_late_arrival_inflection(text: &str) -> bool {
        static FORMS: std::sync::LazyLock<Vec<crate::SurfaceBranchSpec>> =
            std::sync::LazyLock::new(|| {
                let entry = crate::PredicateEntry::new(
                    "늦다",
                    PredicatePos::Verb,
                    LexicalAlternation::Regular,
                );
                crate::generate_predicate_branches(&entry).expect("valid arrival predicate")
            });
        text.starts_with('늦') && matches_predicate_inflection(text, &FORMS)
    }

    #[must_use]
    pub fn is_temporal_nominal(&self, noun: &str) -> bool {
        self.resource.has_nominal_role(noun, NominalRole::Temporal)
    }

    #[must_use]
    pub fn is_attested_do_object(&self, noun: &str) -> bool {
        self.resource.has_nominal_role(noun, NominalRole::DoObject)
    }

    #[must_use]
    pub fn has_nominal_roles(&self) -> bool {
        self.resource.has_nominal_roles()
    }

    #[must_use]
    pub fn has_nominal_role(&self, noun: &str, role: NominalRole) -> bool {
        self.resource.has_nominal_role(noun, role)
    }

    /// Checks the independent lexical motion verb 들어가다 (KRDict 67877).
    #[must_use]
    pub fn has_enter_inflection(text: &str) -> bool {
        static FORMS: std::sync::LazyLock<Vec<crate::SurfaceBranchSpec>> =
            std::sync::LazyLock::new(|| {
                let entry = crate::PredicateEntry::new(
                    "들어가다",
                    PredicatePos::Verb,
                    LexicalAlternation::Regular,
                );
                crate::generate_predicate_branches(&entry).expect("valid motion predicate")
            });
        text.starts_with("들어") && matches_predicate_inflection(text, &FORMS)
    }

    #[must_use]
    pub fn has_derived_ha_past(&self, host: &str, ending: &str) -> bool {
        let Some(noun) = host.strip_suffix('하') else {
            return false;
        };
        let Some(tail) = ending.strip_prefix('였').filter(|tail| !tail.is_empty()) else {
            return false;
        };
        !self.has_exact_nominal_surface(host)
            && self.has_complete_nominal_surface(noun)
            && has_exact_sequence(&self.resource, "하", &[StructuralPos::XSV])
            && has_exact_sequence(&self.resource, "였", &[StructuralPos::EP])
            && complete_suffix(&self.resource, tail, StructuralPos::is_ending)
    }

    #[must_use]
    pub fn has_nominal_genitive_surface(&self, text: &str) -> bool {
        text.strip_suffix('의').is_some_and(|host| {
            !self.has_exact_nominal_surface(text) && self.has_complete_nominal_surface(host)
        })
    }

    #[must_use]
    pub fn has_pronominal_genitive_surface(&self, text: &str) -> bool {
        has_exact_sequence(
            &self.resource,
            text,
            &[StructuralPos::NP, StructuralPos::JKG],
        )
    }

    #[must_use]
    pub fn has_joined_nominal_modifier(&self, modifier: &str, head: &str, whole: &str) -> bool {
        (self.has_nominal_genitive_surface(modifier)
            || super::lexical::complete_predicate_adnominal_path(&self.resource, modifier)
            || super::lexical::complete_derived_adnominal_path(&self.resource, modifier))
            && self.has_independent_nominal_head(head)
            && !self.has_exact_nominal_surface(whole)
            && !self.has_whole_modifier(whole)
    }

    #[must_use]
    pub fn has_derived_predicate_adnominal_surface(&self, text: &str) -> bool {
        super::lexical::complete_derived_adnominal_path(&self.resource, text)
    }

    #[must_use]
    pub fn has_nominal_with_optional_particles(&self, text: &str) -> bool {
        self.has_complete_nominal_surface(text)
            || complete_nominal_particle_host(&self.resource, text).is_some()
    }

    #[must_use]
    pub fn has_exact_noun_with_optional_particles(&self, text: &str) -> bool {
        let mut matched = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |len, positions| {
                if matches!(
                    positions,
                    [StructuralPos::NNG | StructuralPos::NNP | StructuralPos::NNBC]
                ) && (len == text.len()
                    || complete_suffix(&self.resource, &text[len..], StructuralPos::is_particle))
                {
                    matched = true;
                }
            });
        matched
    }

    #[must_use]
    pub fn has_explicit_copula_after_nominal(&self, text: &str) -> bool {
        text.char_indices().skip(1).any(|(split, initial)| {
            crate::drop_final(initial).unwrap_or(initial) == '이'
                && text[..split]
                    .chars()
                    .next_back()
                    .is_some_and(|last| verify_copula_surface_after_nominal(last, &text[split..]))
                && self.has_complete_nominal_surface(&text[..split])
        })
    }

    #[must_use]
    pub fn has_scope_nominal_host(&self, text: &str) -> bool {
        (self.has_exact_nominal_surface(text) || self.has_complete_nominal_surface(text))
            && !self.has_whole_modifier(text)
            && !self.has_derived_predicate_connective_surface(
                text,
                PredicatePosSet::one(PredicatePos::Verb)
                    | PredicatePosSet::one(PredicatePos::Adjective),
            )
            && !complete_ha_predicate_path(&self.resource, text)
            && !self.has_scope_competing_connective(text)
            && !self.has_object_marked_nominal(text)
            && !self.has_subject_marked_nominal(text)
            && !text.strip_suffix(['은', '는']).is_some_and(|host| {
                self.has_complete_nominal_surface(host)
                    && host.chars().next_back().is_some_and(|last| {
                        crate::drop_final(last).is_some() == text.ends_with('은')
                    })
            })
            && (self.has_unambiguous_proper_name_surface(text) || !self.has_nominal_oblique(text))
    }

    #[must_use]
    pub fn has_scope_dependent_noun(&self, text: &str) -> bool {
        SCOPE_DEPENDENT_NOUNS.contains(&text)
            && has_exact_sequence(&self.resource, text, &[StructuralPos::NNB])
    }

    pub fn has_scope_competing_connective(&self, text: &str) -> bool {
        text.char_indices().any(|(start, _)| {
                let mut complete = false;
                self.resource.common_prefix_positions(&text.as_bytes()[start..], |length, positions| {
                    if start + length != text.len() {
                        return;
                    }
                    complete |= if start == 0 {
                        matches!(positions, [predicate, middle @ .., StructuralPos::EC]
                            if predicate.is_predicate_tag() && middle.iter().all(|pos| pos.is_ending()))
                    } else {
                        matches!(positions, [middle @ .., StructuralPos::EC] if middle.iter().all(|pos| pos.is_ending()))
                            && exact_analysis_starts_with_pos(&self.resource, &text[..start], StructuralPos::is_predicate_tag)
                            || matches!(positions, [StructuralPos::VCP, middle @ .., StructuralPos::EC]
                                if middle.iter().all(|pos| pos.is_ending()))
                                && self.has_nominal_with_optional_particles(&text[..start])
                    };
                });
                complete
        })
    }

    #[must_use]
    pub fn has_connective_after_predicate(&self, host: &str, ending: &str) -> bool {
        has_exact_sequence(&self.resource, ending, &[StructuralPos::EC])
            && (exact_analysis_starts_with_pos(&self.resource, host, |pos| pos.is_predicate_tag())
                || complete_predicate_ending_path(&self.resource, host))
    }

    #[must_use]
    pub fn has_competing_nominal_after_connective(
        &self,
        text: &str,
        connective_end: usize,
    ) -> bool {
        text.char_indices().any(|(split, _)| {
            split > connective_end
                && self.has_exact_nominal_surface(&text[..split])
                && self.has_nominal_with_optional_particles(&text[split..])
        }) || self.has_exact_nominal_surface(text)
    }

    #[must_use]
    pub fn has_independent_nominal_head(&self, text: &str) -> bool {
        if self.has_complete_nominal_surface(text) {
            return !has_exact_sequence(&self.resource, text, &[StructuralPos::NNB]);
        }
        complete_nominal_particle_host(&self.resource, text).is_some_and(|span| {
            !has_exact_sequence(&self.resource, &text[span], &[StructuralPos::NNB])
        })
    }

    #[must_use]
    pub fn has_inflected_counted_unit(&self, previous: &str, text: &str) -> bool {
        text.strip_suffix("째")
            .is_some_and(|unit| self.has_counted_unit_frame(previous, unit))
            || complete_nominal_particle_host(&self.resource, text)
                .is_some_and(|host| self.has_counted_unit_frame(previous, &text[host]))
            || self.has_counted_unit_frame(previous, text)
    }

    #[must_use]
    pub fn has_standalone_counted_unit(&self, count: &str, unit: &str) -> bool {
        self.is_counter_quantity(count) && self.has_exact_counter(unit)
    }

    #[must_use]
    pub fn has_counted_unit_with_copula(&self, count: &str, text: &str) -> bool {
        self.is_counter_quantity(count)
            && text.char_indices().skip(1).any(|(split, _)| {
                let (unit, ending) = text.split_at(split);
                self.has_exact_counter(unit)
                    && unit
                        .chars()
                        .next_back()
                        .is_some_and(|last| verify_copula_surface_after_nominal(last, ending))
            })
    }

    /// 곳 is both a place noun and a place-counting unit (KRDict 18661).
    #[must_use]
    pub fn has_counted_place_surface(&self, count: &str, text: &str) -> bool {
        self.is_counter_quantity(count)
            && text.strip_prefix("곳").is_some_and(|suffix| {
                suffix.is_empty()
                    || complete_nominal_particle_host(&self.resource, text)
                        .is_some_and(|host| &text[host] == "곳")
                    || verify_copula_surface_after_nominal('곳', suffix)
            })
    }

    fn is_counter_quantity(&self, count: &str) -> bool {
        UNSPECIFIED_COUNTER_QUANTITIES.contains(&count)
            || INDEFINITE_COUNTER_QUANTITIES.contains(&count)
            || self.has_measure_count(count)
            || count
                .strip_suffix(APPROXIMATE_CARDINAL_SUFFIX)
                .is_some_and(|cardinal| {
                    (!cardinal.is_empty() && cardinal.bytes().all(|byte| byte.is_ascii_digit()))
                        || Self::is_mixed_cardinal(cardinal)
                        || has_exact_fine_pos(&self.resource, cardinal, |pos| {
                            pos == DataFinePos::Nr
                        })
                })
    }

    /// Recognizes an object count without treating date, price or measure phrases as object counts.
    #[must_use]
    pub fn has_postnominal_object_count(&self, first: &str, second: Option<&str>) -> bool {
        if first.starts_with(|ch: char| ch.is_ascii_digit())
            && first[1..].char_indices().rev().any(|(offset, _)| {
                let (count, rest) = first.split_at(offset + 1);
                (self.is_counter_quantity(count) || Self::is_mixed_cardinal(count))
                    && self.has_object_counter(count, rest)
            })
        {
            return true;
        }
        (self.is_counter_quantity(first) || Self::is_mixed_cardinal(first))
            && second.is_some_and(|unit| self.has_object_counter(first, unit))
    }

    #[must_use]
    pub fn is_quantity_modifier(text: &str) -> bool {
        QUANTITY_MODIFIERS.contains(&text)
    }

    #[must_use]
    pub fn is_quantity_approximation_suffix(text: &str) -> bool {
        QUANTITY_APPROXIMATION_SUFFIXES.contains(&text)
    }

    #[must_use]
    pub fn has_quantity_approximation_particle_tail(&self, text: &str) -> bool {
        complete_suffix(&self.resource, text, StructuralPos::is_particle)
    }

    #[must_use]
    pub fn has_quantity_amount(&self, unit: &str, count: Option<&str>) -> bool {
        DURATION_NOUNS.contains(&unit)
            || self.has_numeric_unit_surface(unit)
            || count.is_some_and(|count| {
                self.has_exact_counter(unit)
                    && (self.is_counter_quantity(count) || Self::is_mixed_cardinal(count))
            })
    }

    pub fn split_counter_quantity(first: &str, second: &str) -> Option<&'static str> {
        INDEFINITE_COUNTER_QUANTITIES
            .iter()
            .copied()
            .find(|quantity| {
                !first.is_empty()
                    && !second.is_empty()
                    && quantity.strip_prefix(first) == Some(second)
            })
    }

    fn has_object_counter(&self, count: &str, text: &str) -> bool {
        let mut host = None;
        self.resource
            .common_prefix_positions(text.as_bytes(), |len, positions| {
                if positions == [StructuralPos::NNBC] && {
                    let tail = &text[len..];
                    let particles = QUANTITY_APPROXIMATION_SUFFIXES
                        .iter()
                        .find_map(|suffix| tail.strip_prefix(suffix))
                        .unwrap_or(tail);
                    particles.is_empty()
                        || complete_suffix(&self.resource, particles, StructuralPos::is_particle)
                } {
                    host = Some(&text[..len]);
                }
            });
        host.is_some_and(|unit| {
            !DATE_AND_PRICE_UNITS.contains(&unit) && !self.has_elapsed_measure(unit, Some(count))
        })
    }

    #[must_use]
    pub fn has_exact_counter(&self, unit: &str) -> bool {
        has_exact_sequence(&self.resource, unit, &[StructuralPos::NNBC])
    }

    #[must_use]
    pub fn has_counted_unit_frame(&self, previous: &str, unit: &str) -> bool {
        exact_analysis_starts_with_pos(&self.resource, unit, |pos| pos == StructuralPos::NNBC)
            && (has_exact_fine_pos(&self.resource, previous, |pos| pos == DataFinePos::Nr)
                || Self::is_mixed_cardinal(previous)
                || (has_exact_fine_pos(&self.resource, previous, |pos| pos == DataFinePos::Mm)
                    && !(has_exact_fine_pos(&self.resource, unit, |pos| pos == DataFinePos::Vx)
                        && self.has_exact_predicate_connective_path(
                            previous,
                            PredicatePosSet::one(PredicatePos::Verb),
                        ))))
    }

    #[must_use]
    pub fn is_mixed_cardinal(text: &str) -> bool {
        let mut small_rank = 4;
        let mut large_rank = 6;
        let mut digit_run = false;
        let mut has_digits = false;
        let mut has_rank = false;
        for character in text.chars() {
            if character.is_ascii_digit() {
                digit_run = true;
                has_digits = true;
                continue;
            }
            let (rank, large) = match character {
                '십' => (1, false),
                '백' => (2, false),
                '천' => (3, false),
                '만' => (1, true),
                '억' => (2, true),
                '조' => (3, true),
                '경' => (4, true),
                '해' => (5, true),
                _ => return false,
            };
            if !digit_run && (!large || small_rank == 4) {
                return false;
            }
            let ceiling = if large {
                &mut large_rank
            } else {
                &mut small_rank
            };
            if rank >= *ceiling {
                return false;
            }
            *ceiling = rank;
            if large {
                small_rank = 4;
            }
            digit_run = false;
            has_rank = true;
        }
        has_digits && has_rank
    }

    #[must_use]
    pub fn has_duration_suffix_host(
        &self,
        unit: &str,
        count: Option<&str>,
        attached: bool,
    ) -> bool {
        (!attached && DURATION_NOUNS.contains(&unit))
            || (DURATION_UNITS.contains(&unit)
                && count.is_some_and(|count| {
                    self.has_measure_count(count)
                        || Self::is_mixed_cardinal(count)
                        || self.has_standalone_counted_unit(count, unit)
                }))
    }

    #[must_use]
    pub fn has_attached_duration_measure(&self, text: &str) -> bool {
        DURATION_UNITS.iter().any(|unit| {
            text.strip_suffix(unit).is_some_and(|count| {
                !count.is_empty() && self.has_duration_suffix_host(unit, Some(count), true)
            })
        })
    }

    /// Verifies the duration predicate `걸리다` (KRDict 86940).
    #[must_use]
    pub fn has_elapsed_time_predicate(text: &str, adnominal: bool) -> bool {
        if !text.starts_with('걸') {
            return false;
        }
        static FORMS: std::sync::LazyLock<Vec<crate::SurfaceBranchSpec>> =
            std::sync::LazyLock::new(|| {
                let entry = crate::PredicateEntry::new(
                    "걸리다",
                    PredicatePos::Verb,
                    LexicalAlternation::Regular,
                );
                crate::generate_predicate_branches(&entry).expect("valid duration predicate")
            });
        FORMS.iter().any(|form| {
            matches_predicate_branch_with_ending(text, form, |ending| {
                ending.is_some_and(|rule| {
                    matches!(
                        rule.as_str(),
                        "ending.present-adnominal"
                            | "ending.past-adnominal"
                            | "ending.future-adnominal"
                    ) == adnominal
                })
            })
        })
    }

    #[must_use]
    pub fn has_elapsed_measure(&self, measure: &str, previous: Option<&str>) -> bool {
        const OTHER_MEASURES: &[&str] = &[
            "번",
            "차례",
            "밀리미터",
            "센티미터",
            "킬로미터",
            "미터",
            "마일",
            "해리",
            "리",
            "걸음",
        ];
        DURATION_NOUNS.contains(&measure)
            || DURATION_UNITS
                .iter()
                .chain(OTHER_MEASURES)
                .chain(COUNTABLE_PERIODS)
                .any(|unit| {
                    measure.strip_suffix(unit).is_some_and(|attached| {
                        let count = if attached.is_empty() {
                            previous.unwrap_or_default()
                        } else {
                            attached
                        };
                        self.has_measure_count(count)
                    })
                })
    }

    fn has_measure_count(&self, text: &str) -> bool {
        // A native counting modifier can combine a tens word with a shortened ones form.
        const NATIVE_TENS: &[&str] = &[
            "열", "스물", "서른", "마흔", "쉰", "예순", "일흔", "여든", "아흔",
        ];
        const NATIVE_ONES: &[&str] = &[
            "한", "두", "세", "석", "서", "네", "넉", "너", "다섯", "여섯", "일곱", "여덟", "아홉",
        ];
        !text.is_empty()
            && (text.bytes().all(|byte| byte.is_ascii_digit())
                || text == "몇"
                || text == "스무"
                || NATIVE_ONES.contains(&text)
                || NATIVE_TENS.iter().any(|tens| {
                    text.strip_prefix(tens)
                        .is_some_and(|ones| ones.is_empty() || NATIVE_ONES.contains(&ones))
                })
                || has_exact_fine_pos(&self.resource, text, |pos| pos == DataFinePos::Nr))
    }

    #[must_use]
    pub fn has_quantity_expression(&self, first: &str, second: Option<&str>) -> bool {
        // These lexicalized dates measure proximity to the time of speaking.
        if matches!(
            first,
            "어제" | "어저께" | "그제" | "그저께" | "그끄제" | "그끄저께" | "엊그제" | "엊그저께"
        ) {
            return true;
        }
        let digit_end = first.bytes().take_while(u8::is_ascii_digit).count();
        if digit_end != 0 {
            return digit_end == first.len() || self.has_quantity_unit(&first[digit_end..]);
        }
        if (first == "수" || self.has_measure_count(first))
            && second.is_some_and(|unit| self.has_quantity_unit(unit))
        {
            return true;
        }
        let mut matched = false;
        self.resource
            .common_prefix_positions(first.as_bytes(), |len, positions| {
                let host = &first[..len];
                matched |= (positions == [StructuralPos::NR] || DURATION_NOUNS.contains(&host))
                    && self.has_quantity_nominal_tail(host, &first[len..]);
            });
        matched
    }

    #[must_use]
    pub fn has_attached_unspecified_count(&self, count: &str, token: &str) -> bool {
        if !UNSPECIFIED_COUNTER_QUANTITIES.contains(&count) {
            return false;
        }
        let Some(following) = token.strip_prefix(count) else {
            return false;
        };
        let mut matched = false;
        self.resource
            .common_prefix_positions(following.as_bytes(), |len, positions| {
                matched |= positions == [StructuralPos::NNBC]
                    && complete_suffix(
                        &self.resource,
                        &following[len..],
                        StructuralPos::is_particle,
                    );
            });
        if !matched || self.has_exact_adverb_surface(token) {
            return false;
        }
        let mut competing_nominal = false;
        self.resource
            .common_prefix_positions(token.as_bytes(), |len, positions| {
                competing_nominal |= len > count.len()
                    && matches!(positions, [pos, particles @ ..] if pos.is_nominal()
                        && particles.iter().all(|pos| pos.is_particle()))
                    && complete_suffix(&self.resource, &token[len..], StructuralPos::is_particle);
            });
        !competing_nominal
    }

    fn has_quantity_unit(&self, text: &str) -> bool {
        let mut matched = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |len, positions| {
                matched |= matches!(positions, [StructuralPos::NNB | StructuralPos::NNBC])
                    && self.has_quantity_nominal_tail(&text[..len], &text[len..]);
            });
        matched
    }

    fn has_quantity_nominal_tail(&self, host: &str, suffix: &str) -> bool {
        complete_suffix(&self.resource, suffix, StructuralPos::is_particle)
            || host
                .chars()
                .next_back()
                .is_some_and(|preceding| verify_copula_surface_after_nominal(preceding, suffix))
    }

    #[must_use]
    pub fn has_copula_conditional_adverbial_host(&self, text: &str) -> bool {
        has_exact_fine_pos(&self.resource, text, |pos| pos == DataFinePos::Mag)
            || complete_nominal_particle_host(&self.resource, text).is_some()
    }

    #[must_use]
    pub fn has_adjective_adnominal_surface(&self, text: &str) -> bool {
        super::lexical::adjective_adnominal_path(&self.resource, text)
    }

    #[must_use]
    pub fn has_copular_adnominal_frame(&self, previous: &str, modifier: &str, head: &str) -> bool {
        super::lexical::has_copular_adnominal_split(&self.resource, modifier)
            && (super::lexical::complete_predicate_adnominal_path(&self.resource, previous)
                || super::lexical::complete_derived_adnominal_path(&self.resource, previous))
            && self.has_independent_nominal_head(head)
    }

    #[must_use]
    pub fn has_jeok_nominal_derivation(&self, text: &str) -> bool {
        // 한국어기초사전 88966의 명사 파생 접미사.
        text.strip_suffix('적').is_some_and(|host| {
            self.has_complete_nominal_surface(host)
                && has_exact_sequence(&self.resource, "적", &[StructuralPos::XSN])
        })
    }

    #[must_use]
    pub fn has_attached_adjective_adnominal(&self, text: &str, split: usize) -> bool {
        let Some((left, right)) = text.split_at_checked(split) else {
            return false;
        };
        !left.is_empty()
            && self.has_adjective_adnominal_surface(right)
            && super::lexical::complete_modifier_prefix(&self.resource, left)
            && !self.has_exact_noun_with_optional_particles(text)
            && !left.char_indices().skip(1).any(|(boundary, _)| {
                self.has_exact_proper_name_surface(&text[boundary..])
                    && (super::lexical::complete_predicate_adnominal_path(
                        &self.resource,
                        &text[..boundary],
                    ) || super::lexical::complete_derived_adnominal_path(
                        &self.resource,
                        &text[..boundary],
                    ))
            })
    }

    #[must_use]
    pub fn has_predicate_adnominal_surface(&self, text: &str) -> bool {
        let mut matched = false;
        let mut modifier = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                let Some((first, endings)) = positions.split_first() else {
                    return;
                };
                if length == text.len() {
                    modifier |= matches!(
                        *first,
                        StructuralPos::MM | StructuralPos::MAG | StructuralPos::MAJ
                    );
                }
                if !first.is_predicate() || !endings.iter().all(|pos| pos.is_ending()) {
                    return;
                }
                matched |= if length == text.len() {
                    endings.last() == Some(&StructuralPos::ETM)
                } else {
                    let mut terminal_adnominal = false;
                    self.resource.common_prefix_positions(
                        &text.as_bytes()[length..],
                        |suffix_len, suffix| {
                            terminal_adnominal |=
                                length + suffix_len == text.len() && suffix == [StructuralPos::ETM];
                        },
                    );
                    terminal_adnominal
                };
            });
        matched && !modifier
    }

    #[must_use]
    pub fn has_interrogative_ji_ending(&self, text: &str) -> bool {
        let Some(host) = text.strip_suffix('지') else {
            return false;
        };
        let rieul = host
            .chars()
            .next_back()
            .is_some_and(crate::hangul::has_rieul_final);
        if rieul || host.ends_with(['는', '던']) {
            return super::lexical::complete_interrogative_ji_path(&self.resource, text)
                || ((rieul || host.ends_with('는')) && self.has_predicate_adnominal_surface(host));
        }
        host.chars()
            .next_back()
            .and_then(crate::hangul::decompose_syllable)
            .is_some_and(|syllable| syllable.jongseong == crate::hangul::JONG_NIEUN)
            && self.has_adjective_adnominal_surface(host)
            && !super::lexical::complete_past_verbal_adnominal_path(&self.resource, host)
    }

    #[must_use]
    pub fn has_exact_proper_name_surface(&self, text: &str) -> bool {
        has_exact_fine_pos(&self.resource, text, |pos| pos == DataFinePos::Nnp)
    }

    #[must_use]
    pub fn has_exact_pronoun_surface(&self, text: &str) -> bool {
        has_exact_sequence(&self.resource, text, &[StructuralPos::NP])
    }

    #[must_use]
    pub fn has_surname_title(&self, surname: &str, title: &str) -> bool {
        title == "씨"
            && has_exact_sequence(&self.resource, title, &[StructuralPos::NNB])
            && self.has_exact_proper_name_surface(surname)
    }

    #[must_use]
    pub fn has_topic_marked_nominal(&self, text: &str) -> bool {
        text.strip_suffix(['은', '는']).is_some_and(|host| {
            host.chars()
                .next_back()
                .is_some_and(|last| crate::hangul::has_final(last) == text.ends_with('은'))
                && self.has_complete_nominal_surface(host)
                && has_exact_sequence(&self.resource, &text[host.len()..], &[StructuralPos::JX])
        }) || self.has_pronoun_topic_contraction(text)
    }

    #[must_use]
    pub fn has_unambiguous_proper_name_surface(&self, text: &str) -> bool {
        self.has_exact_proper_name_surface(text)
            && !exact_analysis_starts_with_pos(&self.resource, text, |pos| {
                pos != StructuralPos::NNP
            })
    }

    #[must_use]
    pub fn has_exact_pronoun_case_surface(&self, text: &str) -> bool {
        let mut matched = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                matched |= length == text.len()
                    && matches!(
                        positions,
                        [StructuralPos::NP, StructuralPos::JKS | StructuralPos::JKC]
                    );
            });
        matched
    }

    #[must_use]
    pub fn has_exact_action_predicate_surface(&self, text: &str) -> bool {
        exact_analysis_starts_with_pos(&self.resource, text, |pos| {
            matches!(pos, StructuralPos::VV | StructuralPos::VX)
        })
    }

    /// Verifies the action stem in the volitional -ㅁ세/-음세 (KRDict 78483).
    #[must_use]
    pub fn has_volitional_verb_surface(&self, text: &str) -> bool {
        use crate::hangul::{
            JONG_MIEUM, JONG_NONE, JONG_RIEUL, JONG_RIEUL_MIEUM, decompose_syllable, replace_final,
        };
        let Some(host) = text.strip_suffix('세') else {
            return false;
        };
        if let Some(stem) = host.strip_suffix('음')
            && self.has_exact_action_predicate_surface(stem)
        {
            return true;
        }
        let Some((start, last)) = host.char_indices().next_back() else {
            return false;
        };
        let Some(syllable) = decompose_syllable(last) else {
            return false;
        };
        let final_sound = match syllable.jongseong {
            JONG_MIEUM => JONG_NONE,
            JONG_RIEUL_MIEUM => JONG_RIEUL,
            _ => return false,
        };
        replace_final(last, final_sound).is_some_and(|last| {
            self.has_exact_action_predicate_surface(&format!("{}{last}", &host[..start]))
        })
    }

    #[must_use]
    pub fn has_exact_adjective_surface(&self, text: &str) -> bool {
        exact_analysis_starts_with_pos(&self.resource, text, |pos| pos == StructuralPos::VA)
    }

    #[must_use]
    pub fn has_exact_non_copula_predicate_surface(&self, text: &str) -> bool {
        exact_analysis_starts_with_pos(&self.resource, text, |pos| {
            pos.is_predicate() && pos != StructuralPos::VCP
        })
    }

    #[must_use]
    pub fn has_independent_final_predicate(&self, text: &str) -> bool {
        self.independent_final_predicate::<false>(text, false).0 != PredicatePosSet::EMPTY
    }

    #[must_use]
    pub fn has_attached_auxiliary_final_predicate(&self, text: &str, node_limit: usize) -> bool {
        !self.has_exact_nominal_surface(text)
            && EdgeGraph::collect(&self.resource, text, node_limit).is_ok_and(|graph| {
                crate::structure::paths::predicate::has_attached_auxiliary_path_ending_in(
                    text.len(),
                    &graph,
                    |pos| pos == StructuralPos::EF,
                )
            })
    }

    #[must_use]
    pub fn has_final_adjective_by_ending(&self, text: &str) -> bool {
        text.char_indices().skip(1).any(|(split, _)| {
            let (stem, tail) = text.split_at(split);
            self.has_exact_adjective_surface(stem)
                && (has_exact_sequence(&self.resource, tail, &[StructuralPos::EF])
                    || tail.strip_prefix('습').is_some_and(|ending| {
                        stem.chars().next_back().is_some_and(|last| {
                            crate::hangul::has_final(last) && !crate::hangul::has_rieul_final(last)
                        }) && crate::verify_predicate_continuation(
                            crate::ContinuationState::BieupFinal,
                            PredicatePos::Adjective,
                            stem,
                            ending,
                        )
                        .is_some_and(|matched| {
                            matched.consumed_bytes == ending.len()
                                && matched.rule_path.last().is_some_and(|rule| {
                                    rule.as_str() == "ending.polite-declarative"
                                })
                        })
                    }))
        })
    }

    #[must_use]
    pub fn has_independent_final_adjective(&self, text: &str) -> bool {
        self.independent_final_predicate::<false>(text, false).0
            == PredicatePosSet::one(PredicatePos::Adjective)
    }

    #[must_use]
    pub fn has_independent_final_predicate_without_object(&self, text: &str) -> bool {
        let (positions, excludes_object) = self.independent_final_predicate::<true>(text, false);
        positions == PredicatePosSet::one(PredicatePos::Adjective) || excludes_object
    }

    #[must_use]
    pub fn has_independent_final_verb_with_object(&self, text: &str) -> bool {
        let (positions, excludes_object) = self.independent_final_predicate::<true>(text, false);
        positions.contains(PredicatePos::Verb) && !excludes_object
    }

    /// Finds a final verb analysis when the caller has identified an object in the clause.
    #[must_use]
    pub fn has_final_verb_after_object(&self, text: &str) -> bool {
        let (positions, excludes_object) = self.independent_final_predicate::<true>(text, true);
        positions.contains(PredicatePos::Verb) && !excludes_object
    }

    #[must_use]
    pub fn has_propositive_do_inflection(text: &str) -> bool {
        static FORMS: std::sync::LazyLock<Vec<crate::SurfaceBranchSpec>> =
            std::sync::LazyLock::new(|| {
                let entry =
                    crate::PredicateEntry::new("하다", PredicatePos::Verb, LexicalAlternation::Ha);
                crate::generate_predicate_branches(&entry).expect("valid action predicate")
            });
        FORMS.iter().any(|form| {
            matches_predicate_branch_with_ending(text, form, |rule| {
                rule.is_some_and(|ending| {
                    matches!(
                        ending.as_str(),
                        "ending.propositive-ja" | "ending.propositive" | "ending.propositive-se"
                    )
                })
            })
        })
    }

    #[must_use]
    pub fn has_accusative_verb_stem(&self, text: &str) -> bool {
        has_exact_sequence(&self.resource, text, &[StructuralPos::VV])
            && !stem_excludes_accusative(text)
    }

    #[must_use]
    pub fn has_accusative_verb_connective(&self, text: &str) -> bool {
        !self.has_exact_nominal_surface(text)
            && text.char_indices().skip(1).any(|(split, _)| {
                self.has_accusative_verb_stem(&text[..split])
                    && complete_inflected_ending(&self.resource, &[], &text[split..], |pos| {
                        pos == StructuralPos::EC
                    })
            })
    }

    fn independent_final_predicate<const CHECK_OBJECT: bool>(
        &self,
        text: &str,
        has_object_context: bool,
    ) -> (PredicatePosSet, bool) {
        let has_final_ending = |length: usize, endings: &[StructuralPos]| {
            complete_inflected_ending(&self.resource, endings, &text[length..], |pos| {
                pos == StructuralPos::EF
            })
        };
        let mut positions_found = PredicatePosSet::EMPTY;
        let mut has_final_auxiliary = false;
        let mut excludes_object = true;
        let mut record = |pos, anchor_end| {
            positions_found |= PredicatePosSet::one(pos);
            if CHECK_OBJECT {
                excludes_object &= stem_excludes_accusative(&text[..anchor_end]);
            }
        };
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                let Some((&predicate, endings)) = positions.split_first() else {
                    return;
                };
                match predicate {
                    StructuralPos::VV if has_final_ending(length, endings) => {
                        record(PredicatePos::Verb, length);
                    }
                    StructuralPos::VA if has_final_ending(length, endings) => {
                        record(PredicatePos::Adjective, length);
                    }
                    StructuralPos::VX => {
                        has_final_auxiliary |= has_final_ending(length, endings);
                    }
                    StructuralPos::NNG | StructuralPos::XR
                        if length < text.len() && endings.is_empty() =>
                    {
                        self.resource.common_prefix_positions(
                            &text.as_bytes()[length..],
                            |suffix_len, positions| {
                                if let Some((&suffix, endings)) = positions.split_first()
                                    && matches!(suffix, StructuralPos::XSV | StructuralPos::XSA)
                                    && has_final_ending(length + suffix_len, endings)
                                {
                                    let pos = if suffix == StructuralPos::XSV {
                                        PredicatePos::Verb
                                    } else {
                                        PredicatePos::Adjective
                                    };
                                    record(pos, length + suffix_len);
                                }
                            },
                        );
                    }
                    _ => {}
                }
            });
        if positions_found == PredicatePosSet::EMPTY
            || (!has_object_context
                && (has_final_auxiliary || self.has_exact_nominal_surface(text)))
        {
            return (PredicatePosSet::EMPTY, false);
        }
        (positions_found, excludes_object)
    }
}
