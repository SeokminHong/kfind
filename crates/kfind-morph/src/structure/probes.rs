use crate::structure::ConstraintResolver;
use crate::structure::evidence::TokenEvidence;
use crate::structure::graph::EdgeGraph;
use crate::structure::lexical::{
    complete_dependent_noun_particle_suffix, complete_nominal_host, complete_nominal_particle_host,
    complete_predicate_ending_path, complete_suffix, exact_analysis_starts_with_pos,
    has_adnominal_result_complement, has_complete_object_phrase, has_exact_fine_pos,
    has_exact_sequence, numeric_unit_path_at, predicate_pos_matches,
    structural_predicate_pos_matches,
};
use crate::structure::paths::predicate::{
    has_complete_attached_auxiliary_path, is_attached_auxiliary_whole_path,
};
use crate::{PredicatePos, PredicatePosSet, verify_copula_surface_after_nominal};
use kfind_data::{ComponentPos as StructuralPos, DataFinePos};
use std::ops::Range;

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

// Korean Basic Dictionary -하다 predicates with a 로 frame and no accusative.
// Standard Dictionary and Open Dictionary both attest a transitive 가득하다,
// so that homograph stays outside the list. The verb 충분하다 is intransitive.
const NON_OBJECT_RESULT_HADA_HOSTS: &[&str] = &[
    "가뭇가뭇",
    "그득",
    "너끈",
    "득시글",
    "득실",
    "복잡",
    "빡빡",
    "빼곡",
    "빽빽",
    "유명",
    "자옥",
    "자욱",
    "적합",
    "족",
    "충만",
    "충분",
];

// All -나다 verbs with a 로/으로 frame and no accusative frame in the Korean
// Basic Dictionary snapshot. Slash-separated alternatives count as frames.
const NON_OBJECT_RESULT_NADA_HOSTS: &[&str] =
    &["거듭", "결판", "끝", "나타", "달아", "밀려", "소문"];

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

