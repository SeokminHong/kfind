# 사용법과 CLI 계약

이 문서는 [kfind 기술 사양서](kfind.md)의 현행 규범적 계약이다.

## 0. 제품 계약

이 절은 뒤의 일반 설명보다 우선한다.

### 0.3 CLI 세부 정책

- `smart` query plan에 source component 또는 인접 token 구조 근거가 필요한
  `CandidateProgram`이 하나라도 있으면 matcher 초기화 전에
  `morphology-component-compact.kfc`를 resolve하고 검증한다. resource 누락·손상·schema 또는
  source mismatch는 기존 경계 판정으로 fallback하지 않고 초기화 오류와 exit code 2를 반환한다.
  resource-required program이 없는 계획은 이 resource를 열지 않는다.
- 명시적 `--data-dir`과 `KFIND_DATA_DIR`은 full POS lexicon과 compact component resource가
  함께 있는 디렉터리를 뜻한다. component resource가 필요한 계획에서 해당 파일이 없으면 다른
  후보 경로를 탐색하지 않는다. 자동 탐색은 executable prefix, XDG data, 개발 경로와 Homebrew
  `share/kfind` 순서를 기존 full POS 정책과 공유한다.
- `--embedded`가 아니면 `predicates.enriched.tsv`를 같은 data 경로에서 선택적으로 탐색한다.
  파일이 없으면 core와 full POS만으로 계속 실행하고, 파일이 있으면 전체를 검증한 뒤 query를
  컴파일한다. 명시적 `--data-dir`에도 enriched 파일은 선택 사항이다.
- component 초기화가 실패하면 `--explain-query`와 JSON match 출력도 생성하지 않는다. locale이
  적용된 오류를 stderr에 쓰되 파일 경로와 decoder 오류의 control character escape 정책을
  유지한다.
- 전역 `--pos`와 atom 태그를 함께 사용하면 같은 품사일 때만 허용하고, 다르면 컴파일 오류를 낸다.
- `--literal`은 `--expand literal --pos literal`의 단축 옵션이며 상충하는 `--expand` 또는 `--pos`와 함께 사용할 수 없다.
- `--embedded`는 full POS lexicon과 enriched 용언 데이터를 resolve하거나 읽지 않는다. compact
  component resource의 로드 여부는 `CandidateProgram`이 선언한 resource capability로
  결정한다. `--explain-query`는
  full POS 상태를 `not required (embedded mode)`로 출력한다.
- `--init`과 `--uninstall`은 검색과 분리된 agent 통합 관리 mode다. 이 mode에서는 query가
  필요 없고 검색 옵션·경로를 함께 받을 수 없다. 두 mode는 함께 사용할 수 없다. `--agent`는
  `claude-code`, `codex`, `gemini`, `custom`을 반복해서 받을 수 있으며 통합 관리 mode 없이
  사용할 수 없다. `custom`은 `--init`에서만 유효하다.
- 통합 관리 mode에 `--agent`가 없고 stdin과 진단 출력이 TTY이면 checkbox multi-select를
  표시한다. 제거 선택에는 파일을 설치하는 세 agent만 표시한다. stdin이 TTY가 아니면 공백
  또는 줄바꿈으로 구분된 agent 이름을 읽는다. 비대화형 입력이 비었거나 알 수 없는 이름이
  있으면 변경하지 않고 exit code 2로 종료한다.
- 프로젝트 skill 경로는 실행한 현재 디렉터리를 기준으로 Claude Code
  `.claude/skills/kfind/SKILL.md`, Codex `.agents/skills/kfind/SKILL.md`, Gemini CLI
  `.gemini/skills/kfind/SKILL.md`다. `custom`은 파일을 만들지 않고 같은 `SKILL.md` 원문만
  stdout에 출력한다. 진행 메시지와 오류는 stderr에만 출력한다.
