use std::ops::Range;

use unicode_normalization::UnicodeNormalization;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MorphologyExpressionAlignmentKind {
    SpanAligned,
    Fused,
    Unaligned,
    Invalid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MorphologyExpressionAlignment<'a> {
    pub kind: MorphologyExpressionAlignmentKind,
    pub components: Vec<MorphologyExpressionComponent<'a>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MorphologyExpressionComponent<'a> {
    pub surface: &'a str,
    pub pos: &'a str,
    pub span: Option<Range<usize>>,
}

#[must_use]
pub fn morphology_pos_transitions(pos: &str, expression: &str) -> Vec<(String, String)> {
    let expression_positions = parse_expression(expression).map(|components| {
        components
            .into_iter()
            .map(|component| component.pos)
            .collect::<Vec<_>>()
    });
    let positions = expression_positions
        .filter(|positions| positions.len() > 1)
        .unwrap_or_else(|| pos.split('+').collect());
    positions
        .windows(2)
        .filter(|pair| {
            pair[0] != "*" && pair[1] != "*" && !pair[0].is_empty() && !pair[1].is_empty()
        })
        .map(|pair| (pair[0].to_owned(), pair[1].to_owned()))
        .collect()
}

#[must_use]
pub fn align_morphology_expression<'a>(
    surface: &str,
    expression: &'a str,
) -> MorphologyExpressionAlignment<'a> {
    let Some(parsed) = parse_expression(expression) else {
        return MorphologyExpressionAlignment {
            kind: MorphologyExpressionAlignmentKind::Invalid,
            components: Vec::new(),
        };
    };
    let normalized_surface = surface.nfc().collect::<String>();
    let recomposed = recompose(parsed.iter().map(|component| component.surface));
    if recomposed != normalized_surface {
        return alignment(MorphologyExpressionAlignmentKind::Unaligned, parsed, None);
    }

    let mut offsets = Vec::with_capacity(parsed.len() + 1);
    offsets.push(0);
    for end in 1..=parsed.len() {
        let prefix = recompose(parsed[..end].iter().map(|component| component.surface));
        if !normalized_surface.starts_with(&prefix) {
            return alignment(MorphologyExpressionAlignmentKind::Fused, parsed, None);
        }
        offsets.push(prefix.len());
    }
    if offsets
        .windows(2)
        .any(|pair| pair[0] >= pair[1] || !normalized_surface.is_char_boundary(pair[1]))
    {
        return alignment(MorphologyExpressionAlignmentKind::Fused, parsed, None);
    }
    alignment(
        MorphologyExpressionAlignmentKind::SpanAligned,
        parsed,
        Some(&offsets),
    )
}

#[derive(Clone, Copy)]
struct ParsedComponent<'a> {
    surface: &'a str,
    pos: &'a str,
}

fn parse_expression(expression: &str) -> Option<Vec<ParsedComponent<'_>>> {
    if matches!(expression, "" | "*") {
        return None;
    }
    expression
        .split('+')
        .map(|part| {
            let mut fields = part.splitn(3, '/');
            let surface = fields.next()?;
            let pos = fields.next()?;
            if matches!(surface, "" | "*") || matches!(pos, "" | "*") {
                return None;
            }
            Some(ParsedComponent { surface, pos })
        })
        .collect()
}

fn recompose<'a>(surfaces: impl Iterator<Item = &'a str>) -> String {
    let decomposed = surfaces
        .flat_map(|surface| surface.nfd())
        .collect::<String>();
    decomposed.nfc().collect()
}

fn alignment<'a>(
    kind: MorphologyExpressionAlignmentKind,
    parsed: Vec<ParsedComponent<'a>>,
    offsets: Option<&[usize]>,
) -> MorphologyExpressionAlignment<'a> {
    let components = parsed
        .into_iter()
        .enumerate()
        .map(|(index, component)| MorphologyExpressionComponent {
            surface: component.surface,
            pos: component.pos,
            span: offsets.map(|offsets| offsets[index]..offsets[index + 1]),
        })
        .collect();
    MorphologyExpressionAlignment { kind, components }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligns_compound_components_to_stable_nfc_spans() {
        let aligned = align_morphology_expression("산속", "산/NNG/*+속/NNG/*");

        assert_eq!(aligned.kind, MorphologyExpressionAlignmentKind::SpanAligned);
        assert_eq!(aligned.components[0].span, Some(0..3));
        assert_eq!(aligned.components[1].span, Some(3..6));
    }

    #[test]
    fn marks_a_component_boundary_inside_a_composed_scalar_as_fused() {
        let aligned = align_morphology_expression("한", "하/XSA/*+ᆫ/ETM/*");

        assert_eq!(aligned.kind, MorphologyExpressionAlignmentKind::Fused);
        assert!(
            aligned
                .components
                .iter()
                .all(|component| component.span.is_none())
        );
    }

    #[test]
    fn keeps_contracted_expressions_without_inventing_spans() {
        let aligned = align_morphology_expression("비춰", "비추/VV/*+어/EC/*");

        assert_eq!(aligned.kind, MorphologyExpressionAlignmentKind::Unaligned);
        assert!(
            aligned
                .components
                .iter()
                .all(|component| component.span.is_none())
        );
    }

    #[test]
    fn rejects_missing_or_malformed_expression_fields() {
        for expression in ["*", "", "산", "산/*/*", "/NNG/*"] {
            let aligned = align_morphology_expression("산", expression);
            assert_eq!(aligned.kind, MorphologyExpressionAlignmentKind::Invalid);
            assert!(aligned.components.is_empty());
        }
    }

    #[test]
    fn derives_categorical_transitions_from_expression_or_multi_pos() {
        assert_eq!(
            morphology_pos_transitions("VV+EP+EF", "가/VV/*+었/EP/*+다/EF/*"),
            [
                ("VV".to_owned(), "EP".to_owned()),
                ("EP".to_owned(), "EF".to_owned())
            ]
        );
        assert_eq!(
            morphology_pos_transitions("NNG+JX", "*"),
            [("NNG".to_owned(), "JX".to_owned())]
        );
    }
}
