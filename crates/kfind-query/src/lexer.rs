use kfind_morph::CoarsePos;

use crate::ast::{QueryAst, QueryAtom, QueryComposition};
use crate::error::{QueryError, QueryErrorKind, SourceSpan};
use crate::options::CompileOptions;

/// Parses query text without performing morphological analysis.
pub fn parse_query(source: &str, options: &CompileOptions) -> Result<QueryAst, QueryError> {
    validate_query_length(source, options)?;

    let mut tokens = Vec::new();
    let mut atom_count = 0;
    let mut current: Option<AtomBuilder> = None;
    let mut quote: Option<(char, usize)> = None;
    let mut characters = source.char_indices();

    while let Some((offset, character)) = characters.next() {
        if let Some((delimiter, _)) = quote {
            match character {
                '\\' => append_escape(source, offset, &mut characters, &mut current)?,
                value if value == delimiter => quote = None,
                value => ensure_atom(&mut current, offset).value.push(value),
            }
            continue;
        }

        match character {
            '\\' => {
                ensure_atom(&mut current, offset).plain_prefix = false;
                append_escape(source, offset, &mut characters, &mut current)?;
            }
            '\'' | '"' => {
                let atom = ensure_atom(&mut current, offset);
                atom.saw_quote = true;
                atom.plain_prefix = false;
                quote = Some((character, offset));
            }
            value if value.is_whitespace() => {
                finish_atom(&mut current, offset, options, &mut tokens, &mut atom_count)?;
            }
            '|' => {
                finish_atom(&mut current, offset, options, &mut tokens, &mut atom_count)?;
                tokens.push(LexedToken::Disjunction(SourceSpan::new(
                    offset,
                    offset + character.len_utf8(),
                )));
            }
            ':' => {
                let atom = ensure_atom(&mut current, offset);
                if !atom.consume_tag(offset + character.len_utf8()) {
                    atom.value.push(character);
                }
            }
            value => ensure_atom(&mut current, offset).value.push(value),
        }
    }

    if let Some((delimiter, start)) = quote {
        return Err(QueryError::new(
            QueryErrorKind::UnterminatedQuote { quote: delimiter },
            SourceSpan::new(start, source.len()),
        ));
    }

    finish_atom(
        &mut current,
        source.len(),
        options,
        &mut tokens,
        &mut atom_count,
    )?;

    if tokens.is_empty() {
        return Err(QueryError::new(
            QueryErrorKind::EmptyQuery,
            SourceSpan::new(0, source.len()),
        ));
    }

    let (atoms, composition) = compose(tokens)?;

    Ok(QueryAst {
        atoms,
        composition,
        phrase: options.phrase,
    })
}

fn validate_query_length(source: &str, options: &CompileOptions) -> Result<(), QueryError> {
    let actual = source.chars().count();
    let limit = options.limits.max_query_scalars;
    if actual <= limit {
        return Ok(());
    }

    let overflow_start = source
        .char_indices()
        .nth(limit)
        .map_or(source.len(), |(offset, _)| offset);
    Err(QueryError::new(
        QueryErrorKind::QueryTooLong { actual, limit },
        SourceSpan::new(overflow_start, source.len()),
    ))
}

fn ensure_atom(current: &mut Option<AtomBuilder>, start: usize) -> &mut AtomBuilder {
    current.get_or_insert_with(|| AtomBuilder::new(start))
}

fn append_escape(
    source: &str,
    escape_offset: usize,
    characters: &mut impl Iterator<Item = (usize, char)>,
    current: &mut Option<AtomBuilder>,
) -> Result<(), QueryError> {
    let Some((_, escaped)) = characters.next() else {
        return Err(QueryError::new(
            QueryErrorKind::DanglingEscape,
            SourceSpan::new(escape_offset, source.len()),
        ));
    };
    ensure_atom(current, escape_offset).value.push(escaped);
    Ok(())
}

