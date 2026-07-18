use grep_matcher::{LineMatchKind, LineTerminator, Matcher};
use kfind_data::{
    ComponentResource, MecabSourceMorphologyEntry, decode_component_resource,
    encode_component_resource,
};
use kfind_morph::{CoarsePos, ContinuationState, FinePos, ParticleTransition, RuleId};
use kfind_query::{
    Analysis, AnalysisSource, AtomPlan, BoundaryPolicy, BoundaryProof, CandidateConsumption,
    CandidateDecision, CandidateLeftContext, CandidateProgram, CoreMapping, Morphology,
    NominalMorphology, Origin, PhrasePolicy, PlanLimits, QueryPlan, StructuralConstraint,
};
use unicode_normalization::UnicodeNormalization;

use super::*;

#[test]
fn predicate_continuation_extends_token_and_provenance() {
    let allowed = rules(&["ending.polite-declarative"]);
    let branch = predicate_branch(
        "걸었",
        "걸".len(),
        ContinuationState::Past,
        allowed,
        vec![
            origin(0, &["lexical.d-irregular", "ending.past"]),
            origin(1, &["ending.past"]),
        ],
    );
    let matcher = matcher(vec![atom(BoundaryPolicy::Smart, vec![branch])], 24);

    let matched = matcher
        .find_at_with_meta("걸었습니다.".as_bytes(), 0)
        .expect("predicate continuation should match");
    let atom = &matched.atoms[0];
    assert_eq!(&"걸었습니다."[atom.core.clone()], "걸");
    assert_eq!(&"걸었습니다."[atom.token.clone()], "걸었습니다");
    assert_eq!(atom.origins.len(), 2);
    assert!(atom.origins.iter().all(|origin| {
        origin
            .rule_path
            .last()
            .is_some_and(|rule| rule.as_str() == "ending.polite-declarative")
    }));
}

#[test]
fn predicate_continuation_honors_allowed_rule_vocabulary() {
    let branch = predicate_branch(
        "걸었",
        "걸".len(),
        ContinuationState::Past,
        rules(&[]),
        vec![origin(0, &["ending.past"])],
    );
    let matcher = matcher(vec![atom(BoundaryPolicy::Smart, vec![branch])], 24);

    assert!(
        matcher
            .find_at_with_meta("걸었습니다".as_bytes(), 0)
            .is_none()
    );
}

#[test]
fn auxiliary_component_after_an_aoeo_prefix_is_not_a_whole_predicate_conflict() {
    let text = "보여준다";
    let core = "보여".len().."보여준".len();
    let candidate = ExecutedCandidate {
        anchor: core.clone(),
        consumed: core.clone(),
        suffix_rules: Vec::new(),
        verified: VerifiedSpan {
            core: core.clone(),
            token: core.clone(),
            origins: Vec::new(),
        },
    };
    let branch = predicate_branch(
        "준",
        "준".len(),
        ContinuationState::Terminal,
        rules(&[]),
        vec![origin(0, &[])],
    );
    let resolver = kfind_morph::ConstraintResolver::new(Arc::new(component_resource()));

    assert!(!has_conflicting_whole_predicate(
        text.as_bytes(),
        &candidate,
        &branch,
        &resolver,
    ));
}

#[test]
fn nominal_particle_verifier_consumes_chain_and_checks_allomorphs() {
    let allowed = rules(&["particle.dative", "particle.direction", "particle.plural"]);
    let user_matcher = matcher(
        vec![atom(
            BoundaryPolicy::Smart,
            vec![nominal_branch("사용자", Arc::clone(&allowed))],
        )],
        24,
    );
    let matched = user_matcher
        .find_at_with_meta("사용자들에게".as_bytes(), 0)
        .expect("plural particle chain should match");
    assert_eq!(&"사용자들에게"[matched.atoms[0].core.clone()], "사용자");
    assert_eq!(
        &"사용자들에게"[matched.atoms[0].token.clone()],
        "사용자들에게"
    );
    assert_eq!(
        matched.atoms[0].origins[0]
            .rule_path
            .iter()
            .map(RuleId::as_str)
            .collect::<Vec<_>>(),
        ["particle.plural", "particle.dative"]
    );

    let road_matcher = matcher(
        vec![atom(
            BoundaryPolicy::Smart,
            vec![nominal_branch("길", allowed)],
        )],
        24,
    );
    assert!(
        road_matcher
            .find_at_with_meta("길로".as_bytes(), 0)
            .is_some()
    );
    assert!(
        road_matcher
            .find_at_with_meta("길으로".as_bytes(), 0)
            .is_none()
    );
}

#[test]
fn smart_left_boundary_rejects_compounds_but_any_accepts_them() {
    let smart = matcher(
        vec![atom(
            BoundaryPolicy::Smart,
            vec![nominal_branch("권한", rules(&["particle.topic"]))],
        )],
        24,
    );
    assert!(
        smart
            .find_at_with_meta("사용자권한은".as_bytes(), 0)
            .is_none()
    );

    let mut branch = nominal_branch("권한", rules(&["particle.topic"]));
    branch.set_boundary(proof(false, false, false));
    let any = matcher(vec![atom(BoundaryPolicy::Any, vec![branch])], 24);
    let matched = any
        .find_at_with_meta("사용자권한은".as_bytes(), 0)
        .expect("any boundary should allow compound substrings");
    assert_eq!(matched.span.start, "사용자".len());
}

