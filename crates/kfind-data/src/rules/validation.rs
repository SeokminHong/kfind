use std::collections::{BTreeMap, BTreeSet};

use crate::validation::{require_nfc, require_rule_id};
use crate::{DataError, DataErrorKind};

use super::{ParticleHost, ParticleRuleRole, ParticleSelection, RuleLocations, RuleSet};

pub(super) fn validate_rules(rules: &RuleSet, locations: &RuleLocations) -> Result<(), DataError> {
    let mut ids = BTreeMap::<&str, &str>::new();
    for (source, id) in rules
        .endings
        .iter()
        .map(|rule| ("data/rules/endings.toml", rule.id.as_str()))
        .chain(
            rules
                .alternations
                .iter()
                .map(|rule| ("data/rules/alternations.toml", rule.id.as_str())),
        )
        .chain(
            rules
                .contractions
                .iter()
                .map(|rule| ("data/rules/contractions.toml", rule.id.as_str())),
        )
        .chain(
            rules
                .derivations
                .iter()
                .map(|rule| ("data/rules/derivations.toml", rule.id.as_str())),
        )
        .chain(
            rules
                .particles
                .iter()
                .map(|rule| ("data/rules/particles.toml", rule.id.as_str())),
        )
    {
        require_rule_id(source, id).map_err(|mut error| {
            error.location = locations.get(source, id);
            error
        })?;
        if ids.insert(id, source).is_some() {
            return Err(DataError::new(
                locations.get(source, id),
                DataErrorKind::DuplicateRuleId(id.to_owned()),
            ));
        }
    }

    validate_endings(rules, locations)?;
    validate_alternations(rules, locations)?;
    validate_contractions(rules, locations)?;
    validate_derivations(rules, locations)?;
    validate_particles(rules, locations)?;
    Ok(())
}

fn validate_endings(rules: &RuleSet, locations: &RuleLocations) -> Result<(), DataError> {
    let source = "data/rules/endings.toml";
    let ending_ids = rules
        .endings
        .iter()
        .map(|rule| rule.id.as_str())
        .collect::<BTreeSet<_>>();
    for rule in &rules.endings {
        validate_forms(source, &rule.id, &rule.forms, locations)?;
        validate_features(source, &rule.id, &rule.required, &rule.forbidden, locations)?;
        let required = rule.required.iter().collect::<BTreeSet<_>>();
        if let Some(conflict) = rule.forbidden.iter().find(|item| required.contains(item)) {
            return Err(invalid_rule_value(
                locations,
                source,
                &rule.id,
                "forbidden",
                conflict,
                "required feature와 겹칩니다",
            ));
        }
        for next in &rule.next {
            require_reference(source, &rule.id, next, &ending_ids, locations)?;
        }
        validate_terminal_transition(source, &rule.id, rule.terminal, &rule.next, locations)?;
    }
    validate_graph(
        source,
        Some(rules.max_continuation_depth),
        rules
            .endings
            .iter()
            .map(|rule| (rule.id.as_str(), rule.next.as_slice())),
        locations,
    )
}

