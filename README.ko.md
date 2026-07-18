# kfind

[English](README.md) | [한국어](README.ko.md) |
[기술 문서와 Playground](https://kfind.pages.dev)

한국어 표제어와 활용형을 빠르게 찾는 코드·문서 검색 CLI입니다.

`kfind`는 쿼리를 한 번 분석해 개수가 제한된 candidate program으로 컴파일하고,
말뭉치 전체에 형태소 분석기를 실행하지 않은 채 파일을 검색합니다. 활용형을 찾으면서도
grep과 유사한 경로 필터, 문맥과 출력 방식을 사용할 수 있습니다.

```console
$ kfind -n 걷다 src docs
docs/guide.md:12: 길을 걸어 갔다.
src/example.txt:8: 손님이 오래 걸었습니다.
```

## 목적

`kfind`는 에이전트와 사람의 대화형 검색을 위한 쿼리 중심 text matcher입니다. 짧은
한국어 표제어나 구(句)를 bounded search plan으로 만들고, 파일 또는 메모리 text에서 후보 span과
형태 생성 근거를 반환합니다. 에이전트는 넓은 후보를 빠르게 수집한 뒤 주변 문맥으로 최종
판단할 수 있고, 사람은 더 선택적인 기본 workflow를 사용할 수 있습니다.

형태 지식은 검색 계획과 match 검증을 위한 수단이며 제품의 출력 자체가 아닙니다. `kfind`는
입력 corpus의 모든 문장을 분석하지 않습니다.

## Goal / non-goal

Goal:

- 짧은 query를 bounded plan으로 컴파일하고 큰 text collection을 낮은 overhead로 scan합니다.
- 지원하는 한국어 형태 범위에서 검증된 recall·precision과 match span, 표제어, 품사, 규칙
  provenance를 제공합니다.
- CLI, Rust library와 JavaScript/WebAssembly package에서 재현 가능한 offline 동작을
  제공합니다.

Non-goal:

- 일반 목적 문장 tokenizer·형태소 분석기 또는 형태소 분석기 처리량 순위를 목표로 최적화한
  backend입니다.
- Semantic search, 동의어·바꿔쓰기 확장과 의미 기반 동음이의어 구분입니다.
- 임의 표면형의 완전한 역분석이나 모든 한국어 구성의 제한 없는 지원입니다.

## 주요 기능

- 표제어에서 명사·조사 결합, 용언 어미, 불규칙 활용과 일부 생산적 파생형을 찾습니다.
- atom마다 품사를 지정한 순서형 한 줄 구(句) 검색을 지원합니다.
- 대화형 검색의 정확도를 위한 `smart` 경계와 자동화의 재현율을 위한 `any` 경계를
  제공합니다.
- ignore 규칙, glob, 파일 유형, 숨김 파일 제어, stdin과 명시적 인코딩을 지원하며 파일을
  병렬로 탐색합니다.
- 터미널 text, 문맥, 개수, 파일 목록, JSON Lines, 쿼리·match 생성 근거를 출력합니다.
- 오프라인으로 실행됩니다. 핵심 규칙은 내장하고, Homebrew는 선택적 full POS와 형태
  component resource와 agent skill을 설치합니다.
- Rust와 WebAssembly 라이브러리에서 같은 쿼리 컴파일러와 matcher를 제공합니다.

## 설치

Homebrew 릴리스는 개인 tap을 통해 배포됩니다.

```sh
brew install seokminhong/brew/kfind
```

`brew install`과 `brew upgrade`는 같은 kfind 버전으로 만든 component resource를 설치하고
설치 뒤 무결성 검사를 실행합니다. `kfind --check-data`로 이 검사를 다시 실행할 수 있습니다.

Rust 1.97 이상으로 현재 checkout을 빌드하려면 다음 명령을 실행합니다.

```sh
cargo install --locked --path crates/kfind-cli
```

## Agent skill 설정

프로젝트 디렉터리에서 `kfind --init`을 실행합니다. 터미널에서는 Claude Code, Codex,
Gemini CLI와 custom stdout 출력을 고르는 checkbox가 열립니다. 프로젝트 설치 경로는
각각 `.claude/skills/kfind`, `.agents/skills/kfind`, `.gemini/skills/kfind`입니다.

```sh
# 대화형 checkbox 선택
kfind --init

# 재현 가능한 one-liner
kfind --init --agent codex --agent claude-code

# 비대화형 stdin 선택
printf 'codex\ngemini\n' | kfind --init

# 다른 agent를 위해 SKILL.md 원문만 stdout에 출력
kfind --init --agent custom > path/to/kfind/SKILL.md
```

Homebrew는 배포용 skill 원본을 binary와 함께 `share/kfind`에 설치하지만, 사용자의 프로젝트와 agent를
대신 선택할 수는 없습니다. 각 프로젝트에서 `kfind --init`을 한 번 실행해야 합니다. 이 초기화는
Homebrew의 안정된 `opt/kfind` 경로를 연결하므로 이후 `brew upgrade kfind`가 해당 프로젝트
skill을 자동으로 갱신합니다. Source 또는 Cargo 설치는 관리되는 파일을 복사하며, 갱신하려면
`kfind --init`을 다시 실행합니다. kfind 관리 표식이 없는 기존 skill은 덮어쓰지 않습니다.

## 빠른 시작

```sh
# 품사를 추론하고 활용형을 찾습니다.
kfind 걷다 src docs

# 표제어를 명사로 해석하고 올바른 조사까지 소비합니다.
kfind --pos noun 사용자 src

# 순서가 있는 구(句)를 검색합니다. atom마다 품사를 지정할 수 있습니다.
kfind 'n:권한 v:검증하다' src --max-gap 24

# 형태 확장 없이 입력 문자열을 literal로 검색합니다.
kfind --literal '걸어' data.txt

# 파일을 제한하고 앞뒤 두 줄의 문맥을 출력합니다.
kfind 걷다 . --type-add 'docs:*.{md,mdx,txt}' --type docs -C 2

# 자동화에 안정적인 기계 판독용 record를 출력합니다.
kfind --embedded --boundary any --pos verb --json 걷다 src docs
```

`PATH`를 생략하면 pipe로 들어온 stdin을 읽고, stdin이 terminal이면 `.`을 검색합니다.
`-`로 stdin을 직접 지정할 수 있습니다.

## 검색 모델

### 형태 확장

기본 `inflection`은 명사의 복수·조사 연쇄, 용언 어미, 지정사 형태와 버전 관리되는
규칙·사전에 포함된 불규칙 활용과 교차 검증된 사전 활용형을 검색합니다. `derivation`은
`-적`, `-하다`, `-되다`, `-시키다` 등 등록된 생산적 파생과 사전에서 관계가 확인된
파생형을 추가합니다. `literal`은 형태 확장을 비활성화합니다.

형태 확장은 쿼리에만 적용하며 말뭉치 전체를 tokenize하거나 분석하지 않습니다. 따라서
파일 검색은 빠르지만 의미 검색은 아닙니다. 예를 들어 `v:검증하다`는 `검증을 수행했다`와
같은 의미적 바꿔쓰기를 찾지 않으므로 필요한 경우 `n:검증`을 별도로 검색해야 합니다.
`걸어` 같은 표면형도 표제어나 품사를 직접 지정하지 않으면 가능한 모든 표제어로
역분석하지 않습니다.

### 쿼리 문법

atom은 공백으로 구분합니다. 따옴표 안의 구(句)는 하나의 literal atom이 되고, 백슬래시는
다음 문자를 escape합니다. 지원하는 품사 태그는 다음과 같습니다.

| 태그 | 품사 |
| --- | --- |
| `n:` | 명사 |
| `pro:` | 대명사 |
| `num:` | 수사 |
| `v:` | 동사 |
| `adj:` | 형용사 |
| `det:` | 관형사 |
| `adv:` | 부사 |
| `j:` | 조사 |
| `intj:` | 감탄사 |
| `lit:` | literal |

```sh
kfind 'n:권한 "접근 제어" v:검증하다' src
kfind 'det:새 n:기능' docs
kfind 'lit:걸어' data.txt
```

구(句)의 atom은 같은 줄에서 순서대로 나타나야 합니다. `--max-gap`은 앞의 검증된 token 끝과
다음 token 시작 사이의 Unicode scalar 수를 제한합니다. 전역 `--pos`와 atom 태그를 함께
사용할 때는 품사가 같아야 합니다.

### 경계 정책

| 정책 | 동작 | 주 용도 |
| --- | --- | --- |
| `smart` | 품사별 검증을 적용하고 완성된 token span의 경계를 확인합니다. 정확한 POS/component span과 인접 token 배치를 증명할 때 선택적 구조 resource를 사용할 수 있습니다. | 대화형 검색, 기본값 |
| `token` | 모든 core와 완성된 token span의 좌우 token 경계를 요구합니다. | 독립 token만 엄격하게 검색 |
| `any` | 좌우 token 경계를 요구하지 않습니다. | 후속 문맥 검토를 수행하는 재현율 중심 자동화 |

한 음절 쿼리는 `smart`에서도 보수적인 경계를 사용합니다. 조사 품사를 명시하면 `은/는`,
`이/가`, `으로/로`처럼 등록된 이형태를 확장할 수 있으며, 품사를 생략하면 사용자가 입력한
조사 표면형만 검색합니다.

의미 모호성은 의도적으로 유지합니다. `걷다`와 `걸다`는 모두 `걸었고`에 매칭될 수
있습니다. 문장 성분의 구조적 품사 근거는 이와 다릅니다. `smart`는 `매일 보고 싶어`에서
`매일/MAG`를 선택해 `n:매`를 거부하고, `독수리가 아니라 매일 수도 있어`에서는
`매/NNG + 이/VCP + ㄹ/ETM` 구조를 선택해 `adv:매일`을 거부합니다. 구조도 모호하면
재현율을 위해 지원 가능한 후보를 유지합니다.

### 사람과 에이전트의 권장 경로

대화형 검색에서는 품사를 생략할 수 있습니다. 기본 `auto` 품사와 `smart` 경계는 정확도를
우선하고, 설치된 full POS lexicon이 있으면 자동으로 사용합니다.

```sh
kfind 걷다 src
kfind 사용자 src docs
```

에이전트 자동화에서는 모든 형태 atom의 품사를 명시하고 `any`, 내장 사전과 JSON Lines를
사용합니다.

```sh
kfind --embedded --boundary any --pos verb --json 걷다 src docs
kfind --embedded --boundary any --json 'n:사용자 v:검증하다' src
```

에이전트 경로는 더 넓은 후보를 반환합니다. 주변 문맥을 검사하고, 후보가 너무 많으면
path·glob을 좁히거나 `smart`로 다시 검색합니다.

## CLI 옵션

```text
kfind [OPTIONS] <QUERY> [PATH]...
kfind --init [--agent <AGENT>]...
```

### 쿼리와 컴파일

| 옵션 | 값과 기본값 | 설명 |
| --- | --- | --- |
| `--pos <POS>` | `auto`(기본값), `noun`, `pronoun`, `numeral`, `verb`, `adjective`, `determiner`, `adverb`, `particle`, `interjection`, `literal` | 쿼리 전체의 품사를 강제합니다. |
| `--expand <LEVEL>` | `inflection`(기본값), `literal`, `derivation` | 형태 확장 수준을 선택합니다. `derivation`은 inflection을 포함합니다. |
| `--boundary <POLICY>` | `smart`(기본값), `token`, `any` | match 경계 검증 방식을 선택합니다. |
| `--literal` | 사용 안 함 | `--expand literal --pos literal`의 단축 옵션입니다. 충돌하는 `--expand`나 `--pos` 값은 오류입니다. |
| `--embedded` | 사용 안 함 | full POS 탐색과 decode를 건너뜁니다. `smart` plan은 component resource가 여전히 필요할 수 있습니다. |
| `--max-gap <NUM>` | `24` | 인접한 구(句) atom 사이의 최대 Unicode scalar 거리를 지정합니다. |
| `--unicode-normalization <MODE>` | `nfc`(기본값), `canonical`, `none` | NFC만 사용하거나, NFC·NFD pattern을 함께 생성하거나, 정규화 없이 입력 byte를 검색합니다. |

### 파일과 입력

| 옵션 | 값과 기본값 | 설명 |
| --- | --- | --- |
| `--encoding <ENCODING>` | `auto`(기본값), `utf-8`, `utf-16le`, `utf-16be`, `euc-kr` | 입력 decode 방식을 선택합니다. `auto`는 BOM이 있는 UTF-16을 감지하고 나머지는 UTF-8로 처리하며 EUC-KR은 추정하지 않습니다. |
| `--glob <GLOB>` | 반복 가능 | 포함 glob이나 `!`로 시작하는 제외 glob을 추가합니다. |
| `--type <TYPE>` | 반복 가능 | 지정한 이름의 파일 유형만 검색합니다. |
| `--type-add <NAME:GLOB>` | 반복 가능 | 파일 유형을 정의하거나 확장합니다. |
| `--hidden` | 사용 안 함 | 숨김 파일과 디렉터리를 포함합니다. |
| `--no-ignore` | 사용 안 함 | `.gitignore`, `.ignore`, 전역 Git ignore와 상위 ignore 규칙을 비활성화합니다. |
| `--threads <NUM>` | 자동 | 파일 검색 worker thread 수를 지정합니다. |

디렉터리 탐색은 기본적으로 숨김·ignore 항목을 제외하고 symbolic link를 따라가지 않습니다.
파일을 경로로 직접 지정하면 ignore 규칙과 관계없이 검색합니다. 입력에서 NUL byte를 만나면
그 위치에서 검색을 멈추고 binary로 취급합니다.

### 출력과 진단

| 옵션 | 기본값 | 설명 |
| --- | --- | --- |
| `-n`, `--line-number` | 사용 안 함 | 1부터 시작하는 줄 번호를 출력합니다. |
| `-H`, `--with-filename` | 자동 | 파일 이름을 항상 출력합니다. `-h`와 함께 사용할 수 없습니다. |
| `-h`, `--no-filename` | 자동 | 파일 이름을 출력하지 않습니다. `-H`와 함께 사용할 수 없습니다. |
| `-C`, `--context <NUM>` | `0` | 각 match 앞뒤에 `NUM`개 줄을 출력합니다. |
| `-B`, `--before-context <NUM>` | context 값 | 각 match 앞에 출력할 줄 수를 덮어씁니다. |
| `-A`, `--after-context <NUM>` | context 값 | 각 match 뒤에 출력할 줄 수를 덮어씁니다. |
| `-l`, `--files-with-matches` | 사용 안 함 | match가 있는 파일을 한 번만 출력하고 해당 파일의 첫 match에서 멈춥니다. `--count`, `--quiet`, `--json`과 함께 사용할 수 없습니다. |
| `-c`, `--count` | 사용 안 함 | 파일별로 검증된 match가 하나 이상 있는 줄 수를 출력합니다. `--quiet`, `--json`과 함께 사용할 수 없습니다. |
| `-q`, `--quiet` | 사용 안 함 | match를 출력하지 않고 전체 검색의 첫 match에서 멈춥니다. `--json`과 함께 사용할 수 없습니다. |
| `--json` | 사용 안 함 | match나 문맥 record마다 JSON 객체 하나를 출력합니다. `--explain-query`와 함께 사용할 수 없습니다. |
| `--color <WHEN>` | `auto`; `auto`, `always`, `never` | terminal 강조 표시를 제어합니다. `auto`는 terminal에 기본 출력을 쓸 때만 색상을 사용합니다. |
| `--no-pager` | 사용 안 함 | 일반 text 결과를 TTY에 쓸 때도 pager를 사용하지 않습니다. |
| `--column` | 사용 안 함 | 1부터 시작하는 Unicode scalar 열을 출력하며 줄 번호도 함께 표시합니다. |
| `--explain-query` | 사용 안 함 | 결과보다 먼저 추론한 분석, candidate program, consumption 상태, 정규화와 사전 상태를 출력합니다. |
| `--explain-match` | 사용 안 함 | 각 text match를 생성한 표제어와 규칙 경로를 추가합니다. JSON에는 생성 근거가 기본으로 포함됩니다. |
| `--sort path` | 정렬하지 않는 병렬 stream | 대상 경로를 수집·정렬한 뒤 bounded 병렬 file stream을 경로순으로 출력합니다. 경로 수집 동안 첫 출력이 지연되고 파일 수에 비례한 메모리를 사용하며 처리량이 낮아질 수 있습니다. |

디렉터리나 여러 입력을 검색하면 파일 이름을 자동으로 출력합니다. Match 줄과 문맥 줄은 각각
`:`와 `-` 구분자를 사용합니다. 일반 text 결과를 TTY stdin/stdout에서 쓰면 검색 시작과 함께 내장
TUI를 열고 완성된 결과 행을 점진적으로 반영합니다. 긴 match 줄은
검증된 match마다 별도 행으로 펼치고, 원문에서 target 앞뒤가 차지하는 비율에 맞춰 양쪽을 생략해
target이 보이게 합니다. 마지막 행이 content 영역 아래에 닿으면 더 스크롤하지 않습니다. 검색 중에도
이동과 terminal resize를 처리합니다. `↑`/`↓` 또는 `k`/`j`로
이동하고 `q`나 `Esc`로 종료해 남은 검색도 중단합니다. Redirect와 pipe, JSON Lines, count,
파일명 요약, quiet mode와
`--no-pager`는 기존 stdout stream을 유지합니다. TUI를 시작할 수 없으면 일반 text를 stdout에
직접 출력합니다.

키 반복 이동은 입력된 이동량을 누락하지 않고 content viewport 크기에 맞춰 frame 단위로
합칩니다. 큰 viewport일수록 한 frame에 더 많은 키 반복 입력을 반영해 terminal scroll 횟수를
제한합니다.

Pager index는 완성된 source line과 match별 전개 row 수에 비례합니다. 큰 결과를 대화형으로
탐색할 필요가 없으면 `--no-pager`로 bounded stream을 사용합니다.

JSON Lines record에는 `type`, 경로, 줄 번호, 선택적 열 번호, text, span, core·token byte
범위, 일치한 표면형, 표제어·품사 생성 근거, 규칙 경로와 `offset_unit`이 포함됩니다.
UTF-8이 아닌 경로나 text는 손실 변환하지 않고 Base64 필드를 사용합니다.

### 데이터와 명령 정보

| 옵션 | 기본값 | 설명 |
| --- | --- | --- |
| `--data-dir <PATH>` | 자동 탐색 | 하나의 명시적 디렉터리에서 `lexicon.bin`, 선택적 `predicates.enriched.tsv`, `morphology-component-compact.kfc`를 읽습니다. |
| `--check-data` | 사용 안 함 | 설치된 full POS와 component resource의 무결성과 정확한 component package version을 검증한 뒤 종료합니다. `--json`, `--data-dir`과 함께 사용할 수 있습니다. |
| `--user-lexicon <PATH>` | XDG config 경로 | 기본 config 탐색 대신 지정한 TOML 사용자 사전을 읽습니다. |
| `--init` | 사용 안 함 | query 없이 현재 디렉터리에 kfind skill을 초기화합니다. |
| `--agent <AGENT>` | TTY 선택 또는 stdin; 반복 가능 | `claude-code`, `codex`, `gemini`, `custom` 중 대상을 지정하며 `--init`이 필요합니다. |
| `--help` | — | 현재 locale의 도움말을 출력합니다. `-h`는 `--no-filename`입니다. |
| `-V`, `--version` | — | 버전을 출력합니다. |

사용자 사전은 `--user-lexicon`, `KFIND_USER_LEXICON`,
`$XDG_CONFIG_HOME/kfind/lexicon.toml`, `$HOME/.config/kfind/lexicon.toml` 순서로
확인합니다.

```toml
[[predicate]]
lemma = "플러그인하다"
pos = "verb"
alternation = "Ha"

[[nominal]]
surface = "LLM"
```

사용자 사전 항목은 배포 데이터에 추가됩니다. 같은 표제어의 해당 형태 category 분석을
교체하려면 항목에 `replace = true`를 지정합니다.

### 종료 코드와 표시 언어

| 코드 | 의미 |
| ---: | --- |
| `0` | 하나 이상의 match를 찾았거나 초기화·데이터 검증에 성공했습니다. |
| `1` | match가 없습니다. |
| `2` | 사용법, 쿼리 컴파일, 데이터, I/O 또는 검색 오류입니다. |

사람이 읽는 도움말, 오류, 진단과 `--explain-*` 출력은 `LC_ALL`, `LC_MESSAGES`,
`LANG` 중 비어 있지 않은 첫 값을 따릅니다. `ko` locale이면 한국어를 사용하고 나머지는
영어를 사용합니다. 옵션명, 허용 값, JSON 필드와 종료 코드는 locale에 따라 바뀌지 않습니다.

## 사전 데이터

핵심 불규칙 용언과 규칙은 binary에 포함됩니다. Homebrew는 고정된 full POS lexicon,
CC BY-SA enriched 용언 metadata·표면형과 compact 형태 component resource도 `share/kfind`에
설치하므로 실행 중 네트워크 접근이 필요하지 않습니다.

Component header는 kfind package version을 보존합니다. Binary와 component version이 다르면
오래된 resource로 fallback하지 않고 decode 단계에서 실패합니다. Package upgrade는 두 산출물을
함께 교체하며 kfind가 백그라운드에서 갱신하지는 않습니다.

full POS 파일이 없어도 핵심 사전과 heuristic으로 검색을 계속합니다. `--explain-query`는
이 preview 상태를 표시합니다. `--data-dir` 또는 `KFIND_DATA_DIR`로 resource 디렉터리를
직접 선택할 수 있습니다. `--embedded`가 아니면 `predicates.enriched.tsv`가 있을 때 읽고,
`--embedded`는 full POS와 enriched 용언 탐색을 건너뜁니다. Compile된 `smart` plan에
component 근거가 필요하면 component resource는 계속 찾아 검증하며, 필요하지 않은 plan은
이를 로드하지 않습니다.

고정되고 checksum 검증을 거친 `mecab-ko-dic`과 국립국어원 사전 snapshot에서 외부 사전
데이터를 재현할 수 있습니다.

```sh
scripts/build-full-pos.sh
cargo run --locked -p kfind-testkit --bin verify-gold -- \
  data/generated/full-pos/lexicon.bin
scripts/build-enriched-predicates.sh
```

enriched 생성기는 두 국립국어원 사전이 함께 지지하는 활용형 12,888개 중 규칙으로 만들 수
없는 130개만 저장합니다. 두 사전이 원형 형용사와 결과 부사를 각각 독립 등재한 제한된 부사형
88개는 기본 `inflection`에서 사용합니다. 한국어기초사전의 양방향 용언·부사 파생 관계 153개 중
76개는 이 근거와 겹치며, 나머지 77개는 `derivation`에서만 사용합니다. 결과 TSV는 295개
기존 surface-only 행과 사전 voice 파생 관계 225개를 포함해 520행, 42,910바이트입니다.
Voice 관계는 한국어기초사전이 source·target 동사를 직접 연결하고 표준국어대사전이 두 표제어를
일반어 동사로 확인한 경우에만 기본 `inflection`에서 target 표제어의 활용기를 재사용합니다.
생성기는 candidate를 한 번 만들고 보존하며, 별도 validator가 UTF-8·schema·통계와 64 KiB
배포 한도를 검사합니다.

`누구·무어·무엇 + 이 + -ㄴ가` 축약은 선언된 `누군가·무언가` 표면만 생성합니다.
`smart`는 전체 표면의 `NP + VCP + EC/EF` 원천 분석을 요구하고, 뒤의 조사는 기존 체언 조사
전이로 검증합니다. 별도 사전 표제어를 원 대명사의 alias로 합치지는 않습니다.

`smart`는 token 왼쪽부터 `용언 + EP* + EC + 용언 + E* + J*`가 끝까지 이어질 때
합성용언의 뒤쪽 용언을 독립 query로 유지합니다. 관형·명사형·종결 어미 뒤에서 연결 어미
경로를 다시 열지 않으며, token 시작의 `가` 같은 독립 명령형은 그대로 매칭합니다.

## 벤치마크

kfind는 형태 품질, end-to-end CLI 처리량, resource 초기화와 literal scan을 별도 workload로
측정합니다. benchmark 계약은 재현 명령, 입력, warm-up·반복 횟수와 보고서 요건을 정의합니다.
측정값과 비교 결과는 개별 보고서에만 보존합니다.

- [벤치마크 계약](docs/benchmarks/README.md)

## 라이브러리

### Rust

`kfind` crate는 메모리의 UTF-8 입력을 검색할 수 있도록 CLI와 같은 쿼리 컴파일러와 형태
matcher를 제공합니다.

```rust
use kfind::{CompileOptions, Engine};

let engine = Engine::new()?;
let matcher = engine
    .compile("걷다", &CompileOptions::default())
    .expect("query should compile");
let text = "길을 걸어 갔다.";
let matches = matcher.find_all(text.as_bytes());

assert_eq!(&text[matches[0].span.clone()], "걸어");
```

CLI와 같은 사전 품질 profile은 `ResourceBundle`과 `Engine::with_resources`로 구성합니다. Bundle은
full POS binary, enriched predicate TSV, component bytes를 각각 선택적으로 받습니다. 기존 개별
생성자도 같은 초기화 경로를 사용합니다. Component resource는 해당 resource가 필요한 plan을
compile하기 전에 `load_component_resource`로 나중에 추가할 수도 있습니다.

1.x 안정 facade는 crate root의 engine 생성, compile option·오류, 검색과 match provenance로
구성됩니다. Caller가 조립한 lexicon과 query plan 검사는 `kfind::expert`를 명시적으로 import해야
하며 workspace 구현 crate는 별도로 게시하지 않습니다.

라이브러리와 핵심 의존 crate는 Rust 1.97의 `wasm32-unknown-unknown` target을 지원합니다.

```sh
rustup target add wasm32-unknown-unknown --toolchain 1.97.0
cargo +1.97.0 build --locked --package kfind-wasm --target wasm32-unknown-unknown
```

### JavaScript

Unscoped `kfind` npm package는 브라우저 bundler용 ESM WebAssembly binding과 생성된
TypeScript 선언을 제공합니다.

```js
import { Kfind } from "kfind";

const engine = new Kfind();
const matcher = engine.compile("걷다");
const text = "😀 길을 걸어 갔다.";
const matches = matcher.findAll(text);

console.log(text.slice(matches[0].start, matches[0].end)); // 걸어
```

JavaScript offset은 UTF-16 code unit 기준입니다. `Kfind.withResources`는 선택적 `fullPos`,
`enrichedPredicates`, `component`를 한 profile로 받습니다. Package는 enriched TSV를
`kfind/assets/predicates.enriched.tsv`, component resource를
`kfind/assets/morphology-component-compact.kfc`로 WASM binary와 분리해 배포합니다. Resource 없이
`Kfind`를 만들면 외부 asset을 로드하지 않습니다. Component bytes는 필요한 plan을 compile하기
전에 `loadComponentResource`로 나중에 추가할 수도 있습니다.
Package의 `prepack` 검사는 WASM과 version이 맞는 component를 다시 만들고 Node·TypeScript smoke와
pack asset 목록을 검증합니다.

패키지는 아직 registry에 게시하지 않았습니다. 로컬에서 배포 산출물을 생성하고 검사할 수
있습니다.

```sh
pnpm --dir packages/kfind run pack:check
```

## 개발

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/benchmark-criterion.sh
scripts/benchmark-morphology.sh
pnpm --dir packages/kfind run benchmark:startup
pnpm --dir packages/kfind run pack:check
```

형태론 fixture에는 일치·불일치 회귀 사례 588개가 있습니다. Docker 벤치마크는 UD
Korean-Kaist에서 수동 검토해 뽑은 1,000개 사례로 `kfind`를 측정하고, 고정된 Kiwi, Lindera,
MeCab-ko, KOMORAN snapshot과 비교합니다. Canonical 점수에는 표준 맞춤법을 수동 검토한 문장만
사용합니다. 별도 Robust set은 UD Korean-KSL의 실제 오류 문장을 전부 수동 검토해 만든
250 positive·250 negative이며, `robustness=off`에서 제품별 품질과 성능을 함께 보고합니다.
제외한 Korean-Kaist 문장은 점수 없는 sentence registry로 보존합니다. fuzz target과 고정 seed
corpus는 `fuzz/`에 있습니다. CI는 `scripts/run-fuzz.sh`로 모든 target을 target당 15초 동안
실제 실행합니다.

구현 계약과 릴리스 인수 기준은 [`specs/kfind.md`](specs/kfind.md)에 있습니다.

## 라이선스

kfind 소스 코드와 프로젝트가 작성한 데이터는 [MIT 라이선스](LICENSE)를 따릅니다.
Homebrew full POS·component resource는 `mecab-ko-dic`의 Apache-2.0 고지를,
enriched predicate data는 CC BY-SA 2.0 Korea 고지를 `share/doc/kfind/LICENSES`에 별도로
보존합니다. Formula metadata는 이 조합을 SPDX 식으로 표현할 수 없어
`license :cannot_represent`를 사용합니다. 벤치마크 image의 UD 원문과 파생 fixture는 source별
CC BY-SA 4.0 조건을 따릅니다.

## 릴리스

일치하는 `vX.Y.Z` tag를 push하면 릴리스 workflow가 실행됩니다. Full POS·component resource를
다시 빌드하고 검증한 뒤 source·data·CLI 산출물과 npm package를 게시하고
`SeokminHong/homebrew-brew`에 Formula PR을 엽니다. Prerelease는 npm `next`, stable release는
`latest` tag를 사용합니다. Tap의 `pr-pull` label은 Formula test가 통과한 뒤에만 적용됩니다.

릴리스 workflow에는 tap 쓰기 권한이 있는 `TAP_GITHUB_TOKEN` secret이 필요합니다. Cargo
package metadata의 MIT license도 게시 전에 검증합니다.