#[test]
fn structural_components_remain_possible_when_whole_analyses_compete() {
    let resource = Arc::new(component_resource());
    let accepted = component_matcher("권한", Arc::clone(&resource));
    let source_component = component_matcher("학교", resource);

    let matched = accepted
        .find_at_with_meta("사용자권한".as_bytes(), 0)
        .expect("structurally possible component path should match");
    assert_eq!(matched.span, "사용자".len().."사용자권한".len());
    assert!(
        source_component
            .find_at_with_meta("대학교".as_bytes(), 0)
            .is_some()
    );
}

#[test]
fn repeated_structural_contexts_preserve_each_absolute_span() {
    let matcher = component_matcher("학교", Arc::new(component_resource()));
    let text = "대학교 대학교 대학교 대학교";
    let expected = text
        .match_indices("학교")
        .map(|(start, token)| start..start + token.len())
        .collect::<Vec<_>>();

    assert_eq!(
        matcher
            .find_all_with_meta(text.as_bytes())
            .into_iter()
            .map(|matched| matched.span)
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn nominal_particle_host_rejects_a_component_crossing_substring() {
    let matcher = component_matcher("사과", Arc::new(component_resource()));

    assert!(
        matcher
            .find_at_with_meta("역사과목을".as_bytes(), 0)
            .is_none()
    );
}

#[test]
fn structural_component_support_does_not_depend_on_word_cost() {
    let resource = Arc::new(component_resource());
    let within = component_matcher("안", Arc::clone(&resource));
    let over = component_matcher("밖", resource);

    assert!(within.find_at_with_meta("경계안".as_bytes(), 0).is_some());
    assert!(over.find_at_with_meta("경계밖".as_bytes(), 0).is_some());
}

#[test]
fn registered_context_surface_preserves_the_raw_component_decision() {
    let matcher = component_matcher("매", Arc::new(component_resource()));

    assert!(matcher.find_at_with_meta("매일".as_bytes(), 0).is_none());
}

#[test]
fn exact_structural_token_graph_is_prepared_without_covering_larger_tokens() {
    let matcher = component_matcher_with_analysis(
        "매일",
        non_predicate_analysis("매일", CoarsePos::Adverb, FinePos::GeneralAdverb),
        Arc::new(component_resource()),
    );

    let entries = &matcher.prepared_exact_tokens.entries[prepared_token_slot(false, false)];
    let prepared = entries
        .iter()
        .find(|entry| entry.text.as_ref() == "매일")
        .expect("exact structural token must be registered");
    assert!(prepared.graph.get().is_none());
    assert!(entries.iter().all(|entry| entry.text.as_ref() != "매일가"));
    assert_eq!(matcher.find_all_with_meta("매일 매일".as_bytes()).len(), 2);
    assert!(prepared.graph.get().is_some_and(Option::is_some));
    assert!(matcher.find_all_with_meta("매일가".as_bytes()).is_empty());
}

#[test]
fn exact_structural_token_graph_is_published_once_for_concurrent_searches() {
    let matcher = Arc::new(component_matcher_with_analysis(
        "매일",
        non_predicate_analysis("매일", CoarsePos::Adverb, FinePos::GeneralAdverb),
        Arc::new(component_resource()),
    ));

    std::thread::scope(|scope| {
        let handles = (0..8)
            .map(|_| {
                let matcher = Arc::clone(&matcher);
                scope.spawn(move || matcher.find_all_with_meta("매일 매일".as_bytes()).len())
            })
            .collect::<Vec<_>>();
        for handle in handles {
            assert_eq!(handle.join().expect("structural search must not panic"), 2);
        }
    });

    let entries = &matcher.prepared_exact_tokens.entries[prepared_token_slot(false, false)];
    let prepared = entries
        .iter()
        .find(|entry| entry.text.as_ref() == "매일")
        .expect("exact structural token must be registered");
    assert!(prepared.graph.get().is_some_and(Option::is_some));
}

#[test]
fn exact_structural_token_preparation_is_bounded_by_plan_program_count() {
    let branches = (0..=MAX_PREPARED_EXACT_TOKEN_PROGRAMS)
        .map(|_| {
            let mut branch = exact_branch("공유", true);
            mark_structural(&mut branch);
            branch
        })
        .collect();
    let matcher = contextual_matcher(branches, Arc::new(component_resource()));

    assert!(
        matcher
            .prepared_exact_tokens
            .entries
            .iter()
            .all(|entries| entries.is_empty())
    );
}

#[test]
fn exact_component_accepts_pronoun_numeral_and_determiner_spans() {
    let resource = Arc::new(component_resource());
    let pronoun = component_matcher_with_analysis(
        "자기",
        non_predicate_analysis("자기", CoarsePos::Pronoun, FinePos::Pronoun),
        Arc::clone(&resource),
    );
    let numeral = component_matcher_with_analysis(
        "둘",
        non_predicate_analysis("둘", CoarsePos::Numeral, FinePos::Numeral),
        Arc::clone(&resource),
    );
    let determiner = component_matcher_with_analysis(
        "두",
        non_predicate_analysis("두", CoarsePos::Determiner, FinePos::Determiner),
        resource,
    );

    assert!(
        pronoun
            .find_at_with_meta("자기견해".as_bytes(), 0)
            .is_some()
    );
    assert!(numeral.find_at_with_meta("둘다".as_bytes(), 0).is_some());
    assert!(
        determiner
            .find_at_with_meta("두사람".as_bytes(), 0)
            .is_some()
    );
}

#[test]
fn component_context_without_a_resource_is_a_build_error() {
    let mut branch = exact_branch("일", false);
    mark_structural(&mut branch);
    let plan = Arc::new(QueryPlan {
        raw_query: "test".into(),
        atoms: vec![atom(BoundaryPolicy::Smart, vec![branch])],
        phrase_policy: PhrasePolicy { max_gap: 24 },
        normalization: kfind_query::NormalizationMode::Nfc,
        limits: PlanLimits::default(),
        diagnostics: Vec::new(),
        particle_allomorphs: Arc::from([]),
        particle_transitions: Arc::from([]),
        auxiliary_particle_rules: Arc::from([]),
        predicate_ending_initial_particle_rules: Arc::from([]),
        estimated_matcher_bytes: 0,
    });

    let error = MorphMatcher::new(plan).expect_err("component context must require a resource");
    assert!(matches!(
        error,
        MorphMatcherBuildError::ComponentResourceRequired
    ));
}

#[test]
fn unresolved_homograph_without_sentence_context_remains_recall_first() {
    let mut branch = exact_branch("일", false);
    mark_structural(&mut branch);
    let matcher = contextual_matcher(vec![branch], Arc::new(component_resource()));

    assert!(matcher.find_at_with_meta("매일".as_bytes(), 0).is_some());
    assert!(matcher.find_at_with_meta("일".as_bytes(), 0).is_some());
    assert!(matcher.find_at_with_meta("교사일".as_bytes(), 0).is_some());
    assert!(matcher.find_at_with_meta("학생일".as_bytes(), 0).is_some());
    assert!(matcher.find_at_with_meta("책일".as_bytes(), 0).is_some());
}

#[test]
fn unresolved_homograph_preserves_all_query_branches() {
    let mut predicate = exact_branch("일", false);
    mark_structural(&mut predicate);
    let mut exact = exact_branch("일", false);
    exact.origins = vec![origin(1, &[])];
    let matcher = contextual_matcher(vec![predicate, exact], Arc::new(component_resource()));

    let matched = matcher
        .find_at_with_meta("매일".as_bytes(), 0)
        .expect("the non-predicate query branch should remain");
    assert_eq!(
        matched.atoms[0].origins,
        vec![origin(0, &[]), origin(1, &[])]
    );
}

#[test]
fn any_boundary_keeps_the_same_copula_candidate() {
    let mut branch = exact_branch("일", false);
    branch.set_boundary(proof(false, false, false));
    let matcher = matcher(vec![atom(BoundaryPolicy::Any, vec![branch])], 24);

    assert!(matcher.find_at_with_meta("매일".as_bytes(), 0).is_some());
}

#[test]
fn nominal_component_does_not_bypass_a_rejected_particle_allomorph() {
    let matcher = component_matcher("권한", Arc::new(component_resource()));

    assert!(
        matcher
            .find_at_with_meta("사용자권한는".as_bytes(), 0)
            .is_none()
    );
    assert!(
        matcher
            .find_at_with_meta("사용자권한는관리".as_bytes(), 0)
            .is_none()
    );
}

#[test]
fn nominal_host_can_precede_a_complete_copula_suffix() {
    let resource = Arc::new(component_resource());
    let result = component_matcher("결과", Arc::clone(&resource));
    let solid = component_matcher("고체", Arc::clone(&resource));
    let university = component_matcher("대학", resource);

    assert!(result.find_at_with_meta("결과이다".as_bytes(), 0).is_some());
    assert!(solid.find_at_with_meta("고체이긴".as_bytes(), 0).is_some());
    for text in ["결과다", "결과였다", "결과였고", "결과여서"] {
        assert!(
            result.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "{text}"
        );
    }
    for text in ["결과이", "결과였"] {
        assert!(
            result.find_at_with_meta(text.as_bytes(), 0).is_none(),
            "{text}"
        );
    }
    for text in ["대학다", "대학였다", "대학여서"] {
        assert!(
            university.find_at_with_meta(text.as_bytes(), 0).is_none(),
            "{text}"
        );
    }
}

#[test]
fn verified_particle_chains_can_precede_complete_copula_suffixes() {
    let matcher = component_matcher_with_particle_rules(
        "대학",
        rules(&["particle.restrictive.ppun", "particle.only"]),
        Arc::new(component_resource()),
    );

    for text in ["대학뿐이다", "대학뿐인", "대학뿐만이다"] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_some(),
            "{text}"
        );
    }
    for text in ["대학뿐이", "대학뿐여서", "대학뿐이다른", "대학뿐도만이다"] {
        assert!(
            matcher.find_at_with_meta(text.as_bytes(), 0).is_none(),
            "{text}"
        );
    }
}

