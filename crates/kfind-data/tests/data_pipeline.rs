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
    assert!(data.fixtures.len() >= 300);
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
    for verifier_rule in [
        "particle.source.egeseo",
        "particle.source.hanteseo",
        "particle.limit.ggaji",
        "particle.even.jocha",
        "particle.even.majeo",
    ] {
        assert!(ids.contains(verifier_rule), "missing {verifier_rule}");
    }
    assert!(ids.contains("ending.honorific"));
    for continuation in [
        "ending.connective-jiman",
        "ending.connective-neunde",
        "ending.polite-yo",
        "ending.connective-do",
        "ending.connective-ya",
        "ending.imperative-ra",
    ] {
        assert!(ids.contains(continuation), "missing {continuation}");
    }
    let nominalizer = data
        .rules
        .endings
        .iter()
        .find(|rule| rule.id == "ending.nominalizer")
        .unwrap();
    assert_eq!(nominalizer.initial, kfind_data::EndingInitial::AttachMieum);

    let pos_entries = collect_pos_entries(&data.lexicon);
    assert!(
        pos_entries
            .entries()
            .iter()
            .any(|entry| { entry.lemma == "으로" && entry.pos == DataFinePos::Jkb })
    );
    assert_eq!(data.lexicon.predicate_analyses("걷다").count(), 2);
    assert_eq!(
        pos_entries
            .entries()
            .iter()
            .filter(|entry| entry.lemma == "걷다" && entry.pos == DataFinePos::Vv)
            .count(),
        1
    );
    assert!(decode_pos_lexicon(&encode_pos_lexicon(&pos_entries).unwrap()).is_ok());

    let metadata = read("SOURCES.toml");
    assert!(metadata.contains("mecab-ko-dic-2.1.1-20180720.tar.gz"));
    assert!(metadata.contains("fd62d3d6d8fa85145528065fabad4d7cb20f6b2201e71be4081a4e9701a5b330"));
    assert!(metadata.contains("Apache-2.0"));
    assert!(metadata.contains("--bin kfind-data-extract-mecab"));
    assert!(metadata.contains("data/lexicon/predicates.tsv"));
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

    let invalid_second_record = concat!(
        "[[predicate]]\n",
        "lemma = \"걷다\"\n",
        "pos = \"verb\"\n",
        "alternation = \"Regular\"\n",
        "\n",
        "[[predicate]]\n",
        "lemma = \"듣다\"\n",
        "pos = \"not-a-predicate\"\n",
        "alternation = \"DToL\"\n",
    );
    let error = parse_user_lexicon_toml("user.toml", invalid_second_record, &rules).unwrap_err();
    assert_eq!(error.location.line, Some(8));
    assert!(matches!(
        *error.kind,
        DataErrorKind::InvalidValue { ref field, .. } if field == "pos"
    ));
}

#[test]
fn user_lexicon_append_and_replace_preserve_duplicate_lemma_analyses() {
    let valid = load_data_dir(data_root()).unwrap();
    let append_source = concat!(
        "[[predicate]]\n",
        "lemma = \"걷다\"\n",
        "pos = \"verb\"\n",
        "alternation = \"Regular\"\n",
        "\n",
        "[[predicate]]\n",
        "lemma = \"걷다\"\n",
        "pos = \"verb\"\n",
        "alternation = \"DToL\"\n",
    );
    let user = parse_user_lexicon_toml("user.toml", append_source, &valid.rules).unwrap();
    let mut appended = valid.lexicon.clone();
    appended.apply_user_lexicon(user);
    assert_eq!(appended.predicate_analyses("걷다").count(), 4);

    let replace_source = append_source.replacen(
        "alternation = \"Regular\"",
        "alternation = \"Regular\"\nreplace = true",
        1,
    );
    let user = parse_user_lexicon_toml("user.toml", &replace_source, &valid.rules).unwrap();
    let mut replaced = valid.lexicon;
    replaced.apply_user_lexicon(user);
    let analyses = replaced.predicate_analyses("걷다").collect::<Vec<_>>();
    assert_eq!(analyses.len(), 2);
    assert!(
        analyses
            .iter()
            .any(|entry| entry.alternation == DataAlternation::Regular)
    );
    assert!(
        analyses
            .iter()
            .any(|entry| entry.alternation == DataAlternation::DToL)
    );
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
    assert!(error.location.line.is_some());
}

#[test]
fn rule_parser_rejects_unknown_features_and_nonterminal_leaves() {
    let endings = read("rules/endings.toml").replacen(
        "required = []",
        "required = [\"not-a-morphology-feature\"]",
        1,
    );
    let parse = |endings: &str| {
        parse_rule_set(RuleSources {
            endings,
            alternations: &read("rules/alternations.toml"),
            contractions: &read("rules/contractions.toml"),
            derivations: &read("rules/derivations.toml"),
            particles: &read("rules/particles.toml"),
        })
    };
    let error = parse(&endings).unwrap_err();
    assert!(matches!(
        *error.kind,
        DataErrorKind::InvalidValue { ref field, .. } if field == "required"
    ));
    assert!(error.location.line.is_some());

    let endings = format!(
        "{}\n[[ending]]\nid = \"ending.nonterminal-leaf\"\ncategory = \"final\"\ninitial = \"other\"\nforms = [\"테스트\"]\nrequired = []\nforbidden = []\nnext = []\nterminal = false\n",
        read("rules/endings.toml")
    );
    let error = parse(&endings).unwrap_err();
    assert!(matches!(
        *error.kind,
        DataErrorKind::InvalidValue { ref field, .. } if field == "terminal"
    ));
    assert!(error.location.line.is_some());
}

#[test]
fn particle_lexicon_variants_must_match_their_rule_forms() {
    let valid = load_data_dir(data_root()).unwrap();
    let mut lexicon = valid.lexicon.clone();
    let subject = lexicon
        .particles
        .iter_mut()
        .find(|record| record.rule_id == "particle.subject")
        .unwrap();
    subject.variants = vec!["이".to_owned()];

    let error = validate_data(lexicon, valid.rules, valid.fixtures, Vec::new()).unwrap_err();
    assert!(matches!(
        *error.kind,
        DataErrorKind::InvalidValue { ref field, .. } if field == "variants"
    ));
}

#[test]
fn compact_pos_binary_round_trips_sorted_deduplicated_entries() {
    let csv = concat!(
        "사용자,1,1,1,NNG,*,T,사용자,*,*,*,*\n",
        "걷,1,1,1,VV,*,T,걷,*,*,*,*\n",
        "걷,1,1,1,VV,*,T,걷,*,*,*,*\n",
        "걷다,1,1,1,NNG,*,T,걷다,*,*,*,*\n",
    );
    let extraction = extract_mecab_ko_dic("test.csv", Cursor::new(csv)).unwrap();
    let approved = BTreeSet::from([PosLexiconEntry {
        lemma: "걷다".to_owned(),
        pos: DataFinePos::Vv,
    }]);
    let input = extraction.approve_predicates(&approved).into_pos_lexicon();
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
    let filtered = extraction.approve_predicates(&approved);
    let filtered = filtered.pos_lexicon().entries();
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
