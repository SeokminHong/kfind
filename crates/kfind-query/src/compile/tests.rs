use std::collections::BTreeSet;
use std::sync::Arc;

use kfind_data::{
    DataFinePos, LexiconData, NominalRecord, collect_pos_entries, encode_pos_lexicon,
};
use kfind_morph::{
    CoarsePos, ContinuationState, PredicatePos, RuleId, verify_predicate_continuation,
};

use super::*;
use crate::{
    BoundaryPolicy, CompileOptionOverrides, ContextRequirement, Lexicons, NormalizationMode,
    PlanLimits,
};

fn analyzer() -> LexiconQueryAnalyzer {
    LexiconQueryAnalyzer::new(Arc::new(Lexicons::embedded().unwrap()))
}

fn full_pos_analyzer() -> LexiconQueryAnalyzer {
    let mut lexicons = Lexicons::embedded().unwrap();
    let full_data = LexiconData {
        nominals: vec![NominalRecord {
            lemma: "전체사전표식".to_owned(),
            pos: DataFinePos::Nng,
            flags: Default::default(),
            overrides: Vec::new(),
        }],
        ..LexiconData::default()
    };
    lexicons
        .load_full_pos(&encode_pos_lexicon(&collect_pos_entries(&full_data)).unwrap())
        .unwrap();
    LexiconQueryAnalyzer::new(Arc::new(lexicons))
}

#[test]
fn merges_origins_for_identical_branches() {
    let plan = compile_query("걷다", &CompileOptions::default(), &analyzer()).unwrap();
    let branch = plan.atoms[0]
        .branches
        .iter()
        .find(|branch| branch.anchor.as_ref() == "걷고".as_bytes())
        .unwrap();
    assert_eq!(branch.origins.len(), 2);
}

#[test]
fn global_pos_forces_an_untagged_atom() {
    let options = CompileOptions {
        global_pos: Some(CoarsePos::Verb),
        ..CompileOptions::default()
    };
    let plan = compile_query("커스텀다", &options, &analyzer()).unwrap();
    assert_eq!(plan.atoms[0].analyses[0].source, AnalysisSource::Forced);
    assert!(matches!(
        plan.atoms[0].analyses[0].morphology,
        Morphology::Predicate(_)
    ));
}

#[test]
fn forced_noun_fallback_preserves_supported_fine_positions() {
    let options = CompileOptions {
        global_pos: Some(CoarsePos::Noun),
        ..CompileOptions::default()
    };
    let plan = compile_query("미등록명사", &options, &analyzer()).unwrap();

    let fine_positions = plan.atoms[0]
        .analyses
        .iter()
        .map(|analysis| analysis.fine_pos)
        .collect::<Vec<_>>();
    assert_eq!(
        fine_positions,
        vec![
            kfind_morph::FinePos::CommonNoun,
            kfind_morph::FinePos::ProperNoun,
            kfind_morph::FinePos::DependentNoun,
        ]
    );
    assert_eq!(plan.atoms[0].branches.len(), 1);
    assert_eq!(plan.atoms[0].branches[0].origins.len(), 3);
}

#[test]
fn forced_noun_preserves_missing_fine_positions_with_full_pos_analysis() {
    let mut lexicons = Lexicons::embedded().unwrap();
    let full_data = LexiconData {
        nominals: vec![NominalRecord {
            lemma: "명".to_owned(),
            pos: DataFinePos::Nng,
            flags: Default::default(),
            overrides: Vec::new(),
        }],
        ..LexiconData::default()
    };
    lexicons
        .load_full_pos(&encode_pos_lexicon(&collect_pos_entries(&full_data)).unwrap())
        .unwrap();
    let analyzer = LexiconQueryAnalyzer::new(Arc::new(lexicons));
    let options = CompileOptions {
        global_pos: Some(CoarsePos::Noun),
        ..CompileOptions::default()
    };

    let plan = compile_query("명", &options, &analyzer).unwrap();
    let fine_positions = plan.atoms[0]
        .analyses
        .iter()
        .map(|analysis| analysis.fine_pos)
        .collect::<Vec<_>>();

    assert_eq!(
        fine_positions,
        vec![
            kfind_morph::FinePos::CommonNoun,
            kfind_morph::FinePos::ProperNoun,
            kfind_morph::FinePos::DependentNoun,
        ]
    );
    assert_eq!(plan.atoms[0].branches.len(), 1);
    assert_eq!(plan.atoms[0].branches[0].origins.len(), 3);
}