- Claude Code, Codex와 Gemini CLI 대상은 각각 `.claude/settings.json`,
  `.codex/hooks.json`, `.gemini/settings.json`에 kfind agent hook을 함께 설치한다. 세 agent의
  `SessionStart`에는 한국어 형태 검색에서 설치된 kfind skill과 `kfind`를 사용하라는
  지침을 주입한다. 정확한 표면형 검색은 `kfind --literal` 또는 `rg -F`, `grep -F`,
  `git grep -F`, `fgrep`과 IDE의 고정 문자열 검색을 허용한다. agent가 지원하는 시작·재개·초기화 시점마다 같은 지침을
  다시 주입한다.
- 실행 전 hook은 `.claude/settings.json`의 `PreToolUse/Bash`, `.codex/hooks.json`의
  `PreToolUse/Bash`, `.gemini/settings.json`의 `BeforeTool/run_shell_command`에 설치한다.
  기존 JSON 설정과 다른 hook은 보존하며, JSON이 올바르지 않거나 병합할 필드의 자료형이
  agent 계약과 다르면 어떤 파일도 변경하지 않고 충돌 경로를 포함한 오류와 exit code 2를
  반환한다. 같은 kfind hook은 다시 실행해도 중복하지 않는다.
- 실행 전 agent hook은 shell tool의 명령행에서 직접 실행되는 `rg`, `grep`, `egrep`, `fgrep`과
  `git grep`의 명시적 command-line 검색 패턴을 식별한다. 검색 패턴에 현대·옛한글 음절 또는
  자모가 있고 고정 문자열 모드를 명시하지 않았으면 tool 실행을 거부하고 kfind 사용법을
  agent에 반환한다. `-F`, `--fixed-strings`와 `fgrep`은 허용하며 옵션 값·패턴·`--` 뒤의
  경로에 들어 있는 `-F`는 모드 지정으로 해석하지 않는다. 고정 모드를 해제한 검색은 차단한다. 경로, glob, option
  값과 pattern file의 내용은 검색 패턴으로 간주하지 않는다. kfind 명령과 한글이 없는
  literal 검색은 허용한다.
- Agent hook은 각 agent가 신뢰한 project hook과 관측 가능한 shell tool call에만 적용된다.
  사용자가 hook을 신뢰하지 않거나 비활성화한 환경, agent hook을 거치지 않는 hosted tool과
  사용자가 별도 terminal에서 직접 실행한 명령은 차단하지 않는다. `custom`은 agent별 hook
  계약을 알 수 없으므로 hook 설정을 만들지 않는다.
- init이 만든 파일·link는 다시 실행할 때 같은 배포본으로 갱신할 수 있다. 관리 표식이 없는
  기존 skill을 덮어쓰지 않으며 충돌 경로를 포함한 오류와 exit code 2를 반환한다.
- `--uninstall`은 선택한 agent의 kfind 관리 skill과 `kfind --agent-hook` handler들만 제거한다.
  다른 skill 파일, agent 설정 key와 hook handler는 보존한다. 설정 파일이 kfind hook만 담고
  있으면 파일도 제거하며, 다른 설정이 있으면 남은 JSON을 같은 권한으로 다시 쓴다. 관리
  skill이나 hook이 없으면 변경 없음으로 성공한다. 관리 표식이 없는 skill, 올바르지 않은 JSON
  또는 제거 경로의 잘못된 자료형을 만나면 선택한 어떤 파일도 변경하지 않고 충돌 경로를
  포함한 오류와 exit code 2를 반환한다.
- 사람이 대화형으로 사용하는 기본 경로는 `--pos auto --boundary smart`를 유지한다. 설치된
  full POS lexicon을 자동으로 사용하고, 없으면 core lexicon preview 상태로 계속 실행한다.
- 에이전트 자동화는 모든 형태 atom에 품사를 명시하고 `--boundary any --embedded --json`을
  사용한다. 단일 품사 query는 `--pos`, 혼합 phrase는 atom 태그로 품사를 지정한다. CLI는
  사람의 무품사 입력을 위해 `--pos` 생략을 허용하지만, 에이전트 통합 계약에서는 이를
  잘못된 호출로 취급한다.