#[test]
fn overlapping_anchors_select_leftmost_longest_verified_token() {
    let mut short = exact_branch("가", false);
    short.set_boundary(proof(false, false, true));
    let mut long = exact_branch("가가", false);
    long.set_boundary(proof(false, false, false));
    let branches = vec![short, long];
    let matcher = matcher(vec![atom(BoundaryPolicy::Any, branches)], 24);

    let matched = matcher
        .find_at_with_meta("가가가".as_bytes(), 0)
        .expect("overlapping anchors should match");
    assert_eq!(matched.span, 0.."가가".len());
    assert_eq!(matcher.find_all_with_meta("가가가".as_bytes()).len(), 2);
}

#[test]
fn repeated_single_atom_matches_advance_without_changing_leftmost_longest() {
    let mut short = exact_branch("가", false);
    short.set_boundary(proof(false, false, true));
    let mut long = exact_branch("가가", false);
    long.set_boundary(proof(false, false, false));
    let matcher = matcher(vec![atom(BoundaryPolicy::Any, vec![short, long])], 24);
    let text = "가가 ".repeat(2_048);

    let matches = matcher.find_all_with_meta(text.as_bytes());

    assert_eq!(matches.len(), 2_048);
    assert!(
        matches
            .iter()
            .all(|matched| matched.span.len() == "가가".len())
    );
}