#[test]
fn unregistered_da_is_diagnostic_literal_only() {
    let plan = compile_query("미등록다", &CompileOptions::default(), &analyzer()).unwrap();
    assert_eq!(plan.atoms[0].branches.len(), 1);
    assert_eq!(
        plan.atoms[0].branches[0].anchor.as_ref(),
        "미등록다".as_bytes()
    );
    assert!(plan.diagnostics.iter().any(|diagnostic| matches!(
        diagnostic,
        QueryDiagnostic::UnregisteredDaLiteralOnly { .. }
    )));
}

#[test]
fn embedded_irregular_predicates_preserve_reu_reo_and_homonym_unions() {
    for (lemma, expected) in [
        ("다르다", &["달라"][..]),
        ("누르다", &["눌러", "누르러"]),
        ("오르다", &["올라"]),
        ("이르다", &["일러", "이르러"]),
        ("자르다", &["잘라"]),
        ("푸르다", &["푸르러"]),
        ("흐르다", &["흘러"]),
    ] {
        let plan = compile_query(lemma, &CompileOptions::default(), &analyzer()).unwrap();
        for surface in expected {
            assert!(
                plan.atoms[0]
                    .branches
                    .iter()
                    .any(|branch| branch.anchor.as_ref() == surface.as_bytes()),
                "missing {surface} for {lemma}"
            );
        }
    }
}

#[test]
fn literal_expansion_compiles_only_the_input_surface() {
    let options = CompileOptions {
        expand: ExpandMode::Literal,
        ..CompileOptions::default()
    };
    let plan = compile_query("걷다", &options, &analyzer()).unwrap();
    assert_eq!(plan.atoms[0].branches.len(), 1);
    assert_eq!(plan.atoms[0].branches[0].origins.len(), 2);

    let quoted = compile_query("\"걷다\"", &CompileOptions::default(), &analyzer()).unwrap();
    assert_eq!(quoted.atoms[0].branches.len(), 1);
    assert_eq!(
        quoted.atoms[0].branches[0].anchor.as_ref(),
        "걷다".as_bytes()
    );
    assert!(matches!(
        quoted.atoms[0].analyses[0].morphology,
        Morphology::Exact
    ));
}

#[test]
fn canonical_mode_builds_nfc_and_nfd_anchors() {
    let options = CompileOptions::resolve(CompileOptionOverrides {
        normalization: Some(NormalizationMode::Canonical),
        literal: true,
        ..CompileOptionOverrides::default()
    })
    .unwrap();
    let plan = compile_query("가", &options, &analyzer()).unwrap();
    assert_eq!(plan.atoms[0].branches.len(), 2);
    assert!(
        plan.atoms[0]
            .branches
            .iter()
            .all(|branch| branch.boundary.one_scalar_anchor && branch.boundary.require_left)
    );
}

