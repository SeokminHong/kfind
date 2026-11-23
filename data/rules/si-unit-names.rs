// https://www.kriss.re.kr/menu.es?mid=a10302030000
// https://www.bipm.org/en/measurement-units/si-prefixes
// 질량의 배수와 분수는 그램에 접두어를 붙여 나타낸다.
const SI_UNIT_NAMES: &[(&str, &str)] = &[
    ("s", "초"),
    ("m", "미터"),
    ("g", "그램"),
    ("A", "암페어"),
    ("K", "켈빈"),
    ("mol", "몰"),
    ("cd", "칸델라"),
];

const SI_PREFIX_SYMBOLS: &[&str] = &[
    "", "Q", "R", "Y", "Z", "E", "P", "T", "G", "M", "k", "h", "da", "d", "c", "m", "µ", "μ",
    "n", "p", "f", "a", "z", "y", "r", "q",
];
