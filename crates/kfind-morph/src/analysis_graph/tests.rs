use std::io::Cursor;

use kfind_data::{
    MecabSourceMorphologyEntry, decode_morphology_graph_resource, encode_morphology_graph_resource,
    parse_mecab_connection_matrix,
};

use super::*;

#[test]
fn whole_source_analysis_proves_the_query_without_using_costs() {
    let resolver = resolver(&[atomic("학교", "NNG", -9_999)]);
    let pattern = pattern("학교", DataFinePos::Nng, false);
    let resolution = resolver.resolve(
        "학교",
        0.."학교".len(),
        0.."학교".len(),
        &pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.verdict, ConstraintVerdict::Proven);
    assert_eq!(resolution.proof.known_node_count, 1);
    assert_eq!(resolution.proof.unknown_node_count, 0);
    assert_eq!(
        resolution.proof.paths[0].evidence,
        ConstraintEvidenceKind::SourceWhole
    );
}

#[test]
fn source_component_exposure_remains_an_explicit_profile_decision() {
    let resolver = resolver(&[entry(
        "대학교",
        "NNG",
        "Compound",
        "NNG",
        "NNG",
        "대/NNG/*+학교/NNG/*",
        0,
    )]);
    let hidden = pattern("학교", DataFinePos::Nng, false);
    let exposed = pattern("학교", DataFinePos::Nng, true);
    let resolution = resolver.resolve(
        "대학교",
        "대".len().."대학교".len(),
        "대".len().."대학교".len(),
        &hidden,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(
        resolution.verdict,
        ConstraintVerdict::Ambiguous(ConstraintAmbiguity::CompoundExposure)
    );
    assert_eq!(
        resolution.verdict_for(
            CompoundExposureProfile::Opaque,
            std::slice::from_ref(&hidden)
        ),
        ConstraintVerdict::Contradicted
    );
    assert_eq!(
        resolution.verdict_for(
            CompoundExposureProfile::Transparent,
            std::slice::from_ref(&hidden)
        ),
        ConstraintVerdict::Proven
    );
    assert_eq!(
        resolution.verdict_for(
            CompoundExposureProfile::Explicit,
            std::slice::from_ref(&hidden)
        ),
        ConstraintVerdict::Contradicted
    );
    assert_eq!(
        resolution.verdict_for(
            CompoundExposureProfile::Explicit,
            std::slice::from_ref(&exposed)
        ),
        ConstraintVerdict::Proven
    );
}

#[test]
fn strict_runtime_composition_remains_a_component_exposure_decision() {
    let resolver = resolver(&[atomic("산", "NNG", 8_000), atomic("속", "NNG", -8_000)]);
    let pattern = pattern("속", DataFinePos::Nng, false);
    let resolution = resolver.resolve(
        "산속",
        "산".len().."산속".len(),
        "산".len().."산속".len(),
        &pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(
        resolution.verdict,
        ConstraintVerdict::Ambiguous(ConstraintAmbiguity::CompoundExposure)
    );
    assert_eq!(
        resolution.proof.paths[0].evidence,
        ConstraintEvidenceKind::RuntimeComposed
    );
}

#[test]
fn unrelated_source_whole_analysis_does_not_hide_runtime_exposure() {
    let whole_preferred = resolver(&[
        atomic("산", "NNG", 8_000),
        atomic("속", "NNG", -8_000),
        atomic("산속", "NNG", -30_000),
    ]);
    let runtime_preferred = resolver(&[
        atomic("산", "NNG", -30_000),
        atomic("속", "NNG", -30_000),
        atomic("산속", "NNG", 30_000),
    ]);
    let pattern = pattern("속", DataFinePos::Nng, false);
    let resolution = whole_preferred.resolve(
        "산속",
        "산".len().."산속".len(),
        "산".len().."산속".len(),
        &pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );
    let reversed = runtime_preferred.resolve(
        "산속",
        "산".len().."산속".len(),
        "산".len().."산속".len(),
        &pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(
        resolution.verdict,
        ConstraintVerdict::Ambiguous(ConstraintAmbiguity::CompoundExposure)
    );
    assert_eq!(reversed.verdict, resolution.verdict);
    assert!(
        resolution
            .proof
            .paths
            .iter()
            .any(|path| path.evidence == ConstraintEvidenceKind::RuntimeComposed)
    );
    assert!(
        resolution
            .proof
            .paths
            .iter()
            .any(|path| path.evidence == ConstraintEvidenceKind::Contradiction)
    );
}

#[test]
fn a_matching_same_grade_source_analysis_is_satisfiable_regardless_of_cost() {
    let first = resolver(&[
        atomic("매일", "MAG", -30_000),
        atomic("매일", "NNG", 30_000),
    ]);
    let reversed = resolver(&[
        atomic("매일", "MAG", 30_000),
        atomic("매일", "NNG", -30_000),
    ]);
    let pattern = pattern("매일", DataFinePos::Mag, false);
    let resolve = |resolver: &ConstraintResolver| {
        resolver.resolve(
            "매일",
            0.."매일".len(),
            0.."매일".len(),
            &pattern,
            DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
        )
    };

    assert_eq!(resolve(&first).verdict, ConstraintVerdict::Proven);
    assert_eq!(resolve(&reversed).verdict, resolve(&first).verdict);
}

#[test]
fn pattern_union_does_not_treat_sibling_pos_candidates_as_contradictions() {
    let resolver = resolver(&[atomic("아니", "VCN", 0)]);
    let patterns = QueryMorphPattern::from_fine_pos(FinePos::Adjective, "아니");
    let resolution = resolver.resolve_patterns(
        "아니",
        0.."아니".len(),
        0.."아니".len(),
        &patterns,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.verdict, ConstraintVerdict::Proven);
}

#[test]
fn lexical_identity_is_required_in_addition_to_pos() {
    let resolver = resolver(&[atomic("매일", "MAG", 0)]);
    let pattern = pattern("내일", DataFinePos::Mag, false);
    let resolution = resolver.resolve(
        "매일",
        0.."매일".len(),
        0.."매일".len(),
        &pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.verdict, ConstraintVerdict::Contradicted);
}

#[test]
fn unprojectable_expression_is_ambiguous_instead_of_inventing_a_span() {
    let resolver = resolver(&[entry(
        "갔다",
        "VV+EP+EF",
        "Inflect",
        "VV",
        "EF",
        "가/VV/*+었/EP/*+다/EF/*",
        0,
    )]);
    let pattern = pattern("가", DataFinePos::Vv, false);
    let resolution = resolver.resolve(
        "갔다",
        0.."갔".len(),
        0.."갔".len(),
        &pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(
        resolution.verdict,
        ConstraintVerdict::Ambiguous(ConstraintAmbiguity::OpaqueExpression)
    );
    assert_eq!(
        resolution.proof.paths[0].evidence,
        ConstraintEvidenceKind::OpaqueExpression
    );
}

#[test]
fn opaque_inflection_supports_a_candidate_that_consumes_the_whole_token() {
    let resolver = resolver(&[entry(
        "갔다",
        "VV+EP+EF",
        "Inflect",
        "VV",
        "EF",
        "가/VV/*+었/EP/*+다/EF/*",
        0,
    )]);
    let pattern = pattern("가", DataFinePos::Vv, false);
    let resolution = resolver.resolve(
        "갔다",
        0.."갔".len(),
        0.."갔다".len(),
        &pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );

    assert_eq!(resolution.verdict, ConstraintVerdict::Proven);
}

#[test]
fn unknown_paths_are_used_only_when_no_known_complete_path_exists() {
    let resolver = resolver(&[atomic("학교", "NNG", 0)]);
    let unknown_pattern = pattern("미", DataFinePos::Nng, false);
    let unknown = resolver.resolve(
        "미등록",
        0.."미".len(),
        0.."미".len(),
        &unknown_pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );
    assert_eq!(
        unknown.verdict,
        ConstraintVerdict::Unavailable(ConstraintUnavailable::UnknownOnly)
    );
    assert!(unknown.proof.unknown_node_count > 0);
    assert!(
        unknown
            .proof
            .paths
            .iter()
            .all(|path| path.evidence == ConstraintEvidenceKind::Unknown)
    );

    let known_pattern = pattern("학교", DataFinePos::Nng, false);
    let known = resolver.resolve(
        "학교",
        0.."학교".len(),
        0.."학교".len(),
        &known_pattern,
        DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT,
    );
    assert_eq!(known.verdict, ConstraintVerdict::Proven);
    assert_eq!(known.proof.unknown_node_count, 0);
}

#[test]
fn invalid_spans_and_node_limits_are_observable() {
    let resolver = resolver(&[atomic("산", "NNG", 0), atomic("산속", "NNG", 0)]);
    let invalid = pattern("산", DataFinePos::Nng, false);
    assert_eq!(
        resolver
            .resolve(
                "산속",
                1..2,
                1..2,
                &invalid,
                DEFAULT_ANALYSIS_GRAPH_NODE_LIMIT
            )
            .verdict,
        ConstraintVerdict::Unavailable(ConstraintUnavailable::InvalidPattern)
    );

    let valid = pattern("산", DataFinePos::Nng, false);
    assert!(matches!(
        resolver
            .resolve("산속", 0.."산".len(), 0.."산".len(), &valid, 1)
            .verdict,
        ConstraintVerdict::Unavailable(ConstraintUnavailable::NodeLimit { limit: 1, .. })
    ));
}

#[test]
fn adjective_patterns_preserve_the_negative_copula_candidate() {
    assert_eq!(
        QueryMorphPattern::from_fine_pos(FinePos::Adjective, "아니")
            .into_iter()
            .map(|pattern| pattern.fine_pos)
            .collect::<Vec<_>>(),
        [DataFinePos::Va, DataFinePos::Vcn]
    );
}

fn pattern(
    lexical_form: &str,
    fine_pos: DataFinePos,
    expose_source_components: bool,
) -> QueryMorphPattern {
    QueryMorphPattern {
        fine_pos,
        lexical_form: Arc::from(lexical_form),
        expose_source_components,
    }
}

fn resolver(entries: &[MecabSourceMorphologyEntry]) -> ConstraintResolver {
    let matrix = parse_mecab_connection_matrix(
        "matrix.def",
        Cursor::new("2 2\n0 0 0\n0 1 0\n1 0 0\n1 1 0\n"),
    )
    .unwrap();
    let bytes = encode_morphology_graph_resource(
        [5; 32],
        entries,
        &matrix,
        b"DEFAULT 0 1 0\nHANGUL 1 1 8\n0xAC00..0xD7A3 HANGUL\n",
        b"DEFAULT,1,1,100,SY,*,*,*,*,*,*,*\nHANGUL,1,1,100,UNKNOWN,*,*,*,*,*,*,*\n",
    )
    .unwrap();
    let resource = decode_morphology_graph_resource("fixture", bytes, &[5; 32]).unwrap();
    ConstraintResolver::new(Arc::new(resource))
}

fn atomic(surface: &str, pos: &str, word_cost: i32) -> MecabSourceMorphologyEntry {
    entry(surface, pos, "*", "*", "*", "*", word_cost)
}

fn entry(
    surface: &str,
    pos: &str,
    analysis_type: &str,
    start_pos: &str,
    end_pos: &str,
    expression: &str,
    word_cost: i32,
) -> MecabSourceMorphologyEntry {
    MecabSourceMorphologyEntry {
        surface: surface.to_owned(),
        pos: pos.to_owned(),
        left_id: 1,
        right_id: 1,
        word_cost,
        analysis_type: analysis_type.to_owned(),
        start_pos: start_pos.to_owned(),
        end_pos: end_pos.to_owned(),
        expression: expression.to_owned(),
    }
}
