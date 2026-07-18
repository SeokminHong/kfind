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
            CandidateConsumption::Anchor => (0, Vec::new()),
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
                let (mut consumed, mut rules) =
                    reference_predicate_continuation(*continuation, *pos, consumption_following)?;
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
            CandidateConsumption::StructuralPredicateEnding { .. } => return None,
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
    following: &str,
) -> Option<(usize, Vec<RuleId>)> {
    const AEO: &[ReferenceSuffix] = &[
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
        reference_suffix("습니다", &["ending.polite-declarative"]),
        reference_suffix("으되", &["ending.connective-eudoe"]),
        reference_suffix("느냐는", &["ending.interrogative-neunya", "particle.topic"]),
        reference_suffix("으면", &["ending.conditional"]),
        reference_suffix("지만", &["ending.connective-jiman"]),
        reference_suffix("는데", &["ending.connective-neunde"]),
        reference_suffix("다고", &["ending.quotative-go"]),
        reference_suffix("느냐", &["ending.interrogative-neunya"]),
        reference_suffix("던", &["ending.retrospective-adnominal"]),
        reference_suffix("을", &["ending.future-adnominal"]),
        reference_suffix("다", &["ending.final-da"]),
        reference_suffix("고", &["ending.connective-go"]),
        reference_suffix("어요", &["ending.past-polite-yo"]),
    ];
    const FUTURE: &[ReferenceSuffix] = &[
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
    ];
    const EU: &[ReferenceSuffix] = &[
        reference_suffix("리라고", &["ending.prospective-quotative"]),
        reference_suffix("리라", &["ending.prospective-final"]),
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

    let candidates = match state {
        ContinuationState::Terminal => return Some((0, Vec::new())),
        ContinuationState::AOrEo => AEO,
        ContinuationState::Past => PAST,
        ContinuationState::Future => FUTURE,
        ContinuationState::Declarative => DECLARATIVE,
        ContinuationState::Eu => EU,
    };
    let matched = candidates
        .iter()
        .filter(|suffix| {
            following.starts_with(suffix.surface)
                && (!suffix.rules.contains(&"ending.imperative-ra") || pos.is_action())
        })
        .max_by_key(|suffix| suffix.surface.len());
    if matched.is_none() && state == ContinuationState::Eu {
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
