# kfind 기술 사양서

워크스페이스 버전: 0.3.0-rc.3
문서 역할: 현재 구현과 호환성 계약

이 문서는 현재 제품 계약만 유지한다. 완료한 작업 순서, 폐기한 대안과 배포 운영 상태는
누적하지 않는다. 재현 가능한 시점별 측정값은 `docs/benchmarks`의 보고서에 둔다.

## 0. 제품 계약

이 절은 현재 구현에 적용되는 세부 계약이다. 아래 내용은 뒤 절의 일반 설명보다 우선한다.

### 0.0 제품 목적과 우선순위

- `kfind`의 주 목적은 에이전트와 사람이 한국어 표제어·활용형을 파일 또는 메모리 text에서
  빠르게 찾아 후속 판단에 사용할 후보 span과 생성 근거를 얻는 것이다. 형태 지식은 query를
  유한한 검색 계획으로 만드는 데 사용하며 corpus 전체를 분석하는 제품으로 확장하지 않는다.
- 에이전트 workflow는 명시적 품사, 넓은 경계, 구조화된 출력으로 recall과 scan 처리량을
  우선하고 caller가 문맥으로 false positive를 제거한다. 사람 workflow는 자동 품사와 `smart`
  경계로 precision과 사용성을 우선한다. CLI와 Rust·JavaScript library는 두 workflow에서 같은
  query·span·provenance 계약을 제공한다.
- 명시적 품사와 `smart`를 함께 사용하는 검색은 false negative를 false positive보다 우선해
  줄인다. 고정 development fixture에서 FN을 먼저 최소화하고, 같은 FN이면 FP가 적은 후보를
  선택한다. precision 99.00% 하한과 version-controlled hard-negative의 신규 FP 0은 유지한다.
- 무품사 검색은 query 의도를 하나의 품사로 확정할 수 없는 한계를 결과에 포함한다. 이 한계를
  감추기 위해 fixture, gold, negative 선택이나 지표 정의를 현재 구현 결과에 맞춰 바꾸지 않는다.
- 제품 goal은 짧은 query의 bounded compile, 대규모 text의 빠른 scan, 지원하는 형태 범위의
  검증된 recall·precision, 재현 가능한 결과와 offline 실행이다.
- 일반 목적 문장 형태소 분석·tokenization, 형태소 분석기 자체의 최고 처리량 경쟁, 문맥 의미
  판별, semantic search와 임의 표면형의 완전한 역분석은 non-goal이다. 외부 형태소 분석기 비교는
  제품 workflow의 품질·비용을 보정하는 근거이며 동일한 tokenizer backend 순위를 뜻하지 않는다.
- `smart`는 corpus 전체를 분석하지 않지만, 동일 표면의 형태 경로가 충돌할 때 compact component
  resource가 증명하는 바로 인접한 어절의 구조를 bounded evidence로 사용할 수 있다. 이 판정은
  query와 독립적이어야 하며, 구조가 모호하거나 resource로 증명되지 않으면 기존 결과를 유지한다.

### 0.1 규칙 데이터와 품질 기준

- v0.1의 필수 형태 범위는 9.5절의 활용표, 19.2절의 필수 테스트, 23절의 인수 기준을 모두 포함한다.
- gold corpus에 포함된 현재 평서형 `-ㄴ다/는다`, 회상 관형형 `-던`, 양보 연결형 `-더라도`, 과거 관형 연쇄 `-았/었을`, 과거 의문 종결 연쇄 `-았/었느냐`, `-았/었느냐는`, 이유 연결형 `-(으)니`, 인용 연결형 `-다고`, 전망 인용 연쇄 `-(으)리라고`, 의도 연결형 `-(으)려고`, 상태 변화 보조 용언 `-아/어지다`, 진행 방향 보조 용언 `-아/어가고`, `-아/어가야`도 v0.1의 제한된 continuation vocabulary에 포함한다.
- 실제 코퍼스에서 확인된 해요체 과거형 `-았어요/-었어요`, 지정사 `이다`의 높임 평서형 `입니다`, 부정 지정사 `아니다`의 연결형 `아니라`도 v0.1의 제한된 continuation vocabulary에 포함한다.
- 어미, 조사 연쇄, 파생 규칙은 저장소에서 버전을 관리하는 `data/rules` 파일의 목록과 전이를
  기준으로 삼는다. 목록 밖 조합은 생성하지 않는다.
- full POS lexicon은 `mecab-ko-dic 2.1.1-20180720`의 Apache-2.0 데이터를 bootstrap 원본으로 사용한다. 빌드 시 표제어와 품사만 추출하고, 런타임 문장 분석 데이터와 알고리즘은 포함하지 않는다. `Inflect`와 `Preanalysis` 행은 제외하며, 문맥용 지정사 표면형은 표제어로 승격하지 않고 `VCP=이`, `VCN=아니`만 기본형으로 정규화한다.
- full POS lexicon의 용언 품사 후보도 POS 전용 산출물에 보존한다. 동일 표제어와 coarse 품사에
  core 또는 enriched 용언 분석이 하나라도 있으면 그 coarse 품사의 full POS 규칙형 분석은
  추가하지 않는다. 다른 coarse 품사는 보존한다. 그 밖의 용언은 해당 품사와 일치하는 생산적
  접미 규칙을 먼저 적용하고, 일치하는 규칙이 없을 때만 제한된 규칙형 분석을 사용한다.
- full POS runtime resource는 검증된 정렬 lookup index로 보존한다. CLI, Rust library와 WASM binding은 초기화할 때 전체 entry를 일반 분석 map으로 전개하지 않으며, query atom의 표제어를 조회할 때 일치하는 품사 후보만 `Analysis`로 만든다.
- 지연 조회에서도 기존 우선순위를 보존한다. core와 enriched 용언은 같은 표제어·coarse 품사의
  full POS 용언을 억제하고, 동일한 분석은 중복하지 않는다. user lexicon의 append는 full POS
  후보를 보존하며 `replace = true`는 해당 morphology category의 core, enriched와 full POS
  후보를 모두 대체한다.
- 명시한 coarse 품사와 일치하는 사전 분석이 없으면 해당 coarse 품사가 지원하는 세부 품사를
  모두 fallback 분석으로 만든다. `noun`은 보통명사·고유명사·의존명사를 보존하며 하나의
  보통명사 분석으로 축소하지 않는다. 같은 anchor·verifier를 만드는 분석은 branch를 합치되
  세부 품사 provenance는 모두 유지한다.
- 명시한 coarse `noun`에 full POS 사전 분석이 있으면 그 분석과 누락된 보통명사·고유명사·
  의존명사 fallback을 합집합으로 보존한다. full POS의 단일 세부 품사가 명시적 coarse 품사의
  다른 component 근거를 억제하지 않으며, user lexicon의 `replace = true`는 이 합집합보다
  우선한다.
- core lexicon은 전체 표제어 목록이 아니라 불규칙 활용, 품사 중의성, 기능어, 표면형 override를 담는 예외 계층이다. embedded workflow의 검증된 주요 불규칙은 core에 유지한다. 공개 사전에서
  일괄 승격한 활용 metadata는 별도 enriched 계층으로 관리하며, core entry 수를 corpus recall에
  맞춰 무제한 늘리지 않는다.
- core lexicon의 `DropH` 형용사는 검증된 ㅎ 불규칙 표제어를 명시한다. `어떻다`, `이렇다`,
  `커다랗다`는 각각 `어떤`, `이런`, `커다란` 관형형을 만들고 규칙형 `어떻은`, `이렇은`,
  `커다랗은`은 만들지 않는다.
- full POS 산출물은 전체 entry 수, 고유 표제어 수, 품사별 entry 수를 기계 판독 가능한 통계 파일로 포함한다. source를 추가하거나 갱신할 때는 이 통계와 충돌·제외 건수의 변화를 검토한다.
- 공개 사전은 고정된 전체 내려받기 snapshot만 릴리스 입력으로 사용한다. 원본 URL·버전 또는 생성 일자·SHA-256·라이선스·추출 필드·추출기 버전을 기록하며, 인증키가 필요한 live API 응답은 릴리스 빌드 입력이나 런타임 의존성으로 사용하지 않는다.
- 여러 source의 표제어·품사 후보는 합집합으로 보존하되, 같은 표제어에 core 용언 분석이 있으면 core의 활용 metadata를 우선한다. source 간 품사 충돌과 활용 분류 미확정 항목은 산출물 통계로 보고하고 임의로 한쪽을 삭제하지 않는다.
- 배포 데이터에는 원본 버전, 출처, 라이선스, 추출 명령과 체크섬을 기록한다.
- auto 품사 coverage 기준은 300개 이상의 프로젝트 gold case마다 명시된 기대 품사 분석을 포함하는 것이다. 품사별 형태 match와 no-match는 fixture 품사를 강제해 해당 분석의 허용·금지 형태를 검증하고, 품사를 생략한 제품 동작은 0.6절의 사람용 fixture와 persona profile로 분리한다. 핵심 불규칙 fixture는 core lexicon만으로도 100% 통과해야 한다.
- full POS lexicon이 없으면 core lexicon으로 계속 실행하되, `--explain-query`와 명시적 사전 진단 요청에서 `preview (core lexicon only)` 상태와 자동 탐색한 모든 후보 경로를 우선순위대로 출력한다. 로드했을 때는 `loaded`와 선택된 경로를 출력한다.
- `--explain-query`는 계획 전체의 Unicode 정규화 모드와 atom별 verifier state 수를 출력한다. verifier state 수는 해당 atom의 branch들이 참조하는 서로 다른 verifier 구성의 수다.

### 0.2 토큰 경계와 phrase 거리

- 토큰 문자는 Unicode 문자·숫자·결합 문자와 `_`다. 한글 완성형과 자모도 토큰 문자에 포함한다.
- `smart`는 품사 verifier가 허용된 조사·어미를 소비한 token span의 바깥 경계를 검사한다. 체언, literal, 한 음절 atom은 core 시작도 토큰 경계여야 한다. 단, 조사를 직접 검색할 때는 붙은 조사를 찾을 수 있도록 core 왼쪽 경계 대신 바로 앞 host와 조사 이형태 조건을 검증한다. 무품사 입력은 사용자가 쓴 조사 표면형만 찾고, 조사 이형태 묶음 확장은 명시적 조사 품사 입력에서만 사용한다.
- 일반 용언의 `smart` token span은 core에서 시작한다. 따라서 `가다` 검색은 `친구가`의 붙은 조사 `가`를 활용형으로 인정하지 않는다. 지정사처럼 앞 host에 붙는 분석만 별도 왼쪽 환경 검증을 사용한다.
- 명시적 동사·형용사 품사의 `ending.connective-ji` branch는 `smart`에서도 core 왼쪽 token 경계를
  요구하지 않고 완성된 token span의 오른쪽 경계는 유지한다. 이는 gold 어절의 오른쪽 끝과
  일치하는 suffix candidate만 복구한다. 무품사 `smart`, `token`, `any`와 `ending.connective-ji`
  뒤에 문자가 더 남는 left-edge candidate는 바꾸지 않는다.
- `token`은 모든 품사에서 core 시작과 완성된 token span 끝의 토큰 경계를 검사한다.
- `any`는 좌우 경계를 검사하지 않는다.
- phrase의 `max-gap`은 앞 atom의 `token.end`와 다음 atom의 `token.start` 사이에 있는 Unicode scalar 수다. 음수이거나 순서가 뒤집힌 span은 결합하지 않는다.

### 0.3 CLI 세부 정책

- `smart` query plan에 `NominalComponent`, `PredicateLexical` 또는 `LexicalContext` branch가 하나라도 있으면 matcher 초기화 전에
  `morphology-component-compact.kfc`를 resolve하고 검증한다. resource 누락·손상·schema 또는
  source mismatch는 기존 경계 판정으로 fallback하지 않고 초기화 오류와 exit code 2를 반환한다.
  component branch가 없는 계획은 이 resource를 열지 않는다.
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
  component resource의 로드 여부는 기존 `smart` query plan 정책을 유지한다. `--explain-query`는
  full POS 상태를 `not required (embedded mode)`로 출력한다.
- `--init`은 검색과 분리된 agent skill 초기화 mode다. 이 mode에서는 query가 필요 없고 검색
  옵션·경로를 함께 받을 수 없다. `--agent`는 `claude-code`, `codex`, `gemini`, `custom`을
  반복해서 받을 수 있으며 `--init` 없이 사용할 수 없다.
- `kfind --init`에 `--agent`가 없고 stdin과 진단 출력이 TTY이면 checkbox multi-select를
  표시한다. stdin이 TTY가 아니면 공백 또는 줄바꿈으로 구분된 agent 이름을 읽는다. 비대화형
  입력이 비었거나 알 수 없는 이름이 있으면 설치하지 않고 exit code 2로 종료한다.
- 프로젝트 skill 경로는 실행한 현재 디렉터리를 기준으로 Claude Code
  `.claude/skills/kfind/SKILL.md`, Codex `.agents/skills/kfind/SKILL.md`, Gemini CLI
  `.gemini/skills/kfind/SKILL.md`다. `custom`은 파일을 만들지 않고 같은 `SKILL.md` 원문만
  stdout에 출력한다. 진행 메시지와 오류는 stderr에만 출력한다.
- init이 만든 파일·link는 다시 실행할 때 같은 배포본으로 갱신할 수 있다. 관리 표식이 없는
  기존 skill을 덮어쓰지 않으며 충돌 경로를 포함한 오류와 exit code 2를 반환한다.
- 사람이 대화형으로 사용하는 기본 경로는 `--pos auto --boundary smart`를 유지한다. 설치된
  full POS lexicon을 자동으로 사용하고, 없으면 core lexicon preview 상태로 계속 실행한다.
- 에이전트 자동화는 모든 형태 atom에 품사를 명시하고 `--boundary any --embedded --json`을
  사용한다. 단일 품사 query는 `--pos`, 혼합 phrase는 atom 태그로 품사를 지정한다. CLI는
  사람의 무품사 입력을 위해 `--pos` 생략을 허용하지만, 에이전트 통합 계약에서는 이를
  잘못된 호출로 취급한다.
- 배포용 agent skill은 README나 `--help`를 별도로 읽지 않아도 에이전트가 검색을 실행할 수
  있어야 한다. 단일·혼합 품사 query와 literal 검색, 전체 `--pos` 값과 atom 태그, phrase의
  순서·거리, `embedded + any + JSON Lines` 권장 경로, path·glob 축소, JSON span·provenance와
  종료 코드를 간결한 예시와 함께 설명한다.
- man page와 영어·한국어 README는 사람용 기본 경로와 에이전트 자동화 경로를 구분해 안내한다.
  README는 `--help`를 별도로 읽지 않아도 검색 기능, 쿼리 문법, 옵션의 값·기본값·주요 충돌,
  출력 형식과 종료 코드를 이해할 수 있어야 한다. 최신 benchmark는 workload, 측정일·revision과
  원본 보고서 링크를 함께 요약하고, 품질·CLI 처리량·초기화 비용처럼 단위가 다른 지표를 분리한다.
  제품 persona와 고정 외부 분석기 snapshot을 비교하는 최신 표와 대표 차트도 README에 직접
  싣는다. persona별 입력의 품사 지정 여부, 외부 분석기 버전과 task workload 조건을 함께 적고,
  동일 입력의 형태소 분석기 순위나 순수 tokenizer 처리량으로 해석하지 않도록 비교 조건과
  해석 범위를 밝힌다.
  승인된 benchmark 보고서나 생성 차트가 바뀌면 영어·한국어 README의 요약 수치와 대표 차트도
  같은 변경에서 갱신한다. README 차트의 값과 source report가 일치하는지 검증한다.
- `--column`은 v0.1 정식 옵션이며 1부터 시작하는 Unicode scalar 열을 출력한다.
- `--count`는 파일별로 검증된 span이 하나 이상 있는 줄의 수를 출력한다.
- 일반 text 결과를 TTY stdin/stdout에서 쓰면 내장 TUI pager를 자동으로 사용한다. 검색 시작과 함께
  TUI를 열고 완성된 결과 행을 점진적으로 반영한다. 화면 너비를 넘는 match
  줄은 검증된 match마다 별도 행으로 펼치고 각 행의 target이 보이도록 앞뒤를 생략한다. target의
  화면 위치는 원문에서 target 앞뒤가 차지하는 비율을 따르되, 양쪽 원문이 모두 남아 있으면 가용
  문맥의 20–80% 안으로 제한한다. terminal resize 때 너비, 생략 위치와 행 분할을 다시 계산하며
  위·아래 화살표는 이 행 단위를 이동한다. 마지막 행은 content viewport의 마지막 행에 놓이는
  지점까지만 이동하며, 키 반복 입력은 frame 단위로 합쳐 새로 노출된 행만 갱신한다. `--no-pager`,
  non-TTY stdin/stdout, JSON Lines, count, 파일명
  요약과 quiet mode는 pager를 사용하지 않고 기존 bounded stdout stream을 유지한다. TUI를 시작할
  수 없을 때는 일반 text를 직접 stdout에 쓴다. 에이전트 권장 경로의 JSON Lines는 stdout이 TTY여도
  비대화형 출력을 유지한다.