- 배포용 agent skill의 description은 한국어 표제어·활용형 검색을 선택 조건으로 선언하고
  정확한 표면형 검색은 고정 문자열 도구로 실행할 수 있음을 명시한다. 본문은 README나 `--help`를 별도로 읽지 않아도 에이전트가 검색을
  실행할 수 있어야 한다. 단일·혼합 품사 query와 literal 검색, 전체 `--pos` 값과 atom 태그,
  phrase의 순서·거리, `embedded + any + JSON Lines` 권장 경로, path·glob 축소, JSON
  span·provenance와 종료 코드를 간결한 예시와 함께 설명한다.
- man page와 한국어 README는 사람용 기본 경로와 에이전트 자동화 경로를 구분해 안내한다.
  README는 `--help`를 별도로 읽지 않아도 검색 기능, 쿼리 문법, 옵션의 값·기본값·주요 충돌,
  출력 형식과 종료 코드를 이해할 수 있어야 한다. README에는 현재 제품 동작과 안정적인 사용
  지침만 둔다. 측정일·Git revision, baseline/candidate 증감, 날짜별 보고서·작업 목록, 완료
  이력과 측정 snapshot 표·차트는 넣지 않는다. 재현 명령과 측정 결과는 날짜별 benchmark
  보고서에 보존하고 README는 benchmark 계약 문서만 안내한다. 승인된 보고서나 생성 차트가
  바뀌어도 측정 수치를 README로 복사하지 않으며, 사용자에게 설명할 현재 기능·제약이 달라진
  경우에만 README의 동작 설명을 갱신한다.
- 저장소의 README, 사양서, 도구 안내와 배포 문서는 한국어로 작성한다. 코드 식별자, 명령,
  API 이름과 고유 명칭은 원문 표기를 허용한다. 웹 문서는 한국어 원문과 영어 번역을 같은
  정보 구조로 제공한다.
- 현재 문서는 변경 전 상태, 개선 서사, 날짜·커밋별 증감과 완료 이력을 포함하지 않는다.
  날짜·Git revision·baseline/candidate 비교, 실험 결과와 측정에 근거한 개발 결정은
  `docs/benchmarks`에 보존한다. 그 밖의 개발 결정 기록은 `docs/decisions`에 보존한다.
- `--column`은 v0.1 정식 옵션이며 1부터 시작하는 Unicode scalar 열을 출력한다.
- `--count`는 파일별로 검증된 span이 하나 이상 있는 줄의 수를 출력한다.
- 일반 text 결과를 interactive terminal의 stdin/stdout에서 쓰면 내장 TUI pager를 자동으로
  사용한다. Interactive terminal은 POSIX TTY와 Windows console/ConPTY를 포함한다. 검색 시작과
  함께 TUI를 열고 완성된 결과 행을 점진적으로 반영한다. 화면 너비를 넘는 match
  줄은 검증된 match마다 별도 행으로 펼치고 각 행의 target이 보이도록 앞뒤를 생략한다. target의
  화면 위치는 원문에서 target 앞뒤가 차지하는 비율을 따르되, 양쪽 원문이 모두 남아 있으면 가용
  문맥의 20–80% 안으로 제한한다. terminal resize 때 너비, 생략 위치와 행 분할을 다시 계산하며
  위·아래 화살표는 이 행 단위를 이동한다. 마지막 행은 content viewport의 마지막 행에 놓이는
  지점까지만 이동한다. 키 반복 입력은 content viewport의 cell 수에 따라 16–48 ms 간격의 frame으로
  합치되 입력된 이동량은 보존하고, 새로 노출된 행만 갱신한다. `--no-pager`,
  non-TTY stdin/stdout, JSON Lines, count, 파일명
  요약과 quiet mode는 pager를 사용하지 않고 기존 bounded stdout stream을 유지한다. TUI를 시작할
  수 없을 때는 일반 text를 직접 stdout에 쓴다. 에이전트 권장 경로의 JSON Lines는 stdout이
  interactive terminal이어도 비대화형 출력을 유지한다. Windows CI는 ConPTY 안의 PowerShell에서
  TUI를 실행해 alternate screen 진입, resize 반영, 아래 화살표 이동, `q` 종료, alternate screen
  복구와 exit code 0을 검증한다.
