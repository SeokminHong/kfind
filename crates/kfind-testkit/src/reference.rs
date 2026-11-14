use std::cmp::Reverse;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;
use std::sync::Arc;

use kfind_data::{
    COPULA_RA_SUFFIXES, COPULA_REPORTING_AND_RO_ENDINGS, COPULA_STEM_ENDINGS,
    EndingSurface as ReferenceSuffix,
};
use kfind_morph::{ContinuationState, PredicatePos, RuleId, has_final, has_rieul_final};
use kfind_query::{
    CandidateConsumption, CandidateLeftContext, CandidateProgram, CoreMapping, Origin,
    PhraseJoinError, PhraseMatch, QueryPlan, VerifiedSpan, join_phrase_spans,
};
use unicode_general_category::{GeneralCategory, get_general_category};
use unicode_normalization::{UnicodeNormalization, is_nfc};

#[derive(Debug)]
pub struct ReferenceMatcher {
    plan: Arc<QueryPlan>,
}

impl ReferenceMatcher {
    pub fn new(plan: Arc<QueryPlan>) -> Result<Self, ReferenceMatcherBuildError> {
        if plan.atoms.is_empty() {
            return Err(ReferenceMatcherBuildError::EmptyPlan);
        }
        for (atom_index, atom) in plan.atoms.iter().enumerate() {
            if atom.programs.is_empty() {
                return Err(ReferenceMatcherBuildError::EmptyAtom { atom_index });
            }
            for (program_index, branch) in atom.programs.iter().enumerate() {
                if branch.anchor.is_empty() {
                    return Err(ReferenceMatcherBuildError::EmptyAnchor {
                        atom_index,
                        program_index,
                    });
                }
                if std::str::from_utf8(&branch.anchor).is_err() {
                    return Err(ReferenceMatcherBuildError::InvalidAnchorUtf8 {
                        atom_index,
                        program_index,
                    });
                }
            }
        }
        Ok(Self { plan })
    }

    #[must_use]
    pub fn plan(&self) -> &Arc<QueryPlan> {
        &self.plan
    }

    pub fn find_at_with_meta(
        &self,
        text: &str,
        at: usize,
    ) -> Result<Option<PhraseMatch>, ReferenceMatcherError> {
        if at > text.len() {
            return Ok(None);
        }
        let atom_spans = self.collect_atom_spans(text, at);
        if self.plan.atoms.len() == 1 {
            let matched = atom_spans[0]
                .iter()
                .min_by_key(|span| span_order(span))
                .cloned();
            return Ok(matched.map(|span| PhraseMatch {
                span: span.token.clone(),
                atoms: vec![span],
            }));
        }

        let matches = join_phrase_spans(text, &atom_spans, self.plan.phrase_policy)
            .map_err(ReferenceMatcherError::PhraseJoin)?;
        Ok(matches
            .into_iter()
            .min_by_key(|matched| (matched.span.start, Reverse(matched.span.end))))
    }

    pub fn find_all_with_meta(
        &self,
        text: &str,
    ) -> Result<Vec<PhraseMatch>, ReferenceMatcherError> {
        let mut matches = Vec::new();
        let mut at = 0;
        while let Some(matched) = self.find_at_with_meta(text, at)? {
            at = matched.span.end;
            matches.push(matched);
        }
        Ok(matches)
    }

    fn collect_atom_spans(&self, text: &str, at: usize) -> Vec<Vec<VerifiedSpan>> {
        let mut atom_spans = vec![Vec::new(); self.plan.atoms.len()];
        for start in text
            .char_indices()
            .map(|(index, _)| index)
            .filter(|start| *start >= at)
        {
            for (atom_index, atom) in self.plan.atoms.iter().enumerate() {
                for branch in &atom.programs {
                    if let Some(span) = self.execute_program(text, start, branch) {
                        atom_spans[atom_index].push(span);
                    }
                }
            }
        }
        for spans in &mut atom_spans {
            merge_duplicate_spans(spans);
        }
        atom_spans
    }

    fn execute_program(
        &self,
        text: &str,
        start: usize,
        branch: &CandidateProgram,
    ) -> Option<VerifiedSpan> {
        let anchor = std::str::from_utf8(&branch.anchor).ok()?;
        let anchor_end = start.checked_add(anchor.len())?;
        if !text.get(start..)?.starts_with(anchor) {
            return None;
        }
        let core = mapped_core(start..anchor_end, branch.core_mapping, anchor)?;
        let following = &text[anchor_end..];
        let normalized_anchor = (!is_nfc(anchor)).then(|| anchor.nfc().collect::<String>());
        let normalized_following = normalized_anchor
            .is_some()
            .then(|| following.nfc().collect::<String>());
        let consumption_anchor = normalized_anchor.as_deref().unwrap_or(anchor);
        let consumption_following = normalized_following.as_deref().unwrap_or(following);
        let (normalized_consumed, suffix_rules) = match &branch.consumption {
            CandidateConsumption::Anchor | CandidateConsumption::CopulaHostEndingCompose => {
                (0, Vec::new())
            }
            CandidateConsumption::PredicateContinuation {
                continuation,
                pos,
                nominal_particle_transition,
                left_context,
                ..
            } => {
                if !accepts_left_context(left_context, text, start) {
                    return None;
                }
                let (mut consumed, mut rules) = reference_predicate_continuation(
                    *continuation,
                    *pos,
                    consumption_anchor,
                    consumption_following,
                )?;
                if *pos == PredicatePos::Copula
                    && *continuation == ContinuationState::StemEnding
                    && rules
                        .iter()
                        .any(|rule| rule.as_str().starts_with("ending.copula-"))
                    && !text[..start]
                        .chars()
                        .next_back()
                        .is_some_and(is_reference_token_character)
                {
                    return None;
                }
                if !branch.consumption.allows_rule_path(&rules) {
                    return None;
                }
                if let Some(tail) = consumption_following[consumed..].strip_prefix("부터")
                    && rules.last().map_or_else(
                        || {
                            !branch.origins.is_empty()
                                && branch.origins.iter().all(|origin| {
                                    origin.rule_path.last().is_some_and(|rule| {
                                        rule.as_str() == "ending.simultaneous-myeonseo"
                                    })
                                })
                        },
                        |rule| rule.as_str() == "ending.simultaneous-myeonseo",
                    )
                {
                    let end = tail
                        .find(|character| !is_reference_token_character(character))
                        .unwrap_or(tail.len());
                    if matches!(&tail[..end], "" | "다" | "이다") {
                        consumed += "부터".len() + end;
                        rules.push(RuleId::from("particle.from"));
                        if end != 0 {
                            rules.push(RuleId::from("ending.final-da"));
                        }
                    }
                }
                if *nominal_particle_transition
                    || matches!(rules.as_slice(), [auxiliary, nominalizer]
                        if matches!(auxiliary.as_str(), "ending.auxiliary-jida" | "ending.auxiliary-boda")
                            && matches!(nominalizer.as_str(), "ending.nominalizer" | "ending.nominalizer-gi"))
                {
                    let host = format!(
                        "{consumption_anchor}{}",
                        consumption_following.get(..consumed)?
                    );
                    let (particle_consumed, particle_rules) =
                        self.reference_particles(&host, &consumption_following[consumed..]);
                    consumed = consumed.checked_add(particle_consumed)?;
                    rules.extend(particle_rules);
                }
                (consumed, rules)
            }
            CandidateConsumption::StructuralPredicateEnding { .. }
            | CandidateConsumption::FusedCopulaEnding => return None,
            CandidateConsumption::NominalParticleChain { .. }
            | CandidateConsumption::NominalCopulaEndingChain { .. } => {
                let (consumed, rules) =
                    self.reference_particles(consumption_anchor, consumption_following);
                if !branch.consumption.allows_rule_path(&rules) {
                    return None;
                }
                (consumed, rules)
            }
            CandidateConsumption::DirectParticleHost { rule_id } => {
                if requires_direct_particle_host(branch)
                    && !self.accepts_direct_particle(text, start, anchor_end, rule_id)
                {
                    return None;
                }
                (0, Vec::new())
            }
        };
        let consumed = if let Some(normalized) = normalized_following.as_deref() {
            map_normalized_prefix(following, normalized, normalized_consumed)?
        } else {
            normalized_consumed
        };
        let token = start..anchor_end.checked_add(consumed)?;
        if matches!(
            &branch.consumption,
            CandidateConsumption::PredicateContinuation {
                pos: PredicatePos::Copula,
                ..
            }
        ) && suffix_rules
            .iter()
            .any(|rule| rule.as_str() == "ending.copula-ya")
            && !text.get(token.end..)?.chars().next().is_none_or(|next| {
                matches!(
                    next,
                    '.' | '!' | '?' | '…' | '。' | '！' | '？' | '\n' | '\r' | '"' | '”' | '’'
                )
            })
        {
            return None;
        }
        if !accepts_boundaries(text, &core, &token, branch) {
            return None;
        }
        Some(VerifiedSpan {
            core,
            token,
            origins: extend_origins(&branch.origins, &suffix_rules),
        })
    }

    fn accepts_direct_particle(
        &self,
        text: &str,
        start: usize,
        anchor_end: usize,
        rule_id: &RuleId,
    ) -> bool {
        let Some(anchor) = text.get(start..anchor_end) else {
            return false;
        };
        let Some(left) = text.get(..start) else {
            return false;
        };
        let normalized_anchor = anchor.nfc().collect::<String>();
        let normalized_left = left.nfc().collect::<String>();
        let Some(previous) = normalized_left.chars().next_back() else {
            return false;
        };
        if !is_reference_token_character(previous) {
            return false;
        }
        let normalized_context = format!("{normalized_left}{normalized_anchor}");
        if REFERENCE_PARTICLES.iter().any(|form| {
            form.rule_id == rule_id.as_str()
                && form.surface.len() > normalized_anchor.len()
                && form.surface.ends_with(&normalized_anchor)
                && normalized_context.ends_with(form.surface)
        }) {
            return false;
        }

        REFERENCE_PARTICLES.iter().any(|form| {
            form.surface == normalized_anchor
                && form.rule_id == rule_id.as_str()
                && form.condition.accepts(previous)
        })
    }

    fn reference_particles(&self, core: &str, following: &str) -> (usize, Vec<RuleId>) {
        let mut consumed = 0;
        let mut previous = core.chars().next_back();
        let mut rules = Vec::new();

        if following.starts_with('들') {
            consumed += '들'.len_utf8();
            previous = Some('들');
            rules.push(RuleId::from("particle.plural"));
        }
        while rules.len() < 4 {
            let Some(form) = self.longest_reference_particle(
                &following[consumed..],
                previous,
                rules.last(),
                ReferenceParticleRole::Any,
            ) else {
                break;
            };
            consumed += form.surface.len();
            previous = form.surface.chars().next_back();
            rules.push(RuleId::from(form.rule_id));
        }
        (consumed, rules)
    }