- EUC-KR은 명시적 `--encoding euc-kr`에서 지원한다. `auto`는 BOM 기반 UTF-16과 UTF-8만 판별한다.

### 0.4 Web 문서와 playground

- 공개 문서와 playground는 `https://kfind.pages.dev`의 정적 Cloudflare Pages site로 배포한다.
  문서는 제품 목적과 goal/non-goal, 검색 model, query 문법, 사람·에이전트 workflow, 주요 옵션,
  최신 제품·외부 benchmark를 설명하고 전체 README와 source report로 연결한다.
- 한국어 문장은 기술 개념의 관계와 동작이 바로 드러나도록 쓴다. 제품·도메인의 표준 용어와
  코드 식별자는 원문 표기를 유지하되, 영어 문장 구조를 직역하거나 일반 용어를 기계적으로
  음역하지 않는다. 조건과 결과가 한 문장에 몰리면 문장을 나눠 설명한다.
- 각 문서 route는 전제와 용어를 먼저 정의하고, 원인·동작·결과와 적용 범위를 연속된 문단으로
  논증한다. 핵심 설명은 본문만 읽어도 완결되어야 하며, callout·card·도해와 단편적인 label의
  나열로 본문을 대신하지 않는다. 표, 도해와 코드 예시는 정확한 대응 관계나 실행 흐름을
  보충할 때만 사용하고 앞뒤 문단에서 해석한다.
- 문서 site는 React와 React Router의 data router로 구성한다. Cloudflare Pages의 SPA fallback을
  사용해 clean URL을 직접 열 수 있어야 하며, 공통 shell 안에서 다음 경로를 제공한다.

  ```text
  /                         개요와 제품 범위
  /guide/getting-started    설치와 첫 검색
  /reference/options        query 문법과 compile·search 옵션
  /reference/glossary       문서에서 반복하는 핵심 용어와 정의
  /concepts/analysis        query-directed 형태 분석 원리
  /concepts/architecture    compile·scan·verify 실행 구조
  /concepts/optimization    branch·anchor·resource·streaming 최적화
  /benchmarks               workload별 품질·성능 근거
  /playground               WebAssembly 검색 실습
  ```

- 단어장은 검색 입력, 실행 구조, resource와 품질 지표에 쓰는 핵심 용어를 한곳에서
  정의한다. 한국어 표기와 코드·영문 표기는 같은 항목에서 대응시키고, 다른 문서의 설명은
  이 정의와 모순되지 않아야 한다.
- 각 문서 route는 단어장 용어가 본문에서 처음 등장하는 한 곳에만 tooltip과 해당 정의 link를
  제공한다. 같은 항목의 한국어·영문 별칭은 한 용어로 센다. Tooltip은 hover와 keyboard focus로
  열 수 있어야 한다. Hover를 지원하지 않는 입력에서는 첫 pointer activation으로 tooltip을 열고,
  같은 용어의 다음 activation으로 단어장에 이동한다. Keyboard activation과 hover를 지원하는
  pointer의 link 동작은 유지한다. Code·기존 link·form control과 단어장 자체에는 중첩해서
  적용하지 않는다.
- 일반 UI text는 Pretendard 기반 sans-serif stack을 사용한다. 코드, 명령, query·output label과
  기술 도해의 코드 표기는 기존 monospace stack을 유지한다.
- 공통 spacing scale은 `0.25rem`, `0.5rem`, `0.75rem`, `1rem`, `1.5rem`과 section 간격
  `2.5rem`을 사용한다. 문서 카드와 playground panel은 이 scale로 padding과 gap을 제한하며,
  상태 badge와 짧은 token은 좁은 화면에서도 내용 너비만 차지한다.
- 각 route 구현은 지연 로드해 첫 문서 화면에 불필요한 페이지 코드가 포함되지 않게 한다.
  Playground의 WASM module과 선택적 component resource는 `/playground`에 들어가기 전에는
  불러오지 않는다. 문서 route 전환은 전체 페이지를 다시 요청하지 않고, 현재 경로와 제목을
  접근 가능한 navigation 상태로 표시한다.
- 좁은 화면의 문서 navigation은 두 열 grid로 배치한다. 각 navigation group은 링크 수와 관계없이
  내용 높이를 유지하며, 같은 grid 행의 다른 group 높이에 맞춰 내부 link를 늘리지 않는다.
- 옵션 문서는 `inflection`, `derivation`, `literal`의 생성 범위와 차이, `--literal` 단축 옵션의
  충돌 규칙, boundary·POS·Unicode normalization·phrase gap의 결합을 예제와 함께 설명한다.
  분석·아키텍처·최적화 문서는 query compile부터 anchor scan, 국소 verifier, span·provenance
  반환까지의 흐름과 corpus 전체를 분석하지 않는 이유를 텍스트와 접근 가능한 도해로 설명한다.
- playground는 현재 source의 `kfind-wasm`을 browser용 WebAssembly로 빌드해 embedded lexicon으로
  실행한다. Query, 입력 text, expand·boundary·POS·normalization·max gap을 바꿀 수 있고,
  UTF-16 span에 맞춰 match를 강조하며 surface와 provenance를 표시한다.
- Playground 입력은 browser 밖으로 보내지 않는다. Full POS와 45 MiB 이상의 compact component
  resource는 기본 demo에 포함하지 않는다. 사용자가 고급 `smart` 지원을 요청할 때만 같은 origin의
  Pages Function에서 component resource를 한 번 내려받아 기존 WASM engine에 load한다.
- Component resource는 25 MiB 단일 값 제한이 있는 Workers KV가 아니라 `kfind-assets` R2 bucket에
  둔다. Pages Function은 `KFIND_ASSETS` binding으로 고정 object를 읽어 body를 buffering하지 않고
  stream하며 content type, ETag와 cache header를 보존한다. R2 object가 없거나 손상되면 embedded
  preview로 조용히 fallback하지 않고 playground에 오류를 표시한다.
- `site` package는 현재 source의 WASM과 version control에 보존한 승인 benchmark snapshot에서
  chart를 다시 생성해 정적 `dist`를 만든다. Snapshot은 source report의 revision과 SHA-256을
  기록하며, 승인된 benchmark가 바뀌면 같은 변경에서 갱신한다.
- 기존 `kfind` Pages project는 direct upload 방식을 유지한다. GitHub Actions는 pull request에서
  site format, lint, type check와 build를 검증한다. Format과 lint는 각각 `Site format`,
  `Site lint` 독립 status check이며 `main` branch protection의 required check다. `main` push에서는
  component resource를 생성해 R2에 먼저 upload한 뒤 production site를 배포한다. 배포 인증은
  repository의 `CLOUDFLARE_ACCOUNT_ID`와 `CLOUDFLARE_API_TOKEN` secret을 사용한다. Production
  branch는 `main`, Pages project 이름은 `kfind`로 고정한다.

### 0.5 Homebrew 대상

- tap은 `SeokminHong/homebrew-brew`, formula는 `Formula/kfind.rb`를 사용한다.
- 사용자 설치 명령은 `brew install seokminhong/brew/kfind`다.
- formula 변경은 tap `main`에 직접 push하지 않는다. 브랜치 PR의 CI가 모두 통과한 뒤 `pr-pull`을 적용한다.
- formula의 source build는 release workflow와 같은 고정 Rust toolchain을 `rustup`으로 준비한다.
  Homebrew core의 `rust` 갱신 시점에 빌드 가능 여부가 달라지지 않아야 한다.
- SemVer tag workflow는 고정 checksum으로 full POS lexicon을 재생성하고 source, full POS,
  man/completion 산출물을 GitHub release에 올린 뒤 `TAP_GITHUB_TOKEN`으로 tap formula PR을 연다.
  prerelease tag는 GitHub prerelease로 게시한다. `pr-pull` label은 CI 확인 뒤 사람이 적용한다.
- full POS resource에는 `lexicon.bin`, 생성 manifest, `mecab-ko-dic`의 `COPYING`을 함께 넣는다. formula는 이를 `share/kfind`와 `share/doc/kfind/LICENSES`에 설치한다.
- compact component resource와 manifest도 formula resource로 고정 checksum을 검증해
  `share/kfind/morphology-component-compact.kfc`에 설치한다. formula `test do`는 설치 경로의
  resource로 component positive와 crossing-substring negative를 모두 실행한다.
- distribution asset의 `skills/kfind/SKILL.md`를 formula의 `share/kfind/skills/kfind`에
  설치한다. Homebrew binary의 `--init`은 project skill을 versioned Cellar가 아니라
  `opt/kfind/share/kfind/skills/kfind`에 연결한다. 최초 `brew install`은 skill 원본을 함께
  설치한다. 사용자가 project에서 `kfind --init`을 한 번 실행해 Homebrew 관리 link를 만든
  뒤에는 `brew upgrade`가 그 link의 안정 경로가 가리키는 원본을 자동으로 갱신한다.
  Homebrew hook은 대상 project와 agent를 알 수 없으므로 임의의 project skill 경로를 직접
  만들거나 수정하지 않는다.
- kfind 소스 코드와 프로젝트가 직접 작성한 내장 데이터는 MIT 라이선스로 배포한다. 외부 full POS resource의 Apache-2.0 고지는 별도 `LICENSES` 디렉터리에 보존한다.

### 0.6 재현 가능한 성능 기준

- 저장소가 제공하는 모든 성능 benchmark 진입점은 resource 준비와 build를 시작하기 전에 Git
  common directory의 단일 exclusive advisory lock을 획득한다. 같은 저장소의 여러 worktree에서
  시작한 benchmark는 이 lock을 공유하며, 먼저 시작한 실행이 끝날 때까지 서로 겹치지 않는다.
- lock 대기 시간과 supervisor 자체 비용은 workload 측정 구간에 포함하지 않는다. lock 대기
  timeout과 benchmark 실행 timeout은 독립적으로 설정하며, 기본값 0은 제한 없이 기다리거나
  실행함을 뜻한다. 대기 중에는 현재 owner와 경과 시간을 주기적으로 표시한다.
- lock 소유권은 운영체제가 관리하는 file lock만으로 판정한다. owner metadata의 supervisor와
  자식 PID는 요청 시 생존 상태를 확인하는 데 사용하되, PID 확인 실패나 metadata가 남았다는
  이유만으로 사용 중인 lock을 강제로 해제하지 않는다. supervisor는 자식 종료까지 blocking
  wait해 측정 중 주기적인 I/O나 polling을 만들지 않는다. supervisor가 비정상 종료되어도 실행
  중인 자식 process가 끝날 때까지 lock descriptor를 보존한다.
- 실행 timeout이나 종료 신호를 받으면 supervisor는 benchmark process group을 종료하고 원래
  종료 상태 또는 timeout 상태를 호출자에게 반환한다. `status`와 `doctor`는 owner의 worktree,
  revision, command, PID, 시작 시각, 경과 시간과 생존 상태를 사람이 읽는 형식과 JSON으로 제공한다.
- 저장소의 공식 shell entrypoint, Criterion benchmark와 외부 baseline 갱신은 공통 runner를
  사용한다. 보고서 재현을 위해 raw `cargo`, `docker` 또는 임의 명령을 실행할 때도 공통 runner로
  감싸야 하며, runner를 우회한 process는 직렬 실행 보장 범위에 포함하지 않는다.
- 인수 기준 9의 기준 corpus는 정확히 1 GiB(1,073,741,824 bytes), 한글 line 선택 비율 20%, 한글 line 중 NFD 선택 비율 50%, 고정 seed `0x004b46494e44`를 사용한다.
- 파일 구성은 1,000개의 64 KiB 작은 파일과 남은 bytes를 균등 분배한 24개의 큰 파일로 고정한다. 생성물은 `target/` 아래에 두고 보고서 생성 뒤 기본적으로 삭제한다.
- 낮은 hit 비율 비교는 생성 문장에 없는 고정 literal을 `kfind --literal --quiet --no-ignore`와 `rg -F --quiet --no-ignore`로 각각 전체 scan한다. 두 명령의 종료 코드 1은 정상적인 no-match 결과다.
- 전역 품사가 literal로 확정된 `--literal`과 `--pos literal` 쿼리는 full POS lexicon을 읽거나 디코딩하지 않는다. `--explain-query`는 `not required (literal query)` 상태를 출력하고 full POS lexicon 누락 진단을 내지 않는다.
- 같은 literal 계획은 compact component resource도 resolve, mmap 또는 검증하지 않는다. component
  resource startup은 component branch가 있는 `smart` query와 분리해 측정한다.
- component startup 측정은 resource 없는 Rust/WASM engine 초기화와 47,859,711-byte compact
  resource를 명시한 초기화를 분리한다. full POS도 component 유무를 나눠 warm process 3회 이상의
  초기화 시간과 peak RSS 또는 process RSS 증가량을 기록하며 literal scan benchmark와 분리한다.
- morphology 비교 보고서는 동일한 1,000-case test fixture에서 embedded와 full POS 각각의
  `smart`, `token`, `any` 경계 정책을 별도 profile로 측정한다. `smart` profile만 compact
  component resource를 초기화하며 `token`과 `any`는 해당 resource를 읽거나 검증하지 않는다.
  각 profile은 fresh process에서 1회 warm-up 뒤 5회 측정하고 query compile과 match를 포함한
  initialization, cases/s, p95 latency와 peak RSS의 median/min/max를 기록한다. 품질은 같은 실행의
  TP, FP, TN, FN, precision, recall과 F1을 기록한다. shadow 진단은 이 측정 구간에 포함하지 않는다.
- 보고서는 에이전트용 `embedded + any + 명시적 품사`와 사람용
  `full-POS + smart + 무품사`를 제품 workflow로 먼저 제시한다. 에이전트 workflow는 recall과
  처리량을 주 지표로 삼고 false positive는 문맥 확인이 필요한 후보 수로 보고한다. 사람 workflow는
  precision, recall, 품사 계획 포함률과 초기화 비용을 함께 보고한다. 전체 lexicon/boundary 행렬은
  원인 분석 자료이며 두 workflow를 하나의 종합 점수로 합치지 않는다.
- 품사를 생략하는 사람용 기본 경로는 별도 held-out fixture와 보고서 절에서 측정한다. positive는
  기존 held-out gold span을 사용하고, negative는 같은 source의 완전히 정렬된 문장 중 query
  표제어가 지원하는 어떤 품사로도 존재하지 않는 문장을 고른다. 전역 `--pos`와 atom 태그 없이
  compile하고 embedded/full-POS 각각의 `smart`, `any`를 fresh process에서 1회 warm-up 뒤 5회
  측정한다.
- 무품사 보고서는 TP, FP, TN, FN, precision, recall, F1과 초기화·처리량·p95 latency·peak RSS를
  기록한다. positive query plan의 기대 품사 포함률, 둘 이상의 coarse POS를 만든 plan 비율,
  literal fallback 비율도 함께 기록한다. 이 결과는 명시적 품사 task와 gold 의미가 다르므로
  하나의 F1 순위로 합치지 않는다. 무품사 결과는 제품 한계와 회귀를 관측하는 자료이며, 목표
  수치를 맞추기 위해 fixture·gold·negative 선택을 변경하지 않는다.
- 제품 workflow의 실제 CLI 비용은 morphology fixture의 query별 compile·match 처리량과 분리해
  고정 100 MiB source corpus에서 측정한다. corpus는 1,000개 파일, 작은 파일 976개 x 64 KiB,
  큰 파일 24개, 생성 한글 line 5%, 그중 NFD 50%, seed `0x004b46494e44`를 사용한다. 별도 64 KiB
  fixture 파일에 `학교에서 새 문서를 검토했다.` 한 줄만 match로 넣고 나머지는 ASCII padding으로
  채운다.