#[test]
fn verification_counters_treat_accepted_components_as_verified_programs() {
    let mut contextual = nominal_branch("학교", rules(&["particle.topic"]));
    mark_structural(&mut contextual);
    let mut atom = atom(BoundaryPolicy::Smart, vec![contextual]);
    atom.analyses.push(nominal_analysis("학교"));
    materialize_structural_programs(&mut atom);
    let plan = QueryPlan {
        raw_query: "학교".into(),
        atoms: vec![atom],
        phrase_policy: PhrasePolicy { max_gap: 24 },
        normalization: kfind_query::NormalizationMode::Nfc,
        limits: PlanLimits::default(),
        diagnostics: Vec::new(),
        particle_allomorphs: Arc::from([]),
        particle_transitions: Arc::from([]),
        auxiliary_particle_rules: Arc::from([]),
        predicate_ending_initial_particle_rules: Arc::from([]),
        estimated_matcher_bytes: 0,
    };
    let matcher =
        MorphMatcher::with_component_resource(Arc::new(plan), Arc::new(component_resource()))
            .unwrap();

    assert_eq!(
        matcher.verification_counters("대학교는 학교는".as_bytes()),
        VerificationCounters {
            raw_anchor_hits: 2,
            verified_program_hits: 2,
            structural_candidate_hits: 0,
            unique_structural_windows: 0,
        }
    );
}

#[test]
fn identical_verified_spans_merge_origins_independent_of_branch_order() {
    let mut first = exact_branch("권한", true);
    first.origins = vec![origin(1, &["source.b"]), origin(0, &["source.a"])];
    let mut second = first.clone();
    second.origins = vec![origin(0, &["source.a"]), origin(2, &["source.c"])];
    let matcher = matcher(vec![atom(BoundaryPolicy::Smart, vec![first, second])], 24);

    let matched = matcher.find_at_with_meta("권한".as_bytes(), 0).unwrap();
    assert_eq!(
        matched.atoms[0]
            .origins
            .iter()
            .map(|origin| origin.analysis_index)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
}

#[test]
fn phrase_join_preserves_order_and_unicode_scalar_gap() {
    let first = atom(
        BoundaryPolicy::Smart,
        vec![nominal_branch("권한", rules(&["particle.object"]))],
    );
    let second = atom(BoundaryPolicy::Smart, vec![exact_branch("검증했다", true)]);
    let text = "권한을 먼저 검증했다";

    let too_narrow = matcher(vec![first.clone(), second.clone()], 3);
    assert!(too_narrow.find_at_with_meta(text.as_bytes(), 0).is_none());

    let matcher = matcher(vec![first, second], 4);
    let matched = matcher
        .find_at_with_meta(text.as_bytes(), 0)
        .expect("phrase should join in atom order");
    assert_eq!(matched.atoms.len(), 2);
    assert_eq!(matched.span, 0..text.len());
    assert!(
        matcher
            .find_at_with_meta("검증했다 권한을".as_bytes(), 0)
            .is_none()
    );
}

#[test]
fn phrase_find_all_selects_non_overlapping_matches_in_order() {
    let first = atom(BoundaryPolicy::Smart, vec![exact_branch("권한", true)]);
    let second = atom(BoundaryPolicy::Smart, vec![exact_branch("검증", true)]);
    let matcher = matcher(vec![first, second], 1);
    let text = "권한 검증 권한 검증";

    let matches = matcher.find_all_with_meta(text.as_bytes());

    assert_eq!(matches.len(), 2);
    assert_eq!(&text[matches[0].span.clone()], "권한 검증");
    assert_eq!(&text[matches[1].span.clone()], "권한 검증");
    assert!(matches[0].span.end <= matches[1].span.start);
}

#[test]
fn phrase_find_all_applies_the_match_limit_during_selection() {
    let first = atom(BoundaryPolicy::Smart, vec![exact_branch("권한", true)]);
    let second = atom(BoundaryPolicy::Smart, vec![exact_branch("검증", true)]);
    let matcher = matcher(vec![first, second], 1);
    let text = "권한 검증 권한 검증";

    let error = matcher
        .find_all_with_meta_limit(text.as_bytes(), 1)
        .unwrap_err();

    assert_eq!(error.limit(), 1);
}

#[test]
fn phrase_find_all_bounds_repeated_span_combinations() {
    let mut repeated_branch = exact_branch("가", false);
    repeated_branch.set_boundary(proof(false, false, true));
    let repeated_atom = atom(BoundaryPolicy::Any, vec![repeated_branch]);
    let matcher = matcher(vec![repeated_atom; 8], 128);
    let text = "가".repeat(128);

    let matches = matcher.find_all_with_meta(text.as_bytes());

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].span, 0..text.len());
    assert_eq!(matches[0].atoms.len(), 8);
}

