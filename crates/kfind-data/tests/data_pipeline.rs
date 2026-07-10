use std::collections::{BTreeSet, HashSet};
use std::io::Cursor;
use std::path::{Path, PathBuf};

use kfind_data::{
    DataAlternation, DataErrorKind, DataFinePos, DataWarning, LexiconSources, NominalRecord,
    PosLexiconEntry, RuleSources, SurfaceOverride, collect_pos_entries, decode_pos_lexicon,
    encode_pos_lexicon, extract_mecab_ko_dic, load_data_dir, parse_lexicons, parse_predicates_tsv,
    parse_rule_set, parse_user_lexicon_toml, validate_data,
};

fn data_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data")
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(data_root().join(relative)).expect("repository data must be readable")
}

#[test]
fn repository_data_is_complete_and_valid() {
    let data = load_data_dir(data_root()).expect("repository data should validate");

    assert!(data.warnings.is_empty());
    assert!(data.fixtures.len() >= 50);
    assert!(
        data.lexicon
            .predicates
            .iter()
            .any(|entry| entry.lemma == "걷다" && entry.alternation == DataAlternation::DToL)
    );
    assert!(
        data.lexicon
            .predicates
            .iter()
            .any(|entry| entry.lemma == "걷다" && entry.alternation == DataAlternation::Regular)
    );

    let ids = data.rules.all_ids().collect::<HashSet<_>>();
    assert_eq!(ids.len(), data.rules.all_ids().count());
    assert_eq!(data.rules.max_continuation_depth, 4);
    assert!(ids.contains("particle.direction"));
    assert!(ids.contains("particle.plural"));
    assert!(ids.contains("ending.honorific"));

    let pos_entries = collect_pos_entries(&data.lexicon);
    assert!(
        pos_entries
            .iter()
            .any(|entry| { entry.lemma == "으로" && entry.pos == DataFinePos::Jkb })
    );
    assert!(decode_pos_lexicon(&encode_pos_lexicon(&pos_entries).unwrap()).is_ok());

    let metadata = read("SOURCES.toml");
    assert!(metadata.contains("mecab-ko-dic-2.1.1-20180720.tar.gz"));
    assert!(metadata.contains("fd62d3d6d8fa85145528065fabad4d7cb20f6b2201e71be4081a4e9701a5b330"));
    assert!(metadata.contains("Apache-2.0"));
}

#[test]
fn duplicate_tsv_rows_warn_and_are_deduplicated() {
    let source = concat!(
        "lemma\tpos\talternation\tflags\toverrides\n",
        "걷다\tVV\tDToL\t\t\n",
        "걷다\tVV\tDToL\t\t\n",
    );
    let (records, warnings) = parse_predicates_tsv("predicates.tsv", source).unwrap();

    assert_eq!(records.len(), 1);
    assert!(matches!(
        warnings.as_slice(),
        [DataWarning::DuplicateRow { first_line: 2, .. }]
    ));
}

#[test]
fn non_nfc_and_invalid_predicate_lemmas_are_rejected() {
    let non_nfc = "lemma\tpos\talternation\tflags\toverrides\n가다\tVV\tRegular\t\t\n";
    let error = parse_predicates_tsv("predicates.tsv", non_nfc).unwrap_err();
    assert!(matches!(*error.kind, DataErrorKind::NonNfc { .. }));

    let missing_da = "lemma\tpos\talternation\tflags\toverrides\n걷\tVV\tDToL\t\t\n";
    let error = parse_predicates_tsv("predicates.tsv", missing_da).unwrap_err();
    assert!(matches!(
        *error.kind,
        DataErrorKind::InvalidPredicateLemma(ref lemma) if lemma == "걷"
    ));
}