- TUI는 완성된 source line마다 임시 파일 offset·length를, 현재 너비에서 전개된 화면 row마다
  source·target key를 메모리에 보존한다. 따라서 임시 파일과 별도로 source line 수와 전개된 row
  수에 비례한 index 메모리를 사용한다. 현재 자동 결과 상한이나 대용량 fallback은 없으며 대규모
  결과를 stream으로만 처리하려면 `--no-pager`를 사용한다.
- TUI index benchmark는 plain source line과 한 source line이 여러 match row로 전개되는 입력을
  각각 측정한다. 입력 bytes, source·row 논리 개수, 각 `Vec`의 length·capacity와 entry 크기,
  length·capacity 기준 index bytes, 생성·index·layout 시간과 fresh process peak RSS를 함께 보고한다.
  benchmark용 binary는 release 설치물에 포함하지 않는다.
- EUC-KR은 명시적 `--encoding euc-kr`에서 지원한다. `auto`는 BOM 기반 UTF-16과 UTF-8만 판별한다.

## 4. 사용자 사용법

### 4.1 기본 검색

```bash
kfind 걷다 .
kfind 사용자 src docs
kfind 예쁘다 README.md
```

활용 확장을 끄는 단축 옵션도 제공한다.

```bash
kfind --literal 걸어 .
```

`걷다`는 표제어 검색으로 해석하고 활용형을 확장한다.

### 4.2 품사 강제

```bash
kfind --pos verb 걷다 .
kfind --pos noun 새 .
kfind --pos determiner 새 .
kfind --pos literal 걸어 .
```

짧은 태그 문법도 지원한다.

```bash
kfind 'v:걷다' .
kfind 'n:권한 v:검증하다' src
kfind 'det:새 n:기능' docs
kfind 'lit:걸어' .
```

지원 태그:

```text
n:    noun
pro:  pronoun
num:  numeral
v:    verb
adj:  adjective
det:  determiner
adv:  adverb
j:    particle
intj: interjection
lit:  literal
```

### 4.3 구(句) 검색

```bash
kfind 'n:권한 v:검증하다' src --max-gap 24
```

다음과 같은 문장을 찾는다.

```text
권한을 검증했다.
권한 검증하는 코드를 확인한다.
권한을 먼저 확인한 뒤 검증한다.
```

원자 순서는 유지한다. 기본적으로 줄을 넘지 않으며, atom 사이 최대 거리는 Unicode scalar 기준으로 계산한다. `v:검증하다`는 `검증을 수행했다` 같은 의미적 바꿔쓰기를 검색하지 않는다. 그 경우 `n:검증`을 별도 atom으로 지정해야 한다.

### 4.4 검색 범위 제어

```bash
kfind 걷다 src --glob '*.rs' --glob '*.md'
kfind 걷다 . --hidden
kfind 걷다 . --no-ignore
kfind 걷다 . --type-add 'docs:*.{md,mdx,txt}' --type docs
```

### 4.5 해석 확인

```bash
kfind 걷다 --explain-query
kfind 걷다 src --explain-match
kfind 걷다 src --json
```

### 4.6 사람과 에이전트의 권장 경로

사람은 품사를 생략한 기본 검색을 사용할 수 있다. 이 경로는 auto 품사와 `smart` 경계를 사용하고,
설치된 full POS lexicon이 있으면 자동으로 조회한다.

```bash
kfind 걷다 src
kfind 사용자 src docs
```

에이전트는 검색어의 품사를 명시하고 `any`, embedded, JSON 출력을 함께 사용한다.