#[test]
fn phrase_join_ignores_invalid_utf8_outside_the_candidate_range() {
    let first = atom(
        BoundaryPolicy::Smart,
        vec![nominal_branch("권한", rules(&["particle.object"]))],
    );
    let second = atom(BoundaryPolicy::Smart, vec![exact_branch("검증했다", true)]);
    let matcher = matcher(vec![first, second], 4);
    let mut bytes = b"prefix".to_vec();
    bytes.push(0xff);
    let phrase_start = bytes.len();
    bytes.extend_from_slice(" 권한을 먼저 검증했다 suffix".as_bytes());
    bytes.push(0xfe);

    let matched = matcher
        .find_at_with_meta(&bytes, 0)
        .expect("invalid bytes outside the phrase must not suppress the match");
    assert_eq!(
        matched.span,
        phrase_start + " ".len()..phrase_start + " 권한을 먼저 검증했다".len()
    );
}

#[test]
fn phrase_join_does_not_cross_an_invalid_utf8_gap() {
    let first = atom(
        BoundaryPolicy::Smart,
        vec![nominal_branch("권한", rules(&["particle.object"]))],
    );
    let second = atom(BoundaryPolicy::Smart, vec![exact_branch("검증했다", true)]);
    let matcher = matcher(vec![first, second], 24);
    let mut bytes = "권한을 ".as_bytes().to_vec();
    bytes.push(0xff);
    bytes.extend_from_slice(" 검증했다".as_bytes());

    assert!(matcher.find_at_with_meta(&bytes, 0).is_none());
}

#[test]
fn grep_matcher_adapter_returns_verified_token_range() {
    let matcher = matcher(
        vec![atom(
            BoundaryPolicy::Smart,
            vec![nominal_branch("사용자", rules(&["particle.topic"]))],
        )],
        24,
    );

    let matched = Matcher::find(&matcher, "사용자는".as_bytes())
        .unwrap()
        .expect("grep matcher should find a token");
    assert_eq!((matched.start(), matched.end()), (0, "사용자는".len()));
}

#[test]
fn span_only_search_matches_the_metadata_range() {
    let matcher = matcher(
        vec![atom(
            BoundaryPolicy::Smart,
            vec![nominal_branch("사용자", rules(&["particle.topic"]))],
        )],
        24,
    );
    let text = "사용자는".as_bytes();

    let span = matcher.find_span_at(text, 0).expect("span-only match");
    let matched = matcher.find_at_with_meta(text, 0).expect("metadata match");

    assert_eq!(span, matched.span);
    assert!(!matched.atoms[0].origins.is_empty());
}

#[test]
fn grep_matcher_advertises_raw_anchor_candidates_for_line_local_plans() {
    let matcher = matcher(
        vec![atom(
            BoundaryPolicy::Smart,
            vec![exact_branch("권한", true)],
        )],
        24,
    );
    let text = "사용자권한\n권한";

    assert_eq!(
        Matcher::line_terminator(&matcher),
        Some(LineTerminator::byte(b'\n'))
    );
    assert!(matches!(
        Matcher::find_candidate_line(&matcher, text.as_bytes()).unwrap(),
        Some(LineMatchKind::Candidate(at)) if at == "사용자".len()
    ));
    assert!(
        Matcher::find(&matcher, "사용자권한".as_bytes())
            .unwrap()
            .is_none()
    );
    assert!(
        Matcher::find(&matcher, "권한".as_bytes())
            .unwrap()
            .is_some()
    );
}

#[test]
fn grep_matcher_requires_every_phrase_atom_on_the_same_candidate_line() {
    let matcher = matcher(
        vec![
            atom(BoundaryPolicy::Any, vec![exact_branch("권한", true)]),
            atom(BoundaryPolicy::Any, vec![exact_branch("검증", true)]),
        ],
        24,
    );
    let missing_per_line = "권한 권한\n검증 검증";
    assert!(
        Matcher::find_candidate_line(&matcher, missing_per_line.as_bytes())
            .unwrap()
            .is_none()
    );

    let candidate = "권한 권한\n검증 권한";
    assert!(matches!(
        Matcher::find_candidate_line(&matcher, candidate.as_bytes()).unwrap(),
        Some(LineMatchKind::Candidate(at)) if at == "권한 권한\n".len()
    ));
}