#[test]
fn cross_validation_rejects_unknown_rules_and_override_conflicts() {
    let valid = load_data_dir(data_root()).unwrap();
    let mut unknown = valid.lexicon.clone();
    unknown.particles[0].rule_id = "particle.missing".to_owned();
    let error = validate_data(
        unknown,
        valid.rules.clone(),
        valid.fixtures.clone(),
        Vec::new(),
    )
    .unwrap_err();
    assert!(matches!(
        *error.kind,
        DataErrorKind::UnknownRuleId(ref id) if id == "particle.missing"
    ));

    let mut conflicting = valid.lexicon.clone();
    conflicting.nominals.push(NominalRecord {
        lemma: "나".to_owned(),
        pos: DataFinePos::Np,
        flags: BTreeSet::new(),
        overrides: vec![SurfaceOverride {
            rule_id: "particle.subject".to_owned(),
            surface: "나가".to_owned(),
        }],
    });
    let error = validate_data(conflicting, valid.rules, valid.fixtures, Vec::new()).unwrap_err();
    assert!(matches!(
        *error.kind,
        DataErrorKind::OverrideConflict { .. }
    ));
}

#[test]
fn predicate_flags_must_be_declared_and_covered_by_fixtures() {
    let valid = load_data_dir(data_root()).unwrap();
    let mut undeclared = valid.lexicon.clone();
    undeclared.predicates[0].flags.insert("UNKNOWN".to_owned());
    let error = validate_data(
        undeclared,
        valid.rules.clone(),
        valid.fixtures.clone(),
        Vec::new(),
    )
    .unwrap_err();
    assert!(matches!(
        *error.kind,
        DataErrorKind::InvalidValue { ref field, .. } if field == "flags"
    ));

    let mut fixtures = valid.fixtures.clone();
    fixtures.retain(|case| case.feature != "eu-drop");
    let error = validate_data(valid.lexicon, valid.rules, fixtures, Vec::new()).unwrap_err();
    assert!(matches!(
        *error.kind,
        DataErrorKind::MissingFixtureCoverage(ref feature) if feature == "eu-drop"
    ));
}

#[test]
fn user_lexicon_toml_supports_replace_and_reports_semantic_lines() {
    let rules = load_data_dir(data_root()).unwrap().rules;
    let source = concat!(
        "[[predicate]]\n",
        "lemma = \"플러그인하다\"\n",
        "pos = \"verb\"\n",
        "alternation = \"Ha\"\n",
        "replace = true\n",
        "\n",
        "[[nominal]]\n",
        "surface = \"LLM\"\n",
    );
    let lexicon = parse_user_lexicon_toml("user.toml", source, &rules).unwrap();
    assert_eq!(lexicon.predicates[0].entry.lemma, "플러그인하다");
    assert!(lexicon.predicates[0].replace);
    assert_eq!(lexicon.nominals[0].entry.pos, DataFinePos::Nng);
    assert!(!lexicon.nominals[0].replace);

    let invalid = concat!(
        "[[predicate]]\n",
        "lemma = \"플러그인하\"\n",
        "pos = \"verb\"\n",
        "alternation = \"Ha\"\n",
    );
    let error = parse_user_lexicon_toml("user.toml", invalid, &rules).unwrap_err();
    assert_eq!(error.location.line, Some(2));
    assert!(matches!(
        *error.kind,
        DataErrorKind::InvalidPredicateLemma(_)
    ));
}

#[test]
fn rule_parser_rejects_duplicate_and_unknown_rule_ids() {
    let endings = read("rules/endings.toml");
    let alternations = read("rules/alternations.toml");
    let contractions = read("rules/contractions.toml");
    let derivations = read("rules/derivations.toml");
    let particles = format!(
        "{}\n[[particle]]\nid = \"particle.subject\"\nforms = [\"이\", \"가\"]\nselection = \"final-pair\"\nnext = []\nterminal = true\n",
        read("rules/particles.toml")
    );
    let error = parse_rule_set(RuleSources {
        endings: &endings,
        alternations: &alternations,
        contractions: &contractions,
        derivations: &derivations,
        particles: &particles,
    })
    .unwrap_err();
    assert!(matches!(
        *error.kind,
        DataErrorKind::DuplicateRuleId(ref id) if id == "particle.subject"
    ));

    let endings = format!(
        "{}\n[[ending]]\nid = \"ending.extra\"\ncategory = \"final\"\ninitial = \"consonant\"\nforms = [\"요\"]\nnext = [\"ending.missing\"]\nterminal = true\n",
        read("rules/endings.toml")
    );
    let particles = read("rules/particles.toml");
    let error = parse_rule_set(RuleSources {
        endings: &endings,
        alternations: &alternations,
        contractions: &contractions,
        derivations: &derivations,
        particles: &particles,
    })
    .unwrap_err();
    assert!(matches!(
        *error.kind,
        DataErrorKind::UnknownRuleId(ref id) if id == "ending.missing"
    ));
}

