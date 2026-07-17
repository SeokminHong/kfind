//! Nominal particle allomorphs and bounded particle-chain verification.

use std::sync::Arc;

use crate::RuleId;
use crate::hangul::{decompose_syllable, has_final, has_rieul_final};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParticleKind {
    Topic,
    Subject,
    HonorificSubject,
    Object,
    Comitative,
    InformalComitative,
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
    Similarity,
    Conformance,
    Distributive,
    Extent,
    Exclusion,
    Comparison,
    Contrast,
    Alternative,
    Concessive,
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
    pub max_rules: usize,
}

impl Default for ParticleChainModel {
    fn default() -> Self {
        use FinalCondition::{Any, Consonant, ConsonantExceptRieul, Vowel, VowelOrRieul};
        use ParticleKind::{
            Additive, Alternative, Comitative, Comparison, Concessive, Conformance, Connector,
            Contrast, Dative, Distributive, Even, Exclusion, Extent, HonorificSubject,
            InformalComitative, Instrumental, Limit, Locative, Object, Possessive, Restrictive,
            Similarity, Source, StartingPoint, Subject, Topic,
        };
        use ParticleRole::{Auxiliary, Case};

        let forms = [
            allomorph(Source, Case, "에게서", Any, "source.egeseo"),
            allomorph(Source, Case, "한테서", Any, "source.hanteseo"),
            allomorph(
                Concessive,
                Auxiliary,
                "이라도",
                Consonant,
                "concessive.irado-rado",
            ),
            allomorph(
                Concessive,
                Auxiliary,
                "이나마",
                Consonant,
                "concessive.inama-nama",
            ),
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
            allomorph(HonorificSubject, Case, "께서", Any, "subject.honorific"),
            allomorph(Similarity, Case, "같이", Any, "similarity.gachi"),
            allomorph(Conformance, Auxiliary, "대로", Any, "conformance.daero"),
            allomorph(Dative, Case, "더러", Any, "dative.deoreo"),
            allomorph(Distributive, Auxiliary, "마다", Any, "distributive.mada"),
            allomorph(Extent, Auxiliary, "만큼", Any, "extent.mankeum"),
            allomorph(Exclusion, Auxiliary, "밖에", Any, "exclusive.bakke"),
            allomorph(Dative, Case, "보고", Any, "dative.bogo"),
            allomorph(Comparison, Case, "보다", Any, "comparison.boda"),
            allomorph(Restrictive, Auxiliary, "뿐", Any, "restrictive.ppun"),
            allomorph(Similarity, Case, "처럼", Any, "similarity.cheoreom"),
            allomorph(Contrast, Auxiliary, "커녕", Any, "contrast.keonyeong"),
            allomorph(
                Alternative,
                Auxiliary,
                "이나",
                Consonant,
                "alternative.ina-na",
            ),
            allomorph(
                InformalComitative,
                Case,
                "이랑",
                Consonant,
                "comitative.irang-rang",
            ),
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
            allomorph(
                Concessive,
                Auxiliary,
                "라도",
                Vowel,
                "concessive.irado-rado",
            ),
            allomorph(
                Concessive,
                Auxiliary,
                "나마",
                Vowel,
                "concessive.inama-nama",
            ),
            allomorph(Alternative, Auxiliary, "나", Vowel, "alternative.ina-na"),
            allomorph(
                InformalComitative,
                Case,
                "랑",
                Vowel,
                "comitative.irang-rang",
            ),
        ];
        Self {
            allomorphs: Box::new(forms),
            transitions: default_particle_transitions(),
            allow_plural: true,
            max_rules: 4,
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

        if self.model.allow_plural && self.model.max_rules > 0 && following.starts_with('들') {
            consumed += '들'.len_utf8();
            previous = Some('들');
            let plural = RuleId::from("particle.plural");
            rule_path.push(plural.clone());
            previous_rule = Some(plural);
        }

        for _ in rule_path.len()..self.model.max_rules {
            let Some(particle) =
                self.longest_match(&following[consumed..], previous, previous_rule.as_ref())
            else {
                break;
            };
            consumed += particle.surface.len();
            previous = particle.surface.chars().next_back();
            rule_path.push(particle.rule_id.clone());
            previous_rule = Some(particle.rule_id.clone());
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
    ) -> Option<&ParticleAllomorph> {
        let previous = previous?;
        self.model
            .allomorphs
            .iter()
            .filter(|form| {
                remaining.starts_with(form.surface.as_ref())
                    && form.condition.accepts(previous)
                    && self.transition_allows(previous_rule, &form.rule_id)
            })
            .max_by_key(|form| form.surface.len())
    }

    fn transition_allows(&self, previous: Option<&RuleId>, next: &RuleId) -> bool {
        let Some(previous) = previous else {
            return true;
        };
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

fn default_particle_transitions() -> Arc<[ParticleTransition]> {
    // Fallback for callers that construct a verifier without a compiled QueryPlan.
    // Product plans replace this table with data/rules/particles.toml.
    const FOCUS: &[&str] = &[
        "particle.topic",
        "particle.additive",
        "particle.only",
        "particle.limit.ggaji",
        "particle.even.jocha",
        "particle.even.majeo",
    ];
    Arc::from([
        transition(
            "particle.plural",
            &[
                "particle.subject",
                "particle.subject.honorific",
                "particle.object",
                "particle.topic",
                "particle.comitative",
                "particle.comitative.irang-rang",
                "particle.direction",
                "particle.dative",
                "particle.dative.deoreo",
                "particle.dative.bogo",
                "particle.locative",
                "particle.source",
                "particle.source.egeseo",
                "particle.source.hanteseo",
                "particle.from",
                "particle.genitive",
                "particle.additive",
                "particle.only",
                "particle.limit.ggaji",
                "particle.even.jocha",
                "particle.even.majeo",
                "particle.similarity.gachi",
                "particle.similarity.cheoreom",
                "particle.conformance.daero",
                "particle.distributive.mada",
                "particle.extent.mankeum",
                "particle.comparison.boda",
                "particle.restrictive.ppun",
                "particle.exclusive.bakke",
                "particle.contrast.keonyeong",
                "particle.alternative.ina-na",
                "particle.concessive.inama-nama",
                "particle.concessive.irado-rado",
            ],
        ),
        transition("particle.subject", FOCUS),
        transition("particle.object", FOCUS),
        transition("particle.topic", &["particle.contrast.keonyeong"]),
        transition("particle.comitative", FOCUS),
        transition("particle.connector-myeon", &[]),
        transition(
            "particle.direction",
            &[
                "particle.from",
                "particle.topic",
                "particle.additive",
                "particle.only",
                "particle.limit.ggaji",
                "particle.even.jocha",
                "particle.even.majeo",
                "particle.alternative.ina-na",
                "particle.concessive.inama-nama",
                "particle.concessive.irado-rado",
            ],
        ),
        transition(
            "particle.dative",
            &[
                "particle.direction",
                "particle.from",
                "particle.topic",
                "particle.additive",
                "particle.only",
                "particle.limit.ggaji",
                "particle.even.jocha",
                "particle.even.majeo",
                "particle.alternative.ina-na",
                "particle.concessive.inama-nama",
                "particle.concessive.irado-rado",
            ],
        ),
        transition(
            "particle.locative",
            &[
                "particle.direction",
                "particle.from",
                "particle.genitive",
                "particle.topic",
                "particle.additive",
                "particle.only",
                "particle.limit.ggaji",
                "particle.even.jocha",
                "particle.even.majeo",
                "particle.alternative.ina-na",
                "particle.concessive.inama-nama",
                "particle.concessive.irado-rado",
            ],
        ),
        transition(
            "particle.source",
            &[
                "particle.from",
                "particle.topic",
                "particle.additive",
                "particle.only",
                "particle.limit.ggaji",
                "particle.even.jocha",
                "particle.even.majeo",
                "particle.alternative.ina-na",
                "particle.concessive.inama-nama",
                "particle.concessive.irado-rado",
            ],
        ),
        transition(
            "particle.source.egeseo",
            &[
                "particle.from",
                "particle.topic",
                "particle.additive",
                "particle.only",
                "particle.limit.ggaji",
                "particle.even.jocha",
                "particle.even.majeo",
                "particle.alternative.ina-na",
                "particle.concessive.inama-nama",
                "particle.concessive.irado-rado",
            ],
        ),
        transition(
            "particle.source.hanteseo",
            &[
                "particle.from",
                "particle.topic",
                "particle.additive",
                "particle.only",
                "particle.limit.ggaji",
                "particle.even.jocha",
                "particle.even.majeo",
                "particle.alternative.ina-na",
                "particle.concessive.inama-nama",
                "particle.concessive.irado-rado",
            ],
        ),
        transition(
            "particle.from",
            &[
                "particle.genitive",
                "particle.topic",
                "particle.additive",
                "particle.only",
                "particle.limit.ggaji",
                "particle.even.jocha",
                "particle.even.majeo",
                "particle.alternative.ina-na",
                "particle.concessive.inama-nama",
                "particle.concessive.irado-rado",
            ],
        ),
        transition("particle.genitive", FOCUS),
        transition("particle.additive", &[]),
        transition("particle.only", &["particle.topic", "particle.additive"]),
        transition(
            "particle.limit.ggaji",
            &[
                "particle.topic",
                "particle.additive",
                "particle.only",
                "particle.even.jocha",
                "particle.even.majeo",
            ],
        ),
        transition("particle.even.jocha", &["particle.additive"]),
        transition("particle.even.majeo", &["particle.additive"]),
        transition("particle.subject.honorific", FOCUS),
        transition(
            "particle.similarity.gachi",
            &["particle.topic", "particle.additive", "particle.only"],
        ),
        transition(
            "particle.conformance.daero",
            &["particle.topic", "particle.additive", "particle.only"],
        ),
        transition(
            "particle.dative.deoreo",
            &["particle.topic", "particle.additive", "particle.only"],
        ),
        transition(
            "particle.distributive.mada",
            &["particle.topic", "particle.additive", "particle.only"],
        ),
        transition(
            "particle.extent.mankeum",
            &["particle.topic", "particle.additive", "particle.only"],
        ),
        transition(
            "particle.exclusive.bakke",
            &["particle.topic", "particle.additive", "particle.only"],
        ),
        transition(
            "particle.dative.bogo",
            &["particle.topic", "particle.additive", "particle.only"],
        ),
        transition(
            "particle.comparison.boda",
            &["particle.topic", "particle.additive", "particle.only"],
        ),
        transition(
            "particle.restrictive.ppun",
            &["particle.topic", "particle.additive", "particle.only"],
        ),
        transition(
            "particle.similarity.cheoreom",
            &["particle.topic", "particle.additive", "particle.only"],
        ),
        transition("particle.contrast.keonyeong", &[]),
        transition("particle.alternative.ina-na", &[]),
        transition("particle.concessive.inama-nama", &[]),
        transition("particle.concessive.irado-rado", &[]),
        transition("particle.comitative.irang-rang", FOCUS),
    ])
}

fn transition(rule_id: &str, next: &[&str]) -> ParticleTransition {
    ParticleTransition::new(
        rule_id,
        next.iter()
            .copied()
            .map(RuleId::from)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
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
    fn follows_particle_transitions_instead_of_fixed_role_slots() {
        let verifier = ParticleVerifier::default();
        for suffix in [
            "에서는",
            "까지도",
            "까지만",
            "까지는",
            "까지만은",
            "에게까지만",
            "로부터의",
            "에게로",
            "에서부터",
            "에의",
            "조차도",
            "마저도",
            "들로부터의",
        ] {
            assert!(
                verifier.verify_exact("사용자", suffix).is_some(),
                "rejected 사용자{suffix}"
            );
        }
        for suffix in ["는에게", "도까지", "까지도만", "들로부터까지만"] {
            assert!(
                verifier.verify_exact("사용자", suffix).is_none(),
                "accepted 사용자{suffix}"
            );
        }
    }

    #[test]
    fn covers_dictionary_consensus_nominal_particles_and_allomorphs() {
        let verifier = ParticleVerifier::default();
        for suffix in [
            "께서",
            "같이",
            "대로",
            "더러",
            "마다",
            "만큼",
            "밖에",
            "보고",
            "보다",
            "뿐",
            "처럼",
            "커녕",
            "께서는",
            "뿐만",
            "는커녕",
            "들마다",
            "보다도",
        ] {
            assert!(
                verifier.verify_exact("사용자", suffix).is_some(),
                "rejected 사용자{suffix}"
            );
        }
        for (core, suffix) in [
            ("집", "이나"),
            ("바다", "나"),
            ("집", "이나마"),
            ("바다", "나마"),
            ("집", "이라도"),
            ("바다", "라도"),
            ("집", "이랑"),
            ("바다", "랑"),
            ("집", "은커녕"),
            ("바다", "는커녕"),
        ] {
            assert!(
                verifier.verify_exact(core, suffix).is_some(),
                "rejected {core}{suffix}"
            );
        }
        for (core, suffix) in [
            ("집", "나"),
            ("바다", "이나"),
            ("집", "나마"),
            ("바다", "이나마"),
            ("집", "라도"),
            ("바다", "이라도"),
            ("집", "랑"),
            ("바다", "이랑"),
            ("집", "는커녕"),
            ("바다", "은커녕"),
            ("바다", "ㄴ커녕"),
        ] {
            assert!(
                verifier.verify_exact(core, suffix).is_none(),
                "accepted {core}{suffix}"
            );
        }
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
