use std::collections::BTreeSet;
use std::sync::Arc;

use kfind_data::{
    DataAlternation, DataFinePos, LexiconData, NominalRecord, PredicateRecord, collect_pos_entries,
    encode_pos_lexicon, parse_user_lexicon_toml,
};
use kfind_morph::{CoarsePos, FinePos, LexicalAlternation};
use kfind_query::{
    AnalysisSource, CompileOptions, LexiconQueryAnalyzer, Lexicons, Morphology, QueryAnalyzer,
    QueryAtom, QueryDiagnostic, compile_query,
};

fn atom(value: &str) -> QueryAtom {
    QueryAtom {
        raw: value.into(),
        forced_pos: None,
        quoted_literal: false,
    }
}

#[test]
fn embedded_lexicon_preserves_multiple_predicate_analyses() {
    let lexicons = Arc::new(Lexicons::embedded().unwrap());
    let analyzer = LexiconQueryAnalyzer::new(lexicons);
    let analyses = analyzer.analyze(&atom("걷다")).unwrap();

    assert_eq!(analyses.len(), 2);
    let alternations = analyses
        .iter()
        .filter_map(|analysis| match &analysis.morphology {
            Morphology::Predicate(predicate) => Some(predicate.alternation),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        alternations,
        BTreeSet::from([LexicalAlternation::Regular, LexicalAlternation::DToL])
    );
}

#[test]
fn full_pos_adds_homonymous_pos_without_replacing_core_entries() {
    let full_data = LexiconData {
        nominals: vec![NominalRecord {
            lemma: "새".to_owned(),
            pos: DataFinePos::Nng,
            flags: BTreeSet::new(),
            overrides: Vec::new(),
        }],
        ..LexiconData::default()
    };
    let binary = encode_pos_lexicon(&collect_pos_entries(&full_data)).unwrap();
    let lexicons = Arc::new(Lexicons::embedded_with(Some(&binary), None).unwrap());
    let analyzer = LexiconQueryAnalyzer::new(lexicons);
    let analyses = analyzer.analyze(&atom("새")).unwrap();

    assert!(
        analyses
            .iter()
            .any(|analysis| analysis.coarse_pos == CoarsePos::Noun
                && analysis.source == AnalysisSource::FullPosLexicon)
    );
    assert!(
        analyses
            .iter()
            .any(|analysis| analysis.coarse_pos == CoarsePos::Determiner)
    );
    let plan = compile_query("새", &CompileOptions::default(), &analyzer).unwrap();
    assert!(
        !plan
            .diagnostics
            .contains(&QueryDiagnostic::FullPosLexiconUnavailable)
    );
}

#[test]
fn full_pos_adds_regular_analysis_for_non_core_predicates() {
    let full_data = LexiconData {
        predicates: vec![PredicateRecord {
            lemma: "달리다".to_owned(),
            pos: DataFinePos::Vv,
            alternation: DataAlternation::Regular,
            flags: BTreeSet::new(),
            overrides: Vec::new(),
        }],
        ..LexiconData::default()
    };
    let binary = encode_pos_lexicon(&collect_pos_entries(&full_data)).unwrap();
    let lexicons = Arc::new(Lexicons::embedded_with(Some(&binary), None).unwrap());
    let analyzer = LexiconQueryAnalyzer::new(lexicons);
    let analyses = analyzer.analyze(&atom("달리다")).unwrap();

    assert!(analyses.iter().any(|analysis| {
        analysis.source == AnalysisSource::FullPosLexicon
            && matches!(
                &analysis.morphology,
                Morphology::Predicate(predicate)
                    if predicate.alternation == LexicalAlternation::Regular
            )
    }));
}

#[test]
fn full_pos_preserves_multiple_predicate_pos_candidates() {
    let full_data = LexiconData {
        predicates: vec![
            PredicateRecord {
                lemma: "나쁘다".to_owned(),
                pos: DataFinePos::Vv,
                alternation: DataAlternation::Regular,
                flags: BTreeSet::new(),
                overrides: Vec::new(),
            },
            PredicateRecord {
                lemma: "나쁘다".to_owned(),
                pos: DataFinePos::Va,
                alternation: DataAlternation::Regular,
                flags: BTreeSet::new(),
                overrides: Vec::new(),
            },
        ],
        ..LexiconData::default()
    };
    let binary = encode_pos_lexicon(&collect_pos_entries(&full_data)).unwrap();
    let analyzer = LexiconQueryAnalyzer::new(Arc::new(
        Lexicons::embedded_with(Some(&binary), None).unwrap(),
    ));
    let analyses = analyzer.analyze(&atom("나쁘다")).unwrap();

    assert_eq!(analyses.len(), 2);
    assert!(analyses.iter().all(|analysis| {
        analysis.source == AnalysisSource::FullPosLexicon
            && matches!(analysis.coarse_pos, CoarsePos::Verb | CoarsePos::Adjective)
    }));
}

#[test]
fn full_pos_preserves_productive_alternation_for_non_core_predicates() {
    let full_data = LexiconData {
        predicates: vec![PredicateRecord {
            lemma: "커스텀하다".to_owned(),
            pos: DataFinePos::Vv,
            alternation: DataAlternation::Regular,
            flags: BTreeSet::new(),
            overrides: Vec::new(),
        }],
        ..LexiconData::default()
    };
    let binary = encode_pos_lexicon(&collect_pos_entries(&full_data)).unwrap();
    let lexicons = Arc::new(Lexicons::embedded_with(Some(&binary), None).unwrap());
    let analyzer = LexiconQueryAnalyzer::new(lexicons);
    let analyses = analyzer.analyze(&atom("커스텀하다")).unwrap();

    assert!(analyses.iter().any(|analysis| {
        analysis.source == AnalysisSource::FullPosLexicon
            && matches!(
                &analysis.morphology,
                Morphology::Predicate(predicate)
                    if predicate.alternation == LexicalAlternation::Ha
            )
    }));
    let plan = compile_query("커스텀하다", &CompileOptions::default(), &analyzer).unwrap();
    let anchors = plan.atoms[0]
        .branches
        .iter()
        .map(|branch| String::from_utf8_lossy(&branch.anchor))
        .collect::<BTreeSet<_>>();
    assert!(anchors.contains("커스텀해"));
    assert!(!anchors.contains("커스텀핬"));
}

#[test]
fn core_predicate_analysis_suppresses_full_pos_homonyms() {
    let full_data = LexiconData {
        predicates: vec![PredicateRecord {
            lemma: "걷다".to_owned(),
            pos: DataFinePos::Va,
            alternation: DataAlternation::Regular,
            flags: BTreeSet::new(),
            overrides: Vec::new(),
        }],
        ..LexiconData::default()
    };
    let binary = encode_pos_lexicon(&collect_pos_entries(&full_data)).unwrap();
    let lexicons = Arc::new(Lexicons::embedded_with(Some(&binary), None).unwrap());
    let analyzer = LexiconQueryAnalyzer::new(lexicons);
    let analyses = analyzer.analyze(&atom("걷다")).unwrap();

    assert!(
        analyses
            .iter()
            .all(|analysis| analysis.source != AnalysisSource::FullPosLexicon)
    );
}

#[test]
fn user_replace_suppresses_lazy_full_pos_category() {
    let full_data = LexiconData {
        predicates: vec![PredicateRecord {
            lemma: "달리다".to_owned(),
            pos: DataFinePos::Vv,
            alternation: DataAlternation::Regular,
            flags: BTreeSet::new(),
            overrides: Vec::new(),
        }],
        ..LexiconData::default()
    };
    let binary = encode_pos_lexicon(&collect_pos_entries(&full_data)).unwrap();
    let mut lexicons = Lexicons::embedded_with(Some(&binary), None).unwrap();
    let user = parse_user_lexicon_toml(
        "user.toml",
        concat!(
            "[[predicate]]\n",
            "lemma = \"달리다\"\n",
            "pos = \"verb\"\n",
            "alternation = \"DToL\"\n",
            "replace = true\n",
        ),
        lexicons.rules(),
    )
    .unwrap();
    lexicons.merge_user(&user);
    let analyzer = LexiconQueryAnalyzer::new(Arc::new(lexicons));

    let analyses = analyzer.analyze(&atom("달리다")).unwrap();

    assert_eq!(analyses.len(), 1);
    assert_eq!(analyses[0].source, AnalysisSource::UserLexicon);
    assert!(matches!(
        &analyses[0].morphology,
        Morphology::Predicate(predicate)
            if predicate.alternation == LexicalAlternation::DToL
    ));
}

#[test]
fn user_nominal_replace_prevents_forced_full_pos_fallback_union() {
    let full_data = LexiconData {
        nominals: vec![NominalRecord {
            lemma: "명".to_owned(),
            pos: DataFinePos::Nng,
            flags: BTreeSet::new(),
            overrides: Vec::new(),
        }],
        ..LexiconData::default()
    };
    let binary = encode_pos_lexicon(&collect_pos_entries(&full_data)).unwrap();
    let mut lexicons = Lexicons::embedded_with(Some(&binary), None).unwrap();
    let user = parse_user_lexicon_toml(
        "user.toml",
        concat!(
            "[[nominal]]\n",
            "surface = \"명\"\n",
            "pos = \"dependent-noun\"\n",
            "replace = true\n",
        ),
        lexicons.rules(),
    )
    .unwrap();
    lexicons.merge_user(&user);
    let analyzer = LexiconQueryAnalyzer::new(Arc::new(lexicons));
    let mut query = atom("명");
    query.forced_pos = Some(CoarsePos::Noun);

    let analyses = analyzer.analyze(&query).unwrap();

    assert_eq!(analyses.len(), 1);
    assert_eq!(analyses[0].fine_pos, FinePos::DependentNoun);
    assert_eq!(analyses[0].source, AnalysisSource::Forced);
}

#[test]
fn user_append_preserves_lazy_full_pos_candidate() {
    let full_data = LexiconData {
        predicates: vec![PredicateRecord {
            lemma: "달리다".to_owned(),
            pos: DataFinePos::Vv,
            alternation: DataAlternation::Regular,
            flags: BTreeSet::new(),
            overrides: Vec::new(),
        }],
        ..LexiconData::default()
    };
    let binary = encode_pos_lexicon(&collect_pos_entries(&full_data)).unwrap();
    let mut lexicons = Lexicons::embedded_with(Some(&binary), None).unwrap();
    let user = parse_user_lexicon_toml(
        "user.toml",
        concat!(
            "[[predicate]]\n",
            "lemma = \"달리다\"\n",
            "pos = \"verb\"\n",
            "alternation = \"DToL\"\n",
        ),
        lexicons.rules(),
    )
    .unwrap();
    lexicons.merge_user(&user);
    let analyzer = LexiconQueryAnalyzer::new(Arc::new(lexicons));

    let analyses = analyzer.analyze(&atom("달리다")).unwrap();

    assert_eq!(analyses.len(), 2);
    assert_eq!(analyses[0].source, AnalysisSource::FullPosLexicon);
    assert_eq!(analyses[1].source, AnalysisSource::UserLexicon);
    assert!(analyses.iter().any(|analysis| {
        analysis.source == AnalysisSource::FullPosLexicon
            && matches!(
                &analysis.morphology,
                Morphology::Predicate(predicate)
                    if predicate.alternation == LexicalAlternation::Regular
            )
    }));
    assert!(analyses.iter().any(|analysis| {
        analysis.source == AnalysisSource::UserLexicon
            && matches!(
                &analysis.morphology,
                Morphology::Predicate(predicate)
                    if predicate.alternation == LexicalAlternation::DToL
            )
    }));
}

#[test]
fn user_replace_removes_only_the_matching_morphology_category() {
    let mut lexicons = Lexicons::embedded().unwrap();
    let user = parse_user_lexicon_toml(
        "user.toml",
        concat!(
            "[[predicate]]\n",
            "lemma = \"걷다\"\n",
            "pos = \"verb\"\n",
            "alternation = \"Regular\"\n",
            "replace = true\n",
        ),
        lexicons.rules(),
    )
    .unwrap();
    lexicons.merge_user(&user);
    let analyzer = LexiconQueryAnalyzer::new(Arc::new(lexicons));
    let analyses = analyzer.analyze(&atom("걷다")).unwrap();

    assert_eq!(analyses.len(), 1);
    assert_eq!(analyses[0].source, AnalysisSource::UserLexicon);
    assert!(matches!(
        &analyses[0].morphology,
        Morphology::Predicate(predicate)
            if predicate.alternation == LexicalAlternation::Regular
    ));
}

#[test]
fn productive_and_heuristic_fallbacks_follow_auto_priority() {
    let analyzer = LexiconQueryAnalyzer::new(Arc::new(Lexicons::embedded().unwrap()));

    let productive = analyzer.analyze(&atom("커스텀하다")).unwrap();
    assert_eq!(productive[0].source, AnalysisSource::ProductiveSuffix);
    assert!(matches!(
        &productive[0].morphology,
        Morphology::Predicate(predicate) if predicate.alternation == LexicalAlternation::Ha
    ));

    let unknown_da = analyzer.analyze(&atom("미등록다")).unwrap();
    assert_eq!(unknown_da.len(), 1);
    assert_eq!(unknown_da[0].coarse_pos, CoarsePos::Literal);

    let unknown_hangul = analyzer.analyze(&atom("미등록")).unwrap();
    assert_eq!(unknown_hangul.len(), 2);
    assert!(
        unknown_hangul
            .iter()
            .any(|analysis| analysis.coarse_pos == CoarsePos::Noun)
    );
    assert!(
        unknown_hangul
            .iter()
            .any(|analysis| analysis.coarse_pos == CoarsePos::Literal)
    );
}