#[test]
fn smart_and_token_keep_distinct_left_boundary_semantics() {
    let smart_noun = compile_query("권한", &CompileOptions::default(), &analyzer()).unwrap();
    assert!(
        smart_noun.atoms[0]
            .branches
            .iter()
            .all(|branch| branch.boundary.require_left)
    );
    assert!(smart_noun.atoms[0].branches.iter().any(|branch| {
        matches!(branch.verifier, BranchVerifier::NominalParticles { .. })
            && branch.context_requirement == ContextRequirement::ExactComponent
    }));

    let any_options = CompileOptions {
        boundary: BoundaryPolicy::Any,
        ..CompileOptions::default()
    };
    let any_noun = compile_query("권한", &any_options, &analyzer()).unwrap();
    assert!(
        any_noun.atoms[0]
            .branches
            .iter()
            .all(|branch| !branch.boundary.require_left && !branch.boundary.require_right)
    );
    assert!(
        any_noun.atoms[0]
            .branches
            .iter()
            .all(|branch| branch.context_requirement != ContextRequirement::ExactComponent)
    );

    let smart_predicate =
        compile_query("검증하다", &CompileOptions::default(), &analyzer()).unwrap();
    assert!(smart_predicate.atoms[0].branches.iter().all(|branch| {
        branch.boundary.require_left && branch.context_requirement == ContextRequirement::None
    }));
    assert!(!smart_predicate.requires_component_resource());

    let smart_full_pos_predicate =
        compile_query("검증하다", &CompileOptions::default(), &full_pos_analyzer()).unwrap();
    assert!(
        smart_full_pos_predicate.atoms[0]
            .branches
            .iter()
            .all(|branch| {
                branch.boundary.require_left
                    && branch.context_requirement == ContextRequirement::ExactComponent
            })
    );
    assert!(smart_full_pos_predicate.requires_component_resource());

    let smart_copula = compile_query("이다", &CompileOptions::default(), &analyzer()).unwrap();
    assert!(smart_copula.requires_component_resource());
    assert!(
        smart_copula.atoms[0]
            .branches
            .iter()
            .all(|branch| !branch.boundary.require_left
                && branch.context_requirement == ContextRequirement::PredicateLexical)
    );

    let token_options = CompileOptions {
        boundary: BoundaryPolicy::Token,
        ..CompileOptions::default()
    };
    let token_predicate = compile_query("검증하다", &token_options, &analyzer()).unwrap();
    assert!(token_predicate.atoms[0].branches.iter().all(|branch| {
        branch.boundary.require_left && branch.context_requirement == ContextRequirement::None
    }));
    assert!(!token_predicate.requires_component_resource());

    let token_copula = compile_query("이다", &token_options, &analyzer()).unwrap();
    assert!(!token_copula.requires_component_resource());
    assert!(token_copula.atoms[0].branches.iter().all(|branch| {
        branch.boundary.require_left && branch.context_requirement == ContextRequirement::None
    }));

    let any_copula = compile_query("이다", &any_options, &analyzer()).unwrap();
    assert!(!any_copula.requires_component_resource());
    assert!(any_copula.atoms[0].branches.iter().all(|branch| {
        !branch.boundary.require_right && branch.context_requirement == ContextRequirement::None
    }));
}

#[test]
fn smart_exact_components_cover_nominals_predicates_and_determiners() {
    for pos in [CoarsePos::Noun, CoarsePos::Pronoun, CoarsePos::Numeral] {
        let plan = compile_query(
            "표면",
            &CompileOptions {
                global_pos: Some(pos),
                ..CompileOptions::default()
            },
            &analyzer(),
        )
        .unwrap();
        assert!(plan.requires_component_resource());
        assert!(
            plan.atoms[0]
                .branches
                .iter()
                .all(|branch| { branch.context_requirement == ContextRequirement::ExactComponent })
        );
    }

    let determiner = compile_query(
        "두",
        &CompileOptions {
            global_pos: Some(CoarsePos::Determiner),
            ..CompileOptions::default()
        },
        &analyzer(),
    )
    .unwrap();
    assert!(determiner.requires_component_resource());
    assert!(
        determiner.atoms[0]
            .branches
            .iter()
            .all(|branch| { branch.context_requirement == ContextRequirement::ExactComponent })
    );

    for (query, pos) in [("걷다", CoarsePos::Verb), ("좋다", CoarsePos::Adjective)] {
        let plan = compile_query(
            query,
            &CompileOptions {
                global_pos: Some(pos),
                ..CompileOptions::default()
            },
            &full_pos_analyzer(),
        )
        .unwrap();
        assert!(plan.requires_component_resource());
        assert!(
            plan.atoms[0]
                .branches
                .iter()
                .all(|branch| { branch.context_requirement == ContextRequirement::ExactComponent })
        );
    }

    for pos in [CoarsePos::Adverb, CoarsePos::Interjection] {
        let plan = compile_query(
            "표면",
            &CompileOptions {
                global_pos: Some(pos),
                ..CompileOptions::default()
            },
            &analyzer(),
        )
        .unwrap();
        assert!(!plan.requires_component_resource());
    }

    for boundary in [BoundaryPolicy::Token, BoundaryPolicy::Any] {
        let plan = compile_query(
            "두",
            &CompileOptions {
                global_pos: Some(CoarsePos::Determiner),
                boundary,
                ..CompileOptions::default()
            },
            &analyzer(),
        )
        .unwrap();
        assert!(!plan.requires_component_resource());
    }
}