#[test]
fn grep_matcher_keeps_newline_literal_plans_on_the_safe_path() {
    let matcher = matcher(
        vec![atom(
            BoundaryPolicy::Any,
            vec![exact_branch("권한\n검증", false)],
        )],
        24,
    );

    assert_eq!(Matcher::line_terminator(&matcher), None);
}

#[test]
fn malformed_neighbors_do_not_panic_or_extend_the_match() {
    let matcher = matcher(
        vec![atom(
            BoundaryPolicy::Token,
            vec![exact_branch("권한", true)],
        )],
        24,
    );
    let bytes = [0xff, 0xea, 0xb6, 0x8c, 0xed, 0x95, 0x9c, 0xff];

    let matched = matcher.find_at_with_meta(&bytes, 0).unwrap();
    assert_eq!(matched.span, 1..7);
}

#[test]
fn nfd_branches_verify_nfc_morphology_and_preserve_original_offsets() {
    let predicate_anchor = "걸었".nfd().collect::<String>();
    let predicate_core_len = "걸".nfd().collect::<String>().len();
    let predicate = predicate_branch(
        &predicate_anchor,
        predicate_core_len,
        ContinuationState::Past,
        rules(&["ending.polite-declarative"]),
        vec![origin(0, &["ending.past"])],
    );
    let predicate_matcher = matcher(vec![atom(BoundaryPolicy::Smart, vec![predicate])], 24);
    let text = "걸었습니다".nfd().collect::<String>();
    let matched = predicate_matcher
        .find_at_with_meta(text.as_bytes(), 0)
        .expect("NFD predicate should use NFC rule tables");
    assert_eq!(matched.span, 0..text.len());

    let nominal_anchor = "사용자".nfd().collect::<String>();
    let nominal = nominal_branch(
        &nominal_anchor,
        rules(&["particle.dative", "particle.plural"]),
    );
    let matcher = matcher(vec![atom(BoundaryPolicy::Smart, vec![nominal])], 24);
    let text = "사용자들에게".nfd().collect::<String>();
    let matched = matcher
        .find_at_with_meta(text.as_bytes(), 0)
        .expect("NFD particle chain should use NFC rule tables");
    assert_eq!(matched.span, 0..text.len());
    assert_eq!(matched.atoms[0].core.end, nominal_anchor.len());
}

#[test]
fn exact_nfd_branch_preserves_the_compiled_anchor_span() {
    let anchor = "권한".nfd().collect::<String>();
    let matcher = matcher(
        vec![atom(
            BoundaryPolicy::Smart,
            vec![exact_branch(&anchor, true)],
        )],
        24,
    );
    let text = "권한 확인".nfd().collect::<String>();

    let matched = matcher
        .find_at_with_meta(text.as_bytes(), 0)
        .expect("exact NFD anchor should match without extending the token");

    assert_eq!(matched.span, 0..anchor.len());
}

#[test]
fn contracted_vowel_environment_checks_left_context_without_lemma_special_cases() {
    let mut branch = predicate_branch(
        "여서",
        "여".len(),
        ContinuationState::Terminal,
        rules(&[]),
        vec![origin(0, &["lexical.copula", "ending.aoeo-seo"])],
    );
    let CandidateConsumption::PredicateContinuation { left_context, .. } = &mut branch.consumption
    else {
        unreachable!("predicate branch helper returned another verifier")
    };
    *left_context = CandidateLeftContext::ContractedAfterVowel {
        uncontracted_prefix: "이".into(),
    };
    let matcher = matcher(vec![atom(BoundaryPolicy::Smart, vec![branch])], 24);

    assert!(
        matcher
            .find_at_with_meta("학교여서".as_bytes(), 0)
            .is_some()
    );
    assert!(
        matcher
            .find_at_with_meta("학생이여서".as_bytes(), 0)
            .is_none()
    );
    assert!(matcher.find_at_with_meta("여서".as_bytes(), 0).is_none());

    let mut malformed = vec![0xff];
    malformed.extend_from_slice("학교여서".as_bytes());
    assert!(matcher.find_at_with_meta(&malformed, 0).is_some());
}

fn matcher(atoms: Vec<AtomPlan>, max_gap: usize) -> MorphMatcher {
    MorphMatcher::new(Arc::new(QueryPlan {
        raw_query: "test".into(),
        atoms,
        phrase_policy: PhrasePolicy { max_gap },
        normalization: kfind_query::NormalizationMode::Nfc,
        limits: PlanLimits::default(),
        diagnostics: Vec::new(),
        particle_allomorphs: Arc::from([]),
        particle_transitions: Arc::from([]),
        auxiliary_particle_rules: Arc::from([]),
        predicate_ending_initial_particle_rules: Arc::from([]),
        estimated_matcher_bytes: 0,
    }))
    .unwrap()
}

fn component_matcher(anchor: &str, resource: Arc<ComponentResource>) -> MorphMatcher {
    component_matcher_with_analysis(anchor, nominal_analysis(anchor), resource)
}