```bash
kfind --embedded --boundary any --pos verb --json 걷다 src docs
kfind --embedded --boundary any --json 'n:사용자 v:검증하다' src
```

이 경로는 query-side 형태 확장과 부분 문자열 후보를 빠르게 반환한다. 에이전트는 결과 문맥을 읽고
false positive를 제거해야 한다. 후보가 너무 많으면 검색 path·glob을 좁히거나 `smart`로 다시
검색한다.

## 13. 인코딩과 바이너리 정책

기본 인코딩은 UTF-8이다.

```text
--encoding auto
--encoding utf-8
--encoding utf-16le
--encoding utf-16be
--encoding euc-kr   선택 지원
```

`auto`는 BOM이 있는 UTF-16을 감지한다. EUC-KR 자동 추정은 하지 않는다.

잘못된 UTF-8이 섞인 파일은 바이트 검색 자체는 가능하지만, 한국어 program 판정은 유효 UTF-8 구간에서만 수행한다.

JSON Lines 출력에서 원문 줄을 UTF-8로 표현할 수 없으면 다음 중 하나를 사용한다.

```json
{ "text": null, "text_base64": "...", "encoding": "bytes" }
```

## 14. CLI 사양

### 14.1 기본 구문

```text
kfind [OPTIONS] <QUERY> [PATH]...
kfind --init [--agent <AGENT>]...
kfind --uninstall [--agent <AGENT>]...
```

통합 관리 mode를 사용하지 않으면 query가 필수다. PATH를 생략하면 현재 디렉터리를 검색한다.
stdin이 pipe이면 기본 검색 대상을 stdin으로 전환한다. `-`는 stdin을 명시한다.

### 14.2 주요 옵션

| 옵션                      | 값                                         |              기본값 | 설명                              |
| ------------------------- | ------------------------------------------ | ------------------: | --------------------------------- |
| `--pos`                   | 품사                                       |              `auto` | 쿼리 전체 품사 강제               |
| `--expand`                | `literal`, `inflection`, `derivation`      |        `inflection` | 확장 수준                         |
| `--boundary`              | `smart`, `token`, `any`                    |             `smart` | 경계 정책                         |
| `--embedded`              | flag                                       |               false | full POS lexicon을 로드하지 않음  |
| `--max-gap`               | 정수                                       |                `24` | phrase atom 사이 최대 거리        |
| `--unicode-normalization` | `nfc`, `canonical`, `none`                 |               `nfc` | Unicode 검색 모드                 |
| `--encoding`              | 인코딩                                     |              `auto` | 원문 인코딩                       |
| `--glob`                  | glob                                       |                없음 | 파일 포함·제외 규칙               |
| `--type`, `--type-add`    | 파일 유형                                  |                없음 | 파일 유형 필터                    |
| `--hidden`                | flag                                       |               false | hidden 파일 포함                  |
| `--no-ignore`             | flag                                       |               false | ignore 규칙 무시                  |
| `--threads`               | 정수                                       |                자동 | worker 수                         |
| `--count`                 | flag                                       |               false | 파일별 match 수                   |
| `--files-with-matches`    | flag                                       |               false | 파일명만 출력                     |
| `--json`                  | flag                                       |               false | JSON Lines 출력                   |
| `--color`                 | `auto`, `always`, `never`                  |              `auto` | 터미널 색상                       |
| `--no-pager`              | flag                                       |               false | TTY에서도 pager를 사용하지 않음   |
| `--explain-query`         | flag                                       |               false | 쿼리 계획 출력                    |
| `--explain-match`         | flag                                       |               false | 생성 근거 출력                    |
| `--explain-no-match`      | flag                                       |               false | 0건일 때 재검색 후보 출력         |
| `--sort`                  | `path`                                     |                없음 | 결과 정렬                         |
| `--data-dir`              | 경로                                       |                자동 | 외부 데이터 디렉터리              |
| `--user-lexicon`          | 경로                                       |                자동 | 사용자 사전                       |
| `--init`                  | flag                                       |               false | 현재 디렉터리에 agent 통합 초기화 |
| `--uninstall`             | flag                                       |               false | 현재 디렉터리의 agent 통합 제거   |
| `--agent`                 | `claude-code`, `codex`, `gemini`, `custom` | TTY 선택 또는 stdin | 통합 관리 대상, 반복 가능         |

