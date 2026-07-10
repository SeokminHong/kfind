use std::collections::{BTreeMap, BTreeSet};

use crate::validation::{require_nfc, require_rule_id};
use crate::{DataError, DataErrorKind, SourceLocation};

use super::{ParticleSelection, RuleSet, invalid_value};

pub(super) fn validate_rules(rules: &RuleSet) -> Result<(), DataError> {
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
        require_rule_id(source, id)?;
        if ids.insert(id, source).is_some() {
            return Err(DataError::new(
                SourceLocation::new(source),
                DataErrorKind::DuplicateRuleId(id.to_owned()),
            ));
        }
    }

    validate_endings(rules)?;
    validate_alternations(rules)?;
    validate_contractions(rules)?;
    validate_derivations(rules)?;
    validate_particles(rules)?;
    Ok(())
}

fn validate_endings(rules: &RuleSet) -> Result<(), DataError> {
    let source = "data/rules/endings.toml";
    let ending_ids = rules
        .endings
        .iter()
        .map(|rule| rule.id.as_str())
        .collect::<BTreeSet<_>>();
    for rule in &rules.endings {
        validate_forms(source, &rule.forms)?;
        let required = rule.required.iter().collect::<BTreeSet<_>>();
        if let Some(conflict) = rule.forbidden.iter().find(|item| required.contains(item)) {
            return Err(invalid_value(
                source,
                "forbidden",
                conflict.to_string(),
                "required feature와 겹칩니다",
            ));
        }
        for next in &rule.next {
            require_reference(source, next, &ending_ids)?;
        }
    }
    validate_graph_depth(
        source,
        rules.max_continuation_depth,
        rules
            .endings
            .iter()
            .map(|rule| (rule.id.as_str(), rule.next.as_slice())),
    )
}

fn validate_alternations(rules: &RuleSet) -> Result<(), DataError> {
    let source = "data/rules/alternations.toml";
    let ending_ids = rules
        .endings
        .iter()
        .map(|rule| rule.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut kinds = BTreeSet::new();
    for rule in &rules.alternations {
        if !kinds.insert(rule.kind) {
            return Err(invalid_value(
                source,
                "kind",
                rule.kind.as_str(),
                "alternation kind는 하나의 규범 규칙만 가져야 합니다",
            ));
        }
        for ending_id in &rule.ending_ids {
            require_reference(source, ending_id, &ending_ids)?;
        }
    }
    Ok(())
}

fn validate_contractions(rules: &RuleSet) -> Result<(), DataError> {
    let source = "data/rules/contractions.toml";
    let ending_ids = rules
        .endings
        .iter()
        .map(|rule| rule.id.as_str())
        .collect::<BTreeSet<_>>();
    for rule in &rules.contractions {
        for (field, value) in [
            ("kind", rule.kind.as_str()),
            ("left", rule.left.as_str()),
            ("right", rule.right.as_str()),
            ("result", rule.result.as_str()),
        ] {
            require_nfc(source, None, field, value)?;
            if value.is_empty() {
                return Err(invalid_value(source, field, value, "비어 있습니다"));
            }
        }
        for ending_id in &rule.ending_ids {
            require_reference(source, ending_id, &ending_ids)?;
        }
    }
    Ok(())
}

fn validate_derivations(rules: &RuleSet) -> Result<(), DataError> {
    let source = "data/rules/derivations.toml";
    let all_ids = rules.all_ids().collect::<BTreeSet<_>>();
    for rule in &rules.derivations {
        require_nfc(source, None, "suffix", &rule.suffix)?;
        if rule.suffix.is_empty() || rule.source_pos.is_empty() {
            return Err(invalid_value(
                source,
                "derivation",
                &rule.id,
                "suffix와 source_pos가 필요합니다",
            ));
        }
        if let Some(alternation_id) = &rule.alternation_id {
            require_reference(source, alternation_id, &all_ids)?;
            if !alternation_id.starts_with("lexical.") {
                return Err(invalid_value(
                    source,
                    "alternation_id",
                    alternation_id,
                    "lexical.* 규칙이어야 합니다",
                ));
            }
        }
    }
    Ok(())
}

fn validate_particles(rules: &RuleSet) -> Result<(), DataError> {
    let source = "data/rules/particles.toml";
    let particle_ids = rules
        .particles
        .iter()
        .map(|rule| rule.id.as_str())
        .collect::<BTreeSet<_>>();
    for rule in &rules.particles {
        validate_forms(source, &rule.forms)?;
        let expected_forms = match rule.selection {
            ParticleSelection::Literal => None,
            ParticleSelection::FinalPair | ParticleSelection::EuroRo => Some(2),
        };
        if let Some(expected) = expected_forms {
            if rule.forms.len() != expected {
                return Err(invalid_value(
                    source,
                    "forms",
                    rule.forms.join("|"),
                    "선택 규칙은 정확히 두 이형태를 가져야 합니다",
                ));
            }
        }
        for next in &rule.next {
            require_reference(source, next, &particle_ids)?;
        }
    }
    validate_graph_depth(
        source,
        rules.max_continuation_depth,
        rules
            .particles
            .iter()
            .map(|rule| (rule.id.as_str(), rule.next.as_slice())),
    )
}

fn validate_forms(source: &str, forms: &[String]) -> Result<(), DataError> {
    if forms.is_empty() {
        return Err(invalid_value(
            source,
            "forms",
            "",
            "하나 이상의 표면형이 필요합니다",
        ));
    }
    let mut seen = BTreeSet::new();
    for form in forms {
        require_nfc(source, None, "forms", form)?;
        if form.is_empty() || !seen.insert(form) {
            return Err(invalid_value(
                source,
                "forms",
                form,
                "비어 있거나 중복된 표면형입니다",
            ));
        }
    }
    Ok(())
}

fn validate_graph_depth<'a>(
    source: &str,
    limit: u8,
    rules: impl Iterator<Item = (&'a str, &'a [String])>,
) -> Result<(), DataError> {
    let graph = rules.collect::<BTreeMap<_, _>>();
    fn visit<'a>(
        source: &str,
        id: &'a str,
        graph: &BTreeMap<&'a str, &'a [String]>,
        active: &mut BTreeSet<&'a str>,
        depth: u8,
        limit: u8,
    ) -> Result<(), DataError> {
        if depth > limit {
            return Err(invalid_value(
                source,
                "continuation",
                id,
                "max_continuation_depth를 초과합니다",
            ));
        }
        if !active.insert(id) {
            return Err(invalid_value(
                source,
                "continuation",
                id,
                "순환 전이는 허용하지 않습니다",
            ));
        }
        if let Some(next) = graph.get(id) {
            for next_id in *next {
                visit(source, next_id, graph, active, depth + 1, limit)?;
            }
        }
        active.remove(id);
        Ok(())
    }

    for id in graph.keys().copied() {
        visit(source, id, &graph, &mut BTreeSet::new(), 1, limit)?;
    }
    Ok(())
}

fn require_reference(source: &str, id: &str, known: &BTreeSet<&str>) -> Result<(), DataError> {
    if known.contains(id) {
        Ok(())
    } else {
        Err(DataError::new(
            SourceLocation::new(source),
            DataErrorKind::UnknownRuleId(id.to_owned()),
        ))
    }
}
