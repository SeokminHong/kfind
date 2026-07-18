use std::borrow::Cow;

pub(crate) fn decode_hex(data: &[u8]) -> Cow<'_, [u8]> {
    let Some(hex) = data.strip_prefix(b"hex:") else {
        return Cow::Borrowed(data);
    };
    if hex.len() % 2 != 0 {
        return Cow::Borrowed(data);
    }
    let mut decoded = Vec::with_capacity(hex.len() / 2);
    for pair in hex.chunks_exact(2) {
        let Some(high) = hex_digit(pair[0]) else {
            return Cow::Borrowed(data);
        };
        let Some(low) = hex_digit(pair[1]) else {
            return Cow::Borrowed(data);
        };
        decoded.push((high << 4) | low);
    }
    Cow::Owned(decoded)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
