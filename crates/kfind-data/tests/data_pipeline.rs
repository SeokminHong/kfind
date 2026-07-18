use std::collections::{BTreeSet, HashSet};
use std::io::Cursor;
use std::path::{Path, PathBuf};

use kfind_data::{
    DICTIONARY_ADVERBIAL_I_RULE_ID, DICTIONARY_CONJUGATION_RULE_ID,
    DICTIONARY_RELATED_ADVERB_RULE_ID, DataAlternation, DataErrorKind, DataFinePos, DataWarning,
    LexiconSources, NominalRecord, ParticleHost, ParticleRuleRole, PosLexiconEntry, RuleSources,
    SurfaceOverride, collect_pos_entries, decode_pos_lexicon, encode_pos_lexicon,
    extract_mecab_ko_dic, extract_mecab_morphology, extract_mecab_source_morphology, load_data_dir,
    parse_lexicons, parse_mecab_connection_matrix, parse_predicates_tsv, parse_rule_set,
    parse_user_lexicon_toml, validate_data, validate_predicates,
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
        "particle.subject.honorific",
        "particle.similarity.gachi",
        "particle.conformance.daero",
        "particle.dative.deoreo",
        "particle.distributive.mada",
        "particle.extent.mankeum",
        "particle.exclusive.bakke",
        "particle.dative.bogo",
        "particle.capacity.roseo",
        "particle.instrument.rosseo",
        "particle.comparison.boda",
        "particle.restrictive.ppun",
        "particle.similarity.cheoreom",
        "particle.contrast.keonyeong",
        "particle.alternative.ina-na",
        "particle.concessive.inama-nama",
        "particle.concessive.irado-rado",
        "particle.comitative.irang-rang",
    ] {
        assert!(ids.contains(verifier_rule), "missing {verifier_rule}");
    }
    for (rule_id, next_id) in [
        ("particle.limit.ggaji", "particle.topic"),
        ("particle.limit.ggaji", "particle.additive"),
        ("particle.limit.ggaji", "particle.only"),
        ("particle.from", "particle.genitive"),
        ("particle.source", "particle.from"),
        ("particle.dative", "particle.direction"),
        ("particle.locative", "particle.genitive"),
        ("particle.capacity.roseo", "particle.additive"),
        ("particle.instrument.rosseo", "particle.topic"),
    ] {
        let rule = data
            .rules
            .particles
            .iter()
            .find(|rule| rule.id == rule_id)
            .unwrap_or_else(|| panic!("missing {rule_id}"));
        assert!(
            rule.next.iter().any(|next| next == next_id),
            "missing transition {rule_id} -> {next_id}"
        );
    }
    assert!(data.rules.particles.iter().all(|rule| {
        rule.forms
            .iter()
            .all(|form| !["까지도", "까지만", "까지는", "으로부터의"].contains(&form.as_str()))
    }));
    let nominal_topic_contraction = data
        .rules
        .contractions
        .iter()
        .find(|rule| rule.id == "contraction.nominal-topic-neun")
        .expect("nominal topic contraction rule");
    assert_eq!(nominal_topic_contraction.kind, "nominal-particle-compose");
    assert_eq!(nominal_topic_contraction.left, "거");
    assert_eq!(nominal_topic_contraction.right, "는");
    assert_eq!(nominal_topic_contraction.result, "건");
    assert!(nominal_topic_contraction.ending_ids.is_empty());
    for (id, left, result) in [
        ("contraction.pronoun-copula-nuga", "누구", "누군가"),
        ("contraction.pronoun-copula-mueo", "무어", "무언가"),
        ("contraction.pronoun-copula-mueot", "무엇", "무언가"),
    ] {
        let contraction = data
            .rules
            .contractions
            .iter()
            .find(|rule| rule.id == id)
            .unwrap_or_else(|| panic!("missing {id}"));
        assert_eq!(contraction.kind, "nominal-copula-ending-compose");
        assert_eq!(contraction.left, left);
        assert_eq!(contraction.right, "ㄴ가");
        assert_eq!(contraction.result, result);
        assert!(contraction.ending_ids.is_empty());
    }
    let alternative = data
        .rules
        .particles
        .iter()
        .find(|rule| rule.id == "particle.alternative.ina-na")
        .expect("alternative particle rule");
    assert_eq!(alternative.role, ParticleRuleRole::Auxiliary);
    assert!(alternative.hosts.contains(&ParticleHost::Adverb));
    let contrast = data
        .rules
        .particles
        .iter()
        .find(|rule| rule.id == "particle.contrast.keonyeong")
        .expect("contrast particle rule");
    assert!(!contrast.hosts.contains(&ParticleHost::Adverb));
    assert!(ids.contains("ending.honorific"));
    for continuation in [
        "ending.connective-jiman",
        "ending.connective-neunde",
        "ending.polite-yo",
        "ending.connective-do",
        "ending.connective-ya",
        "ending.imperative-ra",
        "ending.imperative-geora",
        "ending.imperative-neora",
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
    assert!(metadata.contains("normalized headwords and POS only"));
}

