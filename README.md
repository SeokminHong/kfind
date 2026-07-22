# kfind

[기술 문서와 플레이그라운드](https://kfind.pages.dev)

`kfind`는 한국어 표제어와 활용형을 파일과 메모리 문자열에서 찾는 검색
엔진입니다. 검색 질의를 한 번 형태 분석해 개수가 제한된 후보 프로그램으로
컴파일하고, corpus에서는 고정 byte 문자열을 먼저 찾은 뒤 후보 주변만
검증합니다. corpus 전체에 형태소 분석기를 실행하지 않습니다.

```console
$ kfind -n 걷다 src docs
docs/guide.md:12: 길을 걸어 갔다.
src/example.txt:8: 손님이 오래 걸었습니다.
```

## 제품 범위

kfind는 코드와 문서에서 한국어 형태 후보를 수집하는 query-directed text
matcher입니다. 형태 분석은 검색 계획과 일치 후보 검증에 사용하며, 결과에는 원문
span과 생성 근거를 보존합니다.

지원 범위는 다음과 같습니다.

- 명사·대명사·수사와 조사 결합
- 동사·형용사의 어미, 선어말어미와 불규칙 활용
- 지정사, 보조 용언과 제한된 복합 구조
- 등록된 생산적 파생형
- 품사 태그와 최대 간격을 갖는 순서형 구 검색
- `smart`, `token`, `any` 경계 정책
- 파일 ignore 규칙, glob, 파일 유형, stdin과 명시적 입력 인코딩
- 일반 text, 문맥, 집계, JSON Lines와 provenance 출력

일반 목적 문장 형태소 분석, 의미 검색, 동의어·바꿔쓰기 확장, 의미 기반
동형이의어 판별은 제품 범위가 아닙니다. `v:검증하다`는 `검증을 수행했다`를
찾지 않으므로 필요한 경우 `n:검증`을 별도로 검색해야 합니다.

## 설치

macOS와 Linux에서는 Homebrew로 CLI와 같은 버전의 형태 리소스를 설치합니다.

```sh
brew install seokminhong/brew/kfind
kfind --check-data
```

Rust 1.97 이상에서는 현재 source를 설치할 수 있습니다.

```sh
cargo install --locked --path crates/kfind-cli
```

JavaScript와 TypeScript에서는 WebAssembly 패키지를 설치합니다.

```sh
npm install @kfind/kfind@1.0.0-rc.1
```

```js
import { Kfind } from "@kfind/kfind";

const engine = new Kfind();
const matcher = engine.compile("걷다", { pos: "verb" });
const text = "길을 걸어 갔다.";
const matches = matcher.findAll(text);

console.log(text.slice(matches[0].start, matches[0].end));
```

npm match offset은 UTF-16 code unit 기준입니다. 패키지는 full POS와 component
asset의 경로나 URL을 추정하지 않으며, 호출자가 읽은 bytes를 명시적으로
전달합니다.

## 기본 검색

```sh
# 자동 품사와 smart 경계
kfind 걷다 src docs

# 명시적 품사
kfind --pos noun 사용자 src

# 순서형 구 검색
kfind 'n:권한 v:검증하다' src --max-gap 24

# 형태 확장 없는 문자열 검색
kfind --literal '걸어' data.txt

# 기계 판독용 출력
kfind --embedded --boundary any --pos verb --json 걷다 src docs
```

경로를 생략하면 pipe로 받은 stdin을 검색합니다. stdin이 대화형 터미널이면 현재
디렉터리를 검색합니다. `-`는 stdin을 명시합니다.

## 검색 질의

atom은 공백으로 구분합니다. 따옴표 안의 문자열은 하나의 literal atom이며,
백슬래시는 다음 문자를 escape합니다.

| 태그    | 품사    |
| ------- | ------- |
| `n:`    | 명사    |
| `pro:`  | 대명사  |
| `num:`  | 수사    |
| `v:`    | 동사    |
| `adj:`  | 형용사  |
| `det:`  | 관형사  |
| `adv:`  | 부사    |
| `j:`    | 조사    |
| `intj:` | 감탄사  |
| `lit:`  | literal |

구 atom은 같은 줄에서 순서대로 나타나야 합니다. `--max-gap`은 앞 token 끝과
다음 token 시작 사이에 허용할 Unicode scalar 수이며 기본값은 24입니다. 전역
`--pos`와 atom 태그를 함께 사용하면 두 품사가 같아야 합니다.

## 형태 확장

| 값           | 동작                                           |
| ------------ | ---------------------------------------------- |
| `inflection` | 조사, 어미, 이형태와 불규칙 활용을 생성합니다. |
| `derivation` | 활용과 등록된 생산적 파생 표제어를 생성합니다. |
| `literal`    | 입력 문자열만 검색합니다.                      |

`--literal`은 `--expand literal --pos literal`의 단축 옵션입니다. 형태 확장은
검색 질의에만 적용합니다. 표면형을 임의의 모든 표제어로 역분석하지 않습니다.

## 경계 정책

| 값      | 판정                                                     | 용도               |
| ------- | -------------------------------------------------------- | ------------------ |
| `smart` | 조사·어미 소비와 세부 품사 구성 요소를 국소 검증합니다.  | 사람의 기본 검색   |
| `token` | core 시작과 완성된 token 끝의 Unicode 경계를 요구합니다. | 독립 token 검색    |
| `any`   | 좌우 경계 없이 형태 후보의 부분 span을 보존합니다.       | 재현율 중심 자동화 |

의미 모호성은 유지합니다. 문법 구조로 구분 가능한 후보는 `smart`의 component
판정으로 제한하지만, 구조도 모호하면 지원 가능한 분석을 함께 반환합니다.

## 에이전트 자동화

자동화에서는 각 형태 atom의 품사를 지정하고 embedded lexicon, `any` 경계와
JSON Lines를 함께 사용하는 구성이 기본입니다.

```sh
kfind --embedded --boundary any --pos verb --json 걷다 src docs
kfind --embedded --boundary any --json 'n:사용자 v:검증하다' src
```

`any`는 후보를 넓게 보존하므로 호출자는 span 주변의 원문을 확인해야 합니다.
후보가 많으면 path와 glob을 좁히거나 `smart`로 다시 검색합니다.

프로젝트에 코딩 에이전트용 skill을 설치하려면 다음 명령을 사용합니다.

```sh
kfind --init
kfind --init --agent codex --agent claude-code
printf 'codex\ngemini\n' | kfind --init
```

Codex는 `.agents/skills/kfind/SKILL.md`, Claude Code는
`.claude/skills/kfind/SKILL.md`, Gemini CLI는
`.gemini/skills/kfind/SKILL.md`를 사용합니다. kfind 관리 표식이 없는 파일은
덮어쓰지 않습니다.

## 데이터 리소스

CLI는 `--data-dir`, 설치 경로와 개발 경로에서 full POS lexicon, enriched 용언
metadata와 component resource를 찾습니다. component 파일은 실행 파일과 같은 릴리즈
버전, schema와 source digest를 가져야 합니다. `kfind --check-data`는 검색 없이 이
계약을 검증합니다.

사용자 사전은 `--user-lexicon`, `KFIND_USER_LEXICON`,
`$XDG_CONFIG_HOME/kfind/lexicon.toml`, `$HOME/.config/kfind/lexicon.toml` 순서로
탐색합니다.

```toml
[[predicate]]
lemma = "플러그인하다"
pos = "verb"
alternation = "Ha"

[[nominal]]
surface = "LLM"
```

## 개발 검증

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
python3 tools/readme-guard/check_readmes.py
```

형태 품질·성능 벤치마크의 실행과 보고 계약은
[`docs/benchmarks/README.md`](docs/benchmarks/README.md)에 있습니다. 날짜, Git
revision, 실험 조건과 변화량은 `docs/benchmarks`의 기록 문서에만 둡니다.

## 라이선스

소스 코드는 MIT License입니다. 별도 배포 리소스의 출처와 라이선스는 각 data
manifest와 npm 패키지의 `LICENSES.md`에 기록합니다.