#[test]
fn smart_registered_adverb_requires_lexical_context_without_changing_token_or_any() {
    let smart = compile_query("adv:매일", &CompileOptions::default(), &analyzer()).unwrap();
    assert!(smart.requires_component_resource());
    assert!(
        smart.atoms[0]
            .branches
            .iter()
            .all(|branch| { branch.context_requirement == ContextRequirement::LexicalContext })
    );

    for boundary in [BoundaryPolicy::Token, BoundaryPolicy::Any] {
        let plan = compile_query(
            "adv:매일",
            &CompileOptions {
                boundary,
                ..CompileOptions::default()
            },
            &analyzer(),
        )
        .unwrap();
        assert!(!plan.requires_component_resource());
        assert!(
            plan.atoms[0]
                .branches
                .iter()
                .all(|branch| { branch.context_requirement == ContextRequirement::None })
        );
    }
}

#[test]
fn smart_unregistered_adverbs_do_not_require_lexical_context() {
    for query in ["adv:빨리", "adv:매우"] {
        let plan = compile_query(query, &CompileOptions::default(), &analyzer()).unwrap();
        assert!(!plan.requires_component_resource());
        assert!(
            plan.atoms[0]
                .branches
                .iter()
                .all(|branch| branch.context_requirement == ContextRequirement::None)
        );
    }
}

#[test]
fn smart_patterns_cover_supported_analyses_without_a_surface_registry() {
    for query in ["adv:매일", "adv:빨리"] {
        let plan = compile_query(query, &CompileOptions::default(), &analyzer()).unwrap();
        let lexical_form = query.strip_prefix("adv:").unwrap();
        assert!(plan.requires_analysis_graph());
        assert!(plan.atoms[0].branches.iter().all(|branch| {
            branch.morph_patterns.iter().all(|pattern| {
                pattern.fine_pos == DataFinePos::Mag
                    && pattern.lexical_form.as_ref() == lexical_form
            })
        }));
    }

    let noun = compile_query(
        "미등록명사",
        &CompileOptions {
            global_pos: Some(CoarsePos::Noun),
            ..CompileOptions::default()
        },
        &analyzer(),
    )
    .unwrap();
    assert_eq!(noun.atoms[0].branches.len(), 1);
    assert_eq!(
        noun.atoms[0].branches[0]
            .morph_patterns
            .iter()
            .map(|pattern| pattern.fine_pos)
            .collect::<Vec<_>>(),
        [DataFinePos::Nng, DataFinePos::Nnp, DataFinePos::Nnb]
    );
    assert!(
        noun.atoms[0].branches[0]
            .morph_patterns
            .iter()
            .all(|pattern| pattern.lexical_form.as_ref() == "미등록명사")
    );

    let predicate = compile_query("걷다", &CompileOptions::default(), &analyzer()).unwrap();
    assert!(predicate.atoms[0].branches.iter().all(|branch| {
        branch
            .morph_patterns
            .iter()
            .all(|pattern| pattern.lexical_form.as_ref() == "걷")
    }));

    let particle = compile_query("는", &CompileOptions::default(), &analyzer()).unwrap();
    assert!(!particle.requires_analysis_graph());

    for boundary in [BoundaryPolicy::Token, BoundaryPolicy::Any] {
        let plan = compile_query(
            "adv:매일",
            &CompileOptions {
                boundary,
                ..CompileOptions::default()
            },
            &analyzer(),
        )
        .unwrap();
        assert!(!plan.requires_analysis_graph());
        assert!(
            plan.atoms[0]
                .branches
                .iter()
                .all(|branch| branch.morph_patterns.is_empty())
        );
    }

    let literal = compile_query(
        "매일",
        &CompileOptions::resolve(CompileOptionOverrides {
            literal: true,
            ..CompileOptionOverrides::default()
        })
        .unwrap(),
        &analyzer(),
    )
    .unwrap();
    assert!(!literal.requires_analysis_graph());
}

