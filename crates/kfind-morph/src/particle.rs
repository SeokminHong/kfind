//! Nominal particle allomorphs and bounded particle-chain verification.

use std::sync::Arc;

use crate::RuleId;
use crate::hangul::{decompose_syllable, has_final, has_rieul_final};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParticleKind {
    Topic,
    Subject,
    Object,
    Comitative,
    Connector,
    Instrumental,
    Dative,
    Locative,
    Source,
    Possessive,
    Additive,
    Restrictive,
    Limit,
    StartingPoint,
    Even,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParticleRole {
    Case,
    Auxiliary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FinalCondition {
    Any,
    Vowel,
    Consonant,
    VowelOrRieul,
    ConsonantExceptRieul,
}

impl FinalCondition {
    #[must_use]
    pub fn accepts(self, previous: char) -> bool {
        match self {
            Self::Any => true,
            Self::Vowel => {
                decompose_syllable(previous).is_some_and(|syllable| syllable.jongseong == 0)
            }
            Self::Consonant => has_final(previous),
            Self::VowelOrRieul => decompose_syllable(previous)
                .is_some_and(|syllable| syllable.jongseong == 0 || has_rieul_final(previous)),
            Self::ConsonantExceptRieul => has_final(previous) && !has_rieul_final(previous),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticleAllomorph {
    pub kind: ParticleKind,
    pub role: ParticleRole,
    pub surface: Box<str>,
    pub condition: FinalCondition,
    pub rule_id: RuleId,
}

impl ParticleAllomorph {
    #[must_use]
    pub fn new(
        kind: ParticleKind,
        role: ParticleRole,
        surface: impl Into<Box<str>>,
        condition: FinalCondition,
        rule_id: impl Into<RuleId>,
    ) -> Self {
        Self {
            kind,
            role,
            surface: surface.into(),
            condition,
            rule_id: rule_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticleTransition {
    pub rule_id: RuleId,
    pub next: Box<[RuleId]>,
}

impl ParticleTransition {
    #[must_use]
    pub fn new(rule_id: impl Into<RuleId>, next: impl Into<Box<[RuleId]>>) -> Self {
        Self {
            rule_id: rule_id.into(),
            next: next.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticleChainModel {
    pub allomorphs: Box<[ParticleAllomorph]>,
    pub transitions: Arc<[ParticleTransition]>,
    pub allow_plural: bool,
    pub max_auxiliaries: usize,
}

impl Default for ParticleChainModel {
    fn default() -> Self {
        use FinalCondition::{Any, Consonant, ConsonantExceptRieul, Vowel, VowelOrRieul};
        use ParticleKind::{
            Additive, Comitative, Connector, Dative, Even, Instrumental, Limit, Locative, Object,
            Possessive, Restrictive, Source, StartingPoint, Subject, Topic,
        };
        use ParticleRole::{Auxiliary, Case};

        let forms = [
            allomorph(Source, Case, "에게서", Any, "source.egeseo"),
            allomorph(Source, Case, "한테서", Any, "source.hanteseo"),
            allomorph(
                Instrumental,
                Case,
                "으로",
                ConsonantExceptRieul,
                "direction",
            ),
            allomorph(Instrumental, Case, "로", VowelOrRieul, "direction"),
            allomorph(Dative, Case, "에게", Any, "dative"),
            allomorph(Dative, Case, "한테", Any, "dative"),
            allomorph(Dative, Case, "께", Any, "dative"),
            allomorph(Source, Case, "에서", Any, "source"),
            allomorph(Locative, Case, "에", Any, "locative"),
            allomorph(Possessive, Case, "의", Any, "genitive"),
            allomorph(Subject, Case, "이", Consonant, "subject"),
            allomorph(Subject, Case, "가", Vowel, "subject"),
            allomorph(Object, Case, "을", Consonant, "object"),
            allomorph(Object, Case, "를", Vowel, "object"),
            allomorph(Comitative, Case, "과", Consonant, "comitative"),
            allomorph(Comitative, Case, "와", Vowel, "comitative"),
            allomorph(Connector, Case, "이면", Consonant, "connector-myeon"),
            allomorph(Connector, Case, "면", Vowel, "connector-myeon"),
            allomorph(Topic, Auxiliary, "은", Consonant, "topic"),
            allomorph(Topic, Auxiliary, "는", Vowel, "topic"),
            allomorph(Additive, Auxiliary, "도", Any, "additive"),
            allomorph(Restrictive, Auxiliary, "만", Any, "only"),
            allomorph(Limit, Auxiliary, "까지", Any, "limit.ggaji"),
            allomorph(StartingPoint, Auxiliary, "부터", Any, "from"),
            allomorph(Even, Auxiliary, "조차", Any, "even.jocha"),
            allomorph(Even, Auxiliary, "마저", Any, "even.majeo"),
        ];
        Self {
            allomorphs: Box::new(forms),
            transitions: Arc::from([]),
            allow_plural: true,
            max_auxiliaries: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticleMatch {
    /// Byte offset of the verified token end in `core + following`.
    pub token_end: usize,
    /// Number of bytes consumed from `following`.
    pub consumed_bytes: usize,
    pub rule_path: Vec<RuleId>,
}

#[derive(Debug, Clone, Default)]
pub struct ParticleVerifier {
    model: ParticleChainModel,
}

impl ParticleVerifier {
    #[must_use]
    pub fn new(model: ParticleChainModel) -> Self {
        Self { model }
    }

    #[must_use]
    pub fn model(&self) -> &ParticleChainModel {
        &self.model
    }

    /// Returns the longest grammatical particle-chain prefix after `core`.
    #[must_use]
    pub fn verify_prefix(&self, core: &str, following: &str) -> ParticleMatch {
        let mut consumed = 0;
        let mut rule_path = Vec::new();
        let mut previous = core.chars().next_back();
        let mut previous_rule = None;

        if self.model.allow_plural && following.starts_with('들') {
            consumed += '들'.len_utf8();
            previous = Some('들');
            let plural = RuleId::from("particle.plural");
            rule_path.push(plural.clone());
            previous_rule = Some(plural);
        }

        if let Some(case) = self.longest_match(
            &following[consumed..],
            previous,
            previous_rule.as_ref(),
            ParticleRole::Case,
        ) {
            consumed += case.surface.len();
            previous = case.surface.chars().next_back();
            rule_path.push(case.rule_id.clone());
            previous_rule = Some(case.rule_id.clone());
        }

        for _ in 0..self.model.max_auxiliaries {
            let Some(auxiliary) = self.longest_match(
                &following[consumed..],
                previous,
                previous_rule.as_ref(),
                ParticleRole::Auxiliary,
            ) else {
                break;
            };
            consumed += auxiliary.surface.len();
            previous = auxiliary.surface.chars().next_back();
            rule_path.push(auxiliary.rule_id.clone());
            previous_rule = Some(auxiliary.rule_id.clone());
        }

        ParticleMatch {
            token_end: core.len() + consumed,
            consumed_bytes: consumed,
            rule_path,
        }
    }

    /// Verifies that all of `following` is one bounded plural/particle chain.
    #[must_use]
    pub fn verify_exact(&self, core: &str, following: &str) -> Option<ParticleMatch> {
        let matched = self.verify_prefix(core, following);
        (matched.consumed_bytes == following.len()).then_some(matched)
    }

    fn longest_match(
        &self,
        remaining: &str,
        previous: Option<char>,
        previous_rule: Option<&RuleId>,
        role: ParticleRole,
    ) -> Option<&ParticleAllomorph> {
        let previous = previous?;
        self.model
            .allomorphs
            .iter()
            .filter(|form| {
                form.role == role
                    && remaining.starts_with(form.surface.as_ref())
                    && form.condition.accepts(previous)
                    && self.transition_allows(previous_rule, &form.rule_id)
            })
            .max_by_key(|form| form.surface.len())
    }

    fn transition_allows(&self, previous: Option<&RuleId>, next: &RuleId) -> bool {
        let Some(previous) = previous else {
            return true;
        };
        if self.model.transitions.is_empty() {
            return true;
        }
        self.model
            .transitions
            .iter()
            .find(|transition| transition.rule_id == *previous)
            .is_some_and(|transition| transition.next.contains(next))
    }
}

fn allomorph(
    kind: ParticleKind,
    role: ParticleRole,
    surface: &str,
    condition: FinalCondition,
    rule_suffix: &str,
) -> ParticleAllomorph {
    ParticleAllomorph::new(
        kind,
        role,
        surface,
        condition,
        RuleId::from(format!("particle.{rule_suffix}")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_plural_and_bounded_particle_chains() {
        let verifier = ParticleVerifier::default();
        for suffix in ["", "들", "는", "들에게", "들로부터"] {
            assert!(
                verifier.verify_exact("사용자", suffix).is_some(),
                "rejected 사용자{suffix}"
            );
        }
    }

    #[test]
    fn applies_allomorphs_to_the_immediately_preceding_syllable() {
        let verifier = ParticleVerifier::default();
        for (core, suffix) in [
            ("집", "은"),
            ("바다", "는"),
            ("집", "이"),
            ("바다", "가"),
            ("집", "을"),
            ("바다", "를"),
            ("집", "과"),
            ("바다", "와"),
        ] {
            assert!(
                verifier.verify_exact(core, suffix).is_some(),
                "rejected {core}{suffix}"
            );
        }
    }

    #[test]
    fn rieul_final_selects_ro_instead_of_euro() {
        let verifier = ParticleVerifier::default();
        assert!(verifier.verify_exact("길", "로").is_some());
        assert!(verifier.verify_exact("집", "으로").is_some());
        assert!(verifier.verify_exact("바다", "로").is_some());
        assert!(verifier.verify_exact("길", "길으로").is_none());
        assert!(verifier.verify_exact("길", "으로").is_none());
        assert!(verifier.verify_exact("집", "로").is_none());
    }

    #[test]
    fn connector_myeon_obeys_the_host_final_and_stays_terminal() {
        let verifier = ParticleVerifier::new(ParticleChainModel {
            transitions: Arc::from([ParticleTransition::new(
                "particle.connector-myeon",
                Vec::<RuleId>::new().into_boxed_slice(),
            )]),
            ..ParticleChainModel::default()
        });
        assert!(verifier.verify_exact("백", "이면").is_some());
        assert!(verifier.verify_exact("공부", "면").is_some());
        assert!(verifier.verify_exact("백", "면").is_none());
        assert!(verifier.verify_exact("공부", "이면").is_none());
        assert!(verifier.verify_exact("백", "이면도").is_none());
    }

    #[test]
    fn permits_auxiliary_particles_only_after_the_case_slot() {
        let verifier = ParticleVerifier::default();
        assert!(verifier.verify_exact("학교", "에서는").is_some());
        assert!(verifier.verify_exact("사용자", "에게까지만").is_some());
        assert!(verifier.verify_exact("사용자", "는에게").is_none());
    }

    #[test]
    fn reports_consumed_bytes_token_end_and_rules() {
        let verifier = ParticleVerifier::default();
        let matched = verifier.verify_prefix("사용자", "들에게, 다음");
        assert_eq!(matched.consumed_bytes, "들에게".len());
        assert_eq!(matched.token_end, "사용자들에게".len());
        assert_eq!(matched.rule_path[0].as_str(), "particle.plural");
        assert_eq!(matched.rule_path[1].as_str(), "particle.dative");
    }

    #[test]
    fn does_not_guess_pronunciation_for_non_hangul_nominals() {
        let verifier = ParticleVerifier::default();
        assert!(verifier.verify_exact("LLM", "로").is_none());
        assert!(verifier.verify_exact("LLM", "으로").is_none());
    }

    #[test]
    fn transition_graph_rejects_unlisted_particle_chains() {
        let transitions = [
            ParticleTransition::new("particle.topic", Vec::<RuleId>::new().into_boxed_slice()),
            ParticleTransition::new("particle.additive", Vec::<RuleId>::new().into_boxed_slice()),
        ];
        let verifier = ParticleVerifier::new(ParticleChainModel {
            transitions: Arc::from(transitions),
            ..ParticleChainModel::default()
        });

        assert!(verifier.verify_exact("사용자", "는").is_some());
        assert!(verifier.verify_exact("사용자", "는은").is_none());
        assert!(verifier.verify_exact("사용자", "도만").is_none());
    }
}
