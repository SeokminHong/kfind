use std::borrow::Cow;

const UNICODE_ROMAN_DIGITS: [&str; 16] = [
    "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI", "XII", "L", "C", "D", "M",
];

pub(super) fn is_roman_numeral(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let text = if text.is_ascii() {
        Cow::Borrowed(text)
    } else if text.chars().all(is_roman_digit) {
        let mut ascii = String::with_capacity(text.len());
        for ch in text.chars() {
            if ch.is_ascii() {
                ascii.push(ch);
            } else {
                let index = (u32::from(ch) & 0xf) as usize;
                ascii.push_str(UNICODE_ROMAN_DIGITS[index]);
            }
        }
        Cow::Owned(ascii)
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

pub(super) fn is_roman_digit(ch: char) -> bool {
    matches!(
        ch,
        'I' | 'V' | 'X' | 'L' | 'C' | 'D' | 'M' | '\u{2160}'..='\u{217f}'
    )
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