fn finish_atom(
    current: &mut Option<AtomBuilder>,
    end: usize,
    options: &CompileOptions,
    tokens: &mut Vec<LexedToken>,
    atom_count: &mut usize,
) -> Result<(), QueryError> {
    let Some(atom) = current.take() else {
        return Ok(());
    };

    if atom.value.is_empty() {
        return Err(QueryError::new(
            QueryErrorKind::EmptyAtom,
            SourceSpan::new(atom.start, end),
        ));
    }

    if let (Some(global), Some(tagged)) = (options.global_pos, atom.forced_pos)
        && global != tagged
    {
        return Err(QueryError::new(
            QueryErrorKind::ConflictingPos { global, tagged },
            atom.tag_span.unwrap_or(SourceSpan::new(atom.start, end)),
        ));
    }

    let actual = *atom_count + 1;
    let limit = options.limits.max_atoms;
    if actual > limit {
        return Err(QueryError::new(
            QueryErrorKind::TooManyAtoms { actual, limit },
            SourceSpan::new(atom.start, end),
        ));
    }

    tokens.push(LexedToken::Atom {
        value: QueryAtom {
            raw: atom.value.into_boxed_str(),
            forced_pos: atom.forced_pos,
            quoted_literal: atom.saw_quote,
        },
        span: SourceSpan::new(atom.start, end),
    });
    *atom_count = actual;
    Ok(())
}

fn compose(tokens: Vec<LexedToken>) -> Result<(Vec<QueryAtom>, QueryComposition), QueryError> {
    let has_disjunction = tokens
        .iter()
        .any(|token| matches!(token, LexedToken::Disjunction(_)));
    if !has_disjunction {
        let atoms = tokens
            .into_iter()
            .map(|token| match token {
                LexedToken::Atom { value, .. } => value,
                LexedToken::Disjunction(_) => unreachable!("operator presence was checked"),
            })
            .collect();
        return Ok((atoms, QueryComposition::Phrase));
    }

    let mut atoms = Vec::new();
    let mut expect_atom = true;
    let mut last_operator = None;
    for token in tokens {
        match (expect_atom, token) {
            (true, LexedToken::Atom { value, .. }) => {
                atoms.push(value);
                expect_atom = false;
            }
            (true, LexedToken::Disjunction(span)) => {
                return Err(QueryError::new(
                    QueryErrorKind::MissingDisjunctionOperand,
                    span,
                ));
            }
            (false, LexedToken::Disjunction(span)) => {
                last_operator = Some(span);
                expect_atom = true;
            }
            (false, LexedToken::Atom { span, .. }) => {
                return Err(QueryError::new(
                    QueryErrorKind::MixedPhraseAndDisjunction,
                    span,
                ));
            }
        }
    }
    if expect_atom {
        return Err(QueryError::new(
            QueryErrorKind::MissingDisjunctionOperand,
            last_operator.expect("a parsed disjunction has an operator span"),
        ));
    }
    Ok((atoms, QueryComposition::Disjunction))
}

#[derive(Debug)]
enum LexedToken {
    Atom { value: QueryAtom, span: SourceSpan },
    Disjunction(SourceSpan),
}

#[derive(Debug)]
struct AtomBuilder {
    start: usize,
    value: String,
    forced_pos: Option<CoarsePos>,
    tag_span: Option<SourceSpan>,
    saw_quote: bool,
    plain_prefix: bool,
}

impl AtomBuilder {
    fn new(start: usize) -> Self {
        Self {
            start,
            value: String::new(),
            forced_pos: None,
            tag_span: None,
            saw_quote: false,
            plain_prefix: true,
        }
    }

    fn consume_tag(&mut self, end: usize) -> bool {
        if self.forced_pos.is_some() || !self.plain_prefix {
            return false;
        }
        let Some(pos) = tag_pos(&self.value) else {
            return false;
        };

        self.forced_pos = Some(pos);
        self.tag_span = Some(SourceSpan::new(self.start, end));
        self.value.clear();
        self.plain_prefix = false;
        true
    }
}