- 에이전트 CLI use case는 `--embedded --boundary any --pos noun --json 학교`, 사람 CLI use case는
  설치 data directory를 지정한 기본 `학교` 검색이다. 두 명령은 같은 corpus에서 정확히 한 줄을
  출력해야 한다. 한 번의 warm-up 뒤 fresh process 5회의 wall time, corpus 처리량과 maximum RSS를
  기록하며 stdout 직렬화는 측정에 포함하고 destination write는 `/dev/null`로 보낸다.
- 사람 CLI use case는 full POS lexicon과 필요한 compact component resource의 자동 해석을 포함한다.
  라이브러리 use case는 같은 보고서의 resource 없는 embedded, embedded + component, full POS,
  full POS + component 초기화 결과로 분리한다. CLI와 라이브러리 결과를 하나의 처리량 점수로
  합치지 않는다.
- 제품 profile 차트는 실제 CLI wall time·처리량·maximum RSS만 단독으로 비교하지 않는다. 같은
  profile의 held-out precision·recall·F1과 false-positive 후보 수를 나란히 표시하여 Agent의
  recall 우선과 사람 CLI의 precision 우선 trade-off를 보존한다. 품질 fixture와 CLI corpus가
  다른 측정임을 차트에 명시하고 하나의 종합 점수로 합치지 않는다.
- 제품 persona 비교 차트는 동일한 1,000-case explicit-POS fixture와 gold를 사용해 Agent, User,
  Kiwi, Lindera, MeCab-ko, KOMORAN의 precision·recall·F1, 초기화 시간, cases/s, p95 latency와
  peak RSS를 함께 표시한다. Agent는 `embedded + any`에 품사를 명시하고, User는 같은 query에서
  품사를 제거한 `full-POS + smart`, 외부 분석기는 품사를 명시한 고정 snapshot을 사용한다. 차트
  행 label에는 입력의 품사 여부를 넣지 않고 인접한 문서에서 조건을 설명한다.
- 이 차트는 동일 입력의 backend 순위가 아니라 실제 persona 입력을 반영한 제품 비교다. User는
  품사 자동 계획과 모호성 비용을 포함하고, 다른 품사의 lemma match도 explicit-POS gold에 따라
  오답으로 계산하므로 유리한 조건으로 해석하지 않는다. 별도 무품사 fixture의 사람용 profile은
  negative 정의가 다르므로 제품 profile 검증에만 사용한다.
- User precision 오탐은 `query-pos-ambiguity`와 `corpus-homonym`으로 분리한다. 전자는 여러
  coarse POS를 포함한 무품사 query plan의 match에 corpus homonym 근거가 없는 경우, 후자는
  predicate 생성형이 문장 안의 다른 lexical 표면형과 겹친 경우다. gold POS는 보고서 원인
  분류에만 사용하며 제품 query plan이나 match를 변경하는 근거로 주입하지 않는다.
- corpus homonym은 `smart` 지정사 branch의 `whole-token-lexical` 검증으로 제거한다. predicate
  match가 같은 Unicode token의 strict subspan이고 compact component resource에서 token 전체의
  exact 분석이 모두 해석 가능한 non-predicate일 때 해당 predicate branch를 거부한다. match가
  token 전체와 같거나, exact 분석이 없거나, predicate 또는 해석할 수 없는 품사 분석이 하나라도
  있으면 유지한다. 같은 span을 다른 query branch가 증명하면 그 branch의 match도 유지한다.
- `whole-token-lexical` 검증은 기존 `any` candidate를 추가하지 않고 `smart`의 predicate branch만
  필터링한다. `token`과 `any` 결과는 바꾸지 않으며, 평가 fixture·gold·지표 정의도 변경하지 않는다.
  폐기한 copula lattice와 경로 비용 비교는 복원하지 않는다.
- `smart` 무품사 query의 direct-particle branch는 입력과 같은 표면형만 만든다. 다른 조사
  이형태까지 검색하려면 전역 `--pos particle` 또는 atom 품사 태그를 명시한다. 이 제한은
  `smart`에만 적용하며 `token`, `any`와 평가 fixture·gold·지표 정의를 바꾸지 않는다.
- precision 개선은 현재 `boundary=any`가 만드는 candidate 집합의 부분집합만 선택한다. `any`
  밖의 span을 새로 만들거나 coverage를 넓히는 변경은 이 작업 범위에서 제외한다. User profile을
  먼저 검증하고, Agent profile은 같은 상한 아래에서 후속 작업으로 다룬다.
- Agent precision 후보 정책은 제품 동작에 넣기 전에 benchmark shadow로만 평가한다. shadow는
  `embedded + any + 명시적 품사`의 결과를 입력으로 사용하며 candidate를 추가하지 않는다.
  query 품사, 생성 근거와 rule path, core·token·whole-token span, exact whole-token 분석과 bounded
  local lattice의 include/exclude 완전 경로 존재 여부를 성능 측정 구간 밖에서 기록한다.
- Agent shadow의 규칙 선택에는 development와 hard-negative fixture만 사용한다. held-out test는
  규칙을 고정한 뒤 한 번의 회귀 판정에만 사용한다. 제품 후보는 User test precision 100.00%와
  Agent의 기존 true positive를 보존하고 false positive를 줄이며 hard-negative의 새 false
  positive가 없을 때만 검토한다. 비용 우열만으로 지정사 branch를 복구하거나 제거하지 않는다.
- shadow 계측은 기존 `any` 결과, CLI 옵션과 resource 초기화 계약을 바꾸지 않는다. 제품 정책으로
  승격할 때는 공개 profile과 필요한 resource의 시작 시간·RSS 계약을 별도로 정의한다.
- 외부 분석기 성능은 각 backend를 fresh process에서 1회 warm-up 뒤 5회 측정해 품질 결과와 함께
  version-controlled snapshot에 저장한다. 기본 benchmark는 snapshot을 읽으며 test fixture,
  adapter·성능 schema 또는 고정 버전·설정이 바뀔 때만 외부 snapshot을 다시 측정한다. snapshot
  환경과 현재 kfind 환경을 모두 표시하고 환경이 다른 결과를 현재 run처럼 표현하지 않는다.
- 보고서는 corpus 설정과 checksum, 저장소에서 commit object로 해석되는 Git revision, CPU, memory, storage, OS, 도구 버전, 실제 명령, 각 run의 wall time·throughput·maximum RSS, median 비교값을 기록한다.
- 1 GiB low-hit `rg -F` 비교는 한 번의 warm-up 뒤 warm-cache 3회를 수행한다. timer 정밀도를
  확보하기 위해 각 run은 동일 scan 10회의 합산 시간을 측정해 1회당 평균을 기록한다. 권한이
  필요한 cache purge를 자동 실행하지 않으며 cold-cache 결과를 측정하지 않았으면 보고서에
  명시한다.

### 0.6 선택적 국소 형태 추론

- query branch의 context requirement는 `None`, `PredicateLexical`, `NominalComponent`,
  `LexicalContext`다. token
  경계에서 거부될 수 있는 명사 branch는 `NominalComponent`, 왼쪽 token 경계를 열어 둔 `smart`
  지정사 branch는 `PredicateLexical`, 어휘 품사 문맥을 검증하는 modifier branch는
  `LexicalContext`를 사용한다.
- `이다/아니다` 계열 검색은 token 전체와 일치하거나 corpus의 predicate 가능성이 남는 생성형을
  homonym union으로 인정한다. strict-subspan 생성형이 token 전체의 exact non-predicate 분석과
  모순되면 `PredicateLexical`이 해당 branch를 거부한다. corpus-side lattice 비용은 사용하지 않는다.
- `PredicateLexical`은 candidate를 포함하는 Unicode token 전체의 compact component exact 분석만
  확인한다. exact 분석이 모두 해석 가능한 non-predicate이면 strict-subspan predicate branch를
  거부하고, exact 분석이 없거나 predicate 또는 해석할 수 없는 품사가 하나라도 있으면 유지한다.
- `NominalComponent`는 `smart`에서만 동작한다. 기존 경계 검증이 거부한 명사 candidate를
  compact component resource로 평가하고 `accept`만 match로 복구한다. `reject`, `ambiguous`,
  평가 오류와 상한 초과는 거부한다.
- `smart`의 bounded lexical context는 candidate가 포함된 Unicode token과 같은 줄의 바로 앞뒤
  Unicode token만 읽는다. 합친 원문은 최대 256 bytes, NFC 문자열은 최대 64 Unicode scalar다.
  UTF-8 검증도 이 bounded 원문에만 적용하므로 범위 밖의 손상된 byte는 문맥 판정을 억제하지
  않는다. 각 token은 NFC로 정규화하고 원문 byte span과의 안정된 경계를 보존한다.
- bounded lexical context는 corpus 구조에서 하나의 판정만 성립할 때만 기본 경계 결과를
  좁히거나 component candidate를 복구한다. 서로 다른 판정이 동시에 성립하거나 UTF-8·정규화·
  resource 검증이 실패하면 판정하지 않고 기존 `smart` 결과를 유지한다.
- 부정 지정사 연결형으로 끝나는 앞 token, `체언 + VCP+ETM` 완전 분해가 있는 현재 token,
  의존명사로 시작하는 뒤 token이 연속하면 현재 token의 체언 component와 지정사 component를
  선택한다. 같은 token의 whole-token 명사·부사 분석은 선택하지 않는다.
- NFC가 같은 token이 바로 인접해 반복되고 해당 surface의 exact `MAG` 분석이 있으면 두 token을
  부사로 선택한다. 같은 token의 명사 분석과 token 내부 명사 component는 선택하지 않는다.
- bounded lexical context 판정은 query 표제어나 query 품사를 입력으로 사용하지 않는다. 한 번
  선택한 corpus 분석은 같은 span을 검색하는 명사·부사·지정사 query에 동일하게 적용한다.
- `독수리가 아니라 매일 것 같아`에서 `매`와 `이다`는 각각 `매`, `일`을 찾고 `매일`,
  `n:매일`, `adv:매일`은 찾지 않는다. `매일 매일 보고 싶어`에서 `매일`과 `adv:매일`은 두
  token을 찾고 `매`, `n:매`, `n:매일`은 찾지 않는다. `그는 집념으로 매일을 보내고 있었다.`에서
  `매일`과 `n:매일`은 조사까지 소비한 `매일을`을 찾고 `매`, `n:매`, `adv:매일`은 찾지 않는다.
- 이 문맥 판정은 `smart`에만 적용한다. `token`과 `any`의 candidate·span·provenance는 바꾸지 않는다.
- component 근거는 완전한 형태 분석 경로에서 query 표제어·품사와 같은 node의 span이 NFC
  query span과 정확히 일치할 때만 성립한다. 더 큰 node에 포함된 substring이나 여러 component
  경계를 가로지르는 span은 근거가 아니다.
- corpus resource의 `NNBC`는 query-side `NNB`와 같은 의존명사로 비교한다. source POS 문자열은
  artifact와 진단 provenance에 그대로 보존하고 component 일치 판정에서만 호환 태그를
  정규화한다.
- component 판정은 exact node를 포함한 완전 경로와 제외한 완전 경로의 최저 비용을 비교한다.
  `include` 비용이 낮으면 `accept`, `exclude` 비용이 낮으면 `reject`, 동률이면 `ambiguous`다.
  한쪽 경로만 있으면 그 경로를 따르며 exact node를 포함한 고비용 경로의 존재만으로 수용하지
  않는다.
- 제품 matcher의 component 판정은 `include`와 `exclude`의 최저 비용만 유지하며 진단용 N-best 경로를
  생성하지 않는다. shadow와 benchmark 진단은 비용·node·경로 provenance를 포함한 보고서를
  별도로 생성할 수 있다. 같은 resource·문자열·query span·품사에서 제품 판정과 진단 보고서의
  판정과 `include`·`exclude` 최저 비용은 같아야 한다.
- `char.def`와 `unk.def`에서 파생되는 unknown model은 검증된 component resource마다 한 번
  초기화한다. candidate별 판정은 이 정적 model을 다시 파싱하지 않는다. 병렬 검색에서 재사용하는
  evaluator는 공유 불변 상태를 유지하며 candidate별 작업 상태를 공유하지 않는다.
- component 복구는 조사 이형태 판정을 우회하지 않는다. nominal verifier가 소비하지 못한
  query 뒤 어절 suffix 전체가 알려진 조사 이형태와 같으면 component evaluator가 `accept`해도
  거부한다.
- 분석 범위는 candidate를 포함하는 Unicode token이다. 원문은 최대 256 bytes, NFC 문자열은
  최대 64 Unicode scalar, lattice node는 중복 제거 뒤 최대 4,096개로 제한한다. 잘못된 범위,
  UTF-8 오류와 상한 초과를 구분한다.
- NFC의 안정된 경계는 원문 상대 byte offset을 보존한다. 원문 절대 byte span과 NFC byte span은
  양방향으로 변환할 수 있어야 하며 합성·결합 중간처럼 안정되지 않은 경계는 변환하지 않는다.
- compact component resource는 schema 1 container다. NFC surface Double-Array, source node의
  POS·left/right context ID·word cost, 연결 비용 행렬과 `char.def`·`unk.def`를 보존한다.
  loader는 schema, source SHA-256, section length·digest, UTF-8 POS table, payload offset,
  context ID, scoring field와 matrix 범위를 내용을 노출하기 전에 검증한다.
- query-side full POS와 corpus-side resource는 같은 고정 source snapshot에서 생성하되 별도
  산출물로 유지한다. full POS는 정규화된 표제어와 품사를, corpus-side resource는 원본
  표면형별 분석과 연결 비용을 저장한다.
- CLI의 기본 boundary는 `smart`다. plan에 `NominalComponent`, `PredicateLexical` 또는
  `LexicalContext` branch가 있으면 설치된 compact
  resource를 자동으로 찾아 한 번 검증한다. resource 누락·손상·schema 또는 source 불일치는
  초기화 오류이며 기존 경계 판정으로 fallback하지 않는다. literal, `token`, `any` 또는
  component branch가 없는 plan은 resource를 찾거나 읽지 않는다.
- benchmark는 compact resource와 full morphology resource의 exact/common-prefix hit, scoring
  checksum과 candidate별 판정·비용·node·경로 provenance가 일치하는지 검증한다. 불일치나
  resource 오류는 fallback하지 않고 실행을 실패시킨다.
- `학생일`, `책일`은 사전 표제어가 아니라 각각 체언 host와 VCP 관형형 표면 `일`의 결합을
  검증하는 어절 fixture다.

### 0.7 Rust 라이브러리와 WASM 대상

- CLI의 자동 resource 해석과 달리 Rust 라이브러리와 npm binding은 filesystem, URL 또는 package
  asset 위치를 추정하지 않는다. caller가 component 기능을 사용할 때만 bytes를 명시적으로 전달한다.
- `kfind` 파사드 crate의 `Engine::new()`와 `Engine::with_full_pos(full_pos)`는 compact component
  resource를 초기화하지 않는다. `Engine::with_component_resource(component_resource)`와
  `Engine::with_full_pos_and_component(full_pos, component_resource)`가 resource를 명시적으로 검증한다.
  caller-configured lexicon도 resource 없는 생성자와 resource를 명시한 생성자를 분리한다.
- component resource는 생성 이후 first-use에 자동 fetch·load하지 않는다. 검증된 resource는 engine이
  소유하고 여러 matcher에서 재사용하며 query compile마다 다시 decode하지 않는다. resource가 없는
  engine에서 `NominalComponent`, `PredicateLexical` 또는 `LexicalContext`가 필요한 smart plan을
  compile하면 명시적 `ComponentResourceRequired` 오류를 반환하고 기존 경계 판정으로
  fallback하지 않는다.
- 같은 fail-fast 계약은 저수준 `MorphMatcher` 생성자에도 적용한다. resource가 필요한 plan을
  `MorphMatcher::new`로 만들면 `MorphMatcherBuildError::ComponentResourceRequired`를 반환하며,
  resource 또는 evaluator를 받는 생성자만 해당 plan을 초기화할 수 있다.
- 생성 후 `Engine::load_component_resource(component_resource)`와 JavaScript
  `loadComponentResource(componentResource)`로 resource를 명시적으로 초기화하거나 교체할 수 있다.
  새 bytes를 모두 검증한 뒤에만 상태를 교체하며 실패하면 기존에 검증된 resource를 유지한다.
- engine은 component resource가 초기화되었는지 getter로 노출한다. resource가 필요 없는 literal,
  `token`, `any`와 component branch가 없는 plan은 resource 없는 engine에서 그대로 compile한다.
