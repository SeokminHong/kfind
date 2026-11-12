use std::cmp::Reverse;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::ops::Range;
use std::sync::Arc;

use kfind_morph::{ContinuationState, PredicatePos, RuleId};
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
                if *nominal_particle_transition {
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

#[derive(Clone, Copy)]
struct ReferenceSuffix {
    surface: &'static str,
    rules: &'static [&'static str],
}

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
    const AEO: &[ReferenceSuffix] = &[
        reference_suffix("야지", &["ending.necessity-yaji"]),
        reference_suffix("야지요", &["ending.necessity-yaji-polite"]),
        reference_suffix("야죠", &["ending.necessity-yaji-polite"]),
        reference_suffix("지거들랑", &["ending.auxiliary-jida", "ending.geodeun"]),
        reference_suffix("지건", &["ending.auxiliary-jida", "ending.geodeun-short"]),
        reference_suffix("지거든", &["ending.auxiliary-jida", "ending.geodeun"]),
        reference_suffix(
            "지거든요",
            &["ending.auxiliary-jida", "ending.geodeun-polite"],
        ),
        reference_suffix("가고", &["ending.auxiliary-gada", "ending.connective-go"]),
        reference_suffix("가야", &["ending.auxiliary-gada", "ending.connective-ya"]),
        reference_suffix(
            "졌습니다",
            &[
                "ending.auxiliary-jida",
                "ending.past",
                "ending.polite-declarative",
            ],
        ),
        reference_suffix("서도", &["ending.aoeo-seo", "particle.additive"]),
        reference_suffix(
            "졌다",
            &["ending.auxiliary-jida", "ending.past", "ending.final-da"],
        ),
        reference_suffix("지면", &["ending.auxiliary-jida", "ending.conditional"]),
        reference_suffix("지고", &["ending.auxiliary-jida", "ending.connective-go"]),
        reference_suffix("진", &["ending.auxiliary-jida", "ending.past-adnominal"]),
        reference_suffix("질", &["ending.auxiliary-jida", "ending.future-adnominal"]),
        reference_suffix("지다", &["ending.auxiliary-jida", "ending.final-da"]),
        reference_suffix("서", &["ending.aoeo-seo"]),
        reference_suffix("도", &["ending.connective-do"]),
        reference_suffix("야", &["ending.connective-ya"]),
        reference_suffix("요", &["ending.polite-yo"]),
        reference_suffix("라", &["ending.imperative-ra"]),
    ];
    const PAST: &[ReferenceSuffix] = &[
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
        reference_suffix("어요", &["ending.past-polite-yo"]),
        reference_suffix("게", &["ending.final-ge"]),
    ];
    const FUTURE: &[ReferenceSuffix] = &[
        reference_suffix("습니까", &["ending.formal-interrogative"]),
        reference_suffix("습디까", &["ending.retrospective-interrogative"]),
        reference_suffix("습디다", &["ending.retrospective-declarative"]),
        reference_suffix("습니다", &["ending.polite-declarative"]),
        reference_suffix("으되", &["ending.connective-eudoe"]),
        reference_suffix("지만", &["ending.connective-jiman"]),
        reference_suffix("는데", &["ending.connective-neunde"]),
        reference_suffix("다", &["ending.final-da"]),
        reference_suffix("고", &["ending.connective-go"]),
    ];
    const DECLARATIVE: &[ReferenceSuffix] = &[
        reference_suffix("면서", &["ending.quotative-myeonseo"]),
        reference_suffix("는데", &["ending.quotative-neunde"]),
        reference_suffix("고", &["ending.quotative-go"]),
        reference_suffix("는", &["ending.quotative-adnominal"]),
        reference_suffix("던", &["ending.quotative-retrospective"]),
        reference_suffix("면", &["ending.conditional"]),
        reference_suffix("니", &["ending.quotative-ni"]),
        reference_suffix("며", &["ending.quotative-myeo"]),
        reference_suffix("지", &["ending.quotative-ji"]),
        reference_suffix("우", &["ending.hao-final-u"]),
    ];
    const EU: &[ReferenceSuffix] = &[
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
                reference_suffix(concat!($prefix, "지요"), &[$($prior,)* "ending.doubt-rieulji"]),
                reference_suffix(concat!($prefix, "는지"), &[$($prior,)* "ending.doubt-rieulneunji"]),
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
                "거나", "거든", "거든요", "거들랑", "고", "고요", "기에", "나", "는", "는구나", "는군", "는데", "는데요", "느니", "니", "니까", "니까는", "니까요", "니깐", "니요", "더군", "더군요", "더니", "더라", "던", "던가", "던데", "던데요", "데", "든지", "며", "면", "면서", "손", "오", "지", "죠"
            )
        };
    }

    const DIRECTIVE_ENDINGS: &[ReferenceSuffix] = all_directive_endings!("라");
    const HONORIFIC_DIRECTIVE_ENDINGS: &[ReferenceSuffix] =
        all_directive_endings!("시라", "ending.honorific");

    macro_rules! prospective_suffixes {
        ($prefix:literal $(, $rule:literal)*) => {
            &[
                reference_suffix(concat!($prefix, "리"), &[$($rule,)* "ending.prospective-final"]),
                reference_suffix(concat!($prefix, "리라"), &[$($rule,)* "ending.prospective-final"]),
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
        ContinuationState::Interrogative => INTERROGATIVE,
        ContinuationState::CopulaRa => &[
            reference_suffix("", &["ending.copula-ra-connective"]),
            reference_suffix("고", &["ending.copula-quotative-go"]),
            reference_suffix("고요", &["ending.copula-quotative-go", "ending.polite-yo"]),
            reference_suffix("는", &["ending.copula-quotative-adnominal"]),
            reference_suffix("나", &["ending.copula-ra-final"]),
            reference_suffix("네", &["ending.copula-ra-final"]),
            reference_suffix("느니", &["ending.copula-ra-connective"]),
            reference_suffix("니", &["ending.copula-ra-final"]),
            reference_suffix("니까", &["ending.copula-ra-final"]),
            reference_suffix("며", &["ending.copula-ra-connective"]),
            reference_suffix("면", &["ending.copula-ra-connective"]),
            reference_suffix("면서", &["ending.copula-ra-connective"]),
            reference_suffix("서", &["ending.copula-ra-connective"]),
            reference_suffix("손", &["ending.copula-ra-connective"]),
            reference_suffix("야", &["ending.copula-ra-connective"]),
            reference_suffix("야만", &["ending.copula-ra-connective"]),
            reference_suffix("오", &["ending.copula-ra-final"]),
            reference_suffix("지", &["ending.copula-ra-final"]),
        ],
        ContinuationState::RieulFinal => RIEUL_FINAL,
        ContinuationState::BieupFinal => BIEUP_FINAL,
        ContinuationState::Prospective => PROSPECTIVE,
    };
    let additional_endings: &[ReferenceSuffix] = match state {
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
        ContinuationState::StemEnding
            if (pos == PredicatePos::Copula || anchor == "아니")
                && following.starts_with([
                    '게', '고', '구', '군', '길', '네', '니', '면', '세', '야', '어', '에', '자',
                    '죠', '지',
                ]) =>
        {
            &[
                reference_suffix("게", &["ending.copula-ge"]),
                reference_suffix("고말고", &["ending.copula-common-final"]),
                reference_suffix("고요", &["ending.connective-go", "ending.polite-yo"]),
                reference_suffix("구나", &["ending.copula-common-final"]),
                reference_suffix("구려", &["ending.copula-common-final"]),
                reference_suffix("구먼", &["ending.copula-common-final"]),
                reference_suffix("군", &["ending.copula-common-final"]),
                reference_suffix("길래", &["ending.copula-common-connective"]),
                reference_suffix("네", &["ending.copula-common-final"]),
                reference_suffix("네요", &["ending.copula-common-final", "ending.polite-yo"]),
                reference_suffix("니", &["ending.copula-common-connective"]),
                reference_suffix("니까", &["ending.copula-common-connective"]),
                reference_suffix("니까는", &["ending.copula-common-connective"]),
                reference_suffix("니깐", &["ending.copula-common-connective"]),
                reference_suffix("니라", &["ending.copula-common-final"]),
                reference_suffix("면", &["ending.copula-common-connective"]),
                reference_suffix("면서", &["ending.copula-common-connective"]),
                reference_suffix("세요", &["ending.honorific", "ending.copula-polite"]),
                reference_suffix("야", &["ending.copula-ya"]),
                reference_suffix("어요", &["ending.copula-polite"]),
                reference_suffix("에요", &["ending.copula-polite"]),
                reference_suffix("자", &["ending.copula-common-connective"]),
                reference_suffix("죠", &["ending.copula-common-final"]),
                reference_suffix("지요", &["ending.copula-common-final"]),
            ]
        }
        ContinuationState::StemEnding
            if (pos == PredicatePos::Copula || anchor == "아니")
                && following.starts_with(['란', '람', '랍', '래', '로']) =>
        {
            &[
                reference_suffix("란다", &["ending.copula-reporting-final"]),
                reference_suffix("람", &["ending.copula-reporting-final"]),
                reference_suffix("랍니까", &["ending.copula-reporting-final"]),
                reference_suffix("랍니다", &["ending.copula-reporting-final"]),
                reference_suffix("랍시고", &["ending.copula-reporting-connective"]),
                reference_suffix("래", &["ending.copula-reporting-final"]),
                reference_suffix(
                    "래요",
                    &["ending.copula-reporting-final", "ending.polite-yo"],
                ),
                reference_suffix("로고", &["ending.copula-ro-final"]),
                reference_suffix("로구나", &["ending.copula-ro-final"]),
                reference_suffix("로구려", &["ending.copula-ro-final"]),
                reference_suffix("로구먼", &["ending.copula-ro-final"]),
                reference_suffix("로군", &["ending.copula-ro-final"]),
                reference_suffix("로다", &["ending.copula-ro-final"]),
                reference_suffix("로되", &["ending.copula-ro-connective"]),
                reference_suffix("로라", &["ending.copula-ro-final"]),
                reference_suffix("로서니", &["ending.copula-ro-connective"]),
                reference_suffix("로세", &["ending.copula-ro-final"]),
            ]
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
        ContinuationState::Eu if reference_nieun_tail(following, "시는") => SINEUN_EXTENSIONS,
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
        ContinuationState::Past | ContinuationState::Future if following.starts_with("으리") => {
            PREFINAL_PROSPECTIVE
        }
        ContinuationState::Eu if following.starts_with('리') => PROSPECTIVE,
        ContinuationState::Eu if following.starts_with("시리") => HONORIFIC_PROSPECTIVE,
        ContinuationState::Eu if following.starts_with("셨으리") => HONORIFIC_PAST_PROSPECTIVE,
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
            if pos == PredicatePos::Copula && following.starts_with(['오', '우']) =>
        {
            HAO_ENDINGS
        }
        ContinuationState::Eu
            if following.starts_with('오')
                || (following.starts_with('우')
                    && !anchor.strip_suffix('으').is_some_and(takes_bare_u)) =>
        {
            HAO_ENDINGS
        }
        ContinuationState::Eu
            if (following.starts_with("시오") || following.starts_with("시우")) =>
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
        ContinuationState::Past | ContinuationState::Future if following.starts_with("으오") => {
            PREFINAL_HAO_ENDINGS
        }
        ContinuationState::Eu
            if (following.starts_with("셨소") || following.starts_with("셨우")) =>
        {
            HONORIFIC_PAST_HAO_FINALS
        }
        ContinuationState::Eu if following.starts_with("시겠소") => HONORIFIC_FUTURE_HAO_FINALS,
        ContinuationState::Eu if following.starts_with("셨으오") => HONORIFIC_PAST_HAO_ENDINGS,
        ContinuationState::Eu if following.starts_with("시겠으오") => {
            HONORIFIC_FUTURE_HAO_ENDINGS
        }
        _ => &[],
    };
    let matched = candidates
        .iter()
        .chain(additional_endings)
        .filter(|suffix| {
            following.starts_with(suffix.surface)
                && (pos.is_action()
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
    );

    #[test]
    fn reference_backend_matches_optimized_results() {
        let analyzer = LexiconQueryAnalyzer::new(Arc::new(Lexicons::embedded().unwrap()));
        for query in [
            "걷다",
            "예쁘다",
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