#[test]
fn explicit_pos_smart_opens_only_the_connective_ji_left_boundary() {
    let explicit_smart = compile_query(
        "걷다",
        &CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            ..CompileOptions::default()
        },
        &analyzer(),
    )
    .unwrap();
    let connective = explicit_smart.atoms[0]
        .branches
        .iter()
        .find(|branch| branch.anchor.as_ref() == "걷지".as_bytes())
        .unwrap();
    assert!(!connective.boundary.require_left);
    assert!(connective.boundary.require_right);

    let untagged_smart = compile_query("걷다", &CompileOptions::default(), &analyzer()).unwrap();
    let untagged_connective = untagged_smart.atoms[0]
        .branches
        .iter()
        .find(|branch| branch.anchor.as_ref() == "걷지".as_bytes())
        .unwrap();
    assert!(untagged_connective.boundary.require_left);
    assert!(untagged_connective.boundary.require_right);

    let explicit_token = compile_query(
        "걷다",
        &CompileOptions {
            global_pos: Some(CoarsePos::Verb),
            boundary: BoundaryPolicy::Token,
            ..CompileOptions::default()
        },
        &analyzer(),
    )
    .unwrap();
    let token_connective = explicit_token.atoms[0]
        .branches
        .iter()
        .find(|branch| branch.anchor.as_ref() == "걷지".as_bytes())
        .unwrap();
    assert!(token_connective.boundary.require_left);
    assert!(token_connective.boundary.require_right);
}

#[test]
fn smart_direct_particle_uses_host_verification_instead_of_a_left_boundary() {
    let smart = compile_query("는", &CompileOptions::default(), &analyzer()).unwrap();
    assert_eq!(smart.atoms[0].branches.len(), 1);
    assert_eq!(smart.atoms[0].branches[0].anchor.as_ref(), "는".as_bytes());
    assert!(smart.atoms[0].branches.iter().all(|branch| {
        matches!(branch.verifier, BranchVerifier::DirectParticle { .. })
            && !branch.boundary.require_left
            && branch.boundary.require_right
            && branch.boundary.one_scalar_anchor
    }));

    let token = compile_query(
        "는",
        &CompileOptions {
            boundary: BoundaryPolicy::Token,
            ..CompileOptions::default()
        },
        &analyzer(),
    )
    .unwrap();
    assert!(
        token.atoms[0]
            .branches
            .iter()
            .all(|branch| branch.boundary.require_left && branch.boundary.require_right)
    );
}

#[test]
fn smart_one_scalar_rule_uses_the_source_atom_not_generated_surfaces() {
    let plan = compile_query("이다", &CompileOptions::default(), &analyzer()).unwrap();
    for surface in ["인", "일"] {
        let branch = plan.atoms[0]
            .branches
            .iter()
            .find(|branch| branch.anchor.as_ref() == surface.as_bytes())
            .unwrap_or_else(|| panic!("missing copula branch {surface}"));
        assert!(!branch.boundary.one_scalar_anchor);
        assert!(!branch.boundary.require_left);
        assert_eq!(
            branch.context_requirement,
            ContextRequirement::PredicateLexical
        );
    }
}

#[test]
fn analysis_and_memory_limits_fail_observably() {
    let options = CompileOptions {
        limits: PlanLimits {
            max_analyses_per_atom: 1,
            ..PlanLimits::default()
        },
        ..CompileOptions::default()
    };
    let error = compile_query("걷다", &options, &analyzer()).unwrap_err();
    assert!(matches!(
        *error.kind,
        CompileErrorKind::TooManyAnalyses { .. }
    ));

    let mut canonical = CompileOptions::resolve(CompileOptionOverrides {
        normalization: Some(NormalizationMode::Canonical),
        literal: true,
        ..CompileOptionOverrides::default()
    })
    .unwrap();
    canonical.limits.max_branches = 1;
    let error = compile_query("가", &canonical, &analyzer()).unwrap_err();
    assert!(matches!(
        *error.kind,
        CompileErrorKind::TooManyBranches { .. }
    ));

    let memory = CompileOptions {
        limits: PlanLimits {
            max_matcher_bytes: 1,
            ..PlanLimits::default()
        },
        ..CompileOptions::default()
    };
    let error = compile_query("raw", &memory, &analyzer()).unwrap_err();
    assert!(matches!(
        *error.kind,
        CompileErrorKind::MatcherMemoryExceeded { .. }
    ));

    let continuation = CompileOptions {
        limits: PlanLimits {
            max_continuation_depth: 3,
            ..PlanLimits::default()
        },
        ..CompileOptions::default()
    };
    let error = compile_query("걷다", &continuation, &analyzer()).unwrap_err();
    assert!(matches!(
        *error.kind,
        CompileErrorKind::ContinuationDepthExceeded { .. }
    ));
}