- 라이브러리 matcher는 UTF-8 byte slice에서 겹치지 않는 match와 형태 분석 provenance를
  반환한다. 파일 순회, 인코딩 판별, 출력 형식과 CLI locale 처리는 라이브러리 API에
  포함하지 않는다.
- `kfind`, `kfind-wasm`, `kfind-data`, `kfind-morph`, `kfind-query`, `kfind-matcher`는
  Rust 1.97에서 `wasm32-unknown-unknown` 대상으로 빌드되어야 한다.
- `kfind-wasm`은 `wasm-bindgen` JavaScript glue와 TypeScript declaration을 생성한다.
  npm package metadata와 게시 계약은 0.8절을 따른다.
- JavaScript API는 `new Kfind(componentResource?)`와
  `Kfind.withFullPos(fullPos, componentResource?)`, 재사용 가능한 `Matcher`를 만드는 `compile`,
  수동 `loadComponentResource`, UTF-16 JavaScript 문자열을 검색하는 `findAll`을 제공한다.
  resource 인자는 `Uint8Array`다.
  component bytes를 명시했을 때 빈 bytes, 손상, schema·source mismatch는
  `failed to initialize kfind` JavaScript `Error`다. component가 없는 인스턴스의 component smart
  compile은 `failed to compile query` JavaScript `Error`이며 자동 load나 fallback을 수행하지 않는다.
- WASM binary에는 compact component resource bytes를 `include_bytes!` 또는 동등한 방식으로
  포함하지 않는다. binding은 URL fetch, filesystem과 bundler asset resolution을 수행하지 않으며
  호출자가 외부 호스팅 URL 또는 별도 정적 asset에서 bytes를 읽어 생성자에 전달한다.
- `compile`은 선택적 camelCase 객체로 `expand`, `boundary`, `pos`, `normalization`,
  `maxGap`, `literal`을 받는다. 값 집합과 충돌 규칙은 CLI compile option과 동일하며
  알 수 없는 필드, 잘못된 값과 컴파일 실패는 JavaScript `Error`로 드러낸다.
- match와 atom의 `start`, `end` offset은 JavaScript `String.prototype.slice`에 바로
  사용할 수 있는 UTF-16 code unit 기준이다. 각 atom은 core·token span과 모든
  `analysisIndex`, `rulePath` provenance를 보존한다.
- 기본 CI는 Linux와 Apple Silicon macOS에서 네이티브 테스트를 실행하고, Linux에서
  MSRV의 `kfind-wasm` build를 검사한다.

### 0.8 npm package

- npm package 이름은 unscoped `kfind`다. `wasm-pack`의 `bundler` target으로 ESM
  JavaScript glue, WASM binary와 TypeScript declaration을 생성한다.
- compact component artifact는 `assets/morphology-component-compact.kfc` 정적 파일로
  WASM 산출물과 분리해 게시한다. 사용자는 이 파일을 배포물에 복사하거나 별도 호스트에 올릴 수
  있으며 npm binding은 특정 호스팅 URL을 고정하지 않는다.
- package build는 고정 source와 checksum으로 정적 asset을 생성한다. `npm pack --dry-run`은
  asset 포함과 SHA-256을 검증하고 WASM binary에 compact container magic 또는 artifact bytes가
  포함되지 않았음을 확인한다.
- npm 산출물은 브라우저 bundler용 release package로 생성한다. 별도의 Node target
  산출물로 같은 공개 API를 smoke test하고 `npm pack --dry-run`으로 게시 파일과 metadata를
  검증한다.
- npm package 검증은 package version과 Cargo version의 일치, TypeScript declaration의 optional
  resource signature, resource 없는 non-component compile, resource 없는 component smart 오류,
  JavaScript 초기화 오류, component positive/crossing negative와 UTF-16 offset 계약을 확인한다.
- 기본 CI는 npm package build, Node smoke test와 pack 검사를 실행한다.

## 1. 문서 목적

`kfind`는 에이전트와 사람이 입력한 한국어 표제어 또는 짧은 구(句)를 조사 결합, 어미 결합,
불규칙 활용과 일부 생산적 파생 규칙에 따라 검색 계획으로 컴파일하고, 소스 코드·문서 파일과
메모리 text에서 후보 span을 빠르게 찾는 CLI·library다.

이 도구는 코퍼스 전체를 형태소 분석하지 않는다. 입력 쿼리 쪽에서만 형태 정보를 해석하고, 원문에서는 빠른 문자열 앵커 검색과 국소 검증만 수행한다.

형태 분석은 제품 자체가 아니라 빠른 text matching을 위한 query planning 수단이다. 결과는
후속 문맥 판단에 사용할 span과 생성 근거이며, 완전한 문장 형태 분석이나 의미 해석이 아니다.

제품 설명은 다음 문구를 기준으로 한다.

> 한국어 표제어와 활용형을 빠르게 찾는 코드·문서 검색 CLI

영문 설명:

> Fast Korean lemma and inflection search for code and documents.

사용자용 영문 Markdown 문서는 같은 디렉터리에 `.ko.md` 한국어 문서를 함께 두고,
두 문서 상단에서 서로 연결한다. 이미 한국어로 작성된 사양서와 벤치마크 보고서는
언어별 사본을 만들지 않는다.

## 2. 아키텍처

기본 아키텍처는 다음과 같다.

```text
입력 쿼리
  → 정규화
  → 품사 및 사전 항목 조회
  → 어휘 교체 규칙 적용
  → 활용·조사·어미 프로그램 생성
  → 검색 앵커 선택
  → 단일 또는 다중 문자열 matcher 구성

파일 코퍼스
  → ignore 규칙 기반 병렬 순회
  → 바이트 단위 검색
  → 앵커 hit 주변만 형태 규칙 검증
  → phrase span 결합
  → bounded streaming output
```

기본 실행 경로에는 Kiwi, Lindera, MeCab 계열 분석기를 포함하지 않는다. 런타임 모델 다운로드도 하지 않는다.

동음이의어와 동형이의어는 문맥으로 구분하지 않는다. 한 표제어에서 생성 가능한 표면형이면 모두 검색 결과로 인정한다.

예:

```text
검색어: 걷다

길을 걸어 갔다.     match
전화를 걸어 봤다.   match
```

두 번째 결과는 의미상 `걸다`지만, 문맥 판별은 이 제품의 범위가 아니다.

## 3. 핵심 구현 계약

### 3.1 검색 앵커와 verifier를 분리한다

완성된 표면형 문자열을 전부 나열한 구조를 유일한 중간 표현으로 쓰지 않는다. 표면형 수가 늘어날수록 메모리와 matcher 구성 시간이 증가하고, `걸었습니다`, `걸었지만`, `걸으셨다` 같은 연쇄 어미를 모두 전개하기 어렵다.

대신 검색 앵커와 국소 검증 상태를 분리한다.

```rust
pub struct SurfaceBranch {
    pub anchor: Box<[u8]>,
    pub verifier: VerifierId,
    pub core_mapping: CoreMapping,
    pub origins: SmallVec<[Origin; 2]>,
}
```

`걸었`을 앵커로 찾은 뒤 verifier가 `습니다`, `지만`, `는데` 등을 확인한다.

어미와 조사 continuation은 쿼리마다 복제하지 않는다. 빌드 시 생성한 전역 suffix DFA 또는 trie를 공유하고, 각 branch는 시작 상태만 참조한다. Aho-Corasick에는 완성 활용형 전체가 아니라 고유 앵커만 등록한다.

### 3.2 용언 분류와 활용 생성을 분리한다

`pred_class("걷다") -> "d_irregular"` 같은 함수는 사전 조회에만 해당한다. 활용형은 특정 단어 결과를 하드코딩하지 않고 입력 어간으로 계산해야 한다.

잘못된 구조:

```rust
"d_irregular" => ["걸어", "걸었", "걸은"]
```

올바른 구조:

```text
걷 + ㄷ→ㄹ + 어 → 걸어
듣 + ㄷ→ㄹ + 어 → 들어
싣 + ㄷ→ㄹ + 어 → 실어
```

### 3.3 합성 가능한 어휘 특성을 사용한다

한국어 활용은 한 개의 문자열 class로 모두 설명하기 어렵다. 다음과 같이 어휘적 교체와 환경 의존 규칙을 분리한다.

```rust
pub struct PredicateEntry {
    pub lemma: Box<str>,
    pub pos: PredicatePos,
    pub alternation: LexicalAlternation,
    pub flags: PredicateFlags,
    pub overrides: Box<[SurfaceOverride]>,
}

pub enum LexicalAlternation {
    Regular,
    DToL,
    DropS,
    BToWa,
    BToWo,
    DropH,
    ReuDoubleL,
    Reo,
    Ha,
    UToEo,
    Copula,
    Suppletive,
}
```

`ㄹ 탈락`, `ㅡ 탈락`, 모음 축약, 자음 어미 결합은 가능한 한 어간과 어미 환경에서 계산한다. `ㄷ`, `ㅂ`, `ㅅ`, `ㅎ`처럼 같은 철자 끝에서도 규칙형과 불규칙형이 갈리는 경우는 사전으로 판별한다.

### 3.4 한 표제어의 복수 분석을 보존한다

사전은 하나의 표제어에 여러 항목을 허용한다.

```text
묻다  VV  DToL
묻다  VV  Regular
```

검색 결과는 두 분석의 합집합이다.

```text
물어, 물었다, 물으면
묻어, 묻었다, 묻으면
묻고, 묻는, 묻지
```

같은 표면형이 여러 규칙에서 생성되면 결과 span은 한 번만 출력하되, `--explain-match`와 JSON에는 모든 생성 근거를 보존한다.

### 3.5 사전과 명시적 품사로 용언을 판별한다

`바다`, `마다`, `솟대` 등과 같은 입력을 고려하면 `ends_with('다')`는 품사 판별 규칙으로 사용할 수 없다.

`auto` 해석 우선순위는 다음과 같다.

1. 내장 사전의 정확한 표제어 조회
2. 사용자 사전 조회
3. 생산성이 높은 접미 패턴 조회: `하다`, `되다`, `시키다`, `스럽다`, `답다`, `롭다`
4. 알려진 조사·수식언 조회
5. 미등록 한글 입력은 체언 후보와 literal 후보
6. 사용자가 `--pos` 또는 쿼리 태그를 지정하면 그 해석만 사용

미등록 `다` 종결어를 자동으로 용언 처리하지 않는다. 사용자가 `v:커스텀하다` 또는 `--pos verb`로 지정할 수 있다.

자동 품사 판별의 범위는 알고리즘보다 사전 데이터의 범위에 좌우된다. 이를 휴리스틱으로 숨기지 않는다. 배포 데이터는 두 계층으로 나눈다.

```text
core lexicon: 불규칙 용언, 고빈도 중의어, 조사와 수식언
full POS lexicon: 폭넓은 표제어와 품사, Homebrew 기본 설치에 포함
```

full POS lexicon을 찾지 못한 경우에도 검색은 가능하지만, 미등록 `다` 종결어는 literal로만
처리하고 `--explain-query`에 진단을 남긴다. 배포 full POS lexicon은 고정 source, checksum,
라이선스와 gold 품사 검증 결과를 함께 보존한다.

### 3.6 확장, 경계와 품사를 분리한다

검색 정책은 다음 세 축으로 분리한다.

```text
--expand literal|inflection|derivation
--boundary smart|token|any
--pos auto|noun|pronoun|numeral|verb|adjective|determiner|adverb|particle|interjection|literal
```

기본값:

```text
--expand inflection
--boundary smart
--pos auto
```

경계 정책은 다음과 같이 정의한다.

```text
smart: 품사별 verifier가 조사·어미를 소비한 뒤 바깥 토큰 경계를 검사
token: 입력 core 자체가 독립 토큰에서 시작하도록 더 엄격하게 검사
any: 왼쪽과 오른쪽 경계를 검사하지 않는 부분 문자열 검색
```

`smart`는 임의의 한글 연속 문자열을 형태 변화로 보지 않는다. compact component resource가
완전한 형태 분석 component로 증명한 `사용자권한`의 `권한`은 허용하지만, component 경계를
가로지르는 substring은 거부한다. 형태 분석 근거 없이 부분 문자열을 검색하려면
`--boundary any`를 사용한다. 한 음절 쿼리는 `smart`에서도 `token`에 가까운 경계를 적용한다.

`derivation`은 `inflection`을 포함하며 `-적`, `-하다`, `-되다`, `-시키다` 같은 생산적 파생을 추가한다.

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

## 5. 범위와 비범위

### 5.1 지원 범위

지원하는 큰 품사 범주는 다음과 같다.

| 범주 | 세부 범주 | 기본 동작 |
|---|---|---|
| 체언 | 명사, 대명사, 수사, 의존명사 | 기본형, 복수 표지, 조사 연쇄 검증 |
| 용언 | 동사, 형용사, 지정사, 일부 보조용언 | 어간 교체, 어미 결합, 축약 검증 |
| 수식언 | 관형사, 부사 | literal과 품사별 경계, 선택적 보조사 |
| 관계언 | 조사 | 이형태 묶음과 경계 검증 |
| 독립언 | 감탄사 | literal과 토큰 경계 |
| 기타 | 코드 식별자, 외국어, 숫자 | literal |

### 5.2 비범위

현재 제품은 다음을 제공하지 않는다.

- 일반 목적 문장 형태소 분석기 또는 tokenizer API
- 형태소 분석기 자체의 최고 처리량·정확도 경쟁
- 문서 전체 형태소 분석
- 문맥 기반 동음이의어 구분
- 임의 활용형의 표제어 역분석
- 동의어, 유의어, 의미 검색
- 사용자 정규식
- 치환 기능
- 방언, 고어, 비표준 활용의 포괄적 지원
- 문장 단위 구문 분석
- 무제한 어미 연쇄 생성

입력 `걸어`는 기본적으로 literal 또는 미등록 체언 후보로 처리한다. `걸어`에서 `걷다`와 `걸다`를 역추론하는 기능은 별도 후속 범위다.

## 6. 쿼리 언어와 파싱

### 6.1 토큰화

단순 `split_whitespace()`를 사용하지 않는다. 다음을 지원하는 작은 lexer를 둔다.

```text
공백 구분
작은따옴표와 큰따옴표
백슬래시 이스케이프
품사 태그 접두사
literal 강제
```

예:

```bash
kfind 'n:권한 "접근 제어" v:검증하다' src
```

`"접근 제어"`는 하나의 literal atom으로 처리한다.

### 6.2 AST

```rust
pub struct QueryAst {
    pub atoms: Vec<QueryAtom>,
    pub phrase: PhrasePolicy,
}

pub struct QueryAtom {
    pub raw: Box<str>,
    pub forced_pos: Option<CoarsePos>,
    pub quoted_literal: bool,
}
```

### 6.3 분석 결과

```rust
pub struct Analysis {
    pub lemma: Box<str>,
    pub coarse_pos: CoarsePos,
    pub fine_pos: FinePos,
    pub morphology: Morphology,
    pub source: AnalysisSource,
}

pub enum AnalysisSource {
    BuiltinLexicon,
    UserLexicon,
    ProductiveSuffix,
    Heuristic,
    Forced,
}
```

`auto` 모드에서 복수 분석이 가능하다. 예를 들어 `새`는 관형사와 명사 분석을 함께 가질 수 있다.

### 6.4 query analyzer 인터페이스

품사 판별과 검색 계획 생성을 결합하지 않는다.

```rust
pub trait QueryAnalyzer: Send + Sync {
    fn analyze(&self, atom: &QueryAtom) -> Result<Vec<Analysis>, AnalyzeError>;
}

pub struct LexiconQueryAnalyzer {
    builtin: Arc<Lexicons>,
    user: Arc<UserLexicon>,
}
```

제품 query analyzer는 `LexiconQueryAnalyzer`다. 다른 analyzer adapter도 쿼리 atom만 분석하고
결과를 공통 `Analysis`로 변환해야 한다. surface matcher와 파일 검색 계층은 analyzer 종류를
알지 못한다.

## 7. 중간 표현과 검색 계획

### 7.1 상위 구조