impl ConstraintResolver {
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
        ) && super::lexical::complete_predicate_adnominal_path(&self.resource, previous)
    }

    #[must_use]
    pub fn has_numeric_unit_before_dependent_noun(&self, text: &str, unit: &str) -> bool {
        numeric_unit_path_at(&self.resource, text, 0)
            .is_some_and(|path| path.dependent_tail.is_some() && &text[path.unit] == unit)
    }

    #[must_use]
    pub fn has_numeric_unit_surface(&self, text: &str) -> bool {
        let digits = text.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 || digits == text.len() {
            return false;
        }
        Self::is_dictionary_numeric_unit(&text[digits..])
            || numeric_unit_path_at(&self.resource, text, digits).is_some_and(|path| {
                path.dependent_tail.as_ref().unwrap_or(&path.unit).end == text.len()
                    && exact_analysis_starts_with_pos(&self.resource, &text[path.unit], |pos| {
                        matches!(pos, StructuralPos::NNB | StructuralPos::NNBC)
                    })
            })
    }

    #[must_use]
    pub fn has_object_marked_nominal(&self, text: &str) -> bool {
        has_complete_object_phrase(&self.resource, text)
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
    pub fn has_exact_adverb_surface(&self, text: &str) -> bool {
        has_exact_fine_pos(&self.resource, text, |pos| pos == DataFinePos::Mag)
    }

    #[must_use]
    pub fn has_adverb_topic_contraction(&self, adverb: &str, contracted: &str) -> bool {
        has_exact_sequence(&self.resource, adverb, &[StructuralPos::MAG])
            && has_exact_sequence(
                &self.resource,
                contracted,
                &[StructuralPos::MAG, StructuralPos::JX],
            )
    }

    #[must_use]
    pub fn has_boi_predicate(&self, text: &str) -> bool {
        ["보이", "보여", "보였", "보인", "보일", "보입", "보임"]
            .iter()
            .any(|stem| text.starts_with(stem))
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
        })
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
        self.supports_auxiliary_sequence_matching(text, node_limit, |_| true)
    }

    #[must_use]
    pub fn has_adnominal_auxiliary_tail(
        &self,
        text: &str,
        future: bool,
        present_or_past: bool,
    ) -> bool {
        let Some(head) = text.chars().next() else {
            return false;
        };
        let tail = &text[head.len_utf8()..];
        let ha = tail.starts_with(['하', '해', '했', '한', '할', '합', '함']);
        let predicate = match head {
            '만' | '법' | '뻔' if future && ha => tail,
            '척' | '체' | '양' if present_or_past && ha => tail,
            '듯' | '성' if ha || tail.starts_with('싶') => tail,
            '성' if ["부르", "부른", "부를", "부름", "부릅", "불러", "불렀"]
                .iter()
                .any(|form| tail.starts_with(form)) =>
            {
                text
            }
            _ => return false,
        };
        self.has_exact_predicate_ending_path_matching(
            predicate,
            StructuralPos::is_predicate_tag,
            |_| true,
        ) || complete_predicate_ending_path(&self.resource, predicate)
    }

    #[must_use]
    pub fn supports_auxiliary_with_fused_prefinal(&self, text: &str, node_limit: usize) -> bool {
        self.supports_auxiliary_sequence_matching(text, node_limit, |endings| {
            endings.first() == Some(&StructuralPos::EP)
        })
    }

    fn supports_auxiliary_sequence_matching(
        &self,
        text: &str,
        node_limit: usize,
        accepts_auxiliary_endings: impl Fn(&[StructuralPos]) -> bool,
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
                whole_predicate = true;
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
    pub fn has_complete_nominal_surface(&self, text: &str) -> bool {
        !text.is_empty() && complete_nominal_host(&self.resource, text)
    }

    #[must_use]
    pub fn has_exact_nominal_surface(&self, text: &str) -> bool {
        has_exact_fine_pos(&self.resource, text, DataFinePos::is_nominal)
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
                && self.has_exact_predicate_ending_path_matching(
                    &text[split..],
                    |pos| pos == StructuralPos::VCP,
                    |ending| ending == StructuralPos::EF,
                )
                && self.has_complete_nominal_surface(&text[..split])
        })
    }

    #[must_use]
    pub fn has_comparative_nominal_surface(&self, text: &str) -> bool {
        ["이상", "이하", "미만", "초과"].iter().any(|&noun| {
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
    pub fn has_nominal_oblique(&self, text: &str) -> bool {
        text.char_indices().skip(1).any(|(split, _)| {
            has_exact_sequence(&self.resource, &text[split..], &[StructuralPos::JKB])
                && self.has_complete_nominal_surface(&text[..split])
        })
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
    pub fn has_counted_unit_frame(&self, previous: &str, unit: &str) -> bool {
        exact_analysis_starts_with_pos(&self.resource, unit, |pos| pos == StructuralPos::NNBC)
            && (has_exact_fine_pos(&self.resource, previous, |pos| pos == DataFinePos::Nr)
                || (has_exact_fine_pos(&self.resource, previous, |pos| pos == DataFinePos::Mm)
                    && !(has_exact_fine_pos(&self.resource, unit, |pos| pos == DataFinePos::Vx)
                        && self.has_exact_predicate_connective_path(
                            previous,
                            PredicatePosSet::one(PredicatePos::Verb),
                        ))))
    }

    #[must_use]
    pub fn has_elapsed_measure(&self, measure: &str, previous: Option<&str>) -> bool {
        const UNITS: &[&str] = &[
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
            || UNITS.iter().any(|unit| {
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
        const RELATIVE_TIMES: &[&str] = &[
            "그제",
            "어제",
            "엊그제",
            "그끄저께",
            "그저께",
            "오늘",
            "내일",
            "모레",
            "글피",
            "방금",
            "지금",
        ];
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
                matched |= (positions == [StructuralPos::NR]
                    || DURATION_NOUNS.contains(&host)
                    || RELATIVE_TIMES.contains(&host))
                    && self.has_quantity_nominal_tail(host, &first[len..]);
            });
        matched
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
    pub fn has_exact_proper_name_surface(&self, text: &str) -> bool {
        has_exact_fine_pos(&self.resource, text, |pos| pos == DataFinePos::Nnp)
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

    #[must_use]
    pub fn has_exact_non_copula_predicate_surface(&self, text: &str) -> bool {
        exact_analysis_starts_with_pos(&self.resource, text, |pos| {
            pos.is_predicate() && pos != StructuralPos::VCP
        })
    }

    #[must_use]
    pub fn has_independent_final_predicate(&self, text: &str) -> bool {
        self.independent_final_predicate_evidence(text).0 != PredicatePosSet::EMPTY
    }

    #[must_use]
    pub fn has_independent_final_predicate_without_object(&self, text: &str) -> bool {
        let (positions, has_non_object_result) = self.independent_final_predicate_evidence(text);
        has_non_object_result || positions == PredicatePosSet::one(PredicatePos::Adjective)
    }

    fn independent_final_predicate_evidence(&self, text: &str) -> (PredicatePosSet, bool) {
        let has_final_ending = |length: usize, endings: &[StructuralPos]| {
            if length == text.len() {
                return endings.split_last().is_some_and(|(last, prefinals)| {
                    *last == StructuralPos::EF
                        && prefinals.iter().all(|pos| *pos == StructuralPos::EP)
                });
            }
            endings.iter().all(|pos| *pos == StructuralPos::EP)
                && super::lexical::has_exact_sequence(
                    &self.resource,
                    &text[length..],
                    &[StructuralPos::EF],
                )
        };
        let mut positions_found = PredicatePosSet::EMPTY;
        let mut has_final_auxiliary = false;
        let mut has_non_object_result = false;
        self.resource
            .common_prefix_positions(text.as_bytes(), |length, positions| {
                let Some((&predicate, endings)) = positions.split_first() else {
                    return;
                };
                match predicate {
                    StructuralPos::VV if has_final_ending(length, endings) => {
                        positions_found |= PredicatePosSet::one(PredicatePos::Verb);
                        has_non_object_result |= text[..length]
                            .strip_suffix(['나', '났', '난', '납'])
                            .is_some_and(|host| {
                                NON_OBJECT_RESULT_NADA_HOSTS.binary_search(&host).is_ok()
                            });
                    }
                    StructuralPos::VA if has_final_ending(length, endings) => {
                        positions_found |= PredicatePosSet::one(PredicatePos::Adjective);
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
                                    if suffix == StructuralPos::XSV {
                                        positions_found |= PredicatePosSet::one(PredicatePos::Verb);
                                    } else {
                                        positions_found |=
                                            PredicatePosSet::one(PredicatePos::Adjective);
                                        has_non_object_result |= text[length..]
                                            .starts_with(['하', '해', '했', '합'])
                                            && NON_OBJECT_RESULT_HADA_HOSTS
                                                .binary_search(&&text[..length])
                                                .is_ok();
                                    }
                                }
                            },
                        );
                    }
                    _ => {}
                }
            });
        if positions_found == PredicatePosSet::EMPTY
            || has_final_auxiliary
            || self.has_exact_nominal_surface(text)
        {
            return (PredicatePosSet::EMPTY, false);
        }
        (positions_found, has_non_object_result)
    }
}