#[test]
fn direct_particle_allomorph_expansion_requires_an_explicit_pos_in_smart_mode() {
    let untagged = compile_query("이", &CompileOptions::default(), &analyzer()).unwrap();
    assert!(
        untagged.atoms[0]
            .branches
            .iter()
            .any(|branch| branch.anchor.as_ref() == "이".as_bytes())
    );
    assert!(
        untagged.atoms[0]
            .branches
            .iter()
            .all(|branch| branch.anchor.as_ref() != "가".as_bytes())
    );

    let forced = compile_query(
        "이",
        &CompileOptions {
            global_pos: Some(CoarsePos::Particle),
            ..CompileOptions::default()
        },
        &analyzer(),
    )
    .unwrap();
    assert!(["이", "가"].iter().all(|surface| {
        forced.atoms[0]
            .branches
            .iter()
            .any(|branch| branch.anchor.as_ref() == surface.as_bytes())
    }));

    let any = compile_query(
        "이",
        &CompileOptions {
            boundary: BoundaryPolicy::Any,
            ..CompileOptions::default()
        },
        &analyzer(),
    )
    .unwrap();
    assert!(["이", "가"].iter().all(|surface| {
        any.atoms[0]
            .branches
            .iter()
            .any(|branch| branch.anchor.as_ref() == surface.as_bytes())
    }));
}

#[test]
fn required_predicate_surfaces_survive_rule_vocabulary_validation() {
    let walking = compile_query("걷다", &CompileOptions::default(), &analyzer()).unwrap();
    let walking_branches = &walking.atoms[0].branches;
    assert!(
        walking_branches
            .iter()
            .any(|branch| branch.anchor.as_ref() == "걸어".as_bytes())
    );

    let eu = walking_branches
        .iter()
        .find(|branch| {
            branch.anchor.as_ref() == "걸으".as_bytes()
                && matches!(
                    &branch.verifier,
                    BranchVerifier::Predicate {
                        continuation: ContinuationState::Eu,
                        ..
                    }
                )
        })
        .expect("걷다 must retain its Eu continuation branch");
    for following in ["면", "셨다"] {
        let matched = verify_predicate_continuation(
            ContinuationState::Eu,
            PredicatePos::Verb,
            "걸으",
            following,
        )
        .expect("required continuation");
        assert!(eu.verifier.accepts_rule_path(&matched.rule_path));
    }

    let pretty = compile_query("예쁘다", &CompileOptions::default(), &analyzer()).unwrap();
    for surface in ["예쁜", "예쁠"] {
        assert!(
            pretty.atoms[0]
                .branches
                .iter()
                .any(|branch| branch.anchor.as_ref() == surface.as_bytes()),
            "missing required branch {surface}"
        );
    }
}

#[test]
fn only_nominalizer_branches_enable_particle_transition() {
    let walking = compile_query("걷다", &CompileOptions::default(), &analyzer()).unwrap();
    let mut nominalizer_rules = BTreeSet::new();
    for branch in &walking.atoms[0].branches {
        let BranchVerifier::Predicate {
            nominal_particle_transition,
            ..
        } = &branch.verifier
        else {
            continue;
        };
        let has_nominalizer_origin = branch.origins.iter().any(|origin| {
            origin.rule_path.last().is_some_and(|rule| {
                matches!(
                    rule.as_str(),
                    "ending.nominalizer" | "ending.nominalizer-gi"
                )
            })
        });
        if *nominal_particle_transition {
            nominalizer_rules.extend(
                branch
                    .origins
                    .iter()
                    .filter_map(|origin| origin.rule_path.last())
                    .map(|rule| rule.as_str()),
            );
        }
        assert_eq!(*nominal_particle_transition, has_nominalizer_origin);
    }
    assert_eq!(
        nominalizer_rules,
        BTreeSet::from(["ending.nominalizer", "ending.nominalizer-gi"])
    );
}