### 14.3 context와 출력 호환 옵션

다음은 익숙한 CLI 사용성을 위해 지원한다.

```text
-n, --line-number
-H, --with-filename
-h, --no-filename
-C, --context
-B, --before-context
-A, --after-context
-l, --files-with-matches
-c, --count
-q, --quiet
```

정규식 호환을 의미하지 않으며, 출력과 파일 검색 UX만 비슷하게 제공한다.

### 14.4 종료 코드

```text
0: 하나 이상의 match
1: match 없음
2: 사용법, I/O, 데이터, 컴파일 오류
```

`--explain-no-match`는 사람이 읽는 출력에서 검색이 정상 완료되고, 파일을 하나 이상 검색했으며
match가 0건일 때만 stderr에 재검색 후보를 출력한다. 기본 결과와 종료 코드는 바꾸지 않고
다른 경계·품사 설정으로 자동 재검색하지 않는다. 현재 경계가 `any`가 아니면
`--boundary any`를, 품사를 지정하지 않았으면 명시적 품사 지정 검토를 제안한다.
`--embedded`를 사용 중이면 그 옵션을 제거한 재검색을, 필요한 full POS 사전이 없으면
`--check-data`로 상태를 확인하도록 안내한다. 제안은 성공 여부를 검증한 결과가 아님을
분명히 표시한다. 검색 오류, 구조 검증 미완료, 검색한 파일 0개, 닫힌 stdout에서는 출력하지
않는다. `--json`과 `--quiet`에는 사용할 수 없다.

### 14.5 표시 언어

사람이 읽는 도움말, 인수 파싱 오류, 런타임 오류, 검색 진단과
`--explain-query`·`--explain-match`의 설명 레이블은 영어와 한국어를 지원한다.
표시 언어는 비어 있지 않은 첫 환경 변수를 다음 순서로 선택한다.

```text
LC_ALL
LC_MESSAGES
LANG
```

선택한 locale의 언어 구성 요소가 대소문자 구분 없이 `ko`이면 한국어를 사용한다.
`ko`, `ko_KR`, `ko-KR`, `ko_KR.UTF-8`, `ko_KR.UTF-8@modifier`를 같은 언어로
처리한다. `C`, `POSIX`, 미설정 값, 지원하지 않거나 해석할 수 없는 locale은 영어로
대체한다. 우선순위가 높은 값이 비어 있지 않으면 지원하지 않는 locale이더라도 낮은
우선순위 변수로 내려가지 않는다.

옵션명, 옵션 값, 파일 경로, 규칙 ID, JSON Lines의 필드명과 값, 종료 코드는 locale과
무관하게 유지한다. 운영체제와 외부 라이브러리가 제공하는 상세 오류 문구는 kfind가
생성한 현지화된 오류 문맥 뒤에 원문으로 붙일 수 있다. man page와 shell completion은
빌드 환경의 locale에 영향받지 않도록 영어 명령 정의에서 재현 가능하게 생성한다.

### 14.6 Agent 통합 관리

명시적 대상은 대화형 여부와 관계없이 같은 결과를 만든다.

```sh
kfind --init --agent codex --agent claude-code
```

비대화형 stdin은 `--agent` 반복 옵션과 같은 agent 이름 집합을 받는다.

```sh
printf 'codex\nclaude-code\n' | kfind --init
```

`custom`은 다른 대상과 함께 선택할 수 있다. stdout에는 조합용 skill 원문만 쓰므로 다음처럼
임의 경로로 보낼 수 있다.

