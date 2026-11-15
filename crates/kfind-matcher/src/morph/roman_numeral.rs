use std::borrow::Cow;

use unicode_normalization::UnicodeNormalization;

pub(super) fn is_roman_numeral(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let text = if text.is_ascii() {
        Cow::Borrowed(text)
    } else if text
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || matches!(ch, '\u{2160}'..='\u{217f}'))
    {
        Cow::Owned(text.nfkc().map(|ch| ch.to_ascii_uppercase()).collect())
    } else {
        return false;
    };
    let tail = text.trim_start_matches('M');
    if text.len() - tail.len() > 3 {
        return false;
    }
    let tail = consume_place(tail, 'C', 'D', "CD", "CM");
    let tail = consume_place(tail, 'X', 'L', "XL", "XC");
    consume_place(tail, 'I', 'V', "IV", "IX").is_empty()
}

fn consume_place<'a>(text: &'a str, one: char, five: char, four: &str, nine: &str) -> &'a str {
    if let Some(tail) = text.strip_prefix(four).or_else(|| text.strip_prefix(nine)) {
        return tail;
    }
    let tail = text.strip_prefix(five).unwrap_or(text);
    let repeated = tail.trim_start_matches(one);
    if tail.len() - repeated.len() <= 3 {
        repeated
    } else {
        tail
    }
}
