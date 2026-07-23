---
name: kfind
description: 코드나 문서에서 한국어 표제어와 활용형을 검색합니다. 형태 후보, 정확한 span 또는 규칙 provenance가 필요한 한국어 검색에 사용합니다.
---

<!-- managed by kfind init -->

# kfind 한국어 원문 검색

한국어 표제어가 조사, 어미, 불규칙 활용 또는 등록된 파생형으로 나타날 수 있으면
literal grep 대신 `kfind`를 사용합니다. kfind는 query-directed text matcher이며
의미 검색기가 아닙니다. `v:검증하다`는 `검증을 수행했다`를 찾지 않으므로 필요한
경우 `n:검증`을 별도로 검색합니다.

프로젝트의 kfind agent hook이 한글 pattern을 포함한 `rg`·`grep` shell 명령을
거부하면 같은 검색을 kfind로 다시 실행합니다. 정확한 표면형만 의도한 검색은
`kfind --literal`을 사용합니다.

## 에이전트 검색 절차

1. 모든 형태 atom의 품사를 정합니다.
2. 재현 가능하고 재현율 중심인 `--embedded --boundary any --json`으로 시작합니다.
3. 검색 질의를 넓히기 전에 path, `--glob` 또는 `--type`으로 범위를 제한합니다.
4. stdout의 JSON Lines를 parse하고 stderr의 진단을 분리합니다.
5. 일치한 문맥을 읽고 false positive를 제외합니다. 후보가 많으면 `--boundary smart`로 다시 검색합니다.

한 품사의 검색 질의에는 `--pos`를 사용합니다.

```sh
kfind --embedded --boundary any --pos verb --json '검증하다' src docs
```

여러 atom 중 하나를 찾으려면 `|`를 사용하고 shell 해석을 막기 위해 query 전체를
따옴표로 묶습니다. Alternative마다 품사 태그를 붙일 수 있습니다.

```sh
kfind --embedded --boundary any --json 'v:걷다 | n:사용자 | n:검증' src docs
```

각 `|` alternative는 하나의 atom이어야 하며 공백 구와 섞지 않습니다. Literal
`|`는 `\|` 또는 `"|"`로 작성합니다.

품사가 섞인 구는 atom마다 태그를 붙입니다.

```sh
kfind --embedded --boundary any --json 'n:권한 v:검증하다' src
```

형태 확장 없이 표면형만 찾을 때는 `--literal`을 사용합니다.

```sh
kfind --literal --boundary any --json '검증했다' src
```

## 품사

`--pos`에는 긴 값을 전달하고 atom에는 짧은 태그를 붙입니다.

| `--pos` 값 | Atom 태그 | 품사 |
| --- | --- | --- |
| `auto` | 없음 | 후보 품사 추론, 대화형 검색용 |
| `noun` | `n:` | 명사 |
| `pronoun` | `pro:` | 대명사 |
| `numeral` | `num:` | 수사 |
| `verb` | `v:` | 동사 |
| `adjective` | `adj:` | 형용사 |
| `determiner` | `det:` | 관형사 |
| `adverb` | `adv:` | 부사 |
| `particle` | `j:` | 조사 |
| `interjection` | `intj:` | 감탄사 |
| `literal` | `lit:` | literal 표면형 |

전역 `--pos`와 다른 atom 태그를 함께 사용하지 않습니다. 공백을 하나의 literal
atom 안에 유지하려면 따옴표를 사용하고, 다음 문자를 escape하려면 백슬래시를
사용합니다.

```sh
kfind --embedded --boundary any --json 'det:새 n:기능' docs
kfind --embedded --boundary any --json 'n:권한 "접근 제어" v:검증하다' src
```

구 atom은 한 줄에서 순서대로 나타나야 합니다. `--max-gap N`은 인접한 검증
token 사이의 최대 Unicode scalar 수를 지정하며 기본값은 24입니다. 이 옵션은
구에만 적용됩니다. 등록된
파생형도 필요할 때만 `--expand derivation`을 사용합니다. 기본값인
`inflection`은 조사와 활용 어미를 포함합니다.

## 검색 범위와 결과 해석

검색 질의 뒤에 파일이나 디렉터리를 전달합니다. 경로가 없으면 pipe 입력을
검색하며, stdin이 대화형 터미널이면 현재 디렉터리를 검색합니다. 디렉터리 순회는
ignore 파일을 따르고 기본적으로 숨김 경로를 제외합니다.

```sh
kfind --embedded --boundary any --pos noun --json \
  --glob '*.rs' --glob '!target/**' '사용자' crates
```

`type: "match"` record를 일치 결과로 읽습니다. 각 record에는 `path`, `line`,
`text`, `spans`가 있습니다. 각 span은 구의 `atom`, `core`와 완성된 `token`의 byte
범위, 일치한 `surface`, lemma·POS·rule provenance를 담은 `origins`를 제공합니다.
`offset_unit`을 확인합니다. UTF-8이 아닌 path나 text는 Base64 field를 사용합니다.
문맥 옵션을 사용하면 `context`와 `context_break` record를 별도로 처리합니다.

종료 코드 0은 하나 이상의 일치, 1은 일치 없음, 2는 사용법·검색 질의·데이터·I/O
또는 검색 오류를 뜻합니다. 검색 계획을 확인할 때는 `--json` 없이
`--explain-query`를 사용합니다.