```sh
kfind --init --agent custom > path/to/kfind/SKILL.md
```

TTY에서 선택을 취소하거나 아무 항목도 선택하지 않으면 파일을 변경하지 않고 성공한다. 같은
agent를 여러 번 입력해도 한 번만 처리한다. 설치가 하나라도 실패하면 성공으로 보고하지 않는다.

지원 agent를 선택하면 skill과 두 종류의 project hook을 함께 설치한다. `SessionStart` hook은
skill이 자동 선택되지 않아도 한국어 형태 검색에 kfind를 사용하라는 지침을 agent
context에 추가한다. shell tool 실행 전 hook은 고정 문자열 모드를 명시하지 않은 한국어
pattern의 일반 text search 명령을 차단한다. 기존 설정 파일에는 kfind hook만 병합하며 다른 key와 hook 순서를 보존한다. Codex의
project hook은 프로젝트를 신뢰한 뒤 `/hooks`에서 별도로 신뢰해야 실행된다. Claude Code와
Gemini CLI도 각 제품의 project hook 신뢰 절차를 따른다.

통합을 제거할 때도 같은 대상 선택 방식을 사용한다.

```sh
kfind --uninstall --agent codex --agent claude-code
printf 'codex\ngemini\n' | kfind --uninstall
```

제거는 kfind 관리 표식이 있는 skill 또는 kfind가 만든 Homebrew link와
`kfind --agent-hook` handler들만 대상으로 한다. 다른 설정이 없는 kfind 전용 JSON 파일은
제거하고, 설정이 함께 있으면 kfind handler만 뺀 JSON을 보존한다. 이미 제거된 대상을 다시
지정해도 성공한다. `custom`은 파일을 설치하지 않으므로 제거 대상이 아니다.

Agent는 session을 시작할 때 한국어 형태 검색에 설치된 skill과 `kfind`를 사용하라는
지침을 받는다. 정확한 표면형 검색은 `rg -F`, `grep -F`, `git grep -F`, `fgrep`도 허용한다. 그래도 다음 shell tool call을 만들면 실행 전에 거부하고
`kfind`로 다시 검색하도록 안내한다.

```sh
rg '사용자' crates
grep -R --regexp='검증하다' docs
git grep '권한'
```

검색 pattern과 구분되는 한글 경로·glob은 거부하지 않는다.

```sh
rg 'TODO' '한국어 문서'
rg --glob '*한글*' 'TODO' .
```

## 15. 출력 사양

### 15.1 기본 출력

```text
src/walk.rs:42: 길을 걸어 갔다.
```

열 번호는 기본적으로 생략할 수 있다. `--column`에서만 match 줄의 앞부분을 Unicode scalar로 세어 계산한다.

일반 text 결과를 interactive terminal의 stdin/stdout에서 쓰면 검색 시작과 동시에 내장 TUI
pager를 열고, 완성된 결과 행을 점진적으로 반영한다. POSIX TTY와 Windows console/ConPTY는 같은
계약을 사용한다. PowerShell에서 실행한 native Windows binary도 Windows Terminal의 ConPTY 안에서
terminal 크기와 key event를 읽고 alternate screen과 raw mode를 종료 시 복구한다. 검색 중에도
이동과 resize를 처리하며 상태 행에 검색 중임을 표시한다. 검색 완료 뒤 너비와 높이가 모두 한
화면에 들어가면 바로 종료하고 terminal 내용을 남긴다. 한 줄이라도 잘리거나 결과가 화면 높이를
넘으면 TUI를 유지하며 `↑`/`↓` 또는 `k`/`j`로 한 행씩 이동하고 `q` 또는 `Esc`로 종료한다. 이동
offset은 content viewport의 첫 행이며 최대값은 `전체 행 수 - viewport 높이`다. 따라서 마지막
행만 화면 위에 남기고 아래를 비우는 위치까지는 이동하지 않는다. 키 반복 중 한 frame에 쌓인
이동은 한 번에 반영하고, 연속 행 이동은 기존 화면을 유지한 채 새로 노출된 행과 상태 행만
갱신한다. Frame 간격은 content viewport 8,192 cells마다 16 ms씩 늘리되 48 ms를 넘지 않는다.
따라서 73×316 terminal의 72×316 content viewport는 48 ms 간격을 사용하며, 반복 입력을 합쳐도
최종 이동 offset은 같다. 검색 중 종료하면 결과 출력과 남은 검색을 중단한다.

