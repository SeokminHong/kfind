use crate::cli::Cli;
use std::collections::BTreeSet;
use std::io::{self, Write};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Interpretation {
    Auto,
    Literal,
    Noun,
    Predicate,
    Modifier,
    Particle,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MatchMode {
    Strict,
    Normal,
    Loose,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AtomType {
    Literal,
    Nominal,
    Predicate,
    Modifier,
    Particle,
}
impl AtomType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Literal => "literal",
            Self::Nominal => "nominal",
            Self::Predicate => "predicate",
            Self::Modifier => "modifier",
            Self::Particle => "particle",
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Seed {
    pub surface: String,
    pub generated_from: String,
    pub rule_id: String,
    pub atom_type: AtomType,
    pub atom_index: usize,
}
#[derive(Debug)]
pub struct CompiledQuery {
    pub query: String,
    pub atoms: Vec<Vec<Seed>>,
    pub seeds: Vec<Seed>,
    pub phrase: bool,
    pub max_gap: usize,
    pub adjacent: bool,
}

pub fn compile_query(cli: &Cli) -> CompiledQuery {
    let mut parts: Vec<&str> = cli.query.split_whitespace().collect();
    let phrase = parts.len() > 1;
    if !phrase {
        parts = vec![cli.query.as_str()];
    }
    let mut atoms = Vec::new();
    let mut seeds = Vec::new();
    for (i, p) in parts.iter().enumerate() {
        let a = compile_atom(p, i, cli.interpretation, cli.mode, cli.derive, phrase);
        seeds.extend(a.clone());
        atoms.push(a);
    }
    CompiledQuery {
        query: cli.query.clone(),
        atoms,
        seeds,
        phrase,
        max_gap: cli.max_gap,
        adjacent: cli.adjacent,
    }
}

fn compile_atom(
    q: &str,
    idx: usize,
    forced: Interpretation,
    mode: MatchMode,
    derive: bool,
    phrase: bool,
) -> Vec<Seed> {
    let kind = match forced {
        Interpretation::Auto if q.ends_with('다') && has_hangul(q) => Interpretation::Predicate,
        Interpretation::Auto if modifier_lex(q).is_some() => Interpretation::Modifier,
        Interpretation::Auto if !has_hangul(q) => Interpretation::Literal,
        Interpretation::Auto => Interpretation::Noun,
        x => x,
    };
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    let mut push = |surface: String, rule: &str, typ: AtomType| {
        if valid_seed(&surface)
            && (phrase || mode == MatchMode::Loose || surface.chars().count() > 1)
            && seen.insert(surface.clone())
        {
            out.push(Seed {
                surface,
                generated_from: q.into(),
                rule_id: rule.into(),
                atom_type: typ,
                atom_index: idx,
            });
        }
    };
    match kind {
        Interpretation::Literal => push(q.into(), "literal", AtomType::Literal),
        Interpretation::Particle => {
            for p in particle_variants(q) {
                push(p, "particle_variant", AtomType::Particle)
            }
        }
        Interpretation::Modifier => {
            push(q.into(), "modifier_literal", AtomType::Modifier);
            if mode == MatchMode::Loose {
                for p in ["도", "는", "만", "까지", "조차"] {
                    push(
                        format!("{q}{p}"),
                        "modifier_aux_particle",
                        AtomType::Modifier,
                    );
                }
            }
        }
        Interpretation::Predicate => {
            for (s, r) in predicate_surfaces(q) {
                push(s, r, AtomType::Predicate)
            }
        }
        Interpretation::Noun | Interpretation::Auto => {
            for (s, r) in nominal_surfaces(q, mode, derive) {
                push(s, r, AtomType::Nominal)
            }
        }
    }
    out
}
fn valid_seed(s: &str) -> bool {
    !s.is_empty()
        && s.chars().count() <= 128
        && !s
            .chars()
            .any(|c| c == '\0' || (c.is_control() && !c.is_whitespace()))
}
fn has_hangul(s: &str) -> bool {
    s.chars().any(|c| ('가'..='힣').contains(&c))
}
fn nominal_surfaces(n: &str, mode: MatchMode, derive: bool) -> Vec<(String, &'static str)> {
    let mut forms = vec![
        (n.into(), "nominal_base"),
        (format!("{n}들"), "nominal_plural"),
    ];
    let mut bases = vec![n.to_string(), format!("{n}들")];
    if derive {
        bases.push(format!("{n}적"));
    }
    for base in bases {
        for p in particles_for(n, mode) {
            forms.push((format!("{base}{p}"), "nominal_particle"));
        }
    }
    forms
}
fn particles_for(n: &str, mode: MatchMode) -> Vec<&'static str> {
    let all = [
        "은", "는", "이", "가", "을", "를", "에", "에서", "에게", "한테", "께", "로", "으로", "와",
        "과", "랑", "이랑", "하고", "도", "만", "까지", "부터", "조차", "마저", "의",
    ];
    if mode != MatchMode::Strict {
        return all.to_vec();
    }
    let jong = n.chars().last().is_some_and(has_jongseong);
    all.into_iter()
        .filter(|p| {
            (!matches!(*p, "은" | "이" | "을" | "과" | "으로") || jong)
                && (!matches!(*p, "는" | "가" | "를" | "와" | "로") || !jong)
        })
        .collect()
}
fn particle_variants(p: &str) -> Vec<String> {
    match p {
        "으로" | "로" => vec!["으로".into(), "로".into()],
        "은" | "는" => vec!["은".into(), "는".into()],
        "이" | "가" => vec!["이".into(), "가".into()],
        _ => vec![p.into()],
    }
}
fn predicate_surfaces(q: &str) -> Vec<(String, &'static str)> {
    let class = if q.ends_with("하다") {
        "ha"
    } else {
        pred_class(q).unwrap_or("regular")
    };
    let stem = q.strip_suffix('다').unwrap_or(q);
    let mut v = vec![
        (q.into(), "lemma"),
        (format!("{stem}고"), "connective_go"),
        (format!("{stem}지"), "connective_ji"),
        (format!("{stem}게"), "connective_ge"),
        (format!("{stem}겠"), "pre_final_get"),
    ];
    match class {
        "d_irregular" => v.extend(forms(&[
            "걸어", "걸었", "걸으", "걸은", "걸을", "걸음", "걸며", "걸면", "걷는", "걷던",
        ])),
        "b_irregular" => v.extend(forms(&["도와", "도왔", "도운", "도우면"])),
        "s_irregular" => v.extend(forms(&["지어", "지었", "지은", "지으면"])),
        "h_irregular" => v.extend(forms(&["파래", "파랬", "파란", "파랄"])),
        "reu_irregular" => v.extend(forms(&["빨라", "빨랐", "빠른", "빠르게"])),
        "ha" => {
            let base = stem.strip_suffix('하').unwrap_or("");
            v.extend(forms(&[
                &format!("{stem}고"),
                &format!("{stem}는"),
                &format!("{stem}지"),
                &format!("{base}했다"),
                &format!("{base}했"),
                &format!("{base}하면"),
                &format!("{base}해서"),
                &format!("{base}합니다"),
                &format!("{base}한"),
                &format!("{base}할"),
            ]))
        }
        "ida" => v.extend(forms(&["이고", "이어", "였", "인", "일"])),
        "l_drop" => v.extend(forms(&["사는", "삽니다", "살고", "살던", "살면", "살았"])),
        _ => v.extend(forms(&[
            &format!("{stem}어"),
            &format!("{stem}었"),
            &format!("{stem}는"),
            &format!("{stem}은"),
            &format!("{stem}을"),
            &format!("{stem}며"),
            &format!("{stem}면"),
            &format!("{stem}습니다"),
        ])),
    }
    v
}
fn forms(xs: &[&str]) -> Vec<(String, &'static str)> {
    xs.iter().map(|x| ((*x).into(), "conjugation")).collect()
}
fn pred_class(q: &str) -> Option<&'static str> {
    Some(match q {
        "걷다" => "d_irregular",
        "믿다" => "regular",
        "돕다" => "b_irregular",
        "입다" => "regular",
        "짓다" => "s_irregular",
        "벗다" => "regular",
        "파랗다" => "h_irregular",
        "좋다" => "regular",
        "빠르다" => "reu_irregular",
        "하다" => "ha",
        "이다" => "ida",
        "살다" => "l_drop",
        "예쁘다" => "regular",
        _ => return None,
    })
}
fn modifier_lex(q: &str) -> Option<&'static str> {
    Some(match q {
        "새" => "MM",
        "헌" => "MM",
        "모든" => "MM",
        "매우" => "MAG",
        "빨리" => "MAG",
        "잘" => "MAG",
        _ => return None,
    })
}
fn has_jongseong(c: char) -> bool {
    ('가'..='힣').contains(&c) && ((c as u32 - 0xAC00) % 28 != 0)
}
pub fn print_explain_query(w: &mut impl Write, cq: &CompiledQuery) -> io::Result<()> {
    writeln!(w, "query: {}\nanalysis:", cq.query)?;
    for (i, a) in cq.atoms.iter().enumerate() {
        if let Some(s) = a.first() {
            writeln!(
                w,
                "  atom[{i}]:\n    type: {}\n    generated_from: {}\n    seeds:",
                s.atom_type.as_str(),
                s.generated_from
            )?;
            for seed in a {
                writeln!(w, "      {}", seed.surface)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::Cli;
    #[test]
    fn non_korean_auto_is_literal() {
        let c = Cli::parse_from(["README", "."]).unwrap();
        let q = compile_query(&c);
        assert_eq!(q.seeds[0].atom_type, AtomType::Literal);
    }
    #[test]
    fn korean_predicate_generates_irregular() {
        let c = Cli::parse_from(["걷다", "--as=predicate"]).unwrap();
        let q = compile_query(&c);
        assert!(q.seeds.iter().any(|s| s.surface == "걸어"));
    }
}