fn validate_alternations(rules: &RuleSet, locations: &RuleLocations) -> Result<(), DataError> {
    let source = "data/rules/alternations.toml";
    let ending_ids = rules
        .endings
        .iter()
        .map(|rule| rule.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut kinds = BTreeSet::new();
    for rule in &rules.alternations {
        if !kinds.insert(rule.kind) {
            return Err(invalid_rule_value(
                locations,
                source,
                &rule.id,
                "kind",
                rule.kind.as_str(),
                "alternation kind는 하나의 규범 규칙만 가져야 합니다",
            ));
        }
        for ending_id in &rule.ending_ids {
            require_reference(source, &rule.id, ending_id, &ending_ids, locations)?;
        }
    }
    Ok(())
}

fn validate_contractions(rules: &RuleSet, locations: &RuleLocations) -> Result<(), DataError> {
    let source = "data/rules/contractions.toml";
    const KINDS: &[&str] = &[
        "vowel-compose",
        "stem-rewrite",
        "nominal-particle-compose",
        "nominal-copula-ending-compose",
        "copula-host-ending-compose",
    ];
    let ending_ids = rules
        .endings
        .iter()
        .map(|rule| rule.id.as_str())
        .collect::<BTreeSet<_>>();
    let nominal_particle_forms = rules
        .particles
        .iter()
        .filter(|rule| rule.hosts.contains(&ParticleHost::Nominal))
        .flat_map(|rule| rule.forms.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    for rule in &rules.contractions {
        if !KINDS.contains(&rule.kind.as_str()) {
            return Err(invalid_rule_value(
                locations,
                source,
                &rule.id,
                "kind",
                &rule.kind,
                "지원하는 contraction kind여야 합니다",
            ));
        }
        for (field, value) in [
            ("kind", rule.kind.as_str()),
            ("left", rule.left.as_str()),
            ("right", rule.right.as_str()),
            ("result", rule.result.as_str()),
        ] {
            require_nfc(source, locations.get(source, &rule.id).line, field, value)?;
            if value.is_empty() {
                return Err(invalid_rule_value(
                    locations,
                    source,
                    &rule.id,
                    field,
                    value,
                    "비어 있습니다",
                ));
            }
        }
        for ending_id in &rule.ending_ids {
            require_reference(source, &rule.id, ending_id, &ending_ids, locations)?;
        }
        if matches!(
            rule.kind.as_str(),
            "nominal-particle-compose"
                | "nominal-copula-ending-compose"
                | "copula-host-ending-compose"
        ) && !rule.ending_ids.is_empty()
        {
            return Err(invalid_rule_value(
                locations,
                source,
                &rule.id,
                "ending_ids",
                rule.ending_ids.join("|"),
                "nominal contraction은 predicate ending을 참조하지 않습니다",
            ));
        }
        if rule.kind == "nominal-particle-compose"
            && !nominal_particle_forms.contains(rule.right.as_str())
        {
            return Err(invalid_rule_value(
                locations,
                source,
                &rule.id,
                "right",
                &rule.right,
                "nominal host를 허용하는 particle surface여야 합니다",
            ));
        }
    }
    Ok(())
}

fn validate_derivations(rules: &RuleSet, locations: &RuleLocations) -> Result<(), DataError> {
    let source = "data/rules/derivations.toml";
    let all_ids = rules.all_ids().collect::<BTreeSet<_>>();
    for rule in &rules.derivations {
        require_nfc(
            source,
            locations.get(source, &rule.id).line,
            "suffix",
            &rule.suffix,
        )?;
        if rule.suffix.is_empty() || rule.source_pos.is_empty() {
            return Err(invalid_rule_value(
                locations,
                source,
                &rule.id,
                "derivation",
                &rule.id,
                "suffix와 source_pos가 필요합니다",
            ));
        }
        if let Some(alternation_id) = &rule.alternation_id {
            require_reference(source, &rule.id, alternation_id, &all_ids, locations)?;
            if !alternation_id.starts_with("lexical.") {
                return Err(invalid_rule_value(
                    locations,
                    source,
                    &rule.id,
                    "alternation_id",
                    alternation_id,
                    "lexical.* 규칙이어야 합니다",
                ));
            }
        }
    }
    Ok(())
}

fn validate_particles(rules: &RuleSet, locations: &RuleLocations) -> Result<(), DataError> {
    let source = "data/rules/particles.toml";
    let particle_ids = rules
        .particles
        .iter()
        .map(|rule| rule.id.as_str())
        .collect::<BTreeSet<_>>();
    for rule in &rules.particles {
        validate_forms(source, &rule.id, &rule.forms, locations)?;
        let hosts = rule.hosts.iter().copied().collect::<BTreeSet<_>>();
        if hosts.len() != rule.hosts.len() || hosts.is_empty() {
            return Err(invalid_rule_value(
                locations,
                source,
                &rule.id,
                "hosts",
                format!("{:?}", rule.hosts),
                "하나 이상의 중복 없는 host여야 합니다",
            ));
        }
        if (rule.id == "particle.plural") != (rule.role == ParticleRuleRole::Plural) {
            return Err(invalid_rule_value(
                locations,
                source,
                &rule.id,
                "role",
                format!("{:?}", rule.role),
                "particle.plural만 plural role이어야 합니다",
            ));
        }
        if rule.role != ParticleRuleRole::Auxiliary
            && rule
                .hosts
                .iter()
                .any(|host| matches!(host, ParticleHost::Adverb | ParticleHost::PredicateEnding))
        {
            return Err(invalid_rule_value(
                locations,
                source,
                &rule.id,
                "hosts",
                format!("{:?}", rule.hosts),
                "adverb와 predicate-ending host는 auxiliary role에만 허용됩니다",
            ));
        }
        let expected_forms = match rule.selection {
            ParticleSelection::Literal => None,
            ParticleSelection::FinalPair | ParticleSelection::EuroRo => Some(2),
        };
        if let Some(expected) = expected_forms
            && rule.forms.len() != expected
        {
            return Err(invalid_rule_value(
                locations,
                source,
                &rule.id,
                "forms",
                rule.forms.join("|"),
                "선택 규칙은 정확히 두 이형태를 가져야 합니다",
            ));
        }
        for next in &rule.next {
            require_reference(source, &rule.id, next, &particle_ids, locations)?;
        }
        validate_terminal_transition(source, &rule.id, rule.terminal, &rule.next, locations)?;
    }
    validate_graph(
        source,
        None,
        rules
            .particles
            .iter()
            .map(|rule| (rule.id.as_str(), rule.next.as_slice())),
        locations,
    )
}

fn validate_features(
    source: &str,
    id: &str,
    required: &[String],
    forbidden: &[String],
    locations: &RuleLocations,
) -> Result<(), DataError> {
    const KNOWN: &[&str] = &[
        "action-verb",
        "descriptive-verb",
        "copula",
        "vowel-final",
        "consonant-final",
        "rieul-final",
        "light-vowel",
        "dark-vowel",
        "special-ha",
        "special-i",
        "special-ani",
        "special-o",
        "special-itda",
    ];
    let known = KNOWN.iter().copied().collect::<BTreeSet<_>>();
    for (field, features) in [("required", required), ("forbidden", forbidden)] {
        let mut seen = BTreeSet::new();
        for feature in features {
            if !known.contains(feature.as_str()) || !seen.insert(feature) {
                return Err(invalid_rule_value(
                    locations,
                    source,
                    id,
                    field,
                    feature,
                    "알려진 고유 morphology feature여야 합니다",
                ));
            }
        }
    }
    Ok(())
}

fn validate_terminal_transition(
    source: &str,
    id: &str,
    terminal: bool,
    next: &[String],
    locations: &RuleLocations,
) -> Result<(), DataError> {
    if !terminal && next.is_empty() {
        return Err(invalid_rule_value(
            locations,
            source,
            id,
            "terminal",
            id,
            "nonterminal 규칙에는 하나 이상의 next 전이가 필요합니다",
        ));
    }
    Ok(())
}

fn validate_forms(
    source: &str,
    id: &str,
    forms: &[String],
    locations: &RuleLocations,
) -> Result<(), DataError> {
    if forms.is_empty() {
        return Err(invalid_rule_value(
            locations,
            source,
            id,
            "forms",
            "",
            "하나 이상의 표면형이 필요합니다",
        ));
    }
    let mut seen = BTreeSet::new();
    for form in forms {
        require_nfc(source, locations.get(source, id).line, "forms", form)?;
        if form.is_empty() || !seen.insert(form) {
            return Err(invalid_rule_value(
                locations,
                source,
                id,
                "forms",
                form,
                "비어 있거나 중복된 표면형입니다",
            ));
        }
    }
    Ok(())
}

fn validate_graph<'a>(
    source: &str,
    depth_limit: Option<u8>,
    rules: impl Iterator<Item = (&'a str, &'a [String])>,
    locations: &RuleLocations,
) -> Result<(), DataError> {
    let graph = rules.collect::<BTreeMap<_, _>>();

    fn visit<'a>(
        source: &str,
        id: &'a str,
        graph: &BTreeMap<&'a str, &'a [String]>,
        active: &mut BTreeSet<&'a str>,
        depth: u8,
        depth_limit: Option<u8>,
        locations: &RuleLocations,
    ) -> Result<(), DataError> {
        if depth_limit.is_some_and(|limit| depth > limit) {
            return Err(invalid_rule_value(
                locations,
                source,
                id,
                "continuation",
                id,
                "max_continuation_depth를 초과합니다",
            ));
        }
        if !active.insert(id) {
            return Err(invalid_rule_value(
                locations,
                source,
                id,
                "continuation",
                id,
                "순환 전이는 허용하지 않습니다",
            ));
        }
        if let Some(next) = graph.get(id) {
            for next_id in *next {
                visit(
                    source,
                    next_id,
                    graph,
                    active,
                    depth + 1,
                    depth_limit,
                    locations,
                )?;
            }
        }
        active.remove(id);
        Ok(())
    }

    for id in graph.keys().copied() {
        visit(
            source,
            id,
            &graph,
            &mut BTreeSet::new(),
            1,
            depth_limit,
            locations,
        )?;
    }
    Ok(())
}

fn require_reference(
    source: &str,
    owner_id: &str,
    id: &str,
    known: &BTreeSet<&str>,
    locations: &RuleLocations,
) -> Result<(), DataError> {
    if known.contains(id) {
        Ok(())
    } else {
        Err(DataError::new(
            locations.get(source, owner_id),
            DataErrorKind::UnknownRuleId(id.to_owned()),
        ))
    }
}

fn invalid_rule_value(
    locations: &RuleLocations,
    source: &str,
    owner_id: &str,
    field: &str,
    value: impl Into<String>,
    reason: &str,
) -> DataError {
    DataError::new(
        locations.get(source, owner_id),
        DataErrorKind::InvalidValue {
            field: field.to_owned(),
            value: value.into(),
            reason: reason.to_owned(),
        },
    )
}