```rust
pub struct QueryPlan {
    pub raw_query: Box<str>,
    pub atoms: Vec<AtomPlan>,
    pub phrase_policy: PhrasePolicy,
    pub limits: PlanLimits,
}

pub struct AtomPlan {
    pub analyses: Vec<Analysis>,
    pub branches: Vec<SurfaceBranch>,
    pub boundary: BoundaryPolicy,
}

pub struct SurfaceBranch {
    pub anchor: Vec<u8>,
    pub left: LeftVerifier,
    pub right: RightVerifier,
    pub core_mapping: CoreMapping,
    pub origins: Vec<Origin>,
    pub context_requirement: ContextRequirement,
}

pub enum ContextRequirement {
    None,
    PredicateLexical,
    NominalComponent,
    LexicalContext,
}

pub struct Origin {
    pub analysis_index: u16,
    pub rule_path: Vec<RuleId>,
}
```

### 7.2 핵심 span과 토큰 span

검색 결과는 두 범위를 구분한다.

```rust
pub struct VerifiedSpan {
    pub core: Range<usize>,
    pub token: Range<usize>,
    pub origins: SmallVec<[Origin; 2]>,
}
```

예:

```text
사용자들에게
^^^^^^         core: 사용자
^^^^^^^^       token: 사용자들에게
```

기본 터미널 강조는 token span을 사용한다. JSON에는 core와 token을 모두 제공한다.

### 7.3 표면형 provenance

표면 문자열 하나만 `BTreeSet`으로 중복 제거하지 않는다. 다음과 같이 검색 키와 생성 근거를 분리한다.

```rust
HashMap<BranchKey, SmallVec<[Origin; 2]>>
```

동일 branch가 여러 분석에서 생성되면 origins를 합친다.

## 8. 한국어 음절 처리

### 8.1 내부 정규화

쿼리, 사전 표제어, 규칙 파일은 NFC로 정규화한다.

코퍼스 전체를 매번 복사해 정규화하지 않는다. 기본값은 NFC 바이트 검색이다.

```text
--unicode-normalization nfc        기본값
--unicode-normalization canonical NFC와 NFD 패턴을 모두 생성
--unicode-normalization none       입력 바이트를 그대로 사용
```

`canonical`은 완전한 임의 혼합 정규화 비교가 아니라, 쿼리 branch의 NFC·NFD 두 형태를 검색하는 모드다. Exact branch는 선택된 형태의 anchor bytes 자체가 검증 결과이므로 anchor 뒤 입력을 NFC로 변환하지 않는다. 형태 continuation을 소비하는 branch만 bounded suffix를 NFC로 변환하고 원문 byte offset으로 다시 매핑한다.

### 8.2 필요한 음절 연산

```rust
pub struct Syllable {
    pub choseong: u8,
    pub jungseong: u8,
    pub jongseong: u8,
}

pub fn decompose_syllable(c: char) -> Option<Syllable>;
pub fn compose_syllable(s: Syllable) -> Option<char>;
pub fn replace_final(c: char, jong: u8) -> Option<char>;
pub fn drop_final(c: char) -> Option<char>;
pub fn replace_last_final(s: &str, jong: u8) -> Option<String>;
pub fn drop_last_final(s: &str) -> Option<String>;
pub fn add_final(s: &str, jong: u8) -> Option<String>;
pub fn replace_last_vowel(s: &str, jung: u8) -> Option<String>;
pub fn has_final(c: char) -> bool;
pub fn has_rieul_final(c: char) -> bool;
```

한글 완성형 음절은 Unicode 산술 분해와 조합으로 처리한다. 별도의 대형 테이블은 필요하지 않다.

## 9. 형태 규칙 엔진

### 9.1 세 계층

형태 규칙은 다음 세 계층으로 분리한다.

1. 어휘적 교체: 표제어별 예외 사전
2. 어미 이형태 선택: 받침, ㄹ 받침, 모음 시작 여부 등
3. 표면 조합과 축약: `보아 → 봐`, `되어 → 돼`, `하여 → 해`

이 구분을 유지해야 규칙형과 불규칙형을 같은 generator에서 안정적으로 다룰 수 있다.

### 9.2 어미 모델

```rust
pub struct EndingSpec {
    pub id: EndingId,
    pub category: EndingCategory,
    pub initial: EndingInitial,
    pub surface: Box<str>,
    pub required: MorphFeatureMask,
    pub forbidden: MorphFeatureMask,
    pub continuation: ContinuationState,
    pub terminal: bool,
}

pub enum EndingInitial {
    Consonant,
    AOrEo,
    Eu,
    AttachNieun,
    AttachRieul,
    AttachBieup,
    Other,
}
```

예시 범주:

```text
-고, -지, -게, -다
-는, -(으)ㄴ, -(으)ㄹ
-아/-어, -아서/-어서
-았/-었, -았을/-었을, -았/었느냐(는), -겠, -시. 존대 선어말어미는 받침 어간뿐 아니라 모음·ㄹ·불규칙 어간의 올바른 교체형에도 결합한다.
-아요/-어요와 -았어요/-었어요
-면/으면, -며/으며, -니/으니, -더라도, -려고/으려고, -리라고/으리라고
-아/어가고, -아/어가야
-ㅂ니다/습니다
-기, -음/ㅁ
```

선어말어미와 종결·연결어미는 작은 유한 상태 그래프로 표현한다. verifier는 `next`, `required`, `forbidden`을 모두 만족하는 경로만 소비하고, 허용 깊이를 제한해 무제한 조합을 방지한다.

어미 결합 가능성은 용언별 문자열 분기로 작성하지 않고 feature bitset으로 판정한다. 최소 feature는 다음을 포함한다.

```text
action verb
descriptive verb
copula
vowel-final
consonant-final
rieul-final
light-vowel
dark-vowel
special-ha, special-i, special-ani, special-o, special-itda
```

이 구조를 사용하면 어미 목록이 늘어나도 `match lemma` 코드가 증가하지 않고, 규칙 데이터와 테스트 fixture만 확장할 수 있다.

### 9.3 공통 규칙

다음은 사전 class가 아니라 환경 규칙으로 처리한다.

- 받침 유무에 따른 `은/는`, `이/가`, `을/를`, `과/와`
- `로/으로`의 ㄹ 받침 예외
- 조사 연쇄는 `data/rules/particles.toml`의 `next` 전이만 허용한다.
- `-(으)면`, `-(으)며`, `-(으)ㄴ`, `-(으)ㄹ`
- 일반 용언의 이유 연결형 `-(으)니`는 자음 어간에 `으`를 삽입하고 ㄹ 받침 어간의 ㄹ을 탈락시킨다.
- 일반 용언의 양보 연결형 `-더라도`는 어휘적 교체 없이 사전 어간에 직접 결합하고 token 경계에서 끝난다.
- 일반 용언의 전망 인용 연쇄 `-(으)리라고`는 기존 불규칙 교체를 적용한 어간 뒤에서만 완료된 token으로 허용한다.
- 의도 연결형 `-(으)려고`는 동작 용언에만 결합하고, 기존 불규칙 교체 뒤의 모음형 어간을 사용한다.
- 진행 방향 보조 용언 `-아/어가다`는 `-아/어` branch 뒤의 `가고`, `가야`만 continuation으로 소비한다. `가` 자체나 목록 밖 후속 어미는 허용하지 않는다.
- 과거 `-았/었` branch는 의문 종결형 `-느냐`와 이 종결형에 직접 붙는 주제 보조사 `는`까지 소비한다. 다른 조사나 추가 어미는 허용하지 않는다.
- `-기` 명사형은 어휘적 교체 없이 사전 어간에 직접 결합하고, 이 규칙이 만든 terminal
  predicate branch만 nominal particle verifier로 전이한다. verifier는 `기`를 모음 끝 host로
  판정해 `가`, `를`, `는`, `와`, `로` 등의 올바른 이형태와 `data/rules/particles.toml`의
  bounded 조사 연쇄만 소비한다. 다른 명사형·종결형·연결형 branch는 이 전이를 사용하지 않는다.
- ㄹ 받침 뒤 특정 자음 어미에서의 ㄹ 탈락
- 어간 말음 `ㅡ`와 `-아/-어` 결합
- 모음 축약과 준말. `ㅕ` 말음 규칙 어간은 `-어`의 축약형도 보존한다 (`켜어`, `켜`).
- 자음 어미의 종성 결합

`-기` 명사형 뒤의 유효한 조사 연쇄는 predicate token의 일부로 소비한다. 따라서 `걷다`는
`걷기`, `걷기 운동`, `걷기가`, `걷기를`, `걷기에서도`를 찾는다. `걷기이`, `걷기을`,
`걷기으로`, case 조사 두 개를 잇는 `걷기가를`은 `smart`와 `token`에서 거부한다. `any`는 기존
부분 문자열 candidate를 제거하지 않지만 유효한 조사 연쇄가 있으면 그 끝까지 token span을
확장한다. query provenance에는 `ending.nominalizer-gi` 뒤에 소비한 조사 rule path를 순서대로
남긴다.

### 9.4 어휘 사전이 필요한 교체

다음은 철자만으로 안정적으로 판별하지 않는다.

- ㄷ 불규칙과 ㄷ 규칙
- ㅂ 불규칙과 ㅂ 규칙
- ㅅ 불규칙과 ㅅ 규칙
- ㅎ 불규칙과 규칙형
- 르 불규칙과 러 불규칙
- 기타 보충법과 개별 예외
- `아니다`처럼 일반적인 `-이어 → -여` 축약을 허용하지 않는 개별 어휘 제약

### 9.5 필수 활용 범위

| 분류 | 예 | 기대 표면형 |
|---|---|---|
| 규칙 자음 어간 | 먹다 | 먹어, 먹었다, 먹는, 먹은, 먹을 |
| 규칙 모음 어간 | 가다 | 가, 갔다, 가는, 간, 갈 |
| ㅏ/ㅓ 축약 | 보다 | 보아, 봐, 보았다, 봤다 |
| ㅚ/ㅣ 계열 축약 | 되다 | 되어, 돼, 되었다, 됐다 |
| ㄷ 불규칙 | 걷다, 듣다, 싣다 | 걸어, 들어, 실어 |
| ㅅ 불규칙 | 짓다, 낫다, 잇다 | 지어, 나아, 이어 |
| ㅂ 불규칙 | 돕다, 눕다, 아름답다 | 도와, 누워, 아름다워 |
| ㅎ 불규칙 | 파랗다, 그렇다, 어떻다, 이렇다, 커다랗다 | 파래, 파란, 그래, 그런, 어떤, 이런, 커다란 |
| 르 불규칙 | 빠르다, 부르다, 모르다 | 빨라, 불러, 몰라 |
| 러 불규칙 | 푸르다, 이르다 일부 | 푸르러 |
| ㅡ 탈락 | 쓰다, 크다, 예쁘다 | 써, 커, 예뻐 |
| 우 불규칙 | 푸다 | 퍼 |
| 하다 | 하다, 검증하다 | 하여, 해, 하였다, 했다, 검증하여, 검증해, 검증하였다, 검증했다 |
| ㄹ 탈락 | 살다, 알다, 만들다 | 사는, 압니다, 만듭니다 |
| 진행 방향 보조 용언 | 망하다, 만들다 | 망해가고, 만들어가야 |
| 과거 의문 종결 | 하다, 먹다 | 했느냐는, 먹었느냐 |
| 지정사 | 이다 | 이고, 이어, 여서, 인, 일, 입니다 |
| 부정 지정사 | 아니다 | 아니고, 아니어서, 아니라, 아닌, 아닐 |

## 10. 품사별 컴파일 규칙

### 10.1 체언

체언은 모든 완성형을 미리 생성하지 않는다.

```text
anchor: 사용자
right verifier:
  plural: 들?
  particle chain: 조사와 보조사 제한 조합
  optional VCP predicate: 이다 계열, 설정 시
```

예:

```text
사용자
사용자는
사용자들에게
사용자들로부터
```

`--expand derivation`에서는 다음을 추가할 수 있다.

```text
기술 → 기술적
검증 → 검증하다, 검증되다
단순 → 단순화
```

생산적 파생 규칙은 별도 목록으로 관리하며 기본 `inflection`에는 포함하지 않는다.

### 10.2 대명사와 수사

대명사와 수사는 체언 verifier를 공유하되, 사전에 표면 교체를 둘 수 있다.

```text
나 + 가 → 내가
너 + 가 → 네가
저 + 가 → 제가
```

해당 축약을 지원하려면 override로 명시한다. 미지원 항목은 사양의 known limitation에 기록한다.

### 10.3 동사와 형용사

공통 predicate generator를 사용하되 종결형과 관형형 가능 범위를 품사별로 구분한다.

```rust
pub enum PredicatePos {
    Verb,
    Adjective,
    AuxiliaryVerb,
    AuxiliaryAdjective,
    Copula,
}
```

검색 도구이므로 실제 문법에서 드문 형태를 일부 허용할 수 있다. 다만 규칙으로 생성한 비표준형을 기본 결과에 포함해서는 안 된다. 확장 여부는 gold corpus로 결정한다.

### 10.4 관형사

관형사는 활용하지 않는다.

```text
left boundary: 토큰 시작
surface: literal
right condition: 토큰 경계 또는 다음 한국어 토큰 시작
```

`새`의 명사와 관형사 분석이 모두 사전에 있으면 auto 모드에서 두 분석을 합친다.

### 10.5 부사

기본은 literal과 스마트 경계다. `--expand derivation`에서만 규칙 데이터에 등록된 보조사 결합을 허용하고 격조사는 허용하지 않는다.

```text
빨리
빨리도
잘만
```

### 10.6 조사

조사를 직접 검색할 때 품사를 명시하면 이형태 묶음을 사용할 수 있다.

```text
으로 ↔ 로
은 ↔ 는
이 ↔ 가
을 ↔ 를
과 ↔ 와
```

한 음절 조사 검색은 hit가 많으므로 `smart`에서 바로 앞 host의 받침 조건과 조사 뒤 토큰 경계를 검증한다. `token`은 독립 토큰 경계를 요구하고, `--boundary any`에서만 host 검증 없는 임의 부분 문자열을 허용한다.
품사를 생략한 `smart` 검색은 입력한 조사 표면형만 사용한다. 예를 들어 `이`는 붙은 `이`를
찾되 `가`까지 확장하지 않으며, `--pos particle 이`는 `이 ↔ 가` 묶음을 모두 찾는다.

### 10.7 감탄사

literal과 토큰 경계만 적용한다.

## 11. 앵커 계획

### 11.1 앵커 선택 원칙

각 branch에서 가능한 가장 긴 고정 바이트열을 앵커로 선택한다.

우선순위:

1. 어간 교체 이후 첫 어미까지 포함한 문자열
2. 어간 전체
3. 짧은 어간이면 다음 고정 요소와 결합
4. 한 음절 앵커는 경계 verifier 없이는 허용하지 않음

예:

```text
걷다
  걷고
  걷는
  걷지
  걷겠
  걸어
  걸었
  걸으
  걸은
  걸을
```

이 문자열은 특정 단어 목록이 아니라 규칙 계산 결과다.

### 11.2 적응형 matcher

```text
branch 1개: Box에 보관한 memchr::memmem::Finder의 owned variant
branch 2개 이상: Aho-Corasick standard match kind의 overlapping search
```

단일 앵커 Finder는 `Finder::new(needle).into_owned()`로 구성하고 platform별 Finder 내부 크기가 `AnchorEngine` 전체 크기를 키우지 않도록 Box에 보관한다. 후보가 겹칠 수 있으므로 Aho-Corasick에서는 overlapping hit를 받고, 검증 후 가장 왼쪽의 가장 긴 token span을 선택한다.

### 11.3 branch 제한

기본 제한:

```text
쿼리 길이: 최대 256 Unicode scalar
atom 수: 최대 32
atom당 분석 수: 최대 32
전체 branch 수: 최대 4096
matcher 예상 메모리: 최대 64 MiB
어미 continuation 깊이: 최대 4
```

초과 시 조용히 잘라내지 않고 오류를 낸다. `--explain-query`에는 제한에 가까운 항목과 제외된 규칙을 표시한다.

## 12. 검색 실행 엔진

### 12.1 파일 순회

`ignore::WalkParallel`을 사용한다.

기본 정책:

- `.gitignore`, `.ignore`, 전역 ignore 반영
- hidden 파일 제외
- 바이너리 파일 제외
- symlink는 기본적으로 따라가지 않음
- 명시된 파일은 ignore 여부와 관계없이 검색

### 12.2 파일 읽기와 줄 검색

`grep-searcher`를 파일 읽기 계층으로 사용한다.

담당 범위:

- buffered search
- 줄 종결 처리
- 바이너리 감지
- mmap 사용 여부
- context 출력 지원
- 인코딩 변환 설정

형태 matcher는 `grep_matcher::Matcher`를 구현한다. 기본 터미널 출력과 요약 출력은 `grep-printer`를 우선 재사용하고, 형태 생성 근거가 필요한 JSON과 explain 출력만 확장한다.

검색 계획의 anchor가 LF를 포함하지 않으면 matcher는 LF line terminator를 선언한다. `grep-searcher`는 multi-line 기능을 켠 상태에서도 이 선언을 보고 전체 buffer에서 raw anchor가 있는 줄만 후보로 고르고, 후보 줄을 분리한 뒤 형태·경계 검증을 수행한다. LF를 포함하는 literal 계획은 line terminator를 선언하지 않아 multi-line 경로에서 검색한다.

```rust
pub struct MorphMatcher {
    pub plan: Arc<QueryPlan>,
    pub anchor_engine: AnchorEngine,
}
```

`grep_matcher::Matcher::find_at`은 다음 검증된 token span의 바이트 범위만 반환하며 origin과 rule path를 복제하거나 병합하지 않는다. 설명용 메타데이터는 매칭된 줄에 대해서만 `find_all_with_meta`로 다시 계산한다.

### 12.3 검증 단계

```text
anchor hit
  → UTF-8 경계 확인
  → 왼쪽 경계 검사
  → 어간·어미 branch verifier 실행
  → 오른쪽 경계 검사
  → core/token span 계산
  → origins 병합
```

후보 없는 buffer 구간에는 줄별 matcher 호출, Unicode scalar 순회, 형태 규칙 실행을 하지 않는다.

### 12.4 phrase 결합

각 atom의 검증된 span 목록을 구한 뒤 순서 결합한다.

```text
atom 0 spans
atom 1 spans
atom 2 spans
  → two-pointer 또는 제한된 DP
  → 순서 유지
  → max-gap 검사
```

표면형 후보들의 데카르트 곱을 정규식으로 만들지 않는다.

제품 matcher는 가능한 atom 조합마다 `PhraseMatch`를 미리 만들지 않는다. 뒤 atom부터 span별 최적
suffix와 다음 span index만 계산하고 시작 위치 범위의 최적 suffix를 제한된 range join으로
조회한다. DP 상태 수는 검증된 atom span 수의 합에 비례해야 한다. `find_span_at`은 가장 이른
leftmost-longest 결과 하나만 복원하고, `find_all_with_meta`는 입력의 anchor와 atom span을 한 번
수집한 뒤 선택된 non-overlapping 결과의 atom metadata만 복원한다. match 하나를 반환할 때마다
남은 전체 입력의 anchor와 span 결합을 다시 계산하지 않는다. `InputSearcher`가 한 줄의 metadata를
수집하는 경로도 같은 일괄 API를 사용하며, 65,536개 상한은 결과를 모두 만든 뒤가 아니라 선택
중에 적용한다.

reference·expert API의 전체 조합용 `join_phrase_spans`는 중간 partial을 65,536개까지만
허용하고 초과하면 `PhraseJoinError::CandidateLimitExceeded`를 반환한다.

### 12.5 병렬 출력

각 worker는 독립된 `Searcher`, scratch buffer, matcher cursor를 가진다.

```text
WalkParallel workers
  → bounded per-file record stream
  → bounded file-stream channel
  → single writer thread
  → BufWriter<StdoutLock>
```

기본 출력은 match와 context record를 검색 중에 bounded stream으로 전달한다. record callback은 파일 EOF와 검색 완료 이전에 시작할 수 있어야 하며, 출력 종료를 관찰하면 남은 입력을 더 읽지 않고 취소한다. 전체 record를 `Vec`에 모은 뒤 callback으로 재생하는 구현은 기본 경로에서 허용하지 않는다. writer는 stream이 소유한 line bytes와 match metadata를 다시 복제하지 않고 borrowed record로 직렬화한다. writer는 선택한 file stream을 끝까지 비운 뒤 다음 stream을 처리하므로 한 파일의 결과는 연속 블록으로 출력한다. 대기 중인 worker는 bounded stream에 backpressure를 받으며, 기본 경로의 결과 메모리는 corpus 또는 전체 match 수가 아니라 worker 수와 channel capacity에 의해 제한된다. 한 줄의 형태 분석 metadata는 최대 65,536개까지만 수집하고 초과하면 해당 입력을 오류로 보고하여 비정상 종료나 무제한 메모리 증가를 막는다. 기본 출력 순서는 파일 시스템 순회 순서를 보장하지 않는다.

```text
--sort path
```

정렬 옵션만 모든 file stream을 완성된 결과로 버퍼링한 뒤 path로 정렬한다. 따라서 기본값이 아니며, 결과 수에 비례해 메모리를 사용하고 병렬 성능이 낮아질 수 있음을 도움말에 명시한다.

broken pipe는 정상 종료로 처리한다.

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

잘못된 UTF-8이 섞인 파일은 바이트 검색 자체는 가능하지만, 한국어 branch 검증은 유효 UTF-8 구간에서만 수행한다.

JSON Lines 출력에서 원문 줄을 UTF-8로 표현할 수 없으면 다음 중 하나를 사용한다.

```json
{"text":null,"text_base64":"...","encoding":"bytes"}
```

## 14. CLI 사양

### 14.1 기본 구문

```text
kfind [OPTIONS] <QUERY> [PATH]...
kfind --init [--agent <AGENT>]...
```

`--init`을 사용하지 않으면 query가 필수다. PATH를 생략하면 현재 디렉터리를 검색한다. stdin이
pipe이면 기본 검색 대상을 stdin으로 전환한다. `-`는 stdin을 명시한다.

### 14.2 주요 옵션

| 옵션 | 값 | 기본값 | 설명 |
|---|---|---:|---|
| `--pos` | 품사 | `auto` | 쿼리 전체 품사 강제 |
| `--expand` | `literal`, `inflection`, `derivation` | `inflection` | 확장 수준 |
| `--boundary` | `smart`, `token`, `any` | `smart` | 경계 정책 |
| `--embedded` | flag | false | full POS lexicon을 로드하지 않음 |
| `--max-gap` | 정수 | `24` | phrase atom 사이 최대 거리 |
| `--unicode-normalization` | `nfc`, `canonical`, `none` | `nfc` | Unicode 검색 모드 |
| `--encoding` | 인코딩 | `auto` | 원문 인코딩 |
| `--glob` | glob | 없음 | 파일 포함·제외 규칙 |
| `--type`, `--type-add` | 파일 유형 | 없음 | 파일 유형 필터 |
| `--hidden` | flag | false | hidden 파일 포함 |
| `--no-ignore` | flag | false | ignore 규칙 무시 |
| `--threads` | 정수 | 자동 | worker 수 |
| `--count` | flag | false | 파일별 match 수 |
| `--files-with-matches` | flag | false | 파일명만 출력 |
| `--json` | flag | false | JSON Lines 출력 |
| `--color` | `auto`, `always`, `never` | `auto` | 터미널 색상 |
| `--no-pager` | flag | false | TTY에서도 pager를 사용하지 않음 |
| `--explain-query` | flag | false | 쿼리 계획 출력 |
| `--explain-match` | flag | false | 생성 근거 출력 |
| `--sort` | `path` | 없음 | 결과 정렬 |
| `--data-dir` | 경로 | 자동 | 외부 데이터 디렉터리 |
| `--user-lexicon` | 경로 | 자동 | 사용자 사전 |
| `--init` | flag | false | 현재 디렉터리에 agent skill 초기화 |
| `--agent` | `claude-code`, `codex`, `gemini`, `custom` | TTY 선택 또는 stdin | 초기화 대상, 반복 가능 |

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

### 14.6 Agent skill 초기화

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

## 15. 출력 사양

### 15.1 기본 출력

```text
src/walk.rs:42: 길을 걸어 갔다.
```

열 번호는 기본적으로 생략할 수 있다. `--column`에서만 match 줄의 앞부분을 Unicode scalar로 세어 계산한다.

일반 text 결과를 TTY stdin/stdout에서 쓰면 검색 시작과 동시에 내장 TUI pager를 열고, 완성된
결과 행을 점진적으로 반영한다. 검색 중에도 이동과 resize를 처리하며 상태 행에 검색 중임을
표시한다. 검색 완료 뒤 너비와 높이가 모두 한 화면에 들어가면 바로 종료하고 terminal 내용을
남긴다. 한 줄이라도 잘리거나 결과가 화면 높이를 넘으면 TUI를 유지하며 `↑`/`↓` 또는 `k`/`j`로
한 행씩 이동하고 `q` 또는 `Esc`로 종료한다. 이동 offset은 content viewport의 첫 행이며 최대값은
`전체 행 수 - viewport 높이`다. 따라서 마지막 행만 화면 위에 남기고 아래를 비우는 위치까지는
이동하지 않는다. 키 반복 중 한 frame에 쌓인 이동은 한 번에 반영하고, 연속 행 이동은 기존 화면을
유지한 채 새로 노출된 행과 상태 행만 갱신한다. 검색 중 종료하면 결과 출력과 남은 검색을 중단한다.

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

### 15.2 쿼리 설명

```text
query: 걷다
atom[0]:
  analyses:
    - lemma: 걷다
      pos: verb
      alternation: DToL
      source: builtin-lexicon
  branches: 12
  anchors:
    - 걷고
    - 걷는
    - 걷지
    - 걸어
    - 걸었
    - 걸으
  verifier_states: 8
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

### 15.4 JSON Lines

```json
{"type":"match","path":"sample.txt","line":3,"text":"길을 걸었습니다.","spans":[{"core":{"start":7,"end":10},"token":{"start":7,"end":22},"surface":"걸었습니다","origins":[{"lemma":"걷다","pos":"verb","rules":["lexical.d-to-l","ending.past","ending.polite-declarative"]}]}]}
```

유효한 UTF-8 text의 offset은 `utf8-bytes`, raw byte text의 offset은 `bytes`로 명시한다. 선택적으로 scalar column도 제공한다.

## 16. 데이터 사양

### 16.1 저장소 구조

```text
data/
  lexicon/
    predicates.tsv
    nominals.tsv
    modifiers.tsv
    particles.tsv
  enriched/
    predicates.tsv
    MANIFEST.toml
    NOTICE.md
  rules/
    endings.toml
    alternations.toml
    contractions.toml
    derivations.toml
  fixtures/
    morphology_cases.tsv
  generated/
    lexicon.bin
    rules.bin
```

### 16.2 용언 사전

```tsv
lemma	pos	alternation	flags
걷다	VV	DToL	
걷다	VV	Regular	
듣다	VV	DToL	
묻다	VV	DToL	
묻다	VV	Regular	
믿다	VV	Regular	
돕다	VV	BToWa	
눕다	VV	BToWo	
짓다	VV	DropS	
벗다	VV	Regular	
파랗다	VA	DropH	
좋다	VA	Regular	
빠르다	VA	ReuDoubleL	
푸르다	VA	Reo	
쓰다	VV	Regular	EU_DROP
하다	VV	Ha	
이다	VCP	Copula	
```


### 16.3 빌드 산출물

개발용 TSV·TOML을 빌드 스크립트에서 검증하고 compact binary로 변환한다.

검증 항목:

- 중복 항목은 허용하되 완전히 같은 행은 경고
- 존재하지 않는 rule id 거부
- NFC가 아닌 표제어 거부 또는 자동 정규화 후 경고
- 용언 표제어의 기본형 형식 검증
- override 충돌 검증
- fixture에서 모든 사전 class가 최소 한 번 사용되는지 확인

enriched 용언 데이터는 core와 같은 용언 schema를 사용하되 별도 파일과 라이선스로 관리한다.
동일한 `lemma`, `pos`, `alternation`, `flags`, `overrides`가 core에 있으면 core만 보존하고,
alternation이 다르면 같은 세부 품사라도 모두 보존한다. full POS의 규칙형 fallback은 core와
enriched를 합친 결과에 같은 coarse 품사가 없을 때만 추가한다.

내장 데이터는 `include_bytes!`로 실행 파일에 포함해도 된다. 이 데이터는 프로젝트가 직접 관리하고 라이선스를 명확히 할 수 있어야 한다. 사용자가 교체할 사전은 외부 파일로 추가 로딩한다.
Agent skill은 source tree의 `skills/kfind/SKILL.md`를 유일한 원본으로 삼는다. CLI fallback
문자열과 distribution asset은 이 파일에서 생성하며 내용이 서로 달라지지 않아야 한다.

### 16.4 사용자 사전

기본 위치:

```text
$XDG_CONFIG_HOME/kfind/lexicon.toml
$HOME/.config/kfind/lexicon.toml
```

예:

```toml
[[predicate]]
lemma = "플러그인하다"
pos = "verb"
alternation = "Ha"