fn component_matcher_with_analysis(
    anchor: &str,
    analysis: Analysis,
    resource: Arc<ComponentResource>,
) -> MorphMatcher {
    component_matcher_with_analysis_and_particle_rules(anchor, analysis, Arc::from([]), resource)
}

fn component_matcher_with_particle_rules(
    anchor: &str,
    allowed_rule_ids: Arc<[RuleId]>,
    resource: Arc<ComponentResource>,
) -> MorphMatcher {
    component_matcher_with_analysis_and_particle_rules(
        anchor,
        nominal_analysis(anchor),
        allowed_rule_ids,
        resource,
    )
}

fn component_matcher_with_analysis_and_particle_rules(
    anchor: &str,
    analysis: Analysis,
    allowed_rule_ids: Arc<[RuleId]>,
    resource: Arc<ComponentResource>,
) -> MorphMatcher {
    let mut branch = if matches!(&analysis.morphology, Morphology::Nominal(_)) {
        nominal_branch(anchor, allowed_rule_ids)
    } else {
        exact_branch(anchor, true)
    };
    mark_structural(&mut branch);
    let mut atom = atom(BoundaryPolicy::Smart, vec![branch]);
    atom.analyses.push(analysis);
    materialize_structural_programs(&mut atom);
    let plan = QueryPlan {
        raw_query: anchor.into(),
        atoms: vec![atom],
        phrase_policy: PhrasePolicy { max_gap: 24 },
        normalization: kfind_query::NormalizationMode::Nfc,
        limits: PlanLimits::default(),
        diagnostics: Vec::new(),
        particle_allomorphs: Arc::from([]),
        particle_transitions: Arc::from([
            ParticleTransition::new(
                "particle.restrictive.ppun",
                vec![RuleId::from("particle.only")].into_boxed_slice(),
            ),
            ParticleTransition::new("particle.only", Vec::<RuleId>::new().into_boxed_slice()),
        ]),
        auxiliary_particle_rules: Arc::from([]),
        predicate_ending_initial_particle_rules: Arc::from([]),
        estimated_matcher_bytes: 0,
    };
    MorphMatcher::with_component_resource(Arc::new(plan), resource).unwrap()
}

fn contextual_matcher(
    branches: Vec<CandidateProgram>,
    resource: Arc<ComponentResource>,
) -> MorphMatcher {
    let mut query_atom = atom(BoundaryPolicy::Smart, branches);
    query_atom.analyses = vec![
        non_predicate_analysis("이다", CoarsePos::Adjective, FinePos::Copula),
        nominal_analysis("일"),
    ];
    materialize_structural_programs(&mut query_atom);
    let plan = QueryPlan {
        raw_query: "test".into(),
        atoms: vec![query_atom],
        phrase_policy: PhrasePolicy { max_gap: 24 },
        normalization: kfind_query::NormalizationMode::Nfc,
        limits: PlanLimits::default(),
        diagnostics: Vec::new(),
        particle_allomorphs: Arc::from([]),
        particle_transitions: Arc::from([]),
        auxiliary_particle_rules: Arc::from([]),
        predicate_ending_initial_particle_rules: Arc::from([]),
        estimated_matcher_bytes: 0,
    };
    MorphMatcher::with_component_resource(Arc::new(plan), resource).unwrap()
}

fn nominal_analysis(lemma: &str) -> Analysis {
    Analysis {
        lemma: lemma.into(),
        coarse_pos: CoarsePos::Noun,
        fine_pos: FinePos::CommonNoun,
        morphology: Morphology::Nominal(NominalMorphology::default()),
        source: AnalysisSource::Forced,
    }
}

fn non_predicate_analysis(lemma: &str, coarse_pos: CoarsePos, fine_pos: FinePos) -> Analysis {
    Analysis {
        lemma: lemma.into(),
        coarse_pos,
        fine_pos,
        morphology: if matches!(coarse_pos, CoarsePos::Pronoun | CoarsePos::Numeral) {
            Morphology::Nominal(NominalMorphology::default())
        } else {
            Morphology::Exact
        },
        source: AnalysisSource::Forced,
    }
}