#[test]
fn derivation_nominal_particle_and_override_branches_use_distinct_verifiers() {
    let derivation_options = CompileOptions {
        expand: ExpandMode::Derivation,
        ..CompileOptions::default()
    };
    let derived = compile_query("검증", &derivation_options, &analyzer()).unwrap();
    assert!(derived.atoms[0].branches.iter().any(|branch| {
        branch.anchor.starts_with("검증하".as_bytes())
            && branch.origins.iter().any(|origin| {
                origin
                    .rule_path
                    .iter()
                    .any(|rule| rule.as_str() == "derivation.hada")
            })
    }));

    let nominal = compile_query("사용자", &CompileOptions::default(), &analyzer()).unwrap();
    assert!(
        nominal.atoms[0]
            .branches
            .iter()
            .any(|branch| matches!(&branch.verifier, BranchVerifier::NominalParticles { .. }))
    );

    let pronoun = compile_query("나", &CompileOptions::default(), &analyzer()).unwrap();
    assert!(
        pronoun.atoms[0]
            .branches
            .iter()
            .any(|branch| branch.anchor.as_ref() == "내가".as_bytes())
    );
    let base = pronoun.atoms[0]
        .branches
        .iter()
        .find(|branch| branch.anchor.as_ref() == "나".as_bytes())
        .expect("pronoun base branch");
    assert!(
        !base
            .verifier
            .accepts_rule_path(&[RuleId::from("particle.subject")])
    );
    assert!(
        base.verifier
            .accepts_rule_path(&[RuleId::from("particle.topic")])
    );
}

#[test]
fn derivation_allows_adverb_auxiliaries_but_not_case_particles() {
    let options = CompileOptions {
        expand: ExpandMode::Derivation,
        ..CompileOptions::default()
    };

    for query in ["빨리", "잘"] {
        let plan = compile_query(query, &options, &analyzer()).unwrap();
        assert!(!plan.requires_component_resource());
        let branch = plan.atoms[0]
            .branches
            .iter()
            .find(|branch| branch.anchor.as_ref() == query.as_bytes())
            .expect("adverb base branch");

        assert!(
            branch
                .verifier
                .accepts_rule_path(&[RuleId::from("particle.additive")])
        );
        assert!(
            branch
                .verifier
                .accepts_rule_path(&[RuleId::from("particle.only")])
        );
        assert!(
            !branch
                .verifier
                .accepts_rule_path(&[RuleId::from("particle.subject")])
        );
    }
}

#[test]
fn normalization_none_preserves_raw_jamo_while_nfc_composes_it() {
    let raw = "가";
    let none = CompileOptions::resolve(CompileOptionOverrides {
        normalization: Some(NormalizationMode::None),
        literal: true,
        ..CompileOptionOverrides::default()
    })
    .unwrap();
    let none_plan = compile_query(raw, &none, &analyzer()).unwrap();
    assert_eq!(none_plan.normalization, NormalizationMode::None);
    assert_eq!(
        none_plan.atoms[0].branches[0].anchor.as_ref(),
        raw.as_bytes()
    );

    let nfc = CompileOptions::resolve(CompileOptionOverrides {
        normalization: Some(NormalizationMode::Nfc),
        literal: true,
        ..CompileOptionOverrides::default()
    })
    .unwrap();
    let nfc_plan = compile_query(raw, &nfc, &analyzer()).unwrap();
    assert_eq!(nfc_plan.normalization, NormalizationMode::Nfc);
    assert_eq!(
        nfc_plan.atoms[0].branches[0].anchor.as_ref(),
        "가".as_bytes()
    );
    assert!(!nfc_plan.atoms[0].branches[0].boundary.one_scalar_anchor);
}
