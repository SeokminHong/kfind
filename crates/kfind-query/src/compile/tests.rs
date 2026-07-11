use std::sync::Arc;

use kfind_morph::{
    CoarsePos, ContinuationState, PredicatePos, RuleId, verify_predicate_continuation,
};

use super::*;
use crate::{BoundaryPolicy, CompileOptionOverrides, Lexicons, NormalizationMode, PlanLimits};

fn analyzer() -> LexiconQueryAnalyzer {
    LexiconQueryAnalyzer::new(Arc::new(Lexicons::embedded().unwrap()))
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

    let smart_predicate =
        compile_query("검증하다", &CompileOptions::default(), &analyzer()).unwrap();
    assert!(
        smart_predicate.atoms[0]
            .branches
            .iter()
            .all(|branch| branch.boundary.require_left)
    );

    let smart_copula = compile_query("이다", &CompileOptions::default(), &analyzer()).unwrap();
    assert!(
        smart_copula.atoms[0]
            .branches
            .iter()
            .all(|branch| !branch.boundary.require_left)
    );

    let token_options = CompileOptions {
        boundary: BoundaryPolicy::Token,
        ..CompileOptions::default()
    };
    let token_predicate = compile_query("검증하다", &token_options, &analyzer()).unwrap();
    assert!(
        token_predicate.atoms[0]
            .branches
            .iter()
            .all(|branch| branch.boundary.require_left)
    );
}

#[test]
fn smart_direct_particle_uses_host_verification_instead_of_a_left_boundary() {
    let smart = compile_query("는", &CompileOptions::default(), &analyzer()).unwrap();
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