    fn longest_reference_particle(
        &self,
        following: &str,
        previous: Option<char>,
        previous_rule: Option<&RuleId>,
        role: ReferenceParticleRole,
    ) -> Option<&'static ReferenceParticle> {
        let previous = previous?;
        REFERENCE_PARTICLES
            .iter()
            .filter(|form| {
                (role == ReferenceParticleRole::Any || form.role == role)
                    && following.starts_with(form.surface)
                    && form.condition.accepts(previous)
                    && self.reference_transition_allows(previous_rule, form.rule_id)
            })
            .max_by_key(|form| form.surface.len())
    }

    fn reference_transition_allows(&self, previous: Option<&RuleId>, next: &str) -> bool {
        let Some(previous) = previous else {
            return true;
        };
        self.plan
            .particle_transitions
            .iter()
            .find(|transition| transition.rule_id == *previous)
            .is_some_and(|transition| transition.next.iter().any(|rule| rule.as_str() == next))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReferenceParticleRole {
    Any,
    Case,
    Auxiliary,
}

#[derive(Clone, Copy, Debug)]
enum ReferenceFinalCondition {
    Any,
    Vowel,
    Consonant,
    VowelOrRieul,
    ConsonantExceptRieul,
}

impl ReferenceFinalCondition {
    fn accepts(self, previous: char) -> bool {
        match self {
            Self::Any => true,
            Self::Vowel => !kfind_morph::has_final(previous),
            Self::Consonant => kfind_morph::has_final(previous),
            Self::VowelOrRieul => {
                !kfind_morph::has_final(previous) || kfind_morph::has_rieul_final(previous)
            }
            Self::ConsonantExceptRieul => {
                kfind_morph::has_final(previous) && !kfind_morph::has_rieul_final(previous)
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct ReferenceParticle {
    surface: &'static str,
    rule_id: &'static str,
    role: ReferenceParticleRole,
    condition: ReferenceFinalCondition,
}

const fn reference_particle(
    surface: &'static str,
    rule_id: &'static str,
    role: ReferenceParticleRole,
    condition: ReferenceFinalCondition,
) -> ReferenceParticle {
    ReferenceParticle {
        surface,
        rule_id,
        role,
        condition,
    }
}

const REFERENCE_PARTICLES: &[ReferenceParticle] = &[
    reference_particle(
        "에게서",
        "particle.source.egeseo",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "한테서",
        "particle.source.hanteseo",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "으로서",
        "particle.capacity.roseo",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::ConsonantExceptRieul,
    ),
    reference_particle(
        "으로써",
        "particle.instrument.rosseo",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::ConsonantExceptRieul,
    ),
    reference_particle(
        "로서",
        "particle.capacity.roseo",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::VowelOrRieul,
    ),
    reference_particle(
        "로써",
        "particle.instrument.rosseo",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::VowelOrRieul,
    ),
    reference_particle(
        "이라도",
        "particle.concessive.irado-rado",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Consonant,
    ),
    reference_particle(
        "이나마",
        "particle.concessive.inama-nama",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Consonant,
    ),
    reference_particle(
        "으로",
        "particle.direction",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::ConsonantExceptRieul,
    ),
    reference_particle(
        "로",
        "particle.direction",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::VowelOrRieul,
    ),
    reference_particle(
        "에게",
        "particle.dative",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "한테",
        "particle.dative",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "께서",
        "particle.subject.honorific",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "같이",
        "particle.similarity.gachi",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "대로",
        "particle.conformance.daero",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "더러",
        "particle.dative.deoreo",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "마다",
        "particle.distributive.mada",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "만큼",
        "particle.extent.mankeum",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "밖에",
        "particle.exclusive.bakke",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "보고",
        "particle.dative.bogo",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "보다",
        "particle.comparison.boda",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "뿐",
        "particle.restrictive.ppun",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "처럼",
        "particle.similarity.cheoreom",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "치고",
        "particle.standard.chigo",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "커녕",
        "particle.contrast.keonyeong",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "이나",
        "particle.alternative.ina-na",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Consonant,
    ),
    reference_particle(
        "이랑",
        "particle.comitative.irang-rang",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Consonant,
    ),
    reference_particle(
        "하고",
        "particle.comitative.hago",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "께",
        "particle.dative",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "에서",
        "particle.source",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "에",
        "particle.locative",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "의",
        "particle.genitive",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "이",
        "particle.subject",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Consonant,
    ),
    reference_particle(
        "가",
        "particle.subject",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Vowel,
    ),
    reference_particle(
        "을",
        "particle.object",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Consonant,
    ),
    reference_particle(
        "를",
        "particle.object",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Vowel,
    ),
    reference_particle(
        "과",
        "particle.comitative",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Consonant,
    ),
    reference_particle(
        "와",
        "particle.comitative",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Vowel,
    ),
    reference_particle(
        "이면",
        "particle.connector-myeon",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Consonant,
    ),
    reference_particle(
        "면",
        "particle.connector-myeon",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Vowel,
    ),
    reference_particle(
        "은",
        "particle.topic",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Consonant,
    ),
    reference_particle(
        "는",
        "particle.topic",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Vowel,
    ),
    reference_particle(
        "도",
        "particle.additive",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "만",
        "particle.only",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "까지",
        "particle.limit.ggaji",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "부터",
        "particle.from",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "조차",
        "particle.even.jocha",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "마저",
        "particle.even.majeo",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Any,
    ),
    reference_particle(
        "라도",
        "particle.concessive.irado-rado",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Vowel,
    ),
    reference_particle(
        "나마",
        "particle.concessive.inama-nama",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Vowel,
    ),
    reference_particle(
        "나",
        "particle.alternative.ina-na",
        ReferenceParticleRole::Auxiliary,
        ReferenceFinalCondition::Vowel,
    ),
    reference_particle(
        "랑",
        "particle.comitative.irang-rang",
        ReferenceParticleRole::Case,
        ReferenceFinalCondition::Vowel,
    ),
];

const fn reference_suffix(
    surface: &'static str,
    rules: &'static [&'static str],
) -> ReferenceSuffix {
    ReferenceSuffix { surface, rules }
}

fn reference_predicate_continuation(
    state: ContinuationState,
    pos: PredicatePos,
    anchor: &str,
    following: &str,
) -> Option<(usize, Vec<RuleId>)> {
    if pos.is_action()
        && matches!(state, ContinuationState::StemEnding | ContinuationState::Eu)
        && let Some(auxiliary) = reference_gopeuda(state, following)
    {
        return Some(auxiliary);
    }
    if state == ContinuationState::Eu && pos.is_action() {
        for (prefix, contraction) in [("시어", None), ("셔", Some("contraction.i-eo"))] {
            if let Some(tail) = following.strip_prefix(prefix)
                && tail.starts_with(['보', '봐', '봤', '본', '볼', '봅', '봄'])
                && let Some((consumed, auxiliary)) = reference_attached_auxiliary(pos, tail)
            {
                let rules = std::iter::once("ending.honorific")
                    .chain(contraction)
                    .chain(["ending.aoeo"])
                    .map(RuleId::from)
                    .chain(auxiliary)
                    .collect();
                return Some((prefix.len() + consumed, rules));
            }
        }
    }
    if state == ContinuationState::AOrEo
        && let Some(auxiliary) = reference_attached_auxiliary(pos, following)
    {
        return Some(auxiliary);
    }
    reference_report_contraction(state, pos, anchor, following)
        .or_else(|| reference_retrospective_reo(state, pos, anchor, following))
        .or_else(|| reference_simple_continuation(state, pos, anchor, following))
}

fn reference_retrospective_reo(
    state: ContinuationState,
    pos: PredicatePos,
    anchor: &str,
    following: &str,
) -> Option<(usize, Vec<RuleId>)> {
    use ContinuationState::{Eu, Future, Past, Prospective, StemEnding};
    let token = following
        .split(|ch: char| !ch.is_alphanumeric() && ch != '_')
        .next()?;
    let (prefix, ending) = token.split_at(token.find('러')?);
    let prior: &[&str] = match (state, prefix) {
        (StemEnding, "") if pos == PredicatePos::Copula || anchor == "아니" => &[],
        (Prospective, "") if anchor.ends_with('시') => &[],
        (Eu, "시") => &["ending.honorific"],
        (StemEnding | Past | Future, "더") => &["ending.retrospective-prefinal"],
        (Eu, "시더") => &["ending.honorific", "ending.retrospective-prefinal"],
        (Eu | Prospective, "리") | (Past | Future, "으리") => &["ending.prospective-prefinal"],
        (Eu, "시리") => &["ending.honorific", "ending.prospective-prefinal"],
        _ => return None,
    };
    let rule = match ending {
        "러니" => "ending.retrospective-reoni",
        "러니라" | "러니이까" | "러니이다" | "러라" | "러이까" | "러이다" => {
            "ending.retrospective-reo-final"
        }
        _ => return None,
    };
    Some((
        token.len(),
        prior
            .iter()
            .copied()
            .chain([rule])
            .map(RuleId::from)
            .collect(),
    ))
}

fn reference_gopeuda(state: ContinuationState, following: &str) -> Option<(usize, Vec<RuleId>)> {
    use ContinuationState::{AOrEo, BieupFinal, Eu, NieunEnding, Past, RieulFinal, Terminal};
    let (prefix, honorific) = if state == Eu && following.starts_with("시고") {
        ("시고", Some("ending.honorific"))
    } else {
        ("고", None)
    };
    let tail = following.strip_prefix(prefix)?;
    let (form, next, initial) = [
        ("픈", NieunEnding, Some("ending.past-adnominal")),
        ("플", RieulFinal, Some("ending.future-adnominal")),
        ("픕", BieupFinal, None),
        ("픔", Terminal, Some("ending.nominalizer")),
        ("파", AOrEo, Some("ending.aoeo")),
        ("팠", Past, Some("ending.past")),
        ("프다", Terminal, Some("ending.final-da")),
        ("프기", Terminal, Some("ending.nominalizer-gi")),
        ("프고", Terminal, Some("ending.connective-go")),
        ("프", Eu, None),
    ]
    .into_iter()
    .find(|(form, _, _)| tail.starts_with(form))?;
    let remaining = tail.strip_prefix(form)?;
    let (extra, rules) =
        reference_simple_continuation(next, PredicatePos::AuxiliaryAdjective, form, remaining)
            .or_else(|| matches!(next, NieunEnding | RieulFinal).then(|| (0, Vec::new())))?;
    if (matches!(next, Eu | Past) && extra == 0)
        || remaining[extra..]
            .chars()
            .next()
            .is_some_and(|next| next.is_alphanumeric() || next == '_')
    {
        return None;
    }
    let initial = initial.filter(|_| !matches!(next, NieunEnding | RieulFinal) || extra == 0);
    Some((
        prefix.len() + form.len() + extra,
        honorific
            .into_iter()
            .chain(["ending.connective-go", "ending.auxiliary-gopeuda"])
            .chain(initial)
            .map(RuleId::from)
            .chain(rules)
            .collect(),
    ))
}

fn reference_report_contraction(
    state: ContinuationState,
    pos: PredicatePos,
    anchor: &str,
    following: &str,
) -> Option<(usize, Vec<RuleId>)> {
    use ContinuationState::{Eu, Future, NieunEnding, Past, Prospective, ReportEnding, StemEnding};

    let stem = anchor.chars().next_back();
    let rieul = stem.is_some_and(has_rieul_final);
    let prefinal = matches!(state, Past | Future);
    let fused_declarative = state == ReportEnding && pos.is_action() && stem.is_some_and(has_final);
    let (prefix, tail) = if state == Eu {
        ["셨겠", "시겠", "셨", "신", "시"]
            .into_iter()
            .find_map(|prefix| {
                let rest = following.strip_prefix(prefix)?;
                let allowed = rest.starts_with(['대', '댔']) && (prefix != "신" || pos.is_action())
                    || rest.starts_with('냬') && prefix != "신"
                    || rest.starts_with(['래', '랬']) && prefix == "시" && pos.is_action();
                allowed.then_some((prefix, rest))
            })
            .unwrap_or(("", following))
    } else {
        ("", following)
    };
    let declarative = prefinal
        || fused_declarative
        || !prefix.is_empty()
        || (state == StemEnding
            && !pos.is_action()
            && pos != PredicatePos::Copula
            && anchor != "아니")
        || (state == Prospective && anchor.ends_with('시'))
        || (state == NieunEnding && pos.is_action() && anchor.ends_with('신'));
    let closed_declarative =
        state == StemEnding && pos.is_action() && stem.is_some_and(has_final) && !rieul;
    let directive = pos.is_action() && matches!(state, Eu | Prospective);
    let question = prefinal
        || (state == StemEnding && !rieul)
        || (state == ReportEnding && !fused_declarative)
        || (state == Eu && (!pos.is_action() || !prefix.is_empty()));
    let full_question = prefinal
        || (state == StemEnding && !rieul && (pos.is_action() || anchor.ends_with(['있', '없'])))
        || (state == ReportEnding && !fused_declarative && pos.is_action());
    let proposal = state == StemEnding && pos.is_action();
    let (form, past) = [
        ("대", declarative, false),
        ("댔", declarative, true),
        ("는대", closed_declarative, false),
        ("는댔", closed_declarative, true),
        ("래", directive, false),
        (
            "랬",
            directive || (state == StemEnding && (pos == PredicatePos::Copula || anchor == "아니")),
            true,
        ),
        ("냬", question, false),
        ("느냬", full_question, false),
        ("재", proposal, false),
        ("쟀", proposal, true),
    ]
    .into_iter()
    .find_map(|(form, enabled, past)| {
        (enabled && tail.starts_with(form)).then_some((form, past))
    })?;
    let rest = &tail[form.len()..];
    let mut path = Vec::new();
    if !prefix.is_empty() {
        path.push(RuleId::from("ending.honorific"));
        if prefix.starts_with('셨') {
            path.push(RuleId::from("ending.past"));
        }
        if prefix.ends_with('겠') {
            path.push(RuleId::from("ending.future"));
        }
    }
    path.push(RuleId::from("ending.report-contraction"));
    let continuation_len = if past {
        let (consumed, suffix) =
            reference_simple_continuation(Past, PredicatePos::Verb, form, rest)?;
        if consumed == 0 {
            return None;
        }
        path.push(RuleId::from("ending.past"));
        path.extend(suffix);
        consumed
    } else if rest.starts_with('요') {
        path.push(RuleId::from("ending.polite-yo"));
        '요'.len_utf8()
    } else {
        0
    };
    Some((prefix.len() + form.len() + continuation_len, path))
}

fn reference_simple_continuation(
    state: ContinuationState,
    pos: PredicatePos,
    anchor: &str,
    following: &str,
) -> Option<(usize, Vec<RuleId>)> {
    const AEO: &[ReferenceSuffix] = &[
        reference_suffix("야지", &["ending.necessity-yaji"]),
        reference_suffix("야지요", &["ending.necessity-yaji-polite"]),
        reference_suffix("야죠", &["ending.necessity-yaji-polite"]),
        reference_suffix("가고", &["ending.auxiliary-gada", "ending.connective-go"]),
        reference_suffix("가야", &["ending.auxiliary-gada", "ending.connective-ya"]),
        reference_suffix("서도", &["ending.aoeo-seo", "particle.additive"]),
        reference_suffix("서", &["ending.aoeo-seo"]),
        reference_suffix("서야", &["ending.aoeo-seoya"]),
        reference_suffix("도", &["ending.connective-do"]),
        reference_suffix("야", &["ending.connective-ya"]),
        reference_suffix("요", &["ending.polite-yo"]),
        reference_suffix("라", &["ending.imperative-ra"]),
    ];
    const DEON_RECOLLECTION: &[ReferenceSuffix] = &[
        reference_suffix("던가", &["ending.retrospective-question-deon"]),
        reference_suffix("던감", &["ending.retrospective-question-deon"]),
        reference_suffix("던들", &["ending.retrospective-connective-deon"]),
        reference_suffix("던지", &["ending.retrospective-connective-deon"]),
    ];
    macro_rules! retrospective {
        ($prefix:literal $(, $prior:literal)*) => {
            &[
                reference_suffix(concat!($prefix, "더라"), &[$($prior,)* "ending.retrospective-deora"]),
                reference_suffix(concat!($prefix, "더라오"), &[$($prior,)* "ending.retrospective-deora", "ending.hao-reporting-dao"]),
                reference_suffix(concat!($prefix, "더라고"), &[$($prior,)* "ending.retrospective-deora", "ending.quotative-go"]),
                reference_suffix(concat!($prefix, "더라고요"), &[$($prior,)* "ending.retrospective-deora", "ending.quotative-go", "ending.polite-yo"]),
                reference_suffix(concat!($prefix, "더구나"), &[$($prior,)* "ending.retrospective-exclamatory"]),
                reference_suffix(concat!($prefix, "더구려"), &[$($prior,)* "ending.retrospective-exclamatory"]),
                reference_suffix(concat!($prefix, "더구먼"), &[$($prior,)* "ending.retrospective-exclamatory"]),
                reference_suffix(concat!($prefix, "더군"), &[$($prior,)* "ending.final-deogun"]),
                reference_suffix(concat!($prefix, "더냐"), &[$($prior,)* "ending.retrospective-direct-question"]),
                reference_suffix(concat!($prefix, "더뇨"), &[$($prior,)* "ending.retrospective-direct-question"]),
                reference_suffix(concat!($prefix, "더니이까"), &[$($prior,)* "ending.retrospective-direct-question"]),
                reference_suffix(concat!($prefix, "더이까"), &[$($prior,)* "ending.retrospective-direct-question"]),
                reference_suffix(concat!($prefix, "더니라"), &[$($prior,)* "ending.retrospective-direct-final"]),
                reference_suffix(concat!($prefix, "더니이다"), &[$($prior,)* "ending.retrospective-direct-final"]),
                reference_suffix(concat!($prefix, "더이다"), &[$($prior,)* "ending.retrospective-direct-final"]),
                reference_suffix(concat!($prefix, "더람"), &[$($prior,)* "ending.retrospective-direct-final"]),
                reference_suffix(concat!($prefix, "더니"), &[$($prior,)* "ending.retrospective-connective"]),
                reference_suffix(concat!($prefix, "더니마는"), &[$($prior,)* "ending.retrospective-contrast"]),
                reference_suffix(concat!($prefix, "더니만"), &[$($prior,)* "ending.retrospective-contrast"]),
                reference_suffix(concat!($prefix, "더라나"), &[$($prior,)* "ending.retrospective-deora-report"]),
                reference_suffix(concat!($prefix, "더라니"), &[$($prior,)* "ending.retrospective-deora-report"]),
                reference_suffix(concat!($prefix, "더라니까"), &[$($prior,)* "ending.retrospective-deora-report"]),
                reference_suffix(concat!($prefix, "더라며"), &[$($prior,)* "ending.retrospective-deora-report"]),
                reference_suffix(concat!($prefix, "더라면서"), &[$($prior,)* "ending.retrospective-deora-report"]),
                reference_suffix(concat!($prefix, "더라지"), &[$($prior,)* "ending.retrospective-deora-report"]),
                reference_suffix(concat!($prefix, "더라면"), &[$($prior,)* "ending.retrospective-deora-condition"]),
                reference_suffix(concat!($prefix, "더라손"), &[$($prior,)* "ending.retrospective-deora-condition"]),
                reference_suffix(concat!($prefix, "더라도"), &[$($prior,)* "ending.concessive-deorado"]),
                reference_suffix(concat!($prefix, "더군요"), &[$($prior,)* "ending.final-deogun", "ending.polite-yo"]),
                reference_suffix(concat!($prefix, "더라니까요"), &[$($prior,)* "ending.retrospective-deora-report", "ending.polite-yo"]),
            ]
        };
    }
    const PAST: &[ReferenceSuffix] = &[
        reference_suffix("다가는", &["ending.connective-daganeun"]),
        reference_suffix("다간", &["ending.connective-daganeun"]),
        reference_suffix("단", &["ending.connective-daganeun"]),
        reference_suffix("으니", &["ending.connective-ni"]),
        reference_suffix("으니까", &["ending.connective-ni"]),
        reference_suffix("으니까는", &["ending.connective-ni"]),
        reference_suffix("으니깐", &["ending.connective-ni"]),
        reference_suffix("자", &["ending.concessive-eotja"]),
        reference_suffix("제", &["ending.dialect-past-je"]),
        reference_suffix("남", &["ending.self-question-nam"]),
        reference_suffix("군", &["ending.final-gun"]),
        reference_suffix("군요", &["ending.final-gun", "ending.polite-yo"]),
        reference_suffix("습니까", &["ending.formal-interrogative"]),
        reference_suffix("습디까", &["ending.retrospective-interrogative"]),
        reference_suffix("습디다", &["ending.retrospective-declarative"]),
        reference_suffix("어야지", &["ending.necessity-yaji"]),
        reference_suffix("어야지요", &["ending.necessity-yaji-polite"]),
        reference_suffix("어야죠", &["ending.necessity-yaji-polite"]),
        reference_suffix("건", &["ending.geodeun-short"]),
        reference_suffix("거든", &["ending.geodeun"]),
        reference_suffix("거든요", &["ending.geodeun-polite"]),
        reference_suffix("거들랑", &["ending.geodeun"]),
        reference_suffix("습니다", &["ending.polite-declarative"]),
        reference_suffix("으되", &["ending.connective-eudoe"]),
        reference_suffix("느냐는", &["ending.interrogative-neunya", "particle.topic"]),
        reference_suffix("으면", &["ending.conditional"]),
        reference_suffix("으며", &["ending.coordinate-myeo"]),
        reference_suffix("지만", &["ending.connective-jiman"]),
        reference_suffix("는데", &["ending.connective-neunde"]),
        reference_suffix("다고", &["ending.quotative-go"]),
        reference_suffix("느냐", &["ending.interrogative-neunya"]),
        reference_suffix("던", &["ending.retrospective-adnominal"]),
        reference_suffix("을", &["ending.future-adnominal"]),
        reference_suffix("다", &["ending.final-da"]),
        reference_suffix("고", &["ending.connective-go"]),
        reference_suffix("어", &["ending.prefinal-eo"]),
        reference_suffix("어요", &["ending.past-polite-yo"]),
        reference_suffix("게", &["ending.final-ge"]),
    ];
    const FUTURE: &[ReferenceSuffix] = &[
        reference_suffix("으니", &["ending.connective-ni"]),
        reference_suffix("으니까", &["ending.connective-ni"]),
        reference_suffix("으니까는", &["ending.connective-ni"]),
        reference_suffix("으니깐", &["ending.connective-ni"]),
        reference_suffix("남", &["ending.self-question-nam"]),
        reference_suffix("다가는", &["ending.connective-daganeun"]),
        reference_suffix("다간", &["ending.connective-daganeun"]),
        reference_suffix("단", &["ending.connective-daganeun"]),
        reference_suffix("군", &["ending.final-gun"]),
        reference_suffix("군요", &["ending.final-gun", "ending.polite-yo"]),
        reference_suffix("습니까", &["ending.formal-interrogative"]),
        reference_suffix("습디까", &["ending.retrospective-interrogative"]),
        reference_suffix("습디다", &["ending.retrospective-declarative"]),
        reference_suffix("습니다", &["ending.polite-declarative"]),
        reference_suffix("으되", &["ending.connective-eudoe"]),
        reference_suffix("지만", &["ending.connective-jiman"]),
        reference_suffix("는데", &["ending.connective-neunde"]),
        reference_suffix("다", &["ending.final-da"]),
        reference_suffix("고", &["ending.connective-go"]),
        reference_suffix("어요", &["ending.aoeo", "ending.polite-yo"]),
    ];
    const DECLARATIVE: &[ReferenceSuffix] = &[
        reference_suffix(
            "는군요",
            &[
                "ending.quotative-adnominal",
                "ending.final-neun-exclamative",
                "ending.polite-yo",
            ],
        ),
        reference_suffix("더니마는", &["ending.retrospective-contrast"]),
        reference_suffix("더니만", &["ending.retrospective-contrast"]),
        reference_suffix("더니", &["ending.retrospective-connective"]),
        reference_suffix("면서", &["ending.quotative-myeonseo"]),
        reference_suffix("는데", &["ending.quotative-neunde"]),
        reference_suffix("고", &["ending.quotative-go"]),
        reference_suffix("는", &["ending.quotative-adnominal"]),
        reference_suffix("던", &["ending.quotative-retrospective"]),
        reference_suffix("면", &["ending.conditional"]),
        reference_suffix("니", &["ending.quotative-ni"]),
        reference_suffix("며", &["ending.quotative-myeo"]),
        reference_suffix("지", &["ending.quotative-ji"]),
        reference_suffix("오", &["ending.hao-reporting-dao"]),
        reference_suffix("우", &["ending.hao-final-u"]),
    ];
    const EU: &[ReferenceSuffix] = &[
        reference_suffix("시구", &["ending.honorific", "ending.connective-go"]),
        reference_suffix(
            "셨구",
            &["ending.honorific", "ending.past", "ending.connective-go"],
        ),
        reference_suffix(
            "시겠구",
            &["ending.honorific", "ending.future", "ending.connective-go"],
        ),
        reference_suffix("시남", &["ending.honorific", "ending.self-question-nam"]),
        reference_suffix(
            "시다가는",
            &["ending.honorific", "ending.connective-daganeun"],
        ),
        reference_suffix(
            "시다간",
            &["ending.honorific", "ending.connective-daganeun"],
        ),
        reference_suffix("시단", &["ending.honorific", "ending.connective-daganeun"]),
        reference_suffix(
            "셨다가는",
            &[
                "ending.honorific",
                "ending.past",
                "ending.connective-daganeun",
            ],
        ),
        reference_suffix(
            "셨다간",
            &[
                "ending.honorific",
                "ending.past",
                "ending.connective-daganeun",
            ],
        ),
        reference_suffix(
            "셨단",
            &[
                "ending.honorific",
                "ending.past",
                "ending.connective-daganeun",
            ],
        ),
        reference_suffix(
            "시겠다가는",
            &[
                "ending.honorific",
                "ending.future",
                "ending.connective-daganeun",
            ],
        ),
        reference_suffix(
            "시겠다간",
            &[
                "ending.honorific",
                "ending.future",
                "ending.connective-daganeun",
            ],
        ),
        reference_suffix(
            "시겠단",
            &[
                "ending.honorific",
                "ending.future",
                "ending.connective-daganeun",
            ],
        ),
        reference_suffix("시것다", &["ending.honorific", "ending.geotda"]),
        reference_suffix(
            "셨것다",
            &["ending.honorific", "ending.past", "ending.geotda"],
        ),
        reference_suffix("시게", &["ending.honorific", "ending.adverbial-ge"]),
        reference_suffix(
            "셨게",
            &["ending.honorific", "ending.past", "ending.final-ge"],
        ),
        reference_suffix(
            "십니까",
            &["ending.honorific", "ending.formal-interrogative"],
        ),
        reference_suffix(
            "십디까",
            &["ending.honorific", "ending.retrospective-interrogative"],
        ),
        reference_suffix(
            "십디다",
            &["ending.honorific", "ending.retrospective-declarative"],
        ),
        reference_suffix(
            "셨습니까",
            &[
                "ending.honorific",
                "ending.past",
                "ending.formal-interrogative",
            ],
        ),
        reference_suffix(
            "셨습디까",
            &[
                "ending.honorific",
                "ending.past",
                "ending.retrospective-interrogative",
            ],
        ),
        reference_suffix(
            "셨습디다",
            &[
                "ending.honorific",
                "ending.past",
                "ending.retrospective-declarative",
            ],
        ),
        reference_suffix(
            "시겠습니까",
            &[
                "ending.honorific",
                "ending.future",
                "ending.formal-interrogative",
            ],
        ),
        reference_suffix(
            "시겠습디까",
            &[
                "ending.honorific",
                "ending.future",
                "ending.retrospective-interrogative",
            ],
        ),
        reference_suffix(
            "시겠습디다",
            &[
                "ending.honorific",
                "ending.future",
                "ending.retrospective-declarative",
            ],
        ),
        reference_suffix(
            "셔야지",
            &["ending.honorific", "ending.aoeo", "ending.necessity-yaji"],
        ),
        reference_suffix(
            "셔야지요",
            &[
                "ending.honorific",
                "ending.aoeo",
                "ending.necessity-yaji-polite",
            ],
        ),
        reference_suffix(
            "셔야죠",
            &[
                "ending.honorific",
                "ending.aoeo",
                "ending.necessity-yaji-polite",
            ],
        ),
        reference_suffix("시건", &["ending.honorific", "ending.geodeun-short"]),
        reference_suffix(
            "셨건",
            &["ending.honorific", "ending.past", "ending.geodeun-short"],
        ),
        reference_suffix("시거든", &["ending.honorific", "ending.geodeun"]),
        reference_suffix("시거든요", &["ending.honorific", "ending.geodeun-polite"]),
        reference_suffix("시거들랑", &["ending.honorific", "ending.geodeun"]),
        reference_suffix(
            "셨거든",
            &["ending.honorific", "ending.past", "ending.geodeun"],
        ),
        reference_suffix(
            "셨거든요",
            &["ending.honorific", "ending.past", "ending.geodeun-polite"],
        ),
        reference_suffix(
            "셨거들랑",
            &["ending.honorific", "ending.past", "ending.geodeun"],
        ),
        reference_suffix("려", &["ending.intentive-ryeo"]),
        reference_suffix("려면", &["ending.intentive-ryeomyeon"]),
        reference_suffix(
            "시려면",
            &["ending.honorific", "ending.intentive-ryeomyeon"],
        ),
        reference_suffix("십시오", &["ending.honorific", "ending.imperative-formal"]),
        reference_suffix(
            "십쇼",
            &["ending.honorific", "ending.imperative-formal-short"],
        ),
        reference_suffix("십시다", &["ending.honorific", "ending.propositive"]),
        reference_suffix(
            "십시사",
            &["ending.honorific", "ending.earnest-request-sisa"],
        ),
        reference_suffix("면서", &["ending.simultaneous-myeonseo"]),
        reference_suffix("려던", &["ending.intentive-adnominal"]),
        reference_suffix("러", &["ending.purpose-reo"]),
        reference_suffix("나", &["ending.connective-na"]),
        reference_suffix("시오", &["ending.imperative-sio"]),
        reference_suffix("쇼", &["ending.imperative-sio-short"]),
        reference_suffix(
            "시겠습니다",
            &[
                "ending.honorific",
                "ending.future",
                "ending.polite-declarative",
            ],
        ),
        reference_suffix(
            "셨습니다",
            &[
                "ending.honorific",
                "ending.past",
                "ending.polite-declarative",
            ],
        ),
        reference_suffix(
            "셨다",
            &["ending.honorific", "ending.past", "ending.final-da"],
        ),
        reference_suffix("십니다", &["ending.honorific", "ending.polite-declarative"]),
        reference_suffix("시다", &["ending.honorific", "ending.final-da"]),
        reference_suffix("시는", &["ending.honorific", "ending.present-adnominal"]),
        reference_suffix("시고", &["ending.honorific", "ending.connective-go"]),
        reference_suffix("시면", &["ending.honorific", "ending.conditional"]),
        reference_suffix("시어서", &["ending.honorific", "ending.aoeo-seo"]),
        reference_suffix(
            "셔서",
            &["ending.honorific", "contraction.i-eo", "ending.aoeo-seo"],
        ),
        reference_suffix(
            "시어도",
            &["ending.honorific", "ending.aoeo", "ending.connective-do"],
        ),
        reference_suffix(
            "셔도",
            &[
                "ending.honorific",
                "contraction.i-eo",
                "ending.aoeo",
                "ending.connective-do",
            ],
        ),
        reference_suffix(
            "시면서",
            &["ending.honorific", "ending.simultaneous-myeonseo"],
        ),
        reference_suffix("신", &["ending.honorific", "ending.past-adnominal"]),
        reference_suffix("실", &["ending.honorific", "ending.future-adnominal"]),
        reference_suffix("면", &["ending.conditional"]),
        reference_suffix("며", &["ending.coordinate-myeo"]),
        reference_suffix("니까는", &["ending.connective-ni"]),
        reference_suffix("니까", &["ending.connective-ni"]),
        reference_suffix("니깐", &["ending.connective-ni"]),
        reference_suffix("니", &["ending.connective-ni"]),
    ];

    const GEODEUN: &[ReferenceSuffix] = &[
        reference_suffix("거든", &["ending.geodeun"]),
        reference_suffix("거든요", &["ending.geodeun-polite"]),
        reference_suffix("거들랑", &["ending.geodeun"]),
        reference_suffix("건", &["ending.geodeun-short"]),
    ];

    macro_rules! rieul_endings {
        ($prefix:literal, [$($prior:literal),*] $(, $extension:expr)* $(,)?) => {
            &[
                $($extension,)*
                reference_suffix(concat!($prefix, "지"), &[$($prior,)* "ending.doubt-rieulji"]),
                reference_suffix(concat!($prefix, "지는"), &[$($prior,)* "ending.doubt-rieulji", "particle.topic"]),
                reference_suffix(concat!($prefix, "지요"), &[$($prior,)* "ending.doubt-rieulji"]),
                reference_suffix(concat!($prefix, "는지"), &[$($prior,)* "ending.doubt-rieulneunji"]),
                reference_suffix(concat!($prefix, "는지는"), &[$($prior,)* "ending.doubt-rieulneunji", "particle.topic"]),
                reference_suffix(concat!($prefix, "는지요"), &[$($prior,)* "ending.doubt-rieulneunji"]),
                reference_suffix(concat!($prefix, "걸"), &[$($prior,)* "ending.conjecture-rieulgeol"]),
                reference_suffix(concat!($prefix, "걸요"), &[$($prior,)* "ending.conjecture-rieulgeol"]),
                reference_suffix(concat!($prefix, "까"), &[$($prior,)* "ending.interrogative-rieulkka"]),
                reference_suffix(concat!($prefix, "꼬"), &[$($prior,)* "ending.interrogative-rieulkko"]),
                reference_suffix(concat!($prefix, "런가"), &[$($prior,)* "ending.interrogative-rieulreonga"]),
                reference_suffix(concat!($prefix, "라"), &[$($prior,)* "ending.concern-rieulra"]),
                reference_suffix(concat!($prefix, "세라"), &[$($prior,)* "ending.concern-rieulsera"]),
                reference_suffix(concat!($prefix, "수록"), &[$($prior,)* "ending.connective-rieulsurok"]),
                reference_suffix(concat!($prefix, "망정"), &[$($prior,)* "ending.concessive-rieul"]),
                reference_suffix(concat!($prefix, "지라도"), &[$($prior,)* "ending.concessive-rieul"]),
                reference_suffix(concat!($prefix, "지언정"), &[$($prior,)* "ending.concessive-rieul"]),
                reference_suffix(concat!($prefix, "밖에"), &[$($prior,)* "ending.final-rieulbakke"]),
                reference_suffix(concat!($prefix, "쏘냐"), &[$($prior,)* "ending.interrogative-rieulssonya"]),
                reference_suffix(concat!($prefix, "지니"), &[$($prior,)* "ending.connective-rieuljini"]),
                reference_suffix(concat!($prefix, "진대"), &[$($prior,)* "ending.conditional-rieuljindae"]),
                reference_suffix(concat!($prefix, "진저"), &[$($prior,)* "ending.final-rieuljinjeo"]),
                reference_suffix(concat!($prefix, "지로다"), &[$($prior,)* "ending.final-rieuljiroda"]),
                reference_suffix(concat!($prefix, "락"), &[$($prior,)* "ending.near-rieulrak"]),
                reference_suffix(concat!($prefix, "까요"), &[$($prior,)* "ending.interrogative-rieulkka", "ending.polite-yo"]),
            ]
        };
    }

    macro_rules! present_rieul_endings {
        ($prefix:literal $(, $prior:literal)*) => {
            rieul_endings!($prefix, [$($prior),*],
                reference_suffix(concat!($prefix, "세"), &[$($prior,)* "ending.final-rieulse"]),
                reference_suffix(concat!($prefix, "래"), &[$($prior,)* "ending.intent-rieulrae"]),
                reference_suffix(concat!($prefix, "래요"), &[$($prior,)* "ending.intent-rieulrae", "ending.polite-yo"]),
                reference_suffix(concat!($prefix, "라치면"), &[$($prior,)* "ending.conditional-rieulrachimyeon"]),
            )
        };
    }

    const RIEUL_FINAL: &[ReferenceSuffix] = present_rieul_endings!("");
    const PAST_RIEUL_FINAL: &[ReferenceSuffix] = rieul_endings!("을", []);
    const HONORIFIC_RIEUL_FINAL: &[ReferenceSuffix] =
        present_rieul_endings!("실", "ending.honorific");
    const HONORIFIC_PAST_RIEUL_FINAL: &[ReferenceSuffix] =
        rieul_endings!("셨을", ["ending.honorific", "ending.past"]);

    const BIEUP_FINAL: &[ReferenceSuffix] = &[
        reference_suffix("니다", &["ending.polite-declarative"]),
        reference_suffix("니까", &["ending.formal-interrogative"]),
        reference_suffix("디까", &["ending.retrospective-interrogative"]),
        reference_suffix("디다", &["ending.retrospective-declarative"]),
    ];

    macro_rules! formal_bieup_extensions {
        ($prefix:literal $(, $prior:literal)*) => {
            &[
                reference_suffix(concat!($prefix, "네"), &[$($prior,)* "ending.formal-boast-ne"]),
                reference_suffix(concat!($prefix, "늰다"), &[$($prior,)* "ending.formal-declarative-neunda"]),
                reference_suffix(concat!($prefix, "딘다"), &[$($prior,)* "ending.formal-retrospective-dinda"]),
                reference_suffix(concat!($prefix, "지요"), &[$($prior,)* "ending.formal-confirmation-jiyo"]),
                reference_suffix(concat!($prefix, "죠"), &[$($prior,)* "ending.formal-confirmation-jiyo"]),
            ]
        };
    }

    const BIEUP_EXTENSIONS: &[ReferenceSuffix] = formal_bieup_extensions!("");
    const SEUP_EXTENSIONS: &[ReferenceSuffix] = formal_bieup_extensions!("습");
    const GET_SEUP_EXTENSIONS: &[ReferenceSuffix] =
        formal_bieup_extensions!("겠습", "ending.future");
    const SIP_EXTENSIONS: &[ReferenceSuffix] = formal_bieup_extensions!("십", "ending.honorific");
    const SYEOT_SEUP_EXTENSIONS: &[ReferenceSuffix] =
        formal_bieup_extensions!("셨습", "ending.honorific", "ending.past");
    const SIGET_SEUP_EXTENSIONS: &[ReferenceSuffix] =
        formal_bieup_extensions!("시겠습", "ending.honorific", "ending.future");

    macro_rules! hao_endings {
        ($prefix:literal, [$($prior:literal),*] $(, $plain:expr)*) => {
            &[
                $($plain,)*
                reference_suffix(concat!($prefix, "오니까"), &[$($prior,)* "ending.hao-respectful-final"]),
                reference_suffix(concat!($prefix, "오리다"), &[$($prior,)* "ending.hao-respectful-final"]),
                reference_suffix(concat!($prefix, "오리까"), &[$($prior,)* "ending.hao-respectful-final"]),
                reference_suffix(concat!($prefix, "오리이까"), &[$($prior,)* "ending.hao-respectful-final"]),
                reference_suffix(concat!($prefix, "오리이다"), &[$($prior,)* "ending.hao-respectful-final"]),
                reference_suffix(concat!($prefix, "오이다"), &[$($prior,)* "ending.hao-respectful-final"]),
                reference_suffix(concat!($prefix, "외다"), &[$($prior,)* "ending.hao-respectful-final"]),
                reference_suffix(concat!($prefix, "오이까"), &[$($prior,)* "ending.hao-respectful-final"]),
                reference_suffix(concat!($prefix, "오니"), &[$($prior,)* "ending.hao-respectful-connective"]),
                reference_suffix(concat!($prefix, "오면"), &[$($prior,)* "ending.hao-respectful-connective"]),
            ]
        };
    }

    const HAO_ENDINGS: &[ReferenceSuffix] =
        hao_endings!("", [], reference_suffix("오", &["ending.hao-final"]), HAO_U);
    const HONORIFIC_HAO_ENDINGS: &[ReferenceSuffix] = hao_endings!(
        "시",
        ["ending.honorific"],
        reference_suffix("시오", &["ending.honorific", "ending.hao-final"]),
        HONORIFIC_HAO_U
    );
    const HONORIFIC_HAO_AFTER_ACTION: &[ReferenceSuffix] =
        hao_endings!("시", ["ending.honorific"], HONORIFIC_HAO_U);
    const HONORIFIC_HAO_U: ReferenceSuffix =
        reference_suffix("시우", &["ending.honorific", "ending.hao-final-u"]);
    const PREFINAL_HAO_ENDINGS: &[ReferenceSuffix] = hao_endings!("으", []);
    const HONORIFIC_PAST_HAO_ENDINGS: &[ReferenceSuffix] =
        hao_endings!("셨으", ["ending.honorific", "ending.past"]);
    const HONORIFIC_FUTURE_HAO_ENDINGS: &[ReferenceSuffix] =
        hao_endings!("시겠으", ["ending.honorific", "ending.future"]);
    const HAO_SO: ReferenceSuffix = reference_suffix("소", &["ending.hao-final-so"]);
    const HAO_U: ReferenceSuffix = reference_suffix("우", &["ending.hao-final-u"]);
    const SOSEO: ReferenceSuffix = reference_suffix("소서", &["ending.entreaty-soseo"]);
    const SOIDA: ReferenceSuffix = reference_suffix("소이다", &["ending.hao-final-soida"]);
    const CONSONANT_HAO_FINALS: &[ReferenceSuffix] = &[HAO_SO, SOIDA];
    macro_rules! prefinal_so_endings {
        ($prefix:literal, [$($prior:literal),*] $(, $plain:expr)*) => {
            &[
                $($plain,)*
                reference_suffix(concat!($prefix, "소"), &[$($prior,)* "ending.hao-final-so"]),
                reference_suffix(concat!($prefix, "소다"), &[$($prior,)* "ending.hao-final-soda"]),
                reference_suffix(concat!($prefix, "소이까"), &[$($prior,)* "ending.hao-final-soida"]),
                reference_suffix(concat!($prefix, "소이다"), &[$($prior,)* "ending.hao-final-soida"]),
            ]
        };
    }
    const PAST_HAO_FINALS: &[ReferenceSuffix] = prefinal_so_endings!("", [], HAO_U);
    const FUTURE_HAO_FINALS: &[ReferenceSuffix] = prefinal_so_endings!("", []);
    const HONORIFIC_PAST_HAO_FINALS: &[ReferenceSuffix] = prefinal_so_endings!(
        "셨",
        ["ending.honorific", "ending.past"],
        reference_suffix(
            "셨우",
            &["ending.honorific", "ending.past", "ending.hao-final-u"]
        )
    );
    const HONORIFIC_FUTURE_HAO_FINALS: &[ReferenceSuffix] =
        prefinal_so_endings!("시겠", ["ending.honorific", "ending.future"]);

    fn takes_bare_u(stem: &str) -> bool {
        stem.chars()
            .next_back()
            .and_then(kfind_morph::decompose_syllable)
            .is_some_and(|last| {
                matches!(
                    last.jongseong,
                    kfind_morph::hangul::JONG_BIEUP_SIOT | kfind_morph::hangul::JONG_SSANGSIOT
                )
            })
    }

    macro_rules! directive_endings {
        ($prefix:literal, $base:expr, $quote:expr; $($tail:literal),*) => {
            &[
                reference_suffix($prefix, $base),
                $(
                    reference_suffix(concat!($prefix, $tail), $quote),
                )*
            ]
        };
    }

    macro_rules! all_directive_endings {
        ($prefix:literal $(, $prior:literal)*) => {
            directive_endings!(
                $prefix,
                &[$($prior,)* "ending.directive-ra"],
                &[$($prior,)* "ending.directive-quotative"];
                "거나", "거든", "거든요", "거들랑", "고", "고요", "기에", "나", "는", "는구나", "는군", "는군요", "는데", "는데요", "느니", "니", "니까", "니까는", "니까요", "니깐", "니요", "더군", "더군요", "더니", "더라", "던", "던가", "던데", "던데요", "데", "든지", "며", "면", "면서", "손", "오", "지", "죠"
            )
        };
    }

    const DIRECTIVE_ENDINGS: &[ReferenceSuffix] = all_directive_endings!("라");
    const HONORIFIC_DIRECTIVE_ENDINGS: &[ReferenceSuffix] =
        all_directive_endings!("시라", "ending.honorific");

    macro_rules! prospective_suffixes {
        ($prefix:literal $(, $rule:literal)*) => {
            &[
                reference_suffix(concat!($prefix, "랴"), &[$($rule,)* "ending.rya"]),
                reference_suffix(concat!($prefix, "렷다"), &[$($rule,)* "ending.ryeotda"]),
                reference_suffix(concat!($prefix, "리"), &[$($rule,)* "ending.prospective-final"]),
                reference_suffix(concat!($prefix, "리라"), &[$($rule,)* "ending.prospective-final"]),
                reference_suffix(concat!($prefix, "리라오"), &[$($rule,)* "ending.prospective-final", "ending.hao-reporting-dao"]),
                reference_suffix(concat!($prefix, "리라고"), &[$($rule,)* "ending.prospective-quotative"]),
                reference_suffix(concat!($prefix, "리다"), &[$($rule,)* "ending.prospective-final"]),
                reference_suffix(concat!($prefix, "리오"), &[$($rule,)* "ending.prospective-final"]),
                reference_suffix(concat!($prefix, "리까"), &[$($rule,)* "ending.prospective-final"]),
                reference_suffix(concat!($prefix, "리니라"), &[$($rule,)* "ending.prospective-final"]),
                reference_suffix(concat!($prefix, "리로다"), &[$($rule,)* "ending.prospective-final"]),
                reference_suffix(concat!($prefix, "리니"), &[$($rule,)* "ending.prospective-connective"]),
                reference_suffix(concat!($prefix, "리만치"), &[$($rule,)* "ending.prospective-connective"]),
                reference_suffix(concat!($prefix, "리만큼"), &[$($rule,)* "ending.prospective-connective"]),
            ]
        };
    }

    const PROSPECTIVE: &[ReferenceSuffix] = prospective_suffixes!("");
    const PREFINAL_PROSPECTIVE: &[ReferenceSuffix] = prospective_suffixes!("으");
    const HONORIFIC_PROSPECTIVE: &[ReferenceSuffix] =
        prospective_suffixes!("시", "ending.honorific");
    const HONORIFIC_PAST_PROSPECTIVE: &[ReferenceSuffix] =
        prospective_suffixes!("셨으", "ending.honorific", "ending.past");
    const HONORIFIC_FUTURE_PROSPECTIVE: &[ReferenceSuffix] =
        prospective_suffixes!("시겠으", "ending.honorific", "ending.future");
    let starts_prospective = |prefix| {
        following
            .strip_prefix(prefix)
            .is_some_and(|tail| tail.starts_with(['리', '랴', '렷']))
    };

    macro_rules! question {
        ($surface:literal, $rule:literal $(, $prior:literal)*) => {
            &[
                reference_suffix($surface, &[$($prior,)* $rule]),
                reference_suffix(concat!($surface, "고"), &[$($prior,)* $rule, "ending.quotative-go"]),
                reference_suffix(concat!($surface, "고요"), &[$($prior,)* $rule, "ending.quotative-go", "ending.polite-yo"]),
            ]
        };
    }

    macro_rules! background {
        ($surface:literal, $rule:literal $(, $prior:literal)*) => {
            &[
                reference_suffix($surface, &[$($prior,)* $rule]),
                reference_suffix(concat!($surface, "도"), &[$($prior,)* $rule, "particle.additive"]),
                reference_suffix(concat!($surface, "요"), &[$($prior,)* $rule, "ending.polite-yo"]),
            ]
        };
    }

    const NYA: &[ReferenceSuffix] = question!("냐", "ending.interrogative-nya");
    const NEUNYA: &[ReferenceSuffix] = question!("느냐", "ending.interrogative-neunya");
    const SI_NYA: &[ReferenceSuffix] =
        question!("시냐", "ending.interrogative-nya", "ending.honorific");
    const SI_NEUNYA: &[ReferenceSuffix] =
        question!("시느냐", "ending.interrogative-neunya", "ending.honorific");
    const DEONDE: &[ReferenceSuffix] = background!("던데", "ending.connective-deonde");
    const NEUNDE: &[ReferenceSuffix] = background!("는데", "ending.connective-neunde");
    const SINDE: &[ReferenceSuffix] =
        background!("신데", "ending.connective-neunde", "ending.honorific");
    const SINEUNDE: &[ReferenceSuffix] =
        background!("시는데", "ending.connective-neunde", "ending.honorific");
    const SYEOTNEUNDE: &[ReferenceSuffix] = background!(
        "셨는데",
        "ending.connective-neunde",
        "ending.honorific",
        "ending.past"
    );
    const SIGETNEUNDE: &[ReferenceSuffix] = background!(
        "시겠는데",
        "ending.connective-neunde",
        "ending.honorific",
        "ending.future"
    );
    const BACKGROUND: &[ReferenceSuffix] = background!("데", "ending.connective-neunde");
    const SYEOT_NYA: &[ReferenceSuffix] = question!(
        "셨냐",
        "ending.interrogative-nya",
        "ending.honorific",
        "ending.past"
    );
    const SYEOT_NEUNYA: &[ReferenceSuffix] = question!(
        "셨느냐",
        "ending.interrogative-neunya",
        "ending.honorific",
        "ending.past"
    );
    const SIGET_NYA: &[ReferenceSuffix] = question!(
        "시겠냐",
        "ending.interrogative-nya",
        "ending.honorific",
        "ending.future"
    );
    const SIGET_NEUNYA: &[ReferenceSuffix] = question!(
        "시겠느냐",
        "ending.interrogative-neunya",
        "ending.honorific",
        "ending.future"
    );

    const INTERROGATIVE: &[ReferenceSuffix] = &[
        reference_suffix("고", &["ending.quotative-go"]),
        reference_suffix("고요", &["ending.quotative-go", "ending.polite-yo"]),
    ];

    macro_rules! nieun_extensions {
        ($prefix:literal $(, $prior:literal)*) => {
            &[
                reference_suffix(concat!($prefix, "고"), &[$($prior,)* "ending.interrogative-ngo"]),
                reference_suffix(concat!($prefix, "걸"), &[$($prior,)* "ending.explanatory-ngeol"]),
                reference_suffix(concat!($prefix, "걸요"), &[$($prior,)* "ending.explanatory-ngeol", "ending.polite-yo"]),
                reference_suffix(concat!($prefix, "바"), &[$($prior,)* "ending.connective-nba"]),
            ]
        };
    }

    const NIEUN_EXTENSIONS: &[ReferenceSuffix] = nieun_extensions!("");
    const NEUN_EXTENSIONS: &[ReferenceSuffix] = nieun_extensions!("는");
    const DEON_EXTENSIONS: &[ReferenceSuffix] = nieun_extensions!("던");
    const SIDEON_EXTENSIONS: &[ReferenceSuffix] = nieun_extensions!("시던", "ending.honorific");
    const SIN_EXTENSIONS: &[ReferenceSuffix] = nieun_extensions!("신", "ending.honorific");
    const SINEUN_EXTENSIONS: &[ReferenceSuffix] = nieun_extensions!("시는", "ending.honorific");
    const SYEOTNEUN_EXTENSIONS: &[ReferenceSuffix] =
        nieun_extensions!("셨는", "ending.honorific", "ending.past");
    const SIGETNEUN_EXTENSIONS: &[ReferenceSuffix] =
        nieun_extensions!("시겠는", "ending.honorific", "ending.future");
    const COMPLETED_NIEUN: &[ReferenceSuffix] = &[
        reference_suffix("걸", &["ending.explanatory-ngeol"]),
        reference_suffix("걸요", &["ending.explanatory-ngeol", "ending.polite-yo"]),
        reference_suffix("바", &["ending.connective-nba"]),
    ];

    macro_rules! questions {
        ($prefix:literal $(, $prior:literal)*) => {
            &[
                reference_suffix(concat!($prefix, "가"), &[$($prior,)* "ending.interrogative-nga"]),
                reference_suffix(concat!($prefix, "가요"), &[$($prior,)* "ending.interrogative-nga", "ending.polite-yo"]),
                reference_suffix(concat!($prefix, "지"), &[$($prior,)* "ending.interrogative-nji"]),
                reference_suffix(concat!($prefix, "지는"), &[$($prior,)* "ending.interrogative-nji", "particle.topic"]),
                reference_suffix(concat!($prefix, "지요"), &[$($prior,)* "ending.interrogative-nji", "ending.polite-yo"]),
            ]
        };
    }

    let is_question = |prefix| {
        following
            .strip_prefix(prefix)
            .is_some_and(|tail| tail.starts_with(['가', '지']))
    };

    fn reference_nieun_tail(following: &str, prefix: &str) -> bool {
        following
            .strip_prefix(prefix)
            .is_some_and(|tail| tail.starts_with(['고', '걸', '바']))
    }

    let candidates = match state {
        ContinuationState::Terminal => return Some((0, Vec::new())),
        ContinuationState::AOrEo => AEO,
        ContinuationState::Past => PAST,
        ContinuationState::Future => FUTURE,
        ContinuationState::Declarative => DECLARATIVE,
        ContinuationState::Eu => EU,
        ContinuationState::StemEnding => GEODEUN,
        ContinuationState::NieunEnding => BACKGROUND,
        ContinuationState::CompletedNieunEnding => COMPLETED_NIEUN,
        ContinuationState::ReportEnding if pos.is_action() && following.starts_with('답') => &[
            reference_suffix("답니다", &["ending.formal-report-dap"]),
            reference_suffix("답니까", &["ending.formal-report-dap"]),
        ],
        ContinuationState::ReportEnding => return None,
        ContinuationState::Interrogative => INTERROGATIVE,
        ContinuationState::CopulaRa => COPULA_RA_SUFFIXES,
        ContinuationState::RieulFinal => RIEUL_FINAL,
        ContinuationState::BieupFinal => BIEUP_FINAL,
        ContinuationState::Prospective => PROSPECTIVE,
    };
    const JIYO: ReferenceSuffix = reference_suffix("지요", &["ending.confirmation-jiyo"]);
    const JYO: ReferenceSuffix = reference_suffix("죠", &["ending.confirmation-jiyo"]);
    const CONFIRMATION_JIYO: &[ReferenceSuffix] = &[JIYO, JYO];
    const HONORIFIC_JIYO: &[ReferenceSuffix] = &[
        reference_suffix("시지요", &["ending.honorific", "ending.confirmation-jiyo"]),
        reference_suffix("시죠", &["ending.honorific", "ending.confirmation-jiyo"]),
        reference_suffix(
            "셨지요",
            &[
                "ending.honorific",
                "ending.past",
                "ending.confirmation-jiyo",
            ],
        ),
        reference_suffix(
            "셨죠",
            &[
                "ending.honorific",
                "ending.past",
                "ending.confirmation-jiyo",
            ],
        ),
        reference_suffix(
            "시겠지요",
            &[
                "ending.honorific",
                "ending.future",
                "ending.confirmation-jiyo",
            ],
        ),
        reference_suffix(
            "시겠죠",
            &[
                "ending.honorific",
                "ending.future",
                "ending.confirmation-jiyo",
            ],
        ),
        reference_suffix(
            "셨겠지요",
            &[
                "ending.honorific",
                "ending.past",
                "ending.future",
                "ending.confirmation-jiyo",
            ],
        ),
        reference_suffix(
            "셨겠죠",
            &[
                "ending.honorific",
                "ending.past",
                "ending.future",
                "ending.confirmation-jiyo",
            ],
        ),
    ];
    const PAST_FUTURE_JIYO: &[ReferenceSuffix] = &[
        reference_suffix("겠지요", &["ending.future", "ending.confirmation-jiyo"]),
        reference_suffix("겠죠", &["ending.future", "ending.confirmation-jiyo"]),
    ];
    let mut common_copula = false;
    let additional_endings: &[ReferenceSuffix] = match state {
        ContinuationState::NieunEnding | ContinuationState::CompletedNieunEnding
            if following.starts_with("지라") =>
        {
            &[reference_suffix("지라", &["ending.causal-jira"])]
        }
        ContinuationState::Eu if following.starts_with("신지라") => &[reference_suffix(
            "신지라",
            &["ending.honorific", "ending.causal-jira"],
        )],
        ContinuationState::Eu if following.starts_with("시는지라") => &[reference_suffix(
            "시는지라",
            &["ending.honorific", "ending.causal-jira"],
        )],
        ContinuationState::Past | ContinuationState::Future if following.starts_with("는지라") => {
            &[reference_suffix("는지라", &["ending.causal-jira"])]
        }
        ContinuationState::StemEnding | ContinuationState::Past | ContinuationState::Future
            if following.starts_with("건마") || following.starts_with("건만") =>
        {
            &[
                reference_suffix("건마는", &["ending.concessive-geonman"]),
                reference_suffix("건만", &["ending.concessive-geonman"]),
            ]
        }
        ContinuationState::Eu
            if following.starts_with("시건마") || following.starts_with("시건만") =>
        {
            &[
                reference_suffix(
                    "시건마는",
                    &["ending.honorific", "ending.concessive-geonman"],
                ),
                reference_suffix("시건만", &["ending.honorific", "ending.concessive-geonman"]),
            ]
        }
        ContinuationState::StemEnding
            if following.starts_with("니만")
                && (!pos.is_action() || anchor.ends_with('시'))
                && anchor
                    .chars()
                    .next_back()
                    .is_some_and(|last| !has_final(last)) =>
        {
            &[
                reference_suffix("니만큼", &["ending.causal-nimankeum"]),
                reference_suffix("니만치", &["ending.causal-nimankeum"]),
            ]
        }
        ContinuationState::Eu
            if following.starts_with("니만") && (anchor.ends_with('으') || !pos.is_action()) =>
        {
            &[
                reference_suffix("니만큼", &["ending.causal-nimankeum"]),
                reference_suffix("니만치", &["ending.causal-nimankeum"]),
            ]
        }
        ContinuationState::Eu if following.starts_with("시니만") => &[
            reference_suffix("시니만큼", &["ending.honorific", "ending.causal-nimankeum"]),
            reference_suffix("시니만치", &["ending.honorific", "ending.causal-nimankeum"]),
        ],
        ContinuationState::StemEnding
            if following.starts_with("느니만")
                && (pos.is_action() || anchor.ends_with(['있', '없']))
                && !anchor.chars().next_back().is_some_and(has_rieul_final) =>
        {
            &[
                reference_suffix("느니만큼", &["ending.causal-nimankeum"]),
                reference_suffix("느니만치", &["ending.causal-nimankeum"]),
            ]
        }
        ContinuationState::Eu if pos.is_action() && following.starts_with("시느니만") => &[
            reference_suffix(
                "시느니만큼",
                &["ending.honorific", "ending.causal-nimankeum"],
            ),
            reference_suffix(
                "시느니만치",
                &["ending.honorific", "ending.causal-nimankeum"],
            ),
        ],
        ContinuationState::Past | ContinuationState::Future if following.starts_with("으니만") => {
            &[
                reference_suffix("으니만큼", &["ending.causal-nimankeum"]),
                reference_suffix("으니만치", &["ending.causal-nimankeum"]),
            ]
        }
        ContinuationState::StemEnding
            if following.starts_with("나마")
                && pos != PredicatePos::Copula
                && anchor
                    .chars()
                    .next_back()
                    .is_some_and(|last| !has_final(last)) =>
        {
            &[reference_suffix("나마", &["ending.concessive-nama"])]
        }
        ContinuationState::Past | ContinuationState::Future if following.starts_with("으나마") => {
            &[reference_suffix("으나마", &["ending.concessive-nama"])]
        }
        ContinuationState::Eu if following.starts_with("시나마") => &[reference_suffix(
            "시나마",
            &["ending.honorific", "ending.concessive-nama"],
        )],
        ContinuationState::Eu if anchor.ends_with('으') && following.starts_with("나마") => {
            &[reference_suffix("나마", &["ending.concessive-nama"])]
        }
        ContinuationState::StemEnding
            if following.strip_prefix('나').is_some_and(|tail| {
                tail.trim_start().starts_with('?')
                    && (pos.is_action() || anchor.ends_with(['있', '없']))
                    && !anchor.chars().next_back().is_some_and(has_rieul_final)
            }) =>
        {
            &[reference_suffix("나", &["ending.interrogative-na"])]
        }
        ContinuationState::AOrEo
            if matches!(
                pos,
                PredicatePos::Adjective | PredicatePos::AuxiliaryAdjective
            ) && following.starts_with('라') =>
        {
            &[reference_suffix("라", &["ending.exclamative-aoeora"])]
        }
        ContinuationState::AOrEo if pos.is_action() && following.starts_with("다가") => {
            &[reference_suffix("다가", &["ending.aoeo-daga"])]
        }
        ContinuationState::StemEnding
            if pos.is_action()
                && following.starts_with("는답")
                && anchor
                    .chars()
                    .next_back()
                    .is_some_and(|last| has_final(last) && !has_rieul_final(last)) =>
        {
            &[
                reference_suffix("는답니다", &["ending.formal-report-dap"]),
                reference_suffix("는답니까", &["ending.formal-report-dap"]),
            ]
        }
        ContinuationState::StemEnding
            if (matches!(
                pos,
                PredicatePos::Adjective | PredicatePos::AuxiliaryAdjective
            ) || (pos.is_action() && anchor == "있"))
                && following.starts_with('답') =>
        {
            &[
                reference_suffix("답니다", &["ending.formal-report-dap"]),
                reference_suffix("답니까", &["ending.formal-report-dap"]),
            ]
        }
        ContinuationState::Past | ContinuationState::Future if following.starts_with('답') => &[
            reference_suffix("답니다", &["ending.formal-report-dap"]),
            reference_suffix("답니까", &["ending.formal-report-dap"]),
        ],
        ContinuationState::Eu if pos.is_action() && following.starts_with("신답") => &[
            reference_suffix(
                "신답니다",
                &["ending.honorific", "ending.formal-report-dap"],
            ),
            reference_suffix(
                "신답니까",
                &["ending.honorific", "ending.formal-report-dap"],
            ),
        ],
        ContinuationState::Eu if following.starts_with("시답") => &[
            reference_suffix(
                "시답니다",
                &["ending.honorific", "ending.formal-report-dap"],
            ),
            reference_suffix(
                "시답니까",
                &["ending.honorific", "ending.formal-report-dap"],
            ),
        ],
        ContinuationState::Eu if following.starts_with("셨답") => &[
            reference_suffix(
                "셨답니다",
                &[
                    "ending.honorific",
                    "ending.past",
                    "ending.formal-report-dap",
                ],
            ),
            reference_suffix(
                "셨답니까",
                &[
                    "ending.honorific",
                    "ending.past",
                    "ending.formal-report-dap",
                ],
            ),
        ],
        ContinuationState::Eu if following.starts_with("시겠답") => &[
            reference_suffix(
                "시겠답니다",
                &[
                    "ending.honorific",
                    "ending.future",
                    "ending.formal-report-dap",
                ],
            ),
            reference_suffix(
                "시겠답니까",
                &[
                    "ending.honorific",
                    "ending.future",
                    "ending.formal-report-dap",
                ],
            ),
        ],
        ContinuationState::StemEnding | ContinuationState::Eu
            if pos == PredicatePos::Adjective
                && anchor
                    .chars()
                    .next_back()
                    .is_some_and(|last| !has_final(last))
                && following.strip_prefix('이').is_some_and(|tail| {
                    tail.chars()
                        .next()
                        .is_none_or(|next| !next.is_alphanumeric() && next != '_')
                }) =>
        {
            &[reference_suffix("이", &["ending.familiar-final-i"])]
        }
        ContinuationState::StemEnding
            if pos == PredicatePos::Adjective
                && anchor.chars().next_back().is_some_and(has_rieul_final)
                && following.strip_prefix("으이").is_some_and(|tail| {
                    tail.chars()
                        .next()
                        .is_none_or(|next| !next.is_alphanumeric() && next != '_')
                }) =>
        {
            &[reference_suffix("으이", &["ending.familiar-final-i"])]
        }
        ContinuationState::StemEnding if pos.is_action() && following.starts_with('남') => {
            &[reference_suffix("남", &["ending.self-question-nam"])]
        }
        ContinuationState::StemEnding if following.starts_with(['다', '단']) => &[
            reference_suffix("다가는", &["ending.connective-daganeun"]),
            reference_suffix("다간", &["ending.connective-daganeun"]),
            reference_suffix("단", &["ending.connective-daganeun"]),
        ],
        ContinuationState::NieunEnding
            if following.starts_with(['군', '구'])
                && anchor.ends_with('는')
                && (pos.is_action() || anchor.ends_with("있는") || anchor.ends_with("없는")) =>
        {
            &[
                reference_suffix("구나", &["ending.final-neun-exclamative"]),
                reference_suffix("구려", &["ending.final-neun-exclamative"]),
                reference_suffix("구먼", &["ending.final-neun-exclamative"]),
                reference_suffix("군", &["ending.final-neun-exclamative"]),
                reference_suffix(
                    "군요",
                    &["ending.final-neun-exclamative", "ending.polite-yo"],
                ),
            ]
        }
        ContinuationState::StemEnding
            if following.starts_with('군')
                && !pos.is_action()
                && pos != PredicatePos::Copula
                && anchor != "아니" =>
        {
            &[
                reference_suffix("군", &["ending.final-gun"]),
                reference_suffix("군요", &["ending.final-gun", "ending.polite-yo"]),
            ]
        }
        ContinuationState::Prospective if following.starts_with('군') && anchor.ends_with('시') => {
            &[
                reference_suffix("군", &["ending.final-gun"]),
                reference_suffix("군요", &["ending.final-gun", "ending.polite-yo"]),
            ]
        }
        ContinuationState::Eu
            if (following.starts_with("시는군") || following.starts_with("시는구"))
                && (pos.is_action() || anchor.ends_with(['있', '없'])) =>
        {
            &[
                reference_suffix(
                    "시는군",
                    &["ending.honorific", "ending.final-neun-exclamative"],
                ),
                reference_suffix(
                    "시는구나",
                    &["ending.honorific", "ending.final-neun-exclamative"],
                ),
                reference_suffix(
                    "시는구려",
                    &["ending.honorific", "ending.final-neun-exclamative"],
                ),
                reference_suffix(
                    "시는구먼",
                    &["ending.honorific", "ending.final-neun-exclamative"],
                ),
                reference_suffix(
                    "시는군요",
                    &[
                        "ending.honorific",
                        "ending.final-neun-exclamative",
                        "ending.polite-yo",
                    ],
                ),
            ]
        }
        ContinuationState::Eu if following.starts_with("시구") => &[
            reference_suffix(
                "시구나",
                &["ending.honorific", "ending.final-gu-exclamative"],
            ),
            reference_suffix(
                "시구려",
                &["ending.honorific", "ending.final-gu-exclamative"],
            ),
            reference_suffix(
                "시구먼",
                &["ending.honorific", "ending.final-gu-exclamative"],
            ),
        ],
        ContinuationState::Prospective if following.starts_with('구') && anchor.ends_with('시') => {
            &[
                reference_suffix("구나", &["ending.final-gu-exclamative"]),
                reference_suffix("구려", &["ending.final-gu-exclamative"]),
                reference_suffix("구먼", &["ending.final-gu-exclamative"]),
            ]
        }
        ContinuationState::Eu
            if following.starts_with("시군")
                || following.starts_with("셨군")
                || following.starts_with("시겠군") =>
        {
            &[
                reference_suffix("시군", &["ending.honorific", "ending.final-gun"]),
                reference_suffix(
                    "셨군",
                    &["ending.honorific", "ending.past", "ending.final-gun"],
                ),
                reference_suffix(
                    "시겠군",
                    &["ending.honorific", "ending.future", "ending.final-gun"],
                ),
                reference_suffix(
                    "시군요",
                    &["ending.honorific", "ending.final-gun", "ending.polite-yo"],
                ),
                reference_suffix(
                    "셨군요",
                    &[
                        "ending.honorific",
                        "ending.past",
                        "ending.final-gun",
                        "ending.polite-yo",
                    ],
                ),
                reference_suffix(
                    "시겠군요",
                    &[
                        "ending.honorific",
                        "ending.future",
                        "ending.final-gun",
                        "ending.polite-yo",
                    ],
                ),
            ]
        }
        ContinuationState::StemEnding
            if (pos == PredicatePos::Copula || anchor == "아니")
                && (following.starts_with('죠') || following.starts_with("지요")) =>
        {
            common_copula = true;
            &[]
        }
        ContinuationState::StemEnding | ContinuationState::Past | ContinuationState::Future
            if following.starts_with('죠') || following.starts_with("지요") =>
        {
            CONFIRMATION_JIYO
        }
        ContinuationState::Past
            if following.starts_with("겠죠") || following.starts_with("겠지요") =>
        {
            PAST_FUTURE_JIYO
        }
        ContinuationState::Eu
            if ["시", "셨", "시겠", "셨겠"].iter().any(|prefix| {
                following
                    .strip_prefix(*prefix)
                    .is_some_and(|tail| tail.starts_with('죠') || tail.starts_with("지요"))
            }) =>
        {
            HONORIFIC_JIYO
        }
        ContinuationState::StemEnding
            if (pos.is_action() || anchor == "아니") && following.starts_with("고서") =>
        {
            &[
                reference_suffix("고서", &["ending.connective-goseo"]),
                reference_suffix("고서야", &["ending.connective-goseo"]),
            ]
        }
        ContinuationState::Eu if following.starts_with("시고서") => &[
            reference_suffix("시고서", &["ending.honorific", "ending.connective-goseo"]),
            reference_suffix("시고서야", &["ending.honorific", "ending.connective-goseo"]),
        ],
        ContinuationState::Past | ContinuationState::Future if following.starts_with("다오") => {
            &[reference_suffix("다오", &["ending.hao-reporting-dao"])]
        }
        ContinuationState::StemEnding
            if anchor.ends_with('시') && following.starts_with("다오") =>
        {
            &[reference_suffix("다오", &["ending.hao-reporting-dao"])]
        }
        ContinuationState::NieunEnding
            if pos.is_action() && anchor.ends_with('신') && following.starts_with("다오") =>
        {
            &[reference_suffix(
                "다오",
                &["ending.declarative", "ending.hao-reporting-dao"],
            )]
        }
        ContinuationState::Eu
            if following.starts_with("시다오")
                || following.starts_with("셨다오")
                || following.starts_with("시겠다오")
                || (pos.is_action() && following.starts_with("신다오")) =>
        {
            &[
                reference_suffix(
                    "신다오",
                    &[
                        "ending.honorific",
                        "ending.declarative",
                        "ending.hao-reporting-dao",
                    ],
                ),
                reference_suffix("시다오", &["ending.honorific", "ending.hao-reporting-dao"]),
                reference_suffix(
                    "셨다오",
                    &[
                        "ending.honorific",
                        "ending.past",
                        "ending.hao-reporting-dao",
                    ],
                ),
                reference_suffix(
                    "시겠다오",
                    &[
                        "ending.honorific",
                        "ending.future",
                        "ending.hao-reporting-dao",
                    ],
                ),
            ]
        }
        ContinuationState::StemEnding if anchor == "아니" && following.starts_with("라오") => {
            &[reference_suffix("라오", &["ending.copula-ra-final"])]
        }
        ContinuationState::Eu | ContinuationState::Prospective
            if pos != PredicatePos::Copula && following.starts_with('므') =>
        {
            &[reference_suffix("므로", &["ending.connective-meuro"])]
        }
        ContinuationState::StemEnding
            if pos == PredicatePos::Copula && anchor == "이" && following.starts_with('므') =>
        {
            &[reference_suffix("므로", &["ending.connective-meuro"])]
        }
        ContinuationState::Past | ContinuationState::Future if following.starts_with("으므") => {
            &[reference_suffix("으므로", &["ending.connective-meuro"])]
        }
        ContinuationState::Eu | ContinuationState::StemEnding
            if (state == ContinuationState::Eu
                || (pos == PredicatePos::Copula && anchor == "이"))
                && (following.starts_with("시므")
                    || following.starts_with("셨으므")
                    || following.starts_with("시겠으므")) =>
        {
            &[
                reference_suffix("시므로", &["ending.honorific", "ending.connective-meuro"]),
                reference_suffix(
                    "셨으므로",
                    &["ending.honorific", "ending.past", "ending.connective-meuro"],
                ),
                reference_suffix(
                    "시겠으므로",
                    &[
                        "ending.honorific",
                        "ending.future",
                        "ending.connective-meuro",
                    ],
                ),
            ]
        }
        ContinuationState::Eu
            if pos.is_action()
                && (following.starts_with('렴') || following.starts_with("려무나")) =>
        {
            &[
                reference_suffix("렴", &["ending.imperative-ryeom"]),
                reference_suffix("려무나", &["ending.imperative-ryeom"]),
            ]
        }
        ContinuationState::Eu | ContinuationState::Prospective
            if following.starts_with("려나") || following.starts_with("려니") =>
        {
            &[
                reference_suffix("려나", &["ending.conjecture-ryeo"]),
                reference_suffix("려니", &["ending.conjecture-ryeo"]),
                reference_suffix("려니와", &["ending.concessive-ryeoniwa"]),
            ]
        }
        ContinuationState::Eu | ContinuationState::Prospective
            if pos != PredicatePos::Copula && following.starts_with('라') =>
        {
            DIRECTIVE_ENDINGS
        }
        ContinuationState::Eu if following.starts_with("시라") => HONORIFIC_DIRECTIVE_ENDINGS,
        ContinuationState::BieupFinal => BIEUP_EXTENSIONS,
        ContinuationState::Past | ContinuationState::Future if following.starts_with('습') => {
            SEUP_EXTENSIONS
        }
        ContinuationState::Past if following.starts_with("겠습") => GET_SEUP_EXTENSIONS,
        ContinuationState::Eu if following.starts_with('십') => SIP_EXTENSIONS,
        ContinuationState::Eu if following.starts_with("셨습") => SYEOT_SEUP_EXTENSIONS,
        ContinuationState::Eu if following.starts_with("시겠습") => SIGET_SEUP_EXTENSIONS,
        ContinuationState::StemEnding | ContinuationState::Past
            if following.starts_with("것다") =>
        {
            &[reference_suffix("것다", &["ending.geotda"])]
        }
        ContinuationState::StemEnding
            if (pos == PredicatePos::Copula || anchor == "아니")
                && COPULA_STEM_ENDINGS.iter().any(|entry| {
                    entry.suffix.surface.chars().next() == following.chars().next()
                }) =>
        {
            common_copula = true;
            &[]
        }
        ContinuationState::StemEnding
            if (pos == PredicatePos::Copula || anchor == "아니")
                && following.starts_with(['란', '람', '랍', '래', '로']) =>
        {
            COPULA_REPORTING_AND_RO_ENDINGS
        }
        ContinuationState::NieunEnding | ContinuationState::CompletedNieunEnding
            if following.starts_with("지고") =>
        {
            &[reference_suffix("지고", &["ending.exclamative-njigo"])]
        }
        ContinuationState::Eu if following.starts_with("신지고") => &[reference_suffix(
            "신지고",
            &["ending.honorific", "ending.exclamative-njigo"],
        )],
        ContinuationState::NieunEnding if is_question("") => questions!(""),
        ContinuationState::Past | ContinuationState::Future if is_question("는") => {
            questions!("는")
        }
        ContinuationState::Eu if is_question("신") => questions!("신", "ending.honorific"),
        ContinuationState::Eu if is_question("시는") => questions!("시는", "ending.honorific"),
        ContinuationState::Eu if is_question("셨는") => {
            questions!("셨는", "ending.honorific", "ending.past")
        }
        ContinuationState::Eu if is_question("시겠는") => {
            questions!("시겠는", "ending.honorific", "ending.future")
        }
        ContinuationState::NieunEnding => NIEUN_EXTENSIONS,
        ContinuationState::Past | ContinuationState::Future
            if reference_nieun_tail(following, "는") =>
        {
            NEUN_EXTENSIONS
        }
        ContinuationState::StemEnding | ContinuationState::Past
            if reference_nieun_tail(following, "던") =>
        {
            DEON_EXTENSIONS
        }
        ContinuationState::Eu if reference_nieun_tail(following, "신") => SIN_EXTENSIONS,
        ContinuationState::Eu if reference_nieun_tail(following, "시던") => SIDEON_EXTENSIONS,
        ContinuationState::Eu if following.starts_with("시던") => &[
            reference_suffix(
                "시던",
                &["ending.honorific", "ending.retrospective-adnominal"],
            ),
            reference_suffix(
                "시던가",
                &["ending.honorific", "ending.retrospective-question-deon"],
            ),
            reference_suffix(
                "시던감",
                &["ending.honorific", "ending.retrospective-question-deon"],
            ),
        ],
        ContinuationState::Eu if reference_nieun_tail(following, "시는") => SINEUN_EXTENSIONS,
        ContinuationState::StemEnding
            if pos == PredicatePos::Copula && following.starts_with('던') =>
        {
            &[
                reference_suffix("던", &["ending.retrospective-adnominal"]),
                reference_suffix("던가", &["ending.retrospective-question-deon"]),
                reference_suffix("던감", &["ending.retrospective-question-deon"]),
                reference_suffix("던들", &["ending.retrospective-connective-deon"]),
                reference_suffix("던지", &["ending.retrospective-connective-deon"]),
            ]
        }
        ContinuationState::Eu if reference_nieun_tail(following, "셨는") => SYEOTNEUN_EXTENSIONS,
        ContinuationState::Eu if reference_nieun_tail(following, "시겠는") => {
            SIGETNEUN_EXTENSIONS
        }
        ContinuationState::StemEnding | ContinuationState::Past | ContinuationState::Future
            if following.starts_with('냐') =>
        {
            NYA
        }
        ContinuationState::Past | ContinuationState::Future if following.starts_with("느냐") => {
            NEUNYA
        }
        ContinuationState::StemEnding | ContinuationState::Past
            if following.starts_with("던데") =>
        {
            DEONDE
        }
        ContinuationState::StemEnding | ContinuationState::Past | ContinuationState::Future
            if following.starts_with('던') =>
        {
            DEON_RECOLLECTION
        }
        ContinuationState::StemEnding | ContinuationState::Past | ContinuationState::Future
            if following.starts_with('더') =>
        {
            retrospective!("")
        }
        ContinuationState::Past if following.starts_with("겠더") => {
            retrospective!("겠", "ending.future")
        }
        ContinuationState::Eu if following.starts_with("시더") => {
            retrospective!("시", "ending.honorific")
        }
        ContinuationState::Eu if following.starts_with("셨더") => {
            retrospective!("셨", "ending.honorific", "ending.past")
        }
        ContinuationState::Eu if following.starts_with("시겠더") => {
            retrospective!("시겠", "ending.honorific", "ending.future")
        }
        ContinuationState::Past | ContinuationState::Future if following.starts_with("는데") => {
            NEUNDE
        }
        ContinuationState::Eu if following.starts_with("시냐") => SI_NYA,
        ContinuationState::Eu if following.starts_with("시로") => &[
            reference_suffix("시로구나", &["ending.honorific", "ending.copula-ro-final"]),
            reference_suffix("시로구려", &["ending.honorific", "ending.copula-ro-final"]),
            reference_suffix("시로구먼", &["ending.honorific", "ending.copula-ro-final"]),
            reference_suffix("시로군", &["ending.honorific", "ending.copula-ro-final"]),
            reference_suffix(
                "시로군요",
                &[
                    "ending.honorific",
                    "ending.copula-ro-final",
                    "ending.polite-yo",
                ],
            ),
            reference_suffix(
                "시로되",
                &["ending.honorific", "ending.copula-ro-connective"],
            ),
        ],
        ContinuationState::Eu if following.starts_with("시느냐") => SI_NEUNYA,
        ContinuationState::Eu if following.starts_with("셨냐") => SYEOT_NYA,
        ContinuationState::Eu if following.starts_with("셨느냐") => SYEOT_NEUNYA,
        ContinuationState::Eu if following.starts_with("시겠냐") => SIGET_NYA,
        ContinuationState::Eu if following.starts_with("시겠느냐") => SIGET_NEUNYA,
        ContinuationState::Eu if following.starts_with("신데") => SINDE,
        ContinuationState::Eu if following.starts_with("시는데") => SINEUNDE,
        ContinuationState::Eu if following.starts_with("셨는데") => SYEOTNEUNDE,
        ContinuationState::Eu if following.starts_with("시겠는데") => SIGETNEUNDE,
        ContinuationState::Past | ContinuationState::Future if starts_prospective("으") => {
            PREFINAL_PROSPECTIVE
        }
        ContinuationState::Eu if starts_prospective("") => PROSPECTIVE,
        ContinuationState::Eu if starts_prospective("시") => HONORIFIC_PROSPECTIVE,
        ContinuationState::Eu if starts_prospective("셨으") => HONORIFIC_PAST_PROSPECTIVE,
        ContinuationState::Eu if starts_prospective("시겠으") => HONORIFIC_FUTURE_PROSPECTIVE,
        ContinuationState::Eu if following.starts_with('락') => {
            &[reference_suffix("락", &["ending.alternating-rak"])]
        }
        ContinuationState::StemEnding
            if pos != PredicatePos::Copula
                && following.starts_with('락')
                && anchor.chars().next_back().is_some_and(has_rieul_final) =>
        {
            &[reference_suffix("락", &["ending.alternating-rak"])]
        }
        ContinuationState::Past if following.starts_with('을') => PAST_RIEUL_FINAL,
        ContinuationState::Eu if following.starts_with('실') => HONORIFIC_RIEUL_FINAL,
        ContinuationState::Eu if following.starts_with("셨을") => HONORIFIC_PAST_RIEUL_FINAL,
        ContinuationState::StemEnding
            if pos != PredicatePos::Copula && following.starts_with('소') =>
        {
            if anchor
                .chars()
                .next_back()
                .is_some_and(kfind_morph::has_final)
            {
                CONSONANT_HAO_FINALS
            } else {
                std::slice::from_ref(&HAO_SO)
            }
        }
        ContinuationState::StemEnding if following.starts_with('우') && takes_bare_u(anchor) => {
            std::slice::from_ref(&HAO_U)
        }
        ContinuationState::StemEnding
            if pos == PredicatePos::Copula && following.starts_with(['오', '우', '외']) =>
        {
            HAO_ENDINGS
        }
        ContinuationState::Eu
            if following.starts_with(['오', '외'])
                || (following.starts_with('우')
                    && !anchor.strip_suffix('으').is_some_and(takes_bare_u)) =>
        {
            HAO_ENDINGS
        }
        ContinuationState::Eu
            if following.starts_with("시오")
                || following.starts_with("시우")
                || following.starts_with("시외") =>
        {
            if pos.is_action() {
                HONORIFIC_HAO_AFTER_ACTION
            } else {
                HONORIFIC_HAO_ENDINGS
            }
        }
        ContinuationState::Past if following.starts_with(['소', '우']) => PAST_HAO_FINALS,
        ContinuationState::Future if following.starts_with('소') => FUTURE_HAO_FINALS,
        ContinuationState::Eu if following.starts_with("소서") => std::slice::from_ref(&SOSEO),
        ContinuationState::Past | ContinuationState::Future
            if following.starts_with("으오") || following.starts_with("으외") =>
        {
            PREFINAL_HAO_ENDINGS
        }
        ContinuationState::Eu
            if (following.starts_with("셨소") || following.starts_with("셨우")) =>
        {
            HONORIFIC_PAST_HAO_FINALS
        }
        ContinuationState::Eu if following.starts_with("시겠소") => HONORIFIC_FUTURE_HAO_FINALS,
        ContinuationState::Eu
            if following.starts_with("셨으오") || following.starts_with("셨으외") =>
        {
            HONORIFIC_PAST_HAO_ENDINGS
        }
        ContinuationState::Eu
            if following.starts_with("시겠으오") || following.starts_with("시겠으외") =>
        {
            HONORIFIC_FUTURE_HAO_ENDINGS
        }
        ContinuationState::StemEnding if pos.is_action() && following.starts_with('구') => &[
            reference_suffix("구려", &["ending.final-gu-exclamative"]),
            reference_suffix("구", &["ending.connective-go"]),
        ],
        ContinuationState::StemEnding | ContinuationState::Past | ContinuationState::Future
            if following.starts_with('구') =>
        {
            &[
                reference_suffix("구나", &["ending.final-gu-exclamative"]),
                reference_suffix("구려", &["ending.final-gu-exclamative"]),
                reference_suffix("구먼", &["ending.final-gu-exclamative"]),
                reference_suffix("구", &["ending.connective-go"]),
            ]
        }
        _ => &[],
    };
    let matched = candidates
        .iter()
        .chain(additional_endings)
        .chain(
            common_copula
                .then_some(COPULA_STEM_ENDINGS)
                .into_iter()
                .flatten()
                .map(|entry| &entry.suffix),
        )
        .filter(|suffix| {
            following.starts_with(suffix.surface)
                && (suffix.rules.last() != Some(&"ending.near-rieulrak")
                    || (pos.is_action()
                        && following[suffix.surface.len()..]
                            .strip_prefix(char::is_whitespace)
                            .and_then(|tail| tail.trim_start().strip_prefix("말락"))
                            .is_some_and(|tail| !tail.starts_with(is_reference_token_character))))
                && ((!matches!(
                    suffix.rules.last().copied(),
                    Some(
                        "ending.connective-daganeun"
                            | "ending.dialect-past-je"
                            | "ending.exclamative-aoeora"
                            | "ending.final-gu-exclamative"
                            | "ending.concessive-nama"
                            | "ending.alternating-rak"
                            | "ending.causal-nimankeum"
                            | "ending.causal-jira"
                            | "ending.exclamative-njigo"
                            | "ending.final-rieuljiroda"
                            | "ending.copula-olsida"
                            | "ending.concessive-geonman"
                    )
                ) && !(suffix.rules.last() == Some(&"ending.connective-go")
                    && suffix.surface.ends_with('구'))
                    && (suffix.rules != ["ending.connective-ni"]
                        || !suffix.surface.starts_with('으')))
                    || !following[suffix.surface.len()..]
                        .chars()
                        .next()
                        .is_some_and(|next| next.is_alphanumeric() || next == '_'))
                && (pos.is_action()
                    || suffix.rules == ["ending.exclamative-aoeora"]
                    || (!suffix.rules.contains(&"ending.intent-rieulrae")
                        && !suffix.rules.contains(&"ending.conditional-rieulrachimyeon")
                        && !matches!(
                            suffix.surface,
                            "라" | "시오"
                                | "쇼"
                                | "십시오"
                                | "십쇼"
                                | "십시다"
                                | "십시사"
                                | "려"
                                | "려던"
                                | "러"
                                | "시는"
                        )))
        })
        .max_by_key(|suffix| suffix.surface.len());
    if state == ContinuationState::CopulaRa
        && matched.is_some_and(|suffix| suffix.surface.is_empty())
        && following
            .chars()
            .next()
            .is_some_and(|next| next.is_alphanumeric() || next == '_')
    {
        return None;
    }
    if matched.is_none()
        && matches!(
            state,
            ContinuationState::Eu
                | ContinuationState::StemEnding
                | ContinuationState::NieunEnding
                | ContinuationState::CompletedNieunEnding
                | ContinuationState::RieulFinal
                | ContinuationState::BieupFinal
                | ContinuationState::Prospective
        )
    {
        return None;
    }
    Some(matched.map_or_else(
        || (0, Vec::new()),
        |suffix| {
            (
                suffix.surface.len(),
                suffix.rules.iter().copied().map(RuleId::from).collect(),
            )
        },
    ))
}

fn reference_attached_auxiliary(
    pos: PredicatePos,
    following: &str,
) -> Option<(usize, Vec<RuleId>)> {
    use ContinuationState::{
        AOrEo, BieupFinal, CompletedNieunEnding, Declarative, Eu, Future, Interrogative,
        NieunEnding, Past, RieulFinal, StemEnding, Terminal,
    };
    type AuxiliaryForm = (
        &'static str,
        ContinuationState,
        bool,
        &'static [&'static str],
    );
    const RESULTATIVE: &[AuxiliaryForm] = &[
        ("진다", Declarative, true, &["ending.declarative"]),
        ("진", CompletedNieunEnding, true, &["ending.past-adnominal"]),
        ("질", RieulFinal, true, &["ending.future-adnominal"]),
        ("집", BieupFinal, false, &[]),
        ("짐", Terminal, true, &["ending.nominalizer"]),
        ("졌", Past, false, &["ending.past"]),
        ("져", AOrEo, true, &["contraction.i-eo", "ending.aoeo"]),
        ("지겠", Future, false, &["ending.future"]),
        ("지는", NieunEnding, true, &["ending.present-adnominal"]),
        (
            "지느냐",
            Interrogative,
            true,
            &["ending.interrogative-neunya"],
        ),
        ("지어", AOrEo, true, &["ending.aoeo"]),
        ("지기", Terminal, true, &["ending.nominalizer-gi"]),
        ("지", StemEnding, false, &[]),
        ("지", Eu, false, &[]),
    ];
    const TRIAL: &[AuxiliaryForm] = &[
        ("본다", Declarative, true, &["ending.declarative"]),
        ("본", CompletedNieunEnding, true, &["ending.past-adnominal"]),
        ("볼", RieulFinal, true, &["ending.future-adnominal"]),
        ("봅시다", Terminal, true, &["ending.propositive"]),
        ("봅", BieupFinal, false, &[]),
        ("봄세", Terminal, true, &["ending.volitional-mse"]),
        ("봄", Terminal, true, &["ending.nominalizer"]),
        ("봤", Past, false, &["ending.past"]),
        ("보았", Past, false, &["ending.past"]),
        ("봐", AOrEo, true, &["contraction.o-a", "ending.aoeo"]),
        ("보아", AOrEo, true, &["ending.aoeo"]),
        ("보겠", Future, false, &["ending.future"]),
        ("보는", NieunEnding, true, &["ending.present-adnominal"]),
        (
            "보느냐",
            Interrogative,
            true,
            &["ending.interrogative-neunya"],
        ),
        ("보기", Terminal, true, &["ending.nominalizer-gi"]),
        ("보", StemEnding, false, &[]),
        ("보", Eu, false, &[]),
    ];
    let (forms, stem, auxiliary) = match following.chars().next()? {
        '온' | '왔' => (
            &[
                (
                    "온",
                    CompletedNieunEnding,
                    true,
                    &["ending.past-adnominal"][..],
                ),
                ("왔", Past, false, &["ending.past"][..]),
            ][..],
            '오',
            "ending.auxiliary-oda",
        ),
        '보' | '봤' | '봐' | '본' | '볼' | '봅' | '봄' if pos.is_action() => {
            (TRIAL, '보', "ending.auxiliary-boda")
        }
        '지' | '진' | '질' | '짐' | '집' | '져' | '졌' => {
            (RESULTATIVE, '지', "ending.auxiliary-jida")
        }
        _ => return None,
    };
    let mut best = None;
    for &(form, state, standalone, rules) in forms {
        let Some(rest) = following.strip_prefix(form) else {
            continue;
        };
        let (consumed, suffix) =
            reference_simple_continuation(state, PredicatePos::AuxiliaryVerb, form, rest)
                .unwrap_or_default();
        if consumed == 0 && !standalone {
            continue;
        }
        let rules =
            if consumed > 0 && matches!(state, CompletedNieunEnding | NieunEnding | RieulFinal) {
                &[]
            } else {
                rules
            };
        let mut path = vec![auxiliary.into()];
        path.extend(rules.iter().copied().map(RuleId::from));
        path.extend(suffix);
        let end = form.len() + consumed;
        if best.as_ref().is_none_or(|(previous, _)| end >= *previous) {
            best = Some((end, path));
        }
    }
    const FIXED: &[ReferenceSuffix] = &[
        reference_suffix("지요", &["ending.confirmation-jiyo"]),
        reference_suffix("죠", &["ending.confirmation-jiyo"]),
        reference_suffix("다", &["ending.final-da"]),
        reference_suffix("고", &["ending.connective-go"]),
        reference_suffix("고요", &["ending.connective-go", "ending.polite-yo"]),
        reference_suffix("지", &["ending.connective-ji"]),
        reference_suffix("게", &["ending.adverbial-ge"]),
        reference_suffix("던", &["ending.retrospective-adnominal"]),
        reference_suffix("더니", &["ending.retrospective-connective"]),
        reference_suffix("더라도", &["ending.concessive-deorado"]),
        reference_suffix("도록", &["ending.purpose-dorok"]),
        reference_suffix("다가", &["ending.connective-daga"]),
        reference_suffix("네", &["ending.final-ne"]),
        reference_suffix("네요", &["ending.final-ne", "ending.polite-yo"]),
        reference_suffix("더군", &["ending.final-deogun"]),
        reference_suffix("더군요", &["ending.final-deogun", "ending.polite-yo"]),
        reference_suffix("지만", &["ending.connective-jiman"]),
        reference_suffix("므로", &["ending.connective-meuro"]),
        reference_suffix("기에", &["ending.connective-gie"]),
        reference_suffix("는구나", &["ending.final-neun-exclamative"]),
        reference_suffix("는군", &["ending.final-neun-exclamative"]),
    ];
    if let Some(rest) = following.strip_prefix(stem)
        && let Some(fixed) = FIXED
            .iter()
            .filter(|suffix| rest.starts_with(suffix.surface))
            .max_by_key(|suffix| suffix.surface.len())
        && best
            .as_ref()
            .is_none_or(|(previous, _)| stem.len_utf8() + fixed.surface.len() >= *previous)
    {
        let mut path = vec![auxiliary.into()];
        path.extend(fixed.rules.iter().copied().map(RuleId::from));
        return Some((stem.len_utf8() + fixed.surface.len(), path));
    }
    best
}

fn requires_direct_particle_host(branch: &CandidateProgram) -> bool {
    let boundary = branch.boundary();
    !boundary.require_left && boundary.require_right
}

fn accepts_left_context(left_context: &CandidateLeftContext, text: &str, start: usize) -> bool {
    match left_context {
        CandidateLeftContext::Any => true,
        CandidateLeftContext::ContractedAfterVowel {
            uncontracted_prefix,
        } => {
            let normalized_left = text[..start].nfc().collect::<String>();
            let normalized_prefix = uncontracted_prefix.nfc().collect::<String>();
            let Some(previous) = normalized_left.chars().next_back() else {
                return false;
            };
            if kfind_morph::has_final(previous) {
                return false;
            }
            normalized_left
                .strip_suffix(&normalized_prefix)
                .and_then(|host| host.chars().next_back())
                .is_none_or(|host_final| !kfind_morph::has_final(host_final))
        }
    }
}

fn mapped_core(
    anchor_span: Range<usize>,
    mapping: CoreMapping,
    anchor: &str,
) -> Option<Range<usize>> {
    let length = match mapping {
        CoreMapping::WholeAnchor => anchor.len(),
        CoreMapping::PrefixBytes(length) => length,
    };
    if length == 0 || length > anchor.len() || !anchor.is_char_boundary(length) {
        return None;
    }
    Some(anchor_span.start..anchor_span.start + length)
}

fn map_normalized_prefix(
    original: &str,
    normalized: &str,
    normalized_bytes: usize,
) -> Option<usize> {
    if normalized_bytes == 0 {
        return Some(0);
    }
    let expected = normalized.get(..normalized_bytes)?;
    original
        .char_indices()
        .map(|(start, character)| start + character.len_utf8())
        .find(|&end| original[..end].nfc().eq(expected.chars()))
}

fn accepts_boundaries(
    text: &str,
    core: &Range<usize>,
    token: &Range<usize>,
    branch: &CandidateProgram,
) -> bool {
    let boundary = branch.boundary();
    let valid = token.start <= core.start
        && core.start <= core.end
        && core.end <= token.end
        && token.end <= text.len()
        && text.is_char_boundary(token.start)
        && text.is_char_boundary(token.end);
    valid
        && (!boundary.require_left
            || text[..token.start]
                .chars()
                .next_back()
                .is_none_or(|character| !is_reference_token_character(character)))
        && (!boundary.require_right
            || text[token.end..]
                .chars()
                .next()
                .is_none_or(|character| !is_reference_token_character(character)))
}

fn is_reference_token_character(character: char) -> bool {
    character == '_'
        || character.is_alphanumeric()
        || matches!(
            get_general_category(character),
            GeneralCategory::NonspacingMark
                | GeneralCategory::SpacingMark
                | GeneralCategory::EnclosingMark
        )
}

fn extend_origins(origins: &[Origin], suffix_rules: &[RuleId]) -> Vec<Origin> {
    let mut extended = origins
        .iter()
        .map(|origin| {
            let mut rule_path = origin.rule_path.clone();
            rule_path.extend_from_slice(suffix_rules);
            Origin {
                analysis_index: origin.analysis_index,
                rule_path,
            }
        })
        .collect::<Vec<_>>();
    extended.sort();
    extended.dedup();
    extended
}

fn merge_duplicate_spans(spans: &mut Vec<VerifiedSpan>) {
    spans.sort_by_key(span_order);
    let mut merged = Vec::<VerifiedSpan>::with_capacity(spans.len());
    for span in spans.drain(..) {
        if let Some(previous) = merged
            .last_mut()
            .filter(|previous| previous.core == span.core && previous.token == span.token)
        {
            previous.origins.extend(span.origins);
            previous.origins.sort();
            previous.origins.dedup();
        } else {
            merged.push(span);
        }
    }
    *spans = merged;
}

fn span_order(span: &VerifiedSpan) -> (usize, Reverse<usize>, usize, Reverse<usize>) {
    (
        span.token.start,
        Reverse(span.token.end),
        span.core.start,
        Reverse(span.core.end),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceMatcherBuildError {
    EmptyPlan,
    EmptyAtom {
        atom_index: usize,
    },
    EmptyAnchor {
        atom_index: usize,
        program_index: usize,
    },
    InvalidAnchorUtf8 {
        atom_index: usize,
        program_index: usize,
    },
}

impl Display for ReferenceMatcherBuildError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyPlan => {
                formatter.write_str("a reference matcher requires at least one atom")
            }
            Self::EmptyAtom { atom_index } => {
                write!(
                    formatter,
                    "query atom {atom_index} has no reference programs"
                )
            }
            Self::EmptyAnchor {
                atom_index,
                program_index,
            } => write!(
                formatter,
                "query atom {atom_index} program {program_index} has an empty anchor"
            ),
            Self::InvalidAnchorUtf8 {
                atom_index,
                program_index,
            } => write!(
                formatter,
                "query atom {atom_index} program {program_index} has a non-UTF-8 anchor"
            ),
        }
    }
}

impl Error for ReferenceMatcherBuildError {}

#[derive(Debug)]
pub enum ReferenceMatcherError {
    PhraseJoin(PhraseJoinError),
}

impl Display for ReferenceMatcherError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::PhraseJoin(error) => write!(formatter, "reference phrase join failed: {error}"),
        }
    }
}