fn component_resource() -> ComponentResource {
    let entries = [
        component_entry("사용자", "NNG", -5_000),
        component_entry("권한", "NNG", -5_000),
        component_entry("사용자권한", "NNG", 5_000),
        component_entry("대", "XPN", 5_000),
        component_entry("학교", "NNG", 5_000),
        component_expression_entry("대학교", "XPN+NNG", "대/XPN/*+학교/NNG/*", -5_000),
        component_entry("역", "NNG", 5_000),
        component_entry("목", "NNG", 5_000),
        component_entry("역사", "NNG", -5_000),
        component_entry("사과", "NNG", 5_000),
        component_entry("과목", "NNG", -5_000),
        component_entry("역사과목", "NNG", -5_000),
        component_entry("을", "JKO", 0),
        component_entry("매", "NNG", 500),
        component_entry("일", "VCP", 500),
        component_entry("매일", "MAG", 0),
        component_entry("교사", "NNG", -5_000),
        component_entry("학생", "NNG", -5_000),
        component_entry("책", "NNG", -5_000),
        component_entry("학생일", "NNG+VCP+ETM", -5_000),
        component_entry("는", "JX", 0),
        component_entry("는관리", "NNG", -5_000),
        component_entry("자기", "NP", -5_000),
        component_entry("견해", "NNG", -5_000),
        component_entry("둘", "NR", -5_000),
        component_entry("다", "MAG", -5_000),
        component_entry("두", "MM", -5_000),
        component_entry("사람", "NNG", -5_000),
        component_entry("경계", "NNG", 0),
        component_entry("안", "NNG", 1_500),
        component_entry("밖", "NNG", 1_501),
        component_entry("경계안", "NNG", 0),
        component_entry("경계밖", "NNG", 0),
        component_entry("결과", "NNG", 0),
        component_entry("고체", "NNG", 0),
        component_entry("대학", "NNG", 0),
        component_entry("이", "VCP", 0),
        component_entry("다", "EF", 0),
        component_entry("긴", "ETN+JX", 0),
    ];
    let bytes = encode_component_resource([7; 32], &entries).unwrap();
    decode_component_resource("fixture", bytes, &[7; 32]).unwrap()
}

fn component_entry(surface: &str, pos: &str, word_cost: i32) -> MecabSourceMorphologyEntry {
    component_expression_entry(surface, pos, "*", word_cost)
}

fn component_expression_entry(
    surface: &str,
    pos: &str,
    expression: &str,
    word_cost: i32,
) -> MecabSourceMorphologyEntry {
    MecabSourceMorphologyEntry {
        surface: surface.to_owned(),
        pos: pos.to_owned(),
        left_id: 1,
        right_id: 1,
        word_cost,
        analysis_type: "*".to_owned(),
        start_pos: "*".to_owned(),
        end_pos: "*".to_owned(),
        expression: expression.to_owned(),
    }
}

fn atom(boundary: BoundaryPolicy, branches: Vec<CandidateProgram>) -> AtomPlan {
    AtomPlan {
        analyses: Vec::new(),
        programs: branches,
        boundary,
    }
}

fn exact_branch(anchor: &str, require_left: bool) -> CandidateProgram {
    let boundary = proof(require_left, true, anchor.chars().count() == 1);
    CandidateProgram {
        anchor: anchor.as_bytes().into(),
        consumption: CandidateConsumption::Anchor,
        core_mapping: CoreMapping::WholeAnchor,
        origins: vec![origin(0, &[])],
        decision: CandidateDecision::Boundary(boundary),
    }
}

fn nominal_branch(anchor: &str, allowed_rule_ids: Arc<[RuleId]>) -> CandidateProgram {
    let boundary = proof(true, true, anchor.chars().count() == 1);
    CandidateProgram {
        anchor: anchor.as_bytes().into(),
        consumption: CandidateConsumption::NominalParticleChain {
            initial_allowed_rule_ids: Arc::clone(&allowed_rule_ids),
            allowed_rule_ids,
            blocked_rule_ids: Arc::from([]),
        },
        core_mapping: CoreMapping::WholeAnchor,
        origins: vec![origin(0, &[])],
        decision: CandidateDecision::Boundary(boundary),
    }
}

fn predicate_branch(
    anchor: &str,
    core_len: usize,
    continuation: ContinuationState,
    allowed_rule_ids: Arc<[RuleId]>,
    origins: Vec<Origin>,
) -> CandidateProgram {
    let boundary = proof(false, true, anchor.chars().count() == 1);
    CandidateProgram {
        anchor: anchor.as_bytes().into(),
        consumption: CandidateConsumption::PredicateContinuation {
            continuation,
            pos: kfind_morph::PredicatePos::Verb,
            source_positions: kfind_morph::PredicatePosSet::one(kfind_morph::PredicatePos::Verb),
            allowed_rule_ids,
            nominal_particle_transition: false,
            left_context: CandidateLeftContext::Any,
        },
        core_mapping: CoreMapping::PrefixBytes(core_len),
        origins,
        decision: CandidateDecision::Boundary(boundary),
    }
}

fn mark_structural(program: &mut CandidateProgram) {
    program.decision = CandidateDecision::Structural(StructuralConstraint {
        patterns: Vec::new(),
        boundary: program.boundary(),
    });
}

fn materialize_structural_programs(atom: &mut AtomPlan) {
    let analyses = atom.analyses.clone();
    for program in &mut atom.programs {
        if program.decision.is_structural() {
            program.apply_structural_constraint(
                &analyses,
                kfind_morph::ComponentCapability::SourceAndRuntime,
            );
        }
    }
}

fn proof(require_left: bool, require_right: bool, one_scalar_anchor: bool) -> BoundaryProof {
    BoundaryProof {
        require_left,
        require_right,
        one_scalar_anchor,
    }
}

fn rules(values: &[&str]) -> Arc<[RuleId]> {
    let mut rules = values.iter().copied().map(RuleId::from).collect::<Vec<_>>();
    rules.sort();
    rules.into()
}

fn origin(analysis_index: u16, rule_path: &[&str]) -> Origin {
    Origin {
        analysis_index,
        rule_path: rule_path.iter().copied().map(RuleId::from).collect(),
    }
}