#[test]
fn compact_pos_binary_round_trips_sorted_deduplicated_entries() {
    let input = vec![
        PosLexiconEntry {
            lemma: "사용자".to_owned(),
            pos: DataFinePos::Nng,
        },
        PosLexiconEntry {
            lemma: "걷다".to_owned(),
            pos: DataFinePos::Vv,
        },
        PosLexiconEntry {
            lemma: "걷다".to_owned(),
            pos: DataFinePos::Vv,
        },
        PosLexiconEntry {
            lemma: "걷다".to_owned(),
            pos: DataFinePos::Nng,
        },
    ];

    let encoded = encode_pos_lexicon(&input).unwrap();
    let decoded = decode_pos_lexicon(&encoded).unwrap();
    assert_eq!(decoded.entries().len(), 3);
    assert_eq!(decoded.entries()[0].lemma, "걷다");
    assert_eq!(decoded.lookup("걷다").len(), 2);
    assert!(decoded.lookup("없다").is_empty());

    let mut corrupt = encoded;
    corrupt.push(0);
    assert!(matches!(
        *decode_pos_lexicon(&corrupt).unwrap_err().kind,
        DataErrorKind::Binary(_)
    ));
}

#[test]
fn mecab_extractor_marks_predicates_for_gold_filtering() {
    let csv = concat!(
        "가,1,1,1,VV,*,F,가,*,*,*,*\n",
        "가까워,1,1,1,VV,*,F,가까워,*,*,*,*\n",
        "활용,1,1,1,VV,*,F,활용,Inflect,*,*,*\n",
        "사용자,1,1,1,NNG,*,T,사용자,*,*,*,*\n",
        "기호,1,1,1,SY,*,F,기호,*,*,*,*\n",
    );
    let extraction = extract_mecab_ko_dic("VV.csv", Cursor::new(csv)).unwrap();

    assert!(extraction.candidates().contains(&PosLexiconEntry {
        lemma: "가다".to_owned(),
        pos: DataFinePos::Vv,
    }));
    assert!(extraction.candidates().contains(&PosLexiconEntry {
        lemma: "가까워다".to_owned(),
        pos: DataFinePos::Vv,
    }));
    assert_eq!(extraction.skipped_analysis_rows, 1);
    assert_eq!(extraction.skipped_unsupported_pos, 1);
    assert_eq!(extraction.predicate_candidates_requiring_gold, 2);

    let approved = BTreeSet::from([PosLexiconEntry {
        lemma: "가다".to_owned(),
        pos: DataFinePos::Vv,
    }]);
    let filtered = extraction.retain_gold_approved_predicates(&approved);
    assert!(filtered.iter().any(|entry| entry.lemma == "가다"));
    assert!(!filtered.iter().any(|entry| entry.lemma == "가까워다"));
    assert!(filtered.iter().any(|entry| entry.lemma == "사용자"));
}

#[test]
fn individual_lexicon_sources_parse_with_repository_schema() {
    let (lexicon, warnings) = parse_lexicons(LexiconSources {
        predicates: &read("lexicon/predicates.tsv"),
        nominals: &read("lexicon/nominals.tsv"),
        modifiers: &read("lexicon/modifiers.tsv"),
        particles: &read("lexicon/particles.tsv"),
    })
    .unwrap();
    assert!(warnings.is_empty());
    assert!(!lexicon.predicates.is_empty());
    assert!(!lexicon.particles.is_empty());
}