[[nominal]]
surface = "LLM"
```

사용자 사전은 내장 사전에 추가된다. 동일 lemma의 분석을 덮어쓰려면 `replace = true`를 명시한다.

### 16.5 사전 bootstrap 전략

런타임 분석기를 쓰지 않더라도 `auto` 품사 판별에는 폭넓은 표제어 데이터가 필요하다. 다음 자료는 런타임 엔진이 아니라 릴리스 데이터 생성 단계에서만 평가한다.

- `mecab-ko-dic`: 표제어·품사 후보의 bootstrap 자료. MeCab이나 Lindera의 문장 분석 알고리즘은 사용하지 않는다. 독립 어휘 행의 headword와 품사만 추출·정규화·중복 제거하며, `Inflect`·`Preanalysis`와 문맥용 지정사 표면형은 제외한다. 상세 활용 정보는 core 사전과 gold corpus로 관리한다.
- KoParadigm: 용언·어미 분류와 활용 결과의 오프라인 참조 자료. Python 런타임 의존성으로 두지 않고, 규칙 설계와 differential fixture 생성에만 사용한다.
- 국립국어원 사전 자료: 라이선스와 재배포 조건을 충족하는 범위에서 품사와 활용 검증 자료로 사용한다. 전체 내려받기 snapshot은 릴리스 후보 데이터고, Open API는 snapshot 갱신 후보를 조사하는 개발 도구로만 사용한다.

정적 표제어 lookup과 corpus-side component 판정은 별도 resource와 품질 지표로 평가한다.
full POS 경로는 Viterbi 분석, 비용 행렬과 미등록어 처리를 사용하지 않는다. compact component
경로만 후보 token에 한정해 고정 source의 연결 비용과 미등록어 정의를 사용한다.

### 16.6 외부 사전 데이터 정책

Homebrew 패키지는 프로젝트가 작성한 core 예외 사전과 검증된 full POS lexicon을 함께 설치한다.
full POS를 로드하지 않은 `auto` 품사 판별은 preview 상태로 표시하고 명시적 품사 태그 사용을
안내한다.

우리말샘 등 외부 데이터를 활용할 경우 다음을 분리한다.

- 원본 라이선스와 출처 표시
- 예문 등 제3자 권리 가능성이 있는 필드 제외
- 소스 코드와 사전 데이터의 라이선스 구분
- 파생 데이터가 기본 바이너리에 포함되는지 별도 검토
- API 키나 네트워크 접속을 런타임 요구사항으로 만들지 않음

대규모 외부 사전은 코드와 분리된 데이터 산출물로 만든다. full POS lexicon은 최소 배포에서
제외하고 `--pos` 중심으로 동작할 수 있지만, component 판정을 제공하는 `smart` 배포는 compact
component resource를 바이너리 밖의 필수 정적 asset으로 함께 제공해야 한다.

활용 정보가 있는 source는 표제어·품사와 분리해 다음 절차로 처리한다.

1. 공개된 활용형 중 현재 alternation을 구분하는 진단형만 추출한다.
2. 하나의 alternation으로 설명되는 항목만 enriched 후보로 만든다.
3. 여러 규칙이 가능하거나 source가 충돌하면 자동 승격하지 않고 review 목록에 남긴다.
4. core fixture 또는 독립 dev case로 확인된 후보만 활용 metadata 계층에 반영한다.

자동 분류 대상은 사전 판별이 필요한 `DToL`, `DropS`, `BToWa`, `BToWo`, `DropH`,
`ReuDoubleL`, `Reo`, `UToEo`다. 같은 종성의 규칙형과 `RegularEuDrop`은 자동 승격하지 않고
분류기의 대조군으로 기록한다. 단, 같은 `(lemma, fine_pos)`에서 독립 사전 합의가 있는 불규칙형과
규칙형 source record가 각각 확인되면 불규칙형이 full POS fallback을 가리지 않도록 규칙형도
companion 분석으로 함께 보존한다. 진단형은 런타임 `generate_predicate_branches`가 해당 lexical
rule id로 생성한 anchor를 사용하며, importer에서 별도의 한글 교체 규칙을 복제하지 않는다.

자동 승격은 한국어기초사전과 표준국어대사전의 독립된 source record가 같은 `(lemma, fine_pos,
alternation, flags)`를 지지할 때만 허용한다. 우리말샘은 추가 근거와 review 자료로만 사용한다.
서로 다른 source record가 같은 `(lemma, fine_pos)`에 규칙형과 불규칙형을 각각 지지하면 두 분석을
보존한다. 하나의 source record가 둘 이상의 분류 진단형을 동시에 포함하면 자동 집계에서 제외한다.

core와 완전히 같은 분석 및 `derivations.toml`의 생산 접미 규칙으로 이미 생성되는 분석은 enriched
출력에서 제외하고 report에 중복 상태로 남긴다. `UToEo`처럼 독립 사전 합의가 있어도 이미 core에
있는 유형은 신규 행을 만들지 않는다. 승격 건수와 분류별 대조군·중복·review 건수는 생성 통계에
기록한다.

importer의 원시 레코드 grain은 `(source, source_id, raw_homonym, lemma, fine_pos)`다. 동형어
식별자를 제거한 `(lemma, fine_pos)`는 집계 키로만 사용하며, 서로 다른 source record에서
확인된 복수 alternation은 충돌로 간주하지 않고 합집합으로 보존한다. redirect, 비표준어,
방언과 옛말은 자동 승격하지 않는다.

한국어기초사전 snapshot을 XML로 읽기 전에 XML 1.0에서 허용하지 않는 바이트를 검사한다.
고정 snapshot에서 사전에 기록한 값과 위치만 제거할 수 있으며, 종류·개수·위치가 달라지면
생성을 실패시킨다. manifest에는 원본 파일명·생성일·SHA-256, 정제 내역, source별 입력·후보·
충돌·제외 건수와 생성기 version을 기록한다.
검증된 ZIP의 XML은 `${XDG_CACHE_HOME:-~/.cache}/kfind/nikl/<source>/<sha256>`에 한 번만
추출하고 이후 생성에서 재사용한다. `KFIND_NIKL_CACHE`로 cache root를 바꿀 수 있으며,
SHA-256이 달라지면 별도 디렉터리에 다시 추출한다.

사전 의존도는 품사 추측을 넓혀 낮추지 않는다. `하다`, `되다`, `시키다`, `스럽다`,
`답다`, `롭다`처럼 경계가 명확한 생산 접미 규칙과 어미 continuation을 우선 보강한다.
미등록 `다` 종결어 전체를 용언으로 추측하는 fallback은 추가하지 않는다.

## 17. Rust 기술 스택

| 목적 | crate |
|---|---|
| CLI | `clap`, `clap_complete`, `clap_mangen` |
| 파일 순회 | `ignore` |
| 검색 I/O | `grep-searcher`, `grep-matcher` |
| 단일 앵커 | `memchr::memmem` |
| 다중 앵커 | `aho-corasick` |
| 바이트 문자열 | `bstr` |
| Unicode 정규화 | `unicode-normalization` |
| 인코딩 | `encoding_rs` 또는 `grep-searcher` 연동 계층 |
| 출력 | `grep-printer`, `serde`, `serde_json` |
| 오류 | `thiserror` |
| 병렬 결과 채널 | `crossbeam-channel` |
| 작은 벡터 | `smallvec` 선택 |
| 벤치마크 | `criterion` |
| 속성 테스트 | `proptest` |
| fuzz | `cargo-fuzz` |

`ignore::WalkParallel`이 파일 단위 병렬 처리를 담당하므로 별도 `rayon` 의존성은 기본 구조에 필요하지 않다.

`memmap2`를 직접 다루기보다 `grep-searcher`의 mmap 정책을 우선 사용한다.

## 18. crate 구조

```text
crates/
  kfind/
    public engine, compiled matcher, library errors

  kfind-wasm/
    wasm-bindgen API, JavaScript option parsing, match serialization

  kfind-cli/
    args, output, exit status, shell completion

  kfind-query/
    lexer, AST, POS inference, query plan

  kfind-morph/
    Hangul operations, lexicon, endings, alternations, verifier

  kfind-matcher/
    anchor planning, memmem/Aho-Corasick, grep_matcher adapter

  kfind-search/
    ignore walk, grep-searcher integration, parallel output

  kfind-data/
    data validation and binary compilation

  kfind-testkit/
    fixture loader, reference backend, corpus generator
```

## 19. 참조 구현과 검증 전략

reference backend는 production anchor 계획과 결과 타입만 공유한다. 서술어 continuation과 조사 연쇄 판정은 production verifier를 호출하지 않고 별도 순회 구현으로 계산해 동일 결함을 공유하지 않게 한다.

### 19.1 최적화 엔진과 참조 엔진을 분리한다

프로덕션 엔진은 앵커와 verifier를 사용한다.

테스트용 참조 엔진은 동일 규칙 AST를 작은 정규 언어 또는 후보 문자열 집합으로 변환해 `regex-automata`로 실행할 수 있다.

두 엔진의 결과를 작은 corpus에서 비교한다.

```text
optimized(query, corpus) == reference(query, corpus)
```

정규식은 사용자 기능이 아니라 구현 검증 도구로만 사용한다.

### 19.2 단위 테스트

필수 테스트:

```text
걷다 → 걸어, 걸었, 걸으면, 걸으셨다
듣다 → 들어, 들었, 들으면
듣다 → 걸어 아님
묻다 → 물어와 묻어 모두
예쁘다 → 예뻐, 예뻤다, 예쁜, 예쁠
예쁘다 → 예쁘어 아님
좋다 → 좋아요, 좋았어요
아니다 → 아니고, 아니라, 아닌, 아닐
부르다 → 불러
푸르다 → 푸르러
보다 → 보아와 봐
되다 → 되어와 돼
살다 → 사는, 삽니다, 살고
사용자 → 사용자들에게
길 → 길로, 길으로 아님
```

동음이의어 정책 테스트:

```text
query: 걷다
text: 전화를 걸어 봤다.
expected: match
```

### 19.3 속성 테스트

- 음절 분해 후 조합하면 원래 음절과 같음
- 유효한 종성 교체 결과는 다시 분해 가능
- branch verifier는 anchor 밖을 읽지 않음
- 동일 span의 origin 병합은 순서와 무관
- phrase join 결과는 atom 순서를 항상 보존

### 19.4 fuzz

target과 경계:

| target | 경계 |
| --- | --- |
| `query_lexer` | 잘못된 UTF-8을 포함한 임의 query, 매우 긴 combining sequence, lexer와 compile limit |
| `matcher_bytes` | 임의 byte 입력의 anchor 탐색, suffix verifier, match span 범위 |
| `user_lexicon` | malformed 사용자 사전 TOML의 구문·의미 검증 |
| `json_output` | 임의 byte line과 검증된 match metadata의 JSON Lines 직렬화 |
| `binary_detection` | 임의 위치의 최초 NUL과 NUL이 없는 입력의 binary 판별 경계 |

### 19.5 gold corpus

공식 어문 규정의 활용 예와 프로젝트가 직접 작성한 문장을 기반으로 fixture를 만든다.
실제 사용 양상은 재배포 조건이 명확한 공개 코퍼스의 짧은 문장으로 함께 검증한다.
실제 코퍼스 항목의 `feature`는 `corpus.<source>.<split>.<id>` 형식으로 원문을 식별하고,
fixture 디렉터리의 README에 원본 revision, 라이선스, 추출 경로를 기록한다.
뉴스·대화·리뷰에서 나타나는 합성 용언, 띄어쓰기 생략, 비표준 철자는 v0.1 범위와
경계 정책에 따라 기대 결과를 정하며 형태 규칙이 지원하는 것처럼 완화하지 않는다.

각 항목:

```tsv
query	pos	text	expected	feature
걷다	verb	길을 걸어 갔다.	match	d-irregular
걷다	verb	전화를 걸어 봤다.	match	homonym-union
예쁘다	adjective	예쁘어 보인다.	no-match	eu-drop
```

문맥상 다른 표제어인 결과는 현재 정책상 false positive로 계산하지 않는다. 형태 규칙이 만들 수 없는 문자열만 false positive다.

### 19.6 외부 분석기 비교

Kiwi, Lindera, MeCab-ko와 KOMORAN 비교는 저장소의 개발 전용 검증으로 실행하며 제품 바이너리, Homebrew
의존성, 기본 검색 경로에 포함하지 않는다. 제품 fixture는 `kfind` 자체 회귀 검증에만
사용하고 외부 분석기와의 우열 점수에는 사용하지 않는다. adapter 오류와 실행 실패는
성공 결과로 대체하지 않는다.

### 19.7 독립 형태소 벤치마크

기성 분석기와의 품질 비교는 제품 fixture와 분리한 held-out corpus로 수행한다. 기본
데이터는 Universal Dependencies 2.18의 Korean-Kaist와 Korean-KSL test split이며, 원문과
라이선스 파일의 URL·SHA-256·라이선스를 manifest에 고정한다. 다운로드와 fixture 생성은
이미지 빌드 단계에서 끝내고 실제 벤치마크는 네트워크 없이 실행한다.

fixture는 도구 출력과 무관한 고정 seed로 생성한다. 각 corpus에서 명사 90, 동사 60,
형용사 40, 부사 25, 대명사 15, 관형사 10, 수사 10개를 positive로 선택한다. 같은 corpus의
다른 문장 중 동일 표제어·품사가 gold에 없는 문장을 deterministic negative로 하나씩
대응시켜 총 1,000개와 positive/negative 1:1 균형을 유지한다. 정렬과 샘플링은 원본 파일
순서가 아니라 case 식별자의 SHA-256 순서를 사용한다.

gold 후보는 CoNLL-U의 정렬된 lemma/XPOS 형태소 쌍에서 추출하고, lemma가 축약된 KAIST
어절은 `OrigLemma`를 우선 사용한다. 지원 품사에 속하고 표제어가 한글 음절로만 구성된
형태소만 포함한다. VV·VA·VX·VCP·VCN과 이에 대응하는 KAIST 용언 태그는 어간에 `다`를
붙여 사전형으로 정규화한다. 형태소 수와 XPOS 수가 끝까지 다른 어절, 접사·조사·어미,
외국어·숫자·기호는 제외한다. negative는 모든 어절의 lemma/XPOS가 정렬된 문장에서만
선택한다. 이 필터와 제외 건수는 metadata에 기록한다.

모든 도구는 동일한 `(문장, 표제어, 품사)` 존재 여부를 예측한다. positive는 예측 span이
gold 어절의 UTF-8 byte span과 겹쳐야 true positive이고, negative는 문장 어디에서든 같은
표제어·품사를 반환하면 false positive다. 도구마다 accuracy, precision, recall, F1과
TP·FP·TN·FN을 계산하고 corpus별·품사별 결과 및 실패 case를 함께 보존한다.

외부 분석기의 정규화된 결과와 성능은 test fixture SHA-256, adapter·성능 schema,
도구·사전·모델 버전과 설정에 묶인 version-controlled snapshot으로 보존한다. 기본 benchmark는 snapshot을 읽고
`kfind`만 다시 실행한다. fixture SHA-256 또는 adapter schema가 다르면 자동으로 외부 분석기를
실행하거나 오래된 결과를 사용하지 않고 refresh 명령과 함께 실패한다. 도구·사전·모델 버전과
설정은 snapshot을 명시적으로 갱신할 때만 바꾼다.

기본 benchmark 이미지는 `kfind` 측정 runner와 외부 snapshot 검증 코드만 포함한다. 외부 분석기와
전용 runner의 빌드·실행 의존성은 별도 snapshot refresh 이미지에만 포함한다. 기본 CI smoke는 기본
이미지만 빌드하며 외부 분석기 의존성을 컴파일하거나 설치하지 않는다.

성능 측정은 데이터 준비를 제외하고 backend별 warm-up 1회를 버린 뒤 동일한 case
순서로 최소 5회 반복한다. 각 run은 초기화를 한 번만 수행하고 해당 프로세스에서
전체 case를 처리한다. 초기화 시간, 전체 처리 시간, case/s, p50·p95 latency,
peak RSS의 median과 run 간 min/max를 보고한다. `kfind`는 질의 컴파일과 검색, 외부 분석기는
문장 분석과 표제어·품사 조회를 포함한 end-to-end 검색 경로를 측정한다.
이 수치는 서로 다른 검색 전략의 제품 작업량 비교이며 순수 형태소 tokenizer
처리량으로 표현하지 않는다. snapshot에 저장한 외부 성능은 refresh 환경의 참고값으로
분리하며 현재 `kfind` 측정과 같은 표에서 직접 순위를 매기지 않는다.

최종 보고서는 fixture SHA-256, seed, source별 case 수, 도구와 데이터 버전, 전체·source별·
품사별 품질 지표, 성능 지표, adapter 오류를 JSON과 Markdown으로 기록한다. 같은 JSON에서
전체 품질과 성능 trade-off SVG를 재현하고 분석 문서에 포함한다. 1,000개 미만,
class/source/POS quota 불충족, source hash 불일치, adapter 오류가 있으면 실행을 실패시킨다.

품사를 생략하는 사람용 검색은 별도 fixture에서 측정한다. positive는 같은 held-out gold span을
사용하고, negative는 query 표제어가 지원하는 모든 품사에 걸쳐 존재하지 않는 완전히 정렬된
문장으로 대응시킨다. runner는 전역 품사와 atom 태그 없이 query를 compile한다. 보고서의
`human_untagged` 절에는 embedded/full-POS와 `smart`/`any` 조합별 품질·성능, positive plan의
기대 품사 포함률, multi-coarse-POS plan 비율과 literal fallback 비율을 기록한다. fixture와
metadata hash도 명시적 품사 fixture와 분리해 기록한다. 측정 결과를 개선하기 위한 fixture,
gold, negative 선택 변경은 금지하며 생성 계약 자체의 오류를 고칠 때만 독립된 근거와 revision을
남겨 갱신한다.

`kfind` 결과는 `embedded`와 `full-pos` 프로필을 같은 fixture·case 순서로
각각 측정한다. 보고서의 버전 메타데이터에 profile과 full POS lexicon artifact
SHA-256을 기록하고, `embedded`는 artifact가 없음을 명시한다. `full-pos` 실행에서
artifact가 없거나 디코딩하지 못하면 `embedded`로 대체하지 않고 실패시킨다.
프로필별 품질·초기화·처리량·지연·peak RSS를 병렬로 보고하고, `embedded`의
false negative 중 `full-pos`에서 회복된 case와 계속 실패한 case를 별도 목록으로
저장한다.

failure 원인 분류는 성능 측정 구간 밖에서 수집한 질의 계획·anchor·경계 증거를
사용한다. 각 kfind 프로필의 false negative는 다음 우선순위로 하나의 원인을 갖는다.
호환용 `primary_cause`는 embedded 원인을 유지하고, `profile_causes`와
`profile_cause_evidence`에 embedded/full-POS 결과를 모두 기록한다.

1. snapshot의 외부 분석기 중 둘 이상이 있고 모두 같은 gold를 놓치면 `gold-or-adapter`
2. auto 질의 계획에 기대 품사 분석이 없으면 `lexicon-missing`
3. smart 결과는 있지만 gold span과 겹치지 않으면 `span-mismatch`
4. `boundary=any`만 gold span을 찾으면 `boundary-rejected`
5. gold 어절 내부에 core anchor가 있지만 검증 span이 없으면 `continuation-rejected`
6. 그 밖은 `surface-missing`

분류 증거와 profile별 primary cause는 JSON failure record에 저장한다. `boundary-rejected`
진단은 `boundary=any`에서 gold span과 겹친 match의 core·token span과 origin별 analysis index·
rule path도 보존한다. development 보고서는 full-POS positive false negative를 primary cause와
품사로 집계하고, verb·adjective `boundary-rejected` case의 query·품사·rule path를 모두 표시한다.
`ending.connective-ji` case의 any token이 gold의 strict subspan이면 두 span의 시작과 끝을 비교해
`left-edge`, `right-edge`, `internal`로 분류하고 candidate 표면형과 함께 표시한다. 같은 위치
유형을 제품 후보로 열려면 development positive와 동일한 candidate 표면형의 version-controlled
hard-negative가 있어야 한다. 이 대조가 없는 위치 유형은 계측만 유지한다.
분류를 위한 추가 컴파일·검색 비용은 backend 성능에 포함하지 않는다.

규칙 개발은 Korean-Kaist·KSL dev split을 test split과 독립된 seed·fixture
SHA-256로 생성해 사용한다. test 1,000개 baseline은 변경하지 않는다. hard-negative는
도구 출력과 무관한 버전 관리 fixture로 두고 slice별 precision을 전체 품질과 분리해
보고한다. CI smoke set은 dev fixture에서 source·품사·class별 고정 case를
deterministic하게 추출하고, 수동 벤치마크는 dev·test·hard-negative 전체를 사용한다.

명시적 품사 `smart` 형태 품질 변경은 dev precision 99.00% 이상과 version-controlled
hard-negative 신규 FP 0을 지키면서 FN을 늘리지 않아야 한다. FN이 줄어든 후보를 우선하고,
FN이 같을 때만 FP가 줄어든 후보를 선택한다. 고정 test fixture는 규칙 선택에
사용하지 않고 FN 비증가, precision 99.00% 하한과 전체 품질 회귀만 확인한다. 무품사 fixture의
결과도 같은 변경에서 다시 측정해 불리한 변화까지 기록하되 규칙 선택이나 fixture 변경 근거로
사용하지 않는다. 최종 품질 주장은 구현 전에 source·fixture를 고정하고 기존 corpus와 문장 hash
중복이 없는 unseen 평가에서도 같은 기준을 통과해야 한다. 기본 `smart`를 변경하는 구현은 기존
hard-negative에 새 FP를 추가하지 않아야 하며, 이 조건을 만족하지 못하면 별도 boundary policy로
분리한다.

## 20. 성능 사양

### 20.1 목표

기준 장비와 corpus는 벤치마크 보고서에 고정한다. 예시는 Apple Silicon의 최근 세대 장비로 두되, 결과에는 CPU, 메모리, 저장장치, OS를 반드시 기록한다.

제품 목표:

```text
단일 atom query compile p95: 15 ms 이하
8 atom phrase compile p95: 50 ms 이하
낮은 hit 비율의 scan: rg -F wall time의 1.5배 이내
낮은 hit 비율의 처리량: rg -F의 70% 이상
기본 RSS: 40 MiB 이하
corpus 크기에 비례하는 결과 버퍼링 없음
```

`rg -F`와 기능이 동일하지 않으므로 절대 우열이 아니라 I/O 경로의 성능 회귀 감시 기준으로 사용한다.

### 20.2 corpus

```text
100 MiB source corpus
1 GiB mixed corpus
한글 비율 5%, 20%, 80%
작은 파일 다수 corpus
큰 파일 소수 corpus
NFC corpus
NFD corpus
UTF-16 fixture
```

corpus 생성기는 전체 bytes, 파일 수, 작은 파일 수와 크기, 한글 line 선택 비율, 한글 line의 NFD 선택 비율, seed를 명시적으로 받는다. 같은 설정과 seed는 byte 단위로 동일한 파일 tree를 생성해야 한다. NFC/NFD와 한글 비율은 완전한 line을 선택하는 비율이며, 파일 끝의 exact-size padding은 ASCII로 채운다.

### 20.3 측정 구간

다음 시간을 분리한다.

```text
startup
lexicon load
query compile
filesystem walk
scan
verification
output
```

query compile 목표는 lexicon을 미리 로드한 같은 analyzer를 재사용하고 다음 두 입력을 각각
`query_compile/single_atom`과 `query_compile/phrase_8_atoms` Criterion benchmark로 측정한다.

```text
single_atom: 걷다
phrase_8_atoms: n:사용자 n:권한 v:검증하다 adj:예쁘다 det:새 adv:빨리 n:기술 v:걷다
```

`matcher/phrase_find_all`은 1,024개 line 중 4개마다 `n:길 v:걷다`가 일치하는 고정 corpus를 메모리 입력으로 사용한다. smart boundary의 component 검증에 필요한 고정 resource를 matcher 생성 시 제공한다. 전체 phrase match를 반환하는 한 번의 호출을 측정해 match 수에 따른 반복 anchor scan과 span 결합 회귀를 감시한다.

`matcher/phrase_find_all_repeated`는 같은 한 음절 literal atom 8개와 한 줄의 반복 span 128개,
큰 `max-gap`을 사용한다. 가능한 조합 수와 무관하게 bounded DP로 leftmost-longest 결과를 찾는
병적 입력 경로를 측정한다.

`matcher/phrase_input_searcher_repeated_line`은 줄바꿈 없는 한 줄에서 인접한 두 literal atom
phrase가 4,096번 반복되는 입력을 `InputSearcher`의 metadata 출력 경로로 검색한다. 한 줄의
anchor와 atom span을 한 번만 수집하는지와 match 수에 따른 반복 suffix scan 회귀를 감시한다.

`matcher/context_repeated_long_line`은 `매일`이 16,384번 반복되는 줄바꿈 없는 UTF-8 입력을
`smart` 부사 matcher로 검색한다. 각 candidate의 인접 token만 해독하는지와 candidate마다 전체
입력의 UTF-8을 다시 검증하는 회귀를 감시한다.

`local_lattice/component_decision`은 고정 component fixture를 한 번 초기화한 뒤 accept, reject와
ambiguous 입력을 순환하며 제품용 component 판정만 측정한다. `local_lattice/component_report`는
같은 입력에서 진단 경로 생성 비용을 별도로 측정한다. 구현 변경 전후를 같은 build profile과
Criterion 설정으로 비교하고, 제품 판정 p95가 10% 이상 악화되면 회귀로 본다. 이 microbenchmark는
1,000-case morphology 품질·성능 보고서를 대체하지 않는다.

p95는 Criterion `new/sample.json`의 각 sample에 대해 `times[i] / iters[i]`로 계산한
1회당 nanoseconds를 오름차순으로 정렬하고 nearest-rank 방식으로 선택한다. 정식 목표 판정은
기본 sample 설정으로 수행한다. `--quick` 결과는 benchmark가 실행되는지만 확인하는 smoke
측정이며 목표 판정에 사용하지 않는다.

`--count`, `--quiet`, 기본 출력, JSON을 별도로 측정한다. cold cache와 warm cache 결과를 구분한다.

인수 기준 9의 `rg -F` 비교 runner는 동일 corpus와 no-match literal을 대상으로 `--quiet` warm-cache scan을 측정한다. 처리량은 정확한 corpus bytes를 wall time으로 나눈 값이고, maximum RSS의 단위와 수집 도구를 보고서에 함께 쓴다.
각 scan은 새 프로세스의 startup을 포함하되, literal 쿼리에 필요하지 않은 full POS lexicon 로드는 수행하지 않는다.

### 20.4 회귀 정책

동일 CI runner에서 main 기준 다음 중 하나면 경고한다.

```text
query compile 20% 이상 악화
scan throughput 10% 이상 악화
RSS 20% 이상 증가
branch 수 2배 이상 증가
```

## 21. Homebrew 배포

### 21.1 배포 형태

Homebrew는 custom tap으로 배포한다.

```bash
brew install seokminhong/brew/kfind
```

릴리스 구성:

```text
kfind source tarball
Cargo.lock
내장 규칙과 사전 소스
생성된 man page
shell completions
agent skill
checksums
```

런타임 모델 다운로드는 없다. full POS lexicon을 별도 파일로 배포하면 formula의 resource 또는 별도 release artifact로 함께 설치하고, 코드와 데이터의 라이선스를 각각 표시한다.

### 21.2 formula 설치 항목

```text
bin/kfind
share/man/man1/kfind.1
share/zsh/site-functions/_kfind
share/fish/vendor_completions.d/kfind.fish
etc/bash_completion.d/kfind
share/kfind/skills/kfind/SKILL.md
share/doc/kfind/LICENSES/
```

내장 규칙과 프로젝트 자체 사전은 실행 파일에 포함한다. 선택형 대규모 사전만 `share/kfind` 아래에 둘 수 있다.

### 21.3 bottle

검증 대상:

```text
macOS arm64
```

CI에서 tagged release의 bottle을 생성한다. formula test는 임시 파일을 만들고 실제 형태 검색을 확인한다.
JSON 검증은 JSON Lines record의 종단 LF와 `text` 필드를 구분하며, `text`에는 원문 줄의 종단 LF를
포함하지 않는다.

```ruby
test do
  (testpath/"sample.txt").write("길을 걸어 갔다.\n")
  output = shell_output("#{bin}/kfind 걷다 #{testpath}/sample.txt")
  assert_match "걸어", output