impl Error for ReferenceMatcherError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PhraseJoin(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kfind_matcher::MorphMatcher;
    use kfind_query::{
        BoundaryPolicy, CompileOptions, LexiconQueryAnalyzer, Lexicons, compile_query,
    };

    const REPRESENTATIVE_CORPUS: &str = concat!(
        "길을 걸어 갔다. 사용자의 권한을 검증했다.\n",
        "전화를 걸어 보았다. 예쁜 꽃과 파란 하늘을 보았다.\n",
        "사용자권한은 별도 식별자다. 권한을 다시 검증했습니다.\n",
        "학생인 친구는 학교여서 가까운 길로 갔다. 집으로 돌아왔다.\n",
        "천천히 걸으시겠습니다. 꽃이 예뻐라.\n",
        "사용자는은 오류다. 사용자도만 오류다.\n",
        "선생님이 먼저 간대요. 엄마가 나중에 먹으랬어. 다 먹었냬요. 친구가 같이 가재요.\n",
        "꽃이 예쁘군요. 집까지 걸으시는군요.\n",
    );

    #[test]
    fn copula_continuation_preserves_the_span_and_politeness_path() {
        for (pos, stem, following, consumed, expected) in [
            (
                PredicatePos::Copula,
                "이",
                "군요.",
                "군요",
                &["ending.copula-common-final", "ending.polite-yo"][..],
            ),
            (
                PredicatePos::Adjective,
                "아니",
                "올시다.",
                "올시다",
                &["ending.copula-olsida"][..],
            ),
            (PredicatePos::Copula, "이", "올시다말", "", &[][..]),
            (PredicatePos::Verb, "먹", "올시다.", "", &[][..]),
        ] {
            let state = ContinuationState::StemEnding;
            let optimized = kfind_morph::verify_predicate_continuation(state, pos, stem, following)
                .map(|matched| (matched.consumed_bytes, matched.rule_path));
            let reference = reference_predicate_continuation(state, pos, stem, following);
            let expected = (!consumed.is_empty()).then(|| {
                (
                    consumed.len(),
                    expected.iter().copied().map(RuleId::from).collect(),
                )
            });
            assert_eq!(optimized, expected, "{stem}{following}");
            assert_eq!(reference, expected, "{stem}{following}");
        }
    }

    #[test]
    fn reference_backend_matches_optimized_results() {
        let analyzer = LexiconQueryAnalyzer::new(Arc::new(Lexicons::embedded().unwrap()));
        for query in [
            "걷다",
            "예쁘다",
            "가다",
            "먹다",
            "사용자",
            "n:사용자 n:권한",
            "n:권한 v:검증하다",
            "lit:걸어",
            "이다",
            "는",
            "로",
        ] {
            let options = CompileOptions {
                boundary: BoundaryPolicy::Token,
                ..CompileOptions::default()
            };
            let plan = Arc::new(
                compile_query(query, &options, &analyzer)
                    .unwrap_or_else(|error| panic!("failed to compile {query:?}: {error}")),
            );
            let optimized = MorphMatcher::new(Arc::clone(&plan)).unwrap();
            let reference = ReferenceMatcher::new(plan).unwrap();

            assert_eq!(
                optimized.find_all_with_meta(REPRESENTATIVE_CORPUS.as_bytes()),
                reference.find_all_with_meta(REPRESENTATIVE_CORPUS).unwrap(),
                "optimized and reference results differ for {query:?}",
            );
        }
    }
}