#[test]
fn repository_enriched_predicates_are_valid_and_disjoint_from_core() {
    let data = load_data_dir(data_root()).expect("repository data should validate");
    let (enriched, warnings) = parse_predicates_tsv(
        "data/enriched/predicates.tsv",
        &read("enriched/predicates.tsv"),
    )
    .unwrap();

    assert!(warnings.is_empty());
    validate_predicates("data/enriched/predicates.tsv", &enriched, &data.rules).unwrap();
    let core = data
        .lexicon
        .predicates
        .iter()
        .map(|entry| {
            (
                entry.lemma.as_str(),
                entry.pos,
                entry.alternation,
                &entry.flags,
            )
        })
        .collect::<BTreeSet<_>>();
    assert!(enriched.iter().all(|entry| {
        !core.contains(&(
            entry.lemma.as_str(),
            entry.pos,
            entry.alternation,
            &entry.flags,
        ))
    }));
    for (lemma, pos, alternation) in [
        ("깨닫다", DataFinePos::Vv, DataAlternation::DToL),
        ("결정짓다", DataFinePos::Vv, DataAlternation::DropS),
        ("가깝다", DataFinePos::Va, DataAlternation::BToWo),
        ("곱다", DataFinePos::Va, DataAlternation::BToWa),
        ("노랗다", DataFinePos::Va, DataAlternation::DropH),
    ] {
        assert!(enriched.iter().any(|entry| {
            entry.lemma == lemma && entry.pos == pos && entry.alternation == alternation
        }));
    }
    assert!(enriched.iter().any(|entry| {
        entry.lemma == "곱다"
            && entry.pos == DataFinePos::Va
            && entry.alternation == DataAlternation::Regular
    }));
    let surface_only = enriched
        .iter()
        .filter(|entry| entry.alternation == DataAlternation::SurfaceOnly)
        .collect::<Vec<_>>();
    assert_eq!(surface_only.len(), 295);
    assert!(read("enriched/predicates.tsv").len() <= 64 * 1024);
    assert_eq!(
        surface_only
            .iter()
            .filter(|entry| entry.overrides[0].rule_id == DICTIONARY_CONJUGATION_RULE_ID)
            .count(),
        130
    );
    assert_eq!(
        surface_only
            .iter()
            .filter(|entry| entry.overrides[0].rule_id == DICTIONARY_RELATED_ADVERB_RULE_ID)
            .count(),
        77
    );
    assert_eq!(
        surface_only
            .iter()
            .filter(|entry| entry.overrides[0].rule_id == DICTIONARY_ADVERBIAL_I_RULE_ID)
            .count(),
        88
    );
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
        "{}\n[[particle]]\nid = \"particle.subject\"\nrole = \"case\"\nhosts = [\"nominal\"]\nforms = [\"이\", \"가\"]\nselection = \"final-pair\"\nnext = []\nterminal = true\n",
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
fn nominal_particle_contraction_requires_a_nominal_host_particle() {
    let contractions = read("rules/contractions.toml").replacen(
        "right = \"는\"",
        "right = \"등록되지않은조사\"",
        1,
    );
    let error = parse_rule_set(RuleSources {
        endings: &read("rules/endings.toml"),
        alternations: &read("rules/alternations.toml"),
        contractions: &contractions,
        derivations: &read("rules/derivations.toml"),
        particles: &read("rules/particles.toml"),
    })
    .unwrap_err();

    assert!(matches!(
        *error.kind,
        DataErrorKind::InvalidValue { ref field, .. } if field == "right"
    ));
    assert!(error.location.line.is_some());
}

#[test]
fn particle_graph_rejects_cycles_without_rejecting_long_bounded_paths() {
    let particles = read("rules/particles.toml").replacen(
        concat!(
            "id = \"particle.additive\"\n",
            "role = \"auxiliary\"\n",
            "hosts = [\"nominal\", \"adverb\", \"predicate-ending\"]\n",
            "forms = [\"도\"]\n",
            "selection = \"literal\"\n",
            "next = []",
        ),
        concat!(
            "id = \"particle.additive\"\n",
            "role = \"auxiliary\"\n",
            "hosts = [\"nominal\", \"adverb\", \"predicate-ending\"]\n",
            "forms = [\"도\"]\n",
            "selection = \"literal\"\n",
            "next = [\"particle.limit.ggaji\"]",
        ),
        1,
    );
    let error = parse_rule_set(RuleSources {
        endings: &read("rules/endings.toml"),
        alternations: &read("rules/alternations.toml"),
        contractions: &read("rules/contractions.toml"),
        derivations: &read("rules/derivations.toml"),
        particles: &particles,
    })
    .unwrap_err();

    assert!(matches!(
        *error.kind,
        DataErrorKind::InvalidValue {
            ref field,
            ref reason,
            ..
        } if field == "continuation" && reason == "순환 전이는 허용하지 않습니다"
    ));
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
    let input = extraction.into_pos_lexicon();
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
fn mecab_extractor_preserves_predicate_pos_candidates() {
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
    assert_eq!(extraction.predicate_candidates, 2);

    let pos_lexicon = extraction.into_pos_lexicon();
    assert!(
        pos_lexicon
            .entries()
            .iter()
            .any(|entry| entry.lemma == "가다")
    );
    assert!(
        pos_lexicon
            .entries()
            .iter()
            .any(|entry| entry.lemma == "가까워다")
    );
    assert!(
        pos_lexicon
            .entries()
            .iter()
            .any(|entry| entry.lemma == "사용자")
    );
}

#[test]
fn mecab_extractor_rejects_contextual_copula_surfaces() {
    let csv = concat!(
        "이,1,1,1,VCP,*,F,이,*,*,*,*\n",
        "보이,1,1,1,VCP,*,F,보이,*,*,*,*\n",
        "아니,1,1,1,VCN,*,F,아니,*,*,*,*\n",
        "아닌,1,1,1,VCN,*,T,아닌,*,*,*,*\n",
    );
    let extraction = extract_mecab_ko_dic("copula.csv", Cursor::new(csv)).unwrap();

    assert_eq!(extraction.skipped_noncanonical_copula_rows, 2);
    assert_eq!(
        extraction.candidates(),
        &[
            PosLexiconEntry {
                lemma: "아니다".to_owned(),
                pos: DataFinePos::Vcn,
            },
            PosLexiconEntry {
                lemma: "이다".to_owned(),
                pos: DataFinePos::Vcp,
            },
        ]
    );
}

#[test]
fn mecab_morphology_extractor_preserves_context_and_surface_entries() {
    let csv = concat!(
        "이,11,12,-120,VCP,*,F,이,*,*,*,*\n",
        "보이,21,22,340,VCP,*,F,보이,Preanalysis,*,*,*\n",
        "걸어,31,32,450,VV,*,F,걷,Inflect,*,*,*\n",
        "기호,1,1,1,SY,*,F,기호,*,*,*,*\n",
    );
    let extraction = extract_mecab_morphology("morph.csv", Cursor::new(csv)).unwrap();

    assert_eq!(extraction.rows_read, 4);
    assert_eq!(extraction.skipped_unsupported_pos, 1);
    assert_eq!(extraction.entries().len(), 3);
    assert!(extraction.entries().iter().any(|entry| {
        entry.surface == "보이"
            && entry.pos == DataFinePos::Vcp
            && entry.left_id == 21
            && entry.right_id == 22
            && entry.word_cost == 340
    }));
    assert!(extraction.entries().iter().any(|entry| {
        entry.surface == "걸어"
            && entry.pos == DataFinePos::Vv
            && entry.left_id == 31
            && entry.right_id == 32
            && entry.word_cost == 450
    }));
}

#[test]
fn mecab_morphology_extractor_rejects_invalid_context_fields() {
    let csv = "사용자,left,1,1,NNG,*,T,사용자,*,*,*,*\n";
    let error = extract_mecab_morphology("morph.csv", Cursor::new(csv)).unwrap_err();

    assert!(matches!(
        *error.kind,
        DataErrorKind::InvalidValue { ref field, .. } if field == "left_id"
    ));
}

#[test]
fn mecab_source_morphology_preserves_all_pos_and_analysis_fields() {
    let csv = concat!(
        "다,3,5,2700,EF,*,F,다,*,*,*,*\n",
        "인,2240,10,894,VCP+ETM,*,T,인,Inflect,VCP,ETM,이/VCP/*+ᆫ/ETM/*\n",
        "기호,1,1,1,SY,*,F,기호,*,*,*,*\n",
    );
    let extraction = extract_mecab_source_morphology("source.csv", Cursor::new(csv)).unwrap();

    assert_eq!(extraction.rows_read, 3);
    assert_eq!(extraction.entries().len(), 3);
    assert!(
        extraction.entries().iter().any(|entry| {
            entry.surface == "다" && entry.pos == "EF" && entry.word_cost == 2700
        })
    );
    assert!(extraction.entries().iter().any(|entry| {
        entry.surface == "인"
            && entry.pos == "VCP+ETM"
            && entry.analysis_type == "Inflect"
            && entry.start_pos == "VCP"
            && entry.end_pos == "ETM"
            && entry.expression == "이/VCP/*+ᆫ/ETM/*"
    }));
    assert!(extraction.entries().iter().any(|entry| entry.pos == "SY"));
}

#[test]
fn mecab_connection_matrix_preserves_unordered_costs() {
    let matrix = "2 3\n1 2 -7\n0 0 1\n1 0 5\n0 2 3\n0 1 -2\n1 1 4\n";
    let parsed = parse_mecab_connection_matrix("matrix.def", Cursor::new(matrix)).unwrap();

    assert_eq!(parsed.right_contexts(), 2);
    assert_eq!(parsed.left_contexts(), 3);
    assert_eq!(parsed.connection_cost(0, 1), Some(-2));
    assert_eq!(parsed.connection_cost(1, 2), Some(-7));
    assert_eq!(parsed.connection_cost(2, 0), None);
}

#[test]
fn mecab_connection_matrix_rejects_missing_and_duplicate_costs() {
    let missing =
        parse_mecab_connection_matrix("matrix.def", Cursor::new("1 2\n0 0 1\n")).unwrap_err();
    assert!(matches!(
        *missing.kind,
        DataErrorKind::InvalidValue { ref field, .. } if field == "matrix_entries"
    ));

    let duplicate = parse_mecab_connection_matrix("matrix.def", Cursor::new("1 1\n0 0 1\n0 0 2\n"))
        .unwrap_err();
    assert!(matches!(
        *duplicate.kind,
        DataErrorKind::InvalidValue { ref field, .. } if field == "duplicate_context"
    ));
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
