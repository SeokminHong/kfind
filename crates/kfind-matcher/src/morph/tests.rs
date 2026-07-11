use grep_matcher::{LineMatchKind, LineTerminator, Matcher};
use kfind_morph::{ContinuationState, RuleId};
use kfind_query::{
    AtomPlan, BoundaryPolicy, BoundaryProof, BranchEnvironment, BranchVerifier, CoreMapping,
    Origin, PhrasePolicy, PlanLimits, QueryPlan, SurfaceBranch,
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
    branch.boundary = proof(false, false, false);
    let any = matcher(vec![atom(BoundaryPolicy::Any, vec![branch])], 24);
    let matched = any
        .find_at_with_meta("사용자권한은".as_bytes(), 0)
        .expect("any boundary should allow compound substrings");
    assert_eq!(matched.span.start, "사용자".len());
}

#[test]
fn overlapping_anchors_select_leftmost_longest_verified_token() {
    let mut short = exact_branch("가", false);
    short.boundary = proof(false, false, true);
    let mut long = exact_branch("가가", false);
    long.boundary = proof(false, false, false);
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
    short.boundary = proof(false, false, true);
    let mut long = exact_branch("가가", false);
    long.boundary = proof(false, false, false);
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
fn contracted_vowel_environment_checks_left_context_without_lemma_special_cases() {
    let mut branch = predicate_branch(
        "여서",
        "여".len(),
        ContinuationState::Terminal,
        rules(&[]),
        vec![origin(0, &["lexical.copula", "ending.aoeo-seo"])],
    );
    let BranchVerifier::Predicate { environment, .. } = &mut branch.verifier else {
        unreachable!("predicate branch helper returned another verifier")
    };
    *environment = BranchEnvironment::ContractedAfterVowel {
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
        particle_transitions: Arc::from([]),
        estimated_matcher_bytes: 0,
    }))
    .unwrap()
}

fn atom(boundary: BoundaryPolicy, branches: Vec<SurfaceBranch>) -> AtomPlan {
    AtomPlan {
        analyses: Vec::new(),
        branches,
        boundary,
    }
}

fn exact_branch(anchor: &str, require_left: bool) -> SurfaceBranch {
    SurfaceBranch {
        anchor: anchor.as_bytes().into(),
        verifier: BranchVerifier::Exact,
        core_mapping: CoreMapping::WholeAnchor,
        origins: vec![origin(0, &[])],
        boundary: proof(require_left, true, anchor.chars().count() == 1),
    }
}

fn nominal_branch(anchor: &str, allowed_rule_ids: Arc<[RuleId]>) -> SurfaceBranch {
    SurfaceBranch {
        anchor: anchor.as_bytes().into(),
        verifier: BranchVerifier::NominalParticles {
            allowed_rule_ids,
            blocked_rule_ids: Arc::from([]),
        },
        core_mapping: CoreMapping::WholeAnchor,
        origins: vec![origin(0, &[])],
        boundary: proof(true, true, anchor.chars().count() == 1),
    }
}

fn predicate_branch(
    anchor: &str,
    core_len: usize,
    continuation: ContinuationState,
    allowed_rule_ids: Arc<[RuleId]>,
    origins: Vec<Origin>,
) -> SurfaceBranch {
    SurfaceBranch {
        anchor: anchor.as_bytes().into(),
        verifier: BranchVerifier::Predicate {
            continuation,
            pos: kfind_morph::PredicatePos::Verb,
            allowed_rule_ids,
            environment: BranchEnvironment::Unrestricted,
        },
        core_mapping: CoreMapping::PrefixBytes(core_len),
        origins,
        boundary: proof(false, true, anchor.chars().count() == 1),
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