end
```

## 22. 보안과 견고성

- 모든 파일 크기와 branch 수에 상한을 둔다.
- phrase matcher의 DP 상태는 검증된 atom span 수의 합에 비례하며, 전체 조합용 API의 중간
  partial 수에는 명시적 상한을 둔다.
- 사용자 사전 파싱 오류에는 파일명과 줄 번호를 표시한다.
- symlink 순환을 방지한다.
- 검색 결과, 검색 중 issue, 초기화 오류를 포함한 모든 사람이 읽는 출력에 escape 정책을 적용해 제어 문자가 터미널 동작을 바꾸지 않게 한다.
- JSON에는 원문 제어 문자를 정상 escape한다.
- 파일 경로가 유효 UTF-8이 아니어도 처리한다.
- broken pipe에서 panic하지 않는다.
- matcher와 verifier는 unsafe 없이 구현하는 것을 기본 원칙으로 한다.

## 23. 제품 인수 기준

다음 조건을 모두 만족해야 한다.

1. 코퍼스 전체 형태소 분석기 없이 동작한다.
2. `걷다`에서 불규칙 분석의 `걸어`, `걸었다`, `걸으면`, `걸으셨다`와 규칙 분석의 `걷어`, `걷었다`를 모두 찾는다.
3. `듣다`에서 `들어`를 만들고 `걸어`를 만들지 않는다.
4. `묻다`에서 규칙형과 ㄷ 불규칙형을 모두 검색한다.
5. `예쁘다`에서 `예뻐`, `예쁜`, `예쁠`을 찾고 `예쁘어`를 만들지 않는다.
6. 체언에서 모든 조사 문자열을 미리 전개하지 않고 verifier로 처리한다.
7. phrase query를 후보 문자열 데카르트 곱 없이 span 결합으로 처리한다.
8. 동일 표면형의 모든 생성 근거를 JSON에서 보존한다.
9. 1 GiB corpus benchmark와 rg -F 비교 보고서가 있다.
10. Homebrew로 설치한 뒤 네트워크 접속 없이 실행된다.
11. macOS arm64에서 formula test가 통과한다.
12. 사용자 사전 없이도 핵심 불규칙 fixture가 통과한다.
13. Homebrew 기본 설치에서 full POS lexicon이 로드되고, 사전 누락 시 명확한 진단을 출력한다.
14. 공개 Rust 라이브러리가 동일한 query plan과 matcher를 사용해 메모리 입력을 검색한다.
15. 공개 라이브러리와 핵심 의존 crate가 Rust 1.97의 `wasm32-unknown-unknown` target에서
    빌드된다.
16. `kfind` npm 산출물의 Node smoke test, TypeScript declaration 검사와
    `npm pack --dry-run`이 통과한다.
17. native package는 compact component resource를 `share/kfind`에 설치하고 resource 누락·손상 시
    component `smart` query를 초기화 오류로 종료한다.
18. WASM binary는 compact artifact를 포함하지 않고, 외부 또는 별도 정적 asset bytes를 받은
    생성자에서 schema·source·digest를 검증한다.
19. man page와 영어·한국어 README가 사람용 무품사 기본 경로와 에이전트 자동화 경로를 구분하고,
    에이전트 예시는 명시적 품사, `any`, embedded와 JSON 출력을 사용한다.
20. 품사를 생략한 held-out 검색의 품질·성능 benchmark가 별도 fixture와 보고서 절로 존재한다.
21. `kfind --init`은 TTY checkbox, 반복 `--agent`, 비TTY stdin에서 같은 agent 대상 집합을
    설치한다.
22. Claude Code, Codex와 Gemini CLI의 project skill 경로에 같은 원본의 `SKILL.md`를 설치하며
    `custom`은 skill 원문 외의 내용을 stdout에 섞지 않는다.
23. 관리하지 않는 기존 skill은 보존하고 init 실패를 exit code 2와 escape된 진단으로 보고한다.
24. Homebrew formula는 agent skill 원본을 설치하고 project link가 stable `opt` 경로를 사용해
    upgrade 뒤 새 원본을 가리킨다.
25. 일반 text 검색의 TTY stdin/stdout은 검색 중 결과를 점진적으로 표시하는 resize 가능한 내장
    pager에서 긴 match 줄을 match별 행으로 펼치고 target 앞뒤 비율에 맞춰 생략하며,
    `--no-pager`, non-TTY와 agent JSON 출력은 기존
    stdout stream을 유지한다.

## 24. 공개 코드 인터페이스

Rust 공개 API는 재사용 가능한 `Engine`과 컴파일된 `Matcher`를 중심으로 한다.

```rust
let mut engine = Engine::new()?;
engine.load_component_resource(component_bytes)?;

let matcher = engine.compile("권한", &CompileOptions::default())?;
let matches = matcher.find_all("사용자권한을 확인한다.".as_bytes());
```

- `Engine::new`는 embedded lexicon만 초기화한다.
- `with_full_pos`, `with_component_resource`, `with_full_pos_and_component`와
  `from_lexicons_with_component`는 caller가 전달한 resource를 생성 시 검증한다.
- `load_component_resource`는 새 bytes를 모두 검증한 뒤 상태를 교체하며 실패하면 기존
  resource를 보존한다.
- `compile`은 query plan과 anchor matcher를 만들고 component resource가 필요한 plan의 누락을
  `ComponentResourceRequired`로 보고한다.
- `Matcher::find_at`과 `find_all`은 UTF-8 byte offset과 형태 provenance가 포함된
  `PhraseMatch`를 반환한다.
- JavaScript API는 같은 수명 주기를 `Kfind`, `loadComponentResource`, `compile`, `Matcher.findAll`
  로 노출하고 offset을 UTF-16 code unit으로 변환한다.

## 25. 제품 원칙

`kfind`의 핵심은 “모든 문장을 분석하는 것”이 아니라 다음 세 가지다.

```text
표제어를 정확히 해석한다.
검색 가능한 형태 규칙을 유한한 계획으로 컴파일한다.
원문에서는 긴 고정 앵커를 찾고 필요한 위치만 검증한다.
```

이 원칙을 지키면 형태 품질은 사전과 규칙 fixture로 개선할 수 있고, 검색 성능은 기존의 검증된 파일 순회·바이트 검색 계층을 활용해 유지할 수 있다.

## 26. 참고 자료

- [Unicode Standard Annex #15, Unicode Normalization Forms](https://www.unicode.org/reports/tr15/)
- [Unicode 17.0 Core Specification, Chapter 3](https://www.unicode.org/versions/Unicode17.0.0/core-spec/chapter-3/)
- [국립국어원 한국어 어문 규범](https://korean.go.kr/kornorms/regltn/regltnView.do)
- [Rust `aho-corasick` documentation](https://docs.rs/aho-corasick/)
- [Rust `memchr::memmem::Finder` documentation](https://docs.rs/memchr/latest/memchr/memmem/struct.Finder.html)
- [Rust `ignore` documentation](https://docs.rs/ignore/)
- [Rust `grep-searcher` documentation](https://docs.rs/grep-searcher/)
- [Rust `grep-matcher` documentation](https://docs.rs/grep-matcher/)
- [Rust `grep-printer` documentation](https://docs.rs/grep-printer/)
- [Homebrew Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)
- [Homebrew Bottles](https://docs.brew.sh/Bottles)
- [Homebrew Taps](https://docs.brew.sh/How-to-Create-and-Maintain-a-Tap)
- [우리말샘 저작권 정책](https://opendict.korean.go.kr/service/copyrightPolicy)
- [우리말샘 Open API 안내](https://opendict.korean.go.kr/service/openApiInfo)
- [KoParadigm repository](https://github.com/Kyubyong/KoParadigm)
- [KoParadigm paper](https://arxiv.org/abs/2004.13221)
- [mecab-ko-dic repository](https://bitbucket.org/eunjeon/mecab-ko-dic)
