// https://www.kriss.re.kr/menu.es?mid=a10302030000
// https://www.bipm.org/en/measurement-units/si-prefixes
// https://www.law.go.kr/LSW/flDownload.do?bylClsCd=110201&flSeq=42107417&gubun=
// 질량의 배수와 분수는 그램에 접두어를 붙여 나타낸다.
const SI_UNIT_NAMES: &[(&str, &str)] = &[
    ("s", "초"),
    ("m", "미터"),
    ("g", "그램"),
    ("A", "암페어"),
    ("K", "켈빈"),
    ("mol", "몰"),
    ("cd", "칸델라"),
    ("rad", "라디안"),
    ("sr", "스테라디안"),
    ("Hz", "헤르츠"),
    ("N", "뉴턴"),
    ("Pa", "파스칼"),
    ("J", "줄"),
    ("W", "와트"),
    ("°C", "섭씨도"),
    ("C", "쿨롬"),
    ("V", "볼트"),
    ("F", "패럿"),
    ("Ω", "옴"),
    ("S", "지멘스"),
    ("Wb", "웨버"),
    ("T", "테슬라"),
    ("H", "헨리"),
    ("lm", "루멘"),
    ("lx", "럭스"),
    ("Bq", "베크렐"),
    ("Gy", "그레이"),
    ("Sv", "시버트"),
    ("kat", "카탈"),
];

const SI_PREFIX_SYMBOLS: &[&str] = &[
    "", "Q", "R", "Y", "Z", "E", "P", "T", "G", "M", "k", "h", "da", "d", "c", "m", "µ", "μ",
    "n", "p", "f", "a", "z", "y", "r", "q",
];