화면 너비를 넘지 않는 match 줄은 source line 하나를 한 행으로 유지하고 모든 match를 강조한다.
화면 너비를 넘는 match 줄은 source 순서대로 `PhraseMatch` 하나당 한 행을 만든다. 각 행은 target
match의 전체 span이 content 너비 이하면 모두 보이도록 앞뒤 원문을 `…`로 생략하고 target에 속한
token만 강조한다. target span 자체가 content 너비보다 길면 span 중앙을 기준으로 보이는 구간을
잡는다. target 앞뒤의 가용 문맥은 전체 원문에서 target 앞뒤가 차지하는 비율로 나누되 양쪽에 원문이 남아 있으면
각각 최소 20%를 보장한다. 파일 경로 prefix가 content 영역을 잠식하면 prefix의 왼쪽을 먼저
생략하며 prefix는 화면 너비의 40%를 넘지 않는다. `--column`은 분리된 각 행의 target column을
표시한다. match가 없는 긴 context·설명 행은 앞부분을 유지하고 끝을 생략한다.

terminal resize는 현재 보고 있는 source line과 target match를 기준점으로 유지하면서 행 분할,
prefix와 content window를 다시 계산한다. 축소되어 source line이 잘리면 match별 행으로 펼치고,
확대되어 전체 line이 들어오면 다시 한 행으로 합친다. `--no-pager`, 명시적 stdin path `-`, non-TTY
stdin/stdout과 구조화·요약 출력은 pager를 거치지 않으며 원문 line을 생략하거나 match별로 복제하지 않는다.

pager의 임시 파일은 출력 bytes를 보존하고, 메모리에는 완성된 source line의 파일 위치와 현재
layout row key를 각각 연속 벡터로 보존한다. index 메모리는 두 벡터의 capacity에 비례하며 terminal
resize 때 layout 벡터를 다시 만든다. 자동 상한은 두지 않고 `--no-pager`가 bounded stdout stream
경로를 제공한다. 대규모 측정은 0.3절의 TUI index benchmark 계약을 따른다.

### 15.2 쿼리 설명

```text
query: 걷다
atom[0]:
  analyses:
    - lemma: 걷다
      pos: verb
      alternation: DToL
      source: builtin-lexicon
  programs: 12
  anchors:
    - 걷고
    - 걷는
    - 걷지
    - 걸어
    - 걸었
    - 걸으
  consumption_states: 8
  normalization: nfc
  estimated_matcher_bytes: 4288
```

### 15.3 match 설명

```text
sample.txt:3: 길을 걸었습니다.
  token: 걸었습니다
  core: 걸
  generated_from: 걷다
  rules:
    - lexical.d-to-l
    - ending.past
    - ending.polite-declarative
```

### 15.4 JSON Lines 출력

```json
{
  "type": "match",
  "path": "sample.txt",
  "line": 3,
  "text": "길을 걸었습니다.",
  "spans": [
    {
      "core": { "start": 7, "end": 10 },
      "token": { "start": 7, "end": 22 },
      "surface": "걸었습니다",
      "origins": [
        {
          "lemma": "걷다",
          "pos": "verb",
          "rules": [
            "lexical.d-to-l",
            "ending.past",
            "ending.polite-declarative"
          ]
        }
      ]
    }
  ]
}
```

유효한 UTF-8 text의 offset은 `utf8-bytes`, raw byte text의 offset은 `bytes`로 명시한다. 선택적으로 scalar column도 제공한다.