fn tag_pos(tag: &str) -> Option<CoarsePos> {
    match tag {
        "n" => Some(CoarsePos::Noun),
        "pro" => Some(CoarsePos::Pronoun),
        "num" => Some(CoarsePos::Numeral),
        "v" => Some(CoarsePos::Verb),
        "adj" => Some(CoarsePos::Adjective),
        "det" => Some(CoarsePos::Determiner),
        "adv" => Some(CoarsePos::Adverb),
        "j" => Some(CoarsePos::Particle),
        "intj" => Some(CoarsePos::Interjection),
        "lit" => Some(CoarsePos::Literal),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CompileOptionOverrides, PlanLimits};

    #[test]
    fn parses_all_supported_tags() {
        let query = parse_query(
            "n:명사 pro:나 num:하나 v:걷다 adj:예쁘다 det:새 adv:빨리 j:은 intj:아 lit:raw",
            &CompileOptions::default(),
        )
        .unwrap();

        let positions: Vec<_> = query.atoms.iter().map(|atom| atom.forced_pos).collect();
        assert_eq!(
            positions,
            vec![
                Some(CoarsePos::Noun),
                Some(CoarsePos::Pronoun),
                Some(CoarsePos::Numeral),
                Some(CoarsePos::Verb),
                Some(CoarsePos::Adjective),
                Some(CoarsePos::Determiner),
                Some(CoarsePos::Adverb),
                Some(CoarsePos::Particle),
                Some(CoarsePos::Interjection),
                Some(CoarsePos::Literal),
            ]
        );
    }

    #[test]
    fn preserves_quoted_atoms_and_unescapes_characters() {
        let query = parse_query(
            r#"n:권한 "접근 제어" '작은 따옴표' plain\ value n\:권한"#,
            &CompileOptions::default(),
        )
        .unwrap();

        assert_eq!(query.atoms.len(), 5);
        assert_eq!(&*query.atoms[0].raw, "권한");
        assert_eq!(&*query.atoms[1].raw, "접근 제어");
        assert!(query.atoms[1].quoted_literal);
        assert_eq!(&*query.atoms[2].raw, "작은 따옴표");
        assert!(query.atoms[2].quoted_literal);
        assert_eq!(&*query.atoms[3].raw, "plain value");
        assert_eq!(&*query.atoms[4].raw, "n:권한");
        assert_eq!(query.atoms[4].forced_pos, None);
    }

    #[test]
    fn parses_spaced_and_compact_disjunctions() {
        for source in ["n:권한 | v:검증하다", "n:권한|v:검증하다"] {
            let query = parse_query(source, &CompileOptions::default()).unwrap();

            assert_eq!(query.composition, QueryComposition::Disjunction);
            assert_eq!(query.atoms.len(), 2);
            assert_eq!(&*query.atoms[0].raw, "권한");
            assert_eq!(query.atoms[0].forced_pos, Some(CoarsePos::Noun));
            assert_eq!(&*query.atoms[1].raw, "검증하다");
            assert_eq!(query.atoms[1].forced_pos, Some(CoarsePos::Verb));
        }
    }

    #[test]
    fn preserves_quoted_and_escaped_pipes_as_literal_atoms() {
        let query = parse_query(r#""|" \| a\|b"#, &CompileOptions::default()).unwrap();

        assert_eq!(query.composition, QueryComposition::Phrase);
        assert_eq!(query.atoms.len(), 3);
        assert_eq!(&*query.atoms[0].raw, "|");
        assert!(query.atoms[0].quoted_literal);
        assert_eq!(&*query.atoms[1].raw, "|");
        assert_eq!(&*query.atoms[2].raw, "a|b");
    }

    #[test]
    fn rejects_missing_operands_and_mixed_phrase_disjunctions() {
        for source in ["| 권한", "권한 |", "권한 || 검증", "권한 | | 검증"] {
            let error = parse_query(source, &CompileOptions::default()).unwrap_err();
            assert_eq!(error.kind, QueryErrorKind::MissingDisjunctionOperand);
        }

        for source in ["권한 검증 | 사용자", "권한 | 사용자 검증"] {
            let error = parse_query(source, &CompileOptions::default()).unwrap_err();
            assert_eq!(error.kind, QueryErrorKind::MixedPhraseAndDisjunction);
        }
    }

    #[test]
    fn unknown_prefix_remains_literal_text() {
        let query = parse_query("url:https://example.test", &CompileOptions::default()).unwrap();

        assert_eq!(&*query.atoms[0].raw, "url:https://example.test");
        assert_eq!(query.atoms[0].forced_pos, None);
    }

    #[test]
    fn quoted_and_escaped_tag_prefixes_are_not_tags() {
        let query = parse_query(
            r#""n:권한" n\:기능 "인용: \"값\"""#,
            &CompileOptions::default(),
        )
        .unwrap();

        assert_eq!(&*query.atoms[0].raw, "n:권한");
        assert_eq!(query.atoms[0].forced_pos, None);
        assert!(query.atoms[0].quoted_literal);
        assert_eq!(&*query.atoms[1].raw, "n:기능");
        assert_eq!(query.atoms[1].forced_pos, None);
        assert_eq!(&*query.atoms[2].raw, "인용: \"값\"");
        assert!(query.atoms[2].quoted_literal);
    }

    #[test]
    fn allows_a_tag_matching_the_global_pos() {
        let options = CompileOptions::resolve(CompileOptionOverrides {
            pos: Some(CoarsePos::Noun),
            ..CompileOptionOverrides::default()
        })
        .unwrap();

        let query = parse_query("n:권한", &options).unwrap();
        assert_eq!(query.atoms[0].forced_pos, Some(CoarsePos::Noun));
    }

    #[test]
    fn rejects_a_tag_conflicting_with_the_global_pos_at_the_tag() {
        let options = CompileOptions::resolve(CompileOptionOverrides {
            pos: Some(CoarsePos::Verb),
            ..CompileOptionOverrides::default()
        })
        .unwrap();

        let error = parse_query("n:권한", &options).unwrap_err();
        assert_eq!(
            error.kind,
            QueryErrorKind::ConflictingPos {
                global: CoarsePos::Verb,
                tagged: CoarsePos::Noun,
            }
        );
        assert_eq!(error.span, SourceSpan::new(0, 2));
    }

    #[test]
    fn literal_shortcut_rejects_non_literal_atom_tags() {
        let options = CompileOptions::resolve(CompileOptionOverrides {
            literal: true,
            ..CompileOptionOverrides::default()
        })
        .unwrap();

        let error = parse_query("n:권한", &options).unwrap_err();
        assert_eq!(
            error.kind,
            QueryErrorKind::ConflictingPos {
                global: CoarsePos::Literal,
                tagged: CoarsePos::Noun,
            }
        );
    }

    #[test]
    fn reports_unterminated_quotes_and_dangling_escapes() {
        let quote_error = parse_query("앞 '뒤", &CompileOptions::default()).unwrap_err();
        assert_eq!(
            quote_error.kind,
            QueryErrorKind::UnterminatedQuote { quote: '\'' }
        );
        assert_eq!(quote_error.span, SourceSpan::new(4, 8));

        let escape_error = parse_query("앞 \\", &CompileOptions::default()).unwrap_err();
        assert_eq!(escape_error.kind, QueryErrorKind::DanglingEscape);
        assert_eq!(escape_error.span, SourceSpan::new(4, 5));
    }

    #[test]
    fn rejects_empty_queries_and_empty_tagged_atoms() {
        let empty = parse_query(" \t ", &CompileOptions::default()).unwrap_err();
        assert_eq!(empty.kind, QueryErrorKind::EmptyQuery);

        let empty_tag = parse_query("n:", &CompileOptions::default()).unwrap_err();
        assert_eq!(empty_tag.kind, QueryErrorKind::EmptyAtom);
        assert_eq!(empty_tag.span, SourceSpan::new(0, 2));
    }

    #[test]
    fn enforces_scalar_and_atom_limits_without_truncating() {
        let long_query = "가".repeat(257);
        let too_long = parse_query(&long_query, &CompileOptions::default()).unwrap_err();
        assert_eq!(
            too_long.kind,
            QueryErrorKind::QueryTooLong {
                actual: 257,
                limit: 256
            }
        );
        assert_eq!(too_long.span.start, 256 * "가".len());

        let limits = PlanLimits {
            max_query_scalars: 1_000,
            ..PlanLimits::default()
        };
        let options = CompileOptions::resolve(CompileOptionOverrides {
            limits: Some(limits),
            ..CompileOptionOverrides::default()
        })
        .unwrap();
        let too_many = parse_query(&vec!["가"; 33].join(" "), &options).unwrap_err();
        assert_eq!(
            too_many.kind,
            QueryErrorKind::TooManyAtoms {
                actual: 33,
                limit: 32
            }
        );
    }

    #[test]
    fn carries_phrase_gap_policy_into_the_ast() {
        let options = CompileOptions::resolve(CompileOptionOverrides {
            max_gap: Some(7),
            ..CompileOptionOverrides::default()
        })
        .unwrap();

        let query = parse_query("n:권한 v:검증하다", &options).unwrap();
        assert!(query.is_phrase());
        assert_eq!(query.phrase.max_gap, 7);
    }
}
