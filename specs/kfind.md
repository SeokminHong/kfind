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
  판별, 동음이의어·동형이의어 해소, semantic search와 임의 표면형의 완전한 역분석은
  non-goal이다. 외부 형태소 분석기 비교는
  제품 workflow의 품질·비용을 보정하는 근거이며 동일한 tokenizer backend 순위를 뜻하지 않는다.
- `걸음을 걷다`, `팔을 걷다`, `발을 걸다`의 각 표제어 query는 형태·표면 규칙이
  허용하면 모두 `걸었고`에 match할 수 있다. corpus 문맥으로 어느 의미인지
  고르지 않으며, 의미적 중의성은 false positive로 계산하지 않는다.
- `smart`는 corpus 전체를 분석하지 않지만, compact component resource가 증명하는
  바로 인접한 어절의 형태 구조를 bounded evidence로 사용할 수 있다. 이 근거는
  품사·component·continuation 가능성을 검증하지 의미를 판별하지 않는다.

### 0.1 규칙 데이터와 품질 기준

- v0.1의 필수 형태 범위는 9.5절의 활용표, 19.2절의 필수 테스트, 23절의 인수 기준을 모두 포함한다.
- gold corpus에 포함된 현재 평서형 `-ㄴ다/는다`와 상태 용언의 `-다`, 회상 관형형 `-던`, 양보 연결형 `-더라도`, 과거 관형 연쇄 `-았/었을`, 과거 의문 종결 연쇄 `-았/었느냐`, `-았/었느냐는`, 이유 연결형 `-(으)니`, 인용 연결형 `-다고`, 현재 서술형 인용·회상·조건 연쇄, 전망 종결형 `-(으)리라`와 인용 연쇄 `-(으)리라고`, 의도 연결형 `-(으)려고`, 상태 변화 보조 용언 `-아/어지다`, 진행 방향 보조 용언 `-아/어가고`, `-아/어가야`도 v0.1의 제한된 continuation vocabulary에 포함한다.
- 실제 코퍼스에서 확인된 해요체 과거형 `-았어요/-었어요`, 지정사 `이다`의 높임 평서형
  `입니다`와 인용·관형·대조·나열형 `이라고`·`이라는`·`이지`·`이며`, 부정 지정사 `아니다`의
  연결형 `아니라`도 v0.1의 제한된 continuation vocabulary에 포함한다. 지정사 확장은 이 네
  완성형만 직접 생성하며 무표면 축약 `겁니다`와 비표준 `이예요`를 합치지 않는다.
- 어미, 조사 연쇄, 파생 규칙은 저장소에서 버전을 관리하는 `data/rules` 파일의 목록과 전이를
  기준으로 삼는다. 목록 밖 조합은 생성하지 않는다.
- full POS lexicon은 `mecab-ko-dic 2.1.1-20180720`의 Apache-2.0 데이터를 bootstrap 원본으로 사용한다. 빌드 시 표제어와 품사만 추출하고, 런타임 문장 분석 데이터와 알고리즘은 포함하지 않는다. `Inflect`와 `Preanalysis` 행은 제외하며, 문맥용 지정사 표면형은 표제어로 승격하지 않고 `VCP=이`, `VCN=아니`만 기본형으로 정규화한다.
- full POS lexicon의 용언 품사 후보도 POS 전용 산출물에 보존한다. 동일 표제어와 coarse 품사에
  core 또는 enriched 용언 분석이 하나라도 있으면 그 coarse 품사의 full POS 규칙형 분석은
  추가하지 않는다. 다른 coarse 품사는 보존한다. 그 밖의 용언은 해당 품사와 일치하는 생산적
  접미 규칙을 먼저 적용하고, 일치하는 규칙이 없을 때만 제한된 규칙형 분석을 사용한다.
- full POS runtime resource는 검증된 정렬 lookup index로 보존한다. CLI, Rust library와 WASM
  binding은 초기화할 때 전체 entry를 일반 분석 map이나 entry별 소유 문자열로 전개하지 않는다.
  Front-compressed 표제어는 하나의 재사용 문자열 scratch에서 복원·검증한 뒤 packed lemma
  bytes와 offset·품사 index로 보존하고, query atom의 표제어를 조회할 때 일치하는 품사
  후보만 `Analysis`로 만든다. 진단 API가 전체 entry를 명시적으로 요청할 때만 소유 entry
  view를 지연 생성한다. 새 suffix의 UTF-8을 검증한 뒤 이미 검증된 prefix에 붙여 전체
  표제어의 UTF-8을 보장한다. ASCII와 완성형 한글 음절만 있는 표제어는 그 구성으로 NFC를
  증명하고 그 밖의 표제어는 일반 NFC 검사를 수행한다. 엄격한 정렬 순서, entry 수와 누적
  decoded byte 상한 검증도 유지한다.
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
- 명시한 coarse `verb`의 주동사 분석이 있으면 같은 표제어의 보조동사 후보도 보존한다. 이
  후보는 임의의 내부 동사 substring을 허용하지 않고 compact component resource가
  `용언 + 연결 어미 + VX + 선택적 어미`의 완성 경로를 증명할 때만 매치된다. full POS의
  주동사 분석 하나가 coarse `verb`에 포함되는 보조동사 구조를 억제하지 않는다.
- core lexicon은 전체 표제어 목록이 아니라 불규칙 활용, 품사 중의성, 기능어, 표면형 override를 담는 예외 계층이다. embedded workflow의 검증된 주요 불규칙은 core에 유지한다. 자동 승격 기준을
  충족하지 못한 review 항목은 표준국어대사전과 우리말샘의 고정 snapshot이 같은 진단형을
  지지하고 충돌하는 규칙형 record가 없으며 독립 fixture가 활용과 오활용을 함께 검증한 경우에만
  수동 core 예외로 둘 수 있다. 공개 사전에서 일괄 승격한 활용 metadata는 별도 enriched 계층으로
  관리하며, core entry 수를 corpus recall에 맞춰 무제한 늘리지 않는다.
- core lexicon의 `DropH` 형용사는 검증된 ㅎ 불규칙 표제어를 명시한다. `어떻다`, `이렇다`,
  `커다랗다`는 각각 `어떤`, `이런`, `커다란` 관형형을 만들고 규칙형 `어떻은`, `이렇은`,
  `커다랗은`은 만들지 않는다.
- full POS 산출물은 전체 entry 수, 고유 표제어 수, 품사별 entry 수를 기계 판독 가능한 통계 파일로 포함한다. source를 추가하거나 갱신할 때는 이 통계와 충돌·제외 건수의 변화를 검토한다.
- 공개 사전은 고정된 전체 내려받기 snapshot만 릴리스 입력으로 사용한다. 원본 URL·버전 또는 생성 일자·SHA-256·라이선스·추출 필드·추출기 버전을 기록하며, 인증키가 필요한 live API 응답은 릴리스 빌드 입력이나 런타임 의존성으로 사용하지 않는다.
- 여러 source의 표제어·품사 후보는 합집합으로 보존하되, 같은 표제어에 core 용언 분석이 있으면 core의 활용 metadata를 우선한다. source 간 품사 충돌과 활용 분류 미확정 항목은 산출물 통계로 보고하고 임의로 한쪽을 삭제하지 않는다.
- 배포 데이터에는 원본 버전, 출처, 라이선스, 추출 명령과 체크섬을 기록한다.
- auto 품사 coverage 기준은 300개 이상의 프로젝트 gold case마다 명시된 기대 품사 분석을 포함하는 것이다. 품사별 형태 match와 no-match는 fixture 품사를 강제해 해당 분석의 허용·금지 형태를 검증하고, 품사를 생략한 제품 동작은 0.6절의 사람용 fixture와 persona profile로 분리한다. 핵심 불규칙 fixture는 core lexicon만으로도 100% 통과해야 한다.
- full POS lexicon이 없으면 core lexicon으로 계속 실행하되, `--explain-query`와 명시적 사전 진단 요청에서 `preview (core lexicon only)` 상태와 자동 탐색한 모든 후보 경로를 우선순위대로 출력한다. 로드했을 때는 `loaded`와 선택된 경로를 출력한다.
- `--explain-query`는 계획 전체의 Unicode 정규화 모드와 atom별 program 수, structural
  program 수와 consumption state 수를 출력한다. consumption state 수는 해당 atom의
  program들이 참조하는 서로 다른 조사·어미 소비 구성의 수다.

### 0.2 토큰 경계와 phrase 거리

- 토큰 문자는 Unicode 문자·숫자·결합 문자와 `_`다. 한글 완성형과 자모도 토큰 문자에 포함한다.
- `smart`는 program의 consumption이 허용된 조사·어미를 소비한 token span의 바깥 경계를
  검사한다. 체언, literal, 한 음절 atom은 core 시작도 토큰 경계여야 한다. 단, 조사를 직접
  검색할 때는 붙은 조사를 찾을 수 있도록 core 왼쪽 경계 대신 바로 앞 host와 조사 이형태
  조건을 검증한다. 무품사 입력은 사용자가 쓴 조사 표면형만 찾고, 조사 이형태 묶음 확장은
  명시적 조사 품사 입력에서만 사용한다.
- 명시적 체언 품사의 `smart` program에서 query core가 조사 없는 token 전체와 정확히 같으면,
  compact component resource에 같은 whole 분석이 없어도 완성된 체언 token으로 인정한다.
  이 경로는 token 내부에서 시작한 core, 조사나 다른 문자를 소비한 candidate와 component
  경계를 가로지르는 substring에는 적용하지 않는다.
- 일반 용언의 `smart` token span은 core에서 시작한다. 따라서 `가다` 검색은 `친구가`의 붙은 조사 `가`를 활용형으로 인정하지 않는다. 지정사처럼 앞 host에 붙는 분석만 별도 왼쪽 환경 검증을 사용한다.
- 지정사 `smart` candidate는 token 전체가 부사로 분석되지 않고, token 왼쪽 경계부터 VCP core
  직전까지 완성된 체언 host 또는 `체언 + 검증된 조사 연쇄`가 있을 때 host에 붙은 VCP runtime
  component를 유지한다. 생성 branch가 token 끝까지 직접 소비하지 못해도 같은 VCP source node가
  core 시작부터 token 끝까지 이어지면 완성된 continuation으로 인정한다. 모음으로 끝나는 host 뒤의
  `다`, `였-`, `여-` branch는 지정사 탈락·축약의 왼쪽 음운 조건과 완결된 활용을 함께 검증한다.
  따라서 `상표다`, `구경거리였다`, `대학뿐이다`의 지정사를 지원하지만, 체언 host가 없거나
  whole-token 부사인 `매일`, 받침 뒤에서 축약한 `대학다`는 이 경로로 열지 않는다.
- 명시적 동사·형용사 품사의 `ending.connective-ji` program은 `smart`에서도 core 왼쪽 token 경계를
  요구하지 않고 완성된 token span의 오른쪽 경계는 유지한다. 이는 gold 어절의 오른쪽 끝과
  일치하는 suffix candidate만 복구한다. 무품사 `smart`, `token`, `any`와 `ending.connective-ji`
  뒤에 문자가 더 남는 left-edge candidate는 바꾸지 않는다.
- 용언의 `ending.past`와 `ending.future` consumption state는 `ending.connective-eudoe`의 `으되`를
  소비한다. `치렀으되`, `하겠으되`처럼 선어말어미 뒤의 완성된 token만 복구하며, bare stem에
  `으되`를 붙이는 별도 경로는 이 규칙으로 추측하지 않는다.
- `-아/어`가 어간과 같은 음절로 축약된 용언 program도 후행 문자열 전체가 compact resource의
  `VX` 보조용언+어미 연쇄로 증명되면 완성된 token까지 소비한다. 축약 때문에 anchor byte span이
  core보다 길지 않아도 되지만, 후행 연쇄가 완전한 구조 경로를 만들지 못하면 확장하지 않는다.
- full-POS `smart`의 `VX` query는 compact resource가 token 왼쪽 경계부터 일반 용언과
  `EC`로 candidate core 직전까지 이어지고, core에 정렬된 `VX`와 선택적 어미가 token 끝까지
  이어지는 완전한 path를 증명할 때 token 내부 보조용언을 유지한다. 용언 시작은 `VV/VA`
  또는 `XR + XSV/XSA`이며, `EC + VX + E+`가 한 source edge에 묶이거나 여러 edge로 나뉜
  경우를 같은 경로로 조립한다. 선행 일반 용언이나 core 직전 `EC`가 없는 내부 substring은
  이 경로로 열지 않는다.
- 이유·근거·전제를 나타내는 `ending.connective-ni`는 `-니/-으니`, `-니까/-으니까`,
  `-니까는/-으니까는`과 그 준말 `-니깐/-으니깐`을 완성된 predicate token으로
  소비한다. 받침 없는 어간과 `ㄹ` 받침 어간은 `으`가 없는 이형태, 그 밖의 받침
  어간은 `으`가 있는 이형태를 쓴다. `살으니까`, `먹니까`, `먹니깐`처럼 잘못된
  이형태와 `부니깐은`처럼 완성된 어미 뒤의 추가 연쇄는 생성하지 않는다.
- 동작 용언의 `-ㄴ다/는다`와 상태 용언의 `-다` 현재 평서형 program은
  `ending.declarative` consumption state에서 제한된 continuation만 소비한다. 상태 용언은 형용사와
  보조 형용사이며 지정사와 부정 지정사 `아니다`는 포함하지 않는다. 허용 목록은
  `고`(`ending.quotative-go`),
  `는`(`ending.quotative-adnominal`),
  `던`(`ending.quotative-retrospective`), `면`(`ending.conditional`),
  `니`(`ending.quotative-ni`), `며`(`ending.quotative-myeo`),
  `면서`(`ending.quotative-myeonseo`), `는데`(`ending.quotative-neunde`),
  `지`(`ending.quotative-ji`)다. `쓴다고`, `먹는다는`, `받든다는`, `함께한다던`, `좋다는`,
  `나쁘다면`, `어렵다면서`처럼 이 목록으로 끝나는 token과 bare `쓴다`, `먹는다`, `좋다`는
  허용한다. 동작 용언의 사전형 `가다`에는 이 상태를 부여하지 않으므로 `가다면`은 거부한다.
  종결형 뒤의 `거나/든가/든지` 조사와 `니요/던데` 같은 추가 연쇄는 이 상태에서 추측하지 않는다.
- 체언의 접속 조사 `이면/면`은 받침 있는 host 뒤에서 `이면`, 받침 없는 host 뒤에서 `면`을
  소비한다. `백이면 백`, `공부면 공부`처럼 같은 자격의 대상을 잇는 terminal 조사만 허용하고
  `백면`, `공부이면`과 이 조사 뒤의 추가 조사 연쇄는 거부한다.
- `token`은 모든 품사에서 core 시작과 완성된 token span 끝의 토큰 경계를 검사한다.
- `any`는 좌우 경계를 검사하지 않는다.
- phrase의 `max-gap`은 앞 atom의 `token.end`와 다음 atom의 `token.start` 사이에 있는 Unicode scalar 수다. 음수이거나 순서가 뒤집힌 span은 결합하지 않는다.

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
  출력 형식과 종료 코드를 이해할 수 있어야 한다. README에는 현재 제품 동작과 안정적인 사용
  지침만 둔다. 측정일·Git revision, baseline/candidate 증감, 날짜별 보고서·작업 목록, 완료
  이력과 측정 snapshot 표·차트는 넣지 않는다. 재현 명령과 측정 결과는 날짜별 benchmark
  보고서에 보존하고 README는 benchmark 계약 문서만 안내한다. 승인된 보고서나 생성 차트가
  바뀌어도 측정 수치를 README로 복사하지 않으며, 사용자에게 설명할 현재 기능·제약이 달라진
  경우에만 README의 동작 설명을 갱신한다.
- `--column`은 v0.1 정식 옵션이며 1부터 시작하는 Unicode scalar 열을 출력한다.
- `--count`는 파일별로 검증된 span이 하나 이상 있는 줄의 수를 출력한다.
- 일반 text 결과를 TTY stdin/stdout에서 쓰면 내장 TUI pager를 자동으로 사용한다. 검색 시작과 함께
  TUI를 열고 완성된 결과 행을 점진적으로 반영한다. 화면 너비를 넘는 match
  줄은 검증된 match마다 별도 행으로 펼치고 각 행의 target이 보이도록 앞뒤를 생략한다. target의
  화면 위치는 원문에서 target 앞뒤가 차지하는 비율을 따르되, 양쪽 원문이 모두 남아 있으면 가용
  문맥의 20–80% 안으로 제한한다. terminal resize 때 너비, 생략 위치와 행 분할을 다시 계산하며
  위·아래 화살표는 이 행 단위를 이동한다. 마지막 행은 content viewport의 마지막 행에 놓이는
  지점까지만 이동한다. 키 반복 입력은 content viewport의 cell 수에 따라 16–48 ms 간격의 frame으로
  합치되 입력된 이동량은 보존하고, 새로 노출된 행만 갱신한다. `--no-pager`,
  non-TTY stdin/stdout, JSON Lines, count, 파일명
  요약과 quiet mode는 pager를 사용하지 않고 기존 bounded stdout stream을 유지한다. TUI를 시작할
  수 없을 때는 일반 text를 직접 stdout에 쓴다. 에이전트 권장 경로의 JSON Lines는 stdout이 TTY여도
  비대화형 출력을 유지한다.
- TUI는 완성된 source line마다 임시 파일 offset·length를, 현재 너비에서 전개된 화면 row마다
  source·target key를 메모리에 보존한다. 따라서 임시 파일과 별도로 source line 수와 전개된 row
  수에 비례한 index 메모리를 사용한다. 현재 자동 결과 상한이나 대용량 fallback은 없으며 대규모
  결과를 stream으로만 처리하려면 `--no-pager`를 사용한다.
- TUI index benchmark는 plain source line과 한 source line이 여러 match row로 전개되는 입력을
  각각 측정한다. 입력 bytes, source·row 논리 개수, 각 `Vec`의 length·capacity와 entry 크기,
  length·capacity 기준 index bytes, 생성·index·layout 시간과 fresh process peak RSS를 함께 보고한다.
  benchmark용 binary는 release 설치물에 포함하지 않는다.
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
  /concepts/optimization    program·anchor·resource·streaming 최적화
  /benchmarks               workload별 품질·성능 근거
  /playground               WebAssembly 플레이그라운드
  ```

- 문서 site의 popup, select, collapsible과 form control은 `@base-ui/react`의 unstyled primitive로
  구성한다. 링크, label, keyboard와 pointer 동작은 해당 primitive의 접근성 의미를 유지하고,
  제품 고유 동작만 route component에서 추가한다.

- 단어장은 검색 입력, 실행 구조, resource와 품질 지표에 쓰는 핵심 용어를 한곳에서
  정의한다. 한국어 표기와 코드·영문 표기는 같은 항목에서 대응시키고, 다른 문서의 설명은
  이 정의와 모순되지 않아야 한다.
- 각 문서 route는 단어장 용어가 본문에서 처음 등장하는 한 곳에만 tooltip과 해당 정의 link를
  제공한다. 같은 항목의 한국어·영문 별칭은 한 용어로 센다. Tooltip은 hover와 keyboard focus로
  열 수 있어야 한다. 실제 mouse pointer activation과 keyboard Enter activation은 기존 link 동작을
  유지한다. Touch·pen pointer activation과 선행 input event가 없는 link activation은 첫 번째에
  tooltip을 열고, 같은 용어의 다음 activation에 단어장으로 이동한다. 이 구분에 media query나 click
  metadata를 사용하지 않는다. Code·기존 link·form control과 단어장 자체에는 중첩해서 적용하지
  않는다.
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
  분석·아키텍처·최적화 문서는 query compile부터 anchor scan, 국소 구조 판정, span·provenance
  반환까지의 흐름과 corpus 전체를 분석하지 않는 이유를 텍스트와 접근 가능한 도해로 설명한다.
- playground는 현재 source의 `kfind-wasm`을 browser용 WebAssembly로 빌드해 embedded lexicon으로
  실행한다. Query, 입력 text, expand·boundary·POS·max gap을 바꿀 수 있고, UTF-16 span에 맞춰
  match를 강조하며 surface와 provenance를 표시한다. Browser 사용자가 Unicode normalization을
  선택하지 않도록 canonical NFC+NFD 검색을 고정 적용한다.
- Query와 입력 text는 검색 작업의 주 입력으로서 playground 상단의 한 input stack에 이 순서로
  인접 배치한다. 넓은 화면은 짧은 Query와 장문 text editor를 왼쪽 main pane에 두고 예시 action,
  compile option과 component resource를 오른쪽 보조 panel에 둔다. 검색 결과는 두 pane 아래의 전체
  너비 하단 panel에 표시한다. Query control은 일반적인 짧은 입력에 맞춰 너비를 제한하되 text
  editor는 main pane을 채운다.
- 좁은 화면은 Query → text → 결과의 인지 순서를 우선한다. 예시 action, compile option과 component
  resource는 현재 주요 option 요약을 표시하는 `검색 옵션` button으로 여는 modal 안에 두며 결과보다
  앞에서 긴 설정 목록을 펼치지 않는다. Modal은 keyboard focus trap, touch scroll lock, 명시적인 닫기
  control을 제공한다. 모든 화면에서 가로 scroll을 만들지 않는다.
- 검색 예시는 query, text와 관련 compile option을 하나의 설정으로 불러오는 action button으로
  제공한다. 예시 action과 개별 option control은 같은 input state를 갱신하고, 별도의 preset 선택
  상태를 유지하지 않는다. 1 MiB의 결정적인 입력을 만드는 대용량 예시를 제공하며 editor에는
  문자 수와 UTF-8 byte 수를, 검색 결과에는 query compile과 전체 text scan을 합친 실행 시간을
  표시한다. 예시 action은 짧은 button row로 줄바꿈하며 단순 목록을 별도 card grid처럼 크게
  그리지 않는다. Compile option은 현재 값과 설명을 확인할 수 있되 주 입력보다 시각적으로 앞서지
  않는 compact control grid로 배치한다.
- Playground는 query·text·option 변경을 debounce한 뒤 자동으로 검색하며 별도의 검색 실행
  button을 두지 않는다. Query label에서 지원 atom 태그와 품사를 확인할 수 있어야 한다. POS
  control은 atom 태그와 전역 POS 중 어느 쪽도 우선하지 않고, `auto`가 아니면 같은 품사일 때만
  허용하며 다르면 compile 오류라는 규칙을 항상 설명한다. Expand control은 각 값의 생성 범위를
  현재 선택값과 option list에서 설명한다.
- 입력 text는 CodeMirror 기반 plain-text editor에서 수정한다. 검색 span은 UTF-16 document offset을
  사용하는 decoration으로 실제 편집 text에 표시하며 별도의 highlight layer나 결과 preview를 중복해
  두지 않는다. IME composition 상태가 아닐 때 물리·소프트 키보드의 Enter와 Shift+Enter는 editor
  document에 줄바꿈을 삽입한다. Editor와 query control은 IME composition 중 search state를 갱신하지
  않고 composition이 끝난 값만 반영한다. 외부 preset 적용 외에는 editor document를 다시 쓰지 않아
  selection, caret과 undo history를 보존하고, 대용량 입력은 현재 viewport 중심으로 렌더링한다.
  Rich-text document model과 collaboration 기능은 추가하지 않는다.
- 결과 panel은 `Matches`와 `Raw JSON` tab을 제공하고 한 번에 선택한 detail만 표시한다. 기본 tab은
  사람이 읽는 surface·span·provenance 목록이며 Raw JSON은 같은 match의 전체 구조를 표시한다.
  Match 목록은 surface, span과 provenance를 빠르게 훑을 수 있는 compact row로 표시하고 각 항목을
  독립된 큰 card로 확장하지 않는다. 좁은 화면에서는 provenance만 다음 줄로 내려 row의 정보 순서를
  보존한다. Match row를 활성화하면 해당 UTF-16 span을 editor에서 선택하고 editor 내부 scroll과
  문서 viewport를 그 위치로 이동한다.
- Playground 입력은 browser 밖으로 보내지 않는다. Full POS와 약 36 MiB의 compact component
  resource는 기본 demo에 포함하지 않는다. 사용자가 고급 `smart` 지원을 요청할 때만 같은 origin의
  Pages Function에서 component resource를 한 번 내려받아 기존 WASM engine에 load한다. 검증된
  resource response는 browser Cache Storage에 보관하고 호환되는 resource revision으로 playground에
  다시 들어오면 network 요청 없이 자동으로 복원한다. Cache key는 version tag를 정확히 checkout한
  release build에서는 tag를 사용하고, 그 외 개발 build에서는 component artifact checksum을 마지막으로
  고정한 전체 Git commit을 사용한다. Site UI만 바뀐 commit은 동일한 resource를 무효화하지 않는다.
  이 Git commit을 산출하는 site build는 전체 history를 요구하며 shallow checkout에서는 잘못된 현재
  HEAD를 version으로 사용하지 않고 실패한다. CI의 site build와 Pages deploy는 전체 history를 checkout한다.
  Playground 진입 시 현재 key를 먼저 확인하고, 기존 site build key로 저장한 같은-origin entry도 engine의
  schema·version·digest 검증을 통과하면 현재 key로 옮긴다. 호환되지 않는 entry는 삭제한다. 검색은 이
  확인이 끝난 뒤 시작하며 resource row는 확인 중 상태와 저장소 복원 완료 상태를 구분해 처음부터
  표시한다.
- Component resource는 25 MiB 단일 값 제한이 있는 Workers KV가 아니라 `kfind-assets` R2 bucket에
  둔다. Pages Function은 `KFIND_ASSETS` binding으로 고정 object를 읽어 body를 buffering하지 않고
  stream하며 content type, ETag와 cache header를 보존한다. R2 object가 없거나 손상되면 embedded
  preview로 조용히 fallback하지 않고 playground에 오류를 표시한다.
- `site` package는 현재 source의 WASM과 version control에 보존한 승인 benchmark snapshot에서
  chart를 다시 생성해 정적 `dist`를 만든다. Snapshot은 source report의 revision과 SHA-256을
  기록하며, 승인된 benchmark가 바뀌면 같은 변경에서 갱신한다. 형태 품질은 수동 검토를 통과한
  표준 맞춤법 canonical과 실제 오류 문장만 남긴 Robust를 별도 section과 chart로 표시한다.
  Robust chart는 동일한 gold fixture에서 backend별 precision·recall·F1과 실행 비용을 비교하고,
  오류 class, positive/negative 분모, robustness 설정과 표준문 품질에 합산하지 않는다는 점을
  chart subtitle과 인접 본문에 명시한다. Robust 500-case는 positive 250, negative 250으로
  고정하고 positive 중 오류 표식이 gold token에 직접 걸린 `target-span` 100건과 오류가 다른
  token에 있는 `context-only` 150건을 분리해 보고한다.
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
  resource로 component positive와 crossing-substring negative를 모두 실행한다. component
  header는 kfind package version을 보존하고 binary는 exact version mismatch를 초기화 오류로
  보고한다. Formula는 설치·upgrade 뒤 `kfind --check-data --data-dir <pkgshare>`를 실행해 full
  POS와 component의 무결성·호환성을 함께 확인한다. 실패 시 임의 다운로드나 백그라운드
  갱신을 하지 않고 `brew reinstall kfind`를 안내한다. Stable resource와 main source가 섞이는
  `head` build는 제공하지 않는다.
- distribution asset의 `skills/kfind/SKILL.md`를 formula의 `share/kfind/skills/kfind`에
  설치한다. Homebrew binary의 `--init`은 project skill을 versioned Cellar가 아니라
  `opt/kfind/share/kfind/skills/kfind`에 연결한다. 최초 `brew install`은 skill 원본을 함께
  설치한다. 사용자가 project에서 `kfind --init`을 한 번 실행해 Homebrew 관리 link를 만든
  뒤에는 `brew upgrade`가 그 link의 안정 경로가 가리키는 원본을 자동으로 갱신한다.
  Homebrew hook은 대상 project와 agent를 알 수 없으므로 임의의 project skill 경로를 직접
  만들거나 수정하지 않는다.
- kfind 소스 코드와 프로젝트가 직접 작성한 내장 데이터는 MIT 라이선스로 배포한다. 외부 full
  POS와 component resource의 Apache-2.0 고지, enriched predicate data의 CC BY-SA 2.0 Korea
  고지는 별도 `LICENSES` 디렉터리에 보존한다.
- formula가 설치하는 전체 묶음은 MIT, Apache-2.0, CC BY-SA 2.0 Korea 조건을 함께 따른다.
  CC BY-SA 2.0 Korea는 SPDX License List에 없으므로 Homebrew metadata는 존재하지 않는 SPDX
  식별자를 만들거나 Generic license로 대체하지 않고 `license :cannot_represent`를 사용한다.
  renderer와 release workflow는 이 metadata를 검증한다.

### 0.6 구조 기반 국소 형태 판정

- query compiler는 각 anchor를 `CandidateProgram`으로 만든다. program은 core 투영,
  consumption, boundary 또는 구조 제약, 모든 생성 `Origin`을 한번만 보존한다.
- matcher는 program을 실행해 얻은 실제 core·anchor·consumed span과 bounded 주변 token
  span을 resolver에 직접 전달한다. 별도 후보 범위 정책이나 corpus 분석 결과로 같은
  span을 다시 추론하지 않는다.
- 구조 판정이 필요한 `smart` program은 어휘, 세부 품사, continuation DFA, component
  capability와 인접 token 제약으로 이루어진 `QueryMorphPattern` 합집을 소유한다.
  전체 token 표면형 registry나 corpus 단어 denylist를 query 제약으로 사용하지 않는다.
- component capability는 `WholeOnly`, `Source`, `SourceAndRuntime`을 구분한다. plan은
  각 program의 capability를 합성해 compact morphology resource 필요 여부를 결정한다.
  literal, `token`, `any` 및 구조 근거가 필요 없는 `smart` program은 resource를 열지 않는다.
- corpus 쪽은 candidate를 포함한 bounded Unicode token과 바로 인접한 token만
  `BoundedTokenGraph`로 만든다. source whole/component와 runtime node를 구분하고
  원문 byte span과 source provenance를 보존한다.
- resolver는 먼저 query와 독립적인 whole/component·세부 품사·continuation·인접 token
  근거로 corpus의 구조적 후보를 고른다. 어휘 의미만 다르고 span topology, 품사,
  continuation과 문맥 제약이 같은 후보는 하나의 `StructuralSignature`로 합친다.
- 체언 core가 조사 없는 token 전체와 정확히 같은 경우에는 token 경계 자체를 완성된 체언
  구조 근거로 사용한다. Source whole 분석이 없다는 이유만으로 이 경로를 거부하지 않으며,
  core와 token 경계가 다르거나 조사·다른 문자를 소비했으면 이 근거를 사용하지 않는다.
- 체언 core가 token 왼쪽 경계부터 graph로 조합한 완성 체언 host 전체와 정확히 같고,
  이어지는 조사 연쇄를 token 끝까지 소비하면 source whole 분석이나 host 전체를 덮는 단일
  edge가 없어도 체언 core를 유지한다. `대영제국의`, `캠브리지는`처럼 host 내부가 여러 명사
  edge로만 구성된 경우를 복구하며, host의 내부 substring이나 crossing span은 이 근거로
  열지 않는다.
- `ConstraintResolver`는 query pattern의 structural signature가 선택된 corpus 구조와
  일치하면 `Supported`, 다른 구조가 유일하게 선택되면 `Contradicted`, resource 오류나
  상한 초과는 `Unavailable`로 반환한다.
- 구조적으로 다른 경쟁 path는 인접 성분 배치로 하나를 선택할 수 있는지 판정할
  때까지 평가한다. 이때 분해·품사·인접 제약이 같은 어휘 의미 후보는 추가로
  열거하지 않는다.
- query program이 `ending.aoeo`를 거쳐 만든 축약형 뒤의 문자열은 compact resource가
  `VX` 보조용언+어미의 완전한 연쇄로 증명할 때만 같은 predicate token으로 확장한다. 한 음절 안의
  축약은 anchor와 core의 byte 끝이 같을 수 있으므로 span 길이 차이를 별도 증명 조건으로
  요구하지 않는다.
- 체언+조사와 용언+어미 path는 host span이 같을 때만 구조적으로 해결되지 않은
  경쟁으로 본다. host가 다르면 더 긴 조사 host 또는 완성된 용언 host를 선택하고,
  다른 위치에서 우연히 성립한 분할을 후보 근거로 쓰지 않는다. 조사 host는 exact
  whole 명사 host를 먼저 선택한다. exact host가 없을 때 whole-token 단일 품사 또는
  완성된 용언 분석이 있으면 이를 graph로 조합한 명사 host보다 우선한다.
- whole-token 단일 체언 분석과 더 짧은 `체언+조사` 분할이 경쟁해도 whole 분석이 정렬해
  선언한 체언 source component를 조사 host 선택으로 가리지 않는다. 이 추가 근거는 source
  component와 정확히 일치하는 체언 query에만 적용한다. 따라서 `자본주의`의 선언된 `주의`
  component는 유지하지만, 비체언 분석이나 큰 component의 substring, 여러 component 경계를
  가로지르는 span은 열지 않는다.
- host span이 같은 체언+조사와 용언+어미 path가 경쟁해도 candidate program이 실제로
  소비한 continuation과 맞지 않는 path까지 허용하지 않는다. 예를 들어 `걸을`에서
  `걷다`의 `걸으-+-ㄹ` program은 유지하지만, `걸다`의 bare `걸` program은 `-을`을
  소비하지 않았으므로 제외한다.
- source가 정렬해 선언한 component는 같은 span의 runtime 분할보다 우선한다. 조사로
  완결되는 체언 host가 없는 token에서, 왼쪽 경계부터 시작한 더 긴 source 용언 분석과
  `E+` suffix가 token 끝까지 완성되면 그 안의 runtime 체언·부사 prefix는 component로
  추측하지 않는다. 따라서 부사와 용언 사이에 어절 경계가 필요한 `안 팔아서`, `못 했다`를
  `안팔아서`, `못했다` 안의 component로 열지 않고, source 파생 근거가 없는 `못하다` 안의
  명사 `못`도 열지 않는다. `공부하다`처럼 같은 source 분석이 정렬된 명사 component와 파생
  접미사를 선언한 경우에는 source component를 유지한다. 한 source 분석에 component가
  정렬되지 않았더라도, 두 음절 이상인 체언 뒤에 `XSV`, `XSA` 또는 용언 source edge가 붙어
  완전한 별도 path를 이루고 더 긴 whole 용언 분석도 있으면 보수적 runtime 파생 근거로
  인정한다. 따라서 `시작했습니다`, `진정한`, `재미있어요`의 체언은 유지하되, 한 음절 체언은
  정렬된 source component 없이는 이 fallback을 사용하지 않는다. 이 판정은 runtime path
  전체의 품사 전이를 제한하지 않으므로 `MAG + JX`인 `드디어는`, `많이들`과 검증된
  `NNG + XSV` 파생을 보존한다. `안팔아서`, `안좋습니다`, `안나와요` 같은
  nonstandard-spacing 입력은 향후 별도 robust 지원에서 다루며 현재 표준형 `smart` 계약에서는
  FP 또는 FN을 허용한다. continuation을 하나도 소비하지 않은 bare predicate가 더 큰 token의
  일부이거나, predicate component 직후의 체언+조사 후보이면 구조적으로 반증한다.
- 현재 token에 whole `MAG`와 whole 체언이 경쟁하고 다음 token의 완전한 component path가
  `하다` 활용의 `하/VV` 또는 교체형 `해-/했-/VV`로 시작하면 부사 구조를 선택한다. 따라서
  `못 하겠어요`, `못 했다`의
  `못`은 `MAG`로만 인정한다. 다음 token이 다른 용언인 `못 박았다`에는 이 frame을 적용하지
  않아 source가 선언한 동형 품사를 그대로 유지한다.
- `smart` 체언 query의 core가 token 왼쪽 경계부터 완성된 체언 host와 정확히
  일치하고, `이`·`입`으로 시작하는 source graph가 그 직후부터 token 끝까지
  `VCP + E+`와 선택적 조사 연쇄를 완성하면 체언 core를 유지한다. 조사는 어미를 하나 이상
  지난 뒤에만 허용한다. 이 경로는
  `결과이다`, `왕친입니다`, `고체이긴`, `것이었다`, `바튼반도이다`의 체언 host를
  복구하지만, core와 지정사 사이에 다른 체언이 남는 `홍씨이다`, 지정사가 아닌 용언이
  이어지는 `맛있다`, 지정사 자체와 겹치는 `이다` 안의 체언 `이`는 열지 않는다.
- 체언 host의 마지막 음절에 받침이 없으면 지정사 `이-`가 탈락한 `다`와 `였-` 활용,
  `이어-`가 줄어든 `여-` 활용도 같은 지정사 구조로 검증한다. 탈락·축약 표면을 완전한
  `이다` 활용으로 복원했을 때 predicate generator가 token 끝까지 정확히 소비해야 한다.
  따라서 `상표다`, `구경거리였다`, `학교여서`는 유지하지만 받침 뒤에서 같은 축약을 쓴
  `대학다`, `대학였다`, `대학여서`는 열지 않는다.
- `smart` 체언 query가 token 왼쪽 경계부터 시작하고 query program이 하나 이상의 조사를
  소비한 경우에도, 조사 verifier가 허용한 연쇄의 끝에서 시작하는 나머지 표면 전체가
  predicate generator의 지정사 활용과 정확히 일치하면 체언 core를 유지한다. 조사 연쇄는
  candidate program이, 지정사와 어미는 기존 생산 문법이 각각 증명하며 compact source
  graph에 같은 `J+VCP+E+` 분할을 중복 요구하지 않는다. 지정사 탈락·축약의 음운 조건은
  체언 core가 아니라 조사 연쇄가 끝난 마지막 음절을 기준으로 판정한다. 따라서
  `대학뿐이다`, `대학뿐만이다`, `학교까지다`처럼 조사구 뒤 지정사를 일반적으로 지원하되,
  허용되지 않은 조사 연쇄나 지정사·어미가 완결되지 않은 표면은 열지 않는다.
- predicate program이 `-기` 또는 `-ㅁ/음`을 실제로 소비했고 그 nominalized span이
  whole nominal 또는 source nominal component와 일치하면 predicate query를 유지한다.
  이 규칙은 `걷기`, `걸음`, `발걸음`, `걸음걸이`처럼 명사형 자체와 compound 내부의
  정렬된 component에 적용한다.
- 앞 token이 관형형 어미로 끝나고 현재 token에 의존명사 whole 분석이 있으면 현재
  token의 동형 predicate 분석보다 의존명사 구조를 선택한다. 따라서 `걷곤 하는 걸`의
  `걸`은 `v:걸다`에 매칭하지 않는다.
- full-POS `smart` predicate plan은 고정 anchor 목록만으로 어미 coverage를 제한하지
  않는다. generator가 만든 사전 어간과 어휘 교체형을 fallback anchor로 공유하고,
  compact resource에서 해당 predicate 품사 뒤로 `EP/EC/EF/ETM/ETN` path가 token 끝까지
  이어질 때 전체 token을 소비한다. fallback은 token 시작에서만 동작하고 ending이 하나
  이상 있어야 하며, 모음·자음·ㄹ 어간의 `으` 삽입 조건을 만족해야 한다. 따라서 새로운
  현대 표준어 어미는 query별 anchor 열거 없이 resource와 문법 환경으로 수용하지만,
  `걸다 + -을 → 걸을` 같은 잘못된 결합은 만들지 않는다.
- generator branch가 어휘 교체형과 일부 어미만 소비한 뒤 token 내부에 멈춰도, query core와
  같은 predicate 품사로 정렬된 source prefix에서 시작해 `EP/EC/EF/ETM/ETN`만으로 token
  끝까지 이어지는 path가 있으면 전체 token을 소비한다.
  지정사는 왼쪽 체언 host가 있는 경우에만 이 경로를
  사용한다. 일반 용언은 query core가 token 왼쪽 경계에서 시작하고 token 전체의 관형사·부사
  분석이 없으며 generator continuation state가 terminal이 아닐 때만 사용한다. 남은 suffix가
  조사 allomorph로도 시작하거나 조사·체언이 남는 path는 predicate ending path로 확장하지
  않는다.
- declarative candidate가 `다`까지 소비한 뒤 정확히 `는`만 남기고, 같은 품사의 source
  graph가 query core부터 token 끝까지 완성된 어미 path를 증명하면 구조 검증 범위를 `-다는`
  전체로 확장한다.
  따라서 `왔다는`, `있다는`, `않다는`을 회수하지만, source 어미 근거가 없거나 `왔다를`처럼
  다른 조사 모양 suffix가 남는 후보는 열지 않는다.
- predicate candidate가 어미를 하나 이상 소비한 뒤 보조사열을 남기면, product 조사 전이
  graph가 남은 표면 전체를 `ParticleRole::Auxiliary` 연쇄로 검증하고 같은 품사의 source
  graph가 query core부터 token 끝까지 `predicate + E+ + J+` 순서의 완성된 path를 증명하는
  경우에만 구조 검증 범위를 전체 token으로 확장한다. 따라서 `위해서는`, `대해서는`,
  `없지는`, `이렇게도`, `이기리라고는`을 같은 규칙으로 회수한다. 격조사, 허용되지 않은
  조사 전이, 어미나 조사 중 한쪽의 source path가 없는 표면은 열지 않는다.
- 관형형 candidate 뒤에 의존명사 `지`와 조사가 붙으면, 같은 품사의 source graph가
  candidate가 소비한 경계까지 `predicate + E* + ETM`, 그 뒤 token 끝까지
  `NNB + J+` 순서의 완성된 path를 증명하는 경우에만 구조 검증 범위를 전체 token으로
  확장한다. 따라서 `오다`는 `온지를`에서 회수하지만, source 관형형·의존명사·조사 중 하나가
  없거나 순서가 다른 path는 열지 않는다.
- 관형형 candidate 뒤에 조사 없는 `지`가 남아도 token 전체와 정확히 일치하는 source
  분석이 같은 품사의 `predicate + E+`를 선언하면 의존명사가 아닌 어미 경로로 전체 token을
  소비한다. 따라서 `들리다`는 exact `VV+EC` 근거가 있는 `들릴지`에서 회수하지만, 분리된
  `ETM`과 `지/NNB` 또는 일반 suffix 조합만 있는 `온지`는 열지 않는다.
- 관형형 candidate 뒤에 정확히 `가`만 남으면, 같은 품사의 source graph가 query core부터
  token 끝까지 `predicate + E+` 순서의 완성된 path를 증명하는 경우에만 구조 검증 범위를
  전체 token으로 확장한다. `MM + E` 경쟁 path는 단독 근거로 사용하지 않으며, predicate
  path와 함께 있으면 recall-first 정책에 따라 용언 후보를 유지한다. 따라서 `어떻다`는
  `어떤가`에서 회수하지만, predicate path가 없거나 조사까지 더 남는 후보는 열지 않는다.
  그 밖의 runtime compound와 해결되지 않은 complete path 경쟁은 순위를 매기지 않는다.
- 구조적 경쟁이 여전히 모호하면 `ProductPolicy`는 recall을 우선해 지원 가능한
  query 후보를 유지한다. `Ambiguous`와 경쟁 proof 전체는 진단 evaluator에서만 물질화한다.
- program이 보존한 모든 query `Origin`은 결과 provenance에 남기되, corpus 의미 분석을
  추가하지 않는다.
- exact component 근거는 완전한 graph path에서 query와 같은 세부 품사 node의 span이
  query core와 정확히 일치할 때만 성립한다. 더 큰 node의 substring이나 여러 component
  경계를 가로지르는 span은 근거가 아니다. nominal component path는 source가 선언한
  성분 수가 가장 적은 완전 경로를 선택하고, 성분 수가 같으면 source가 선언한 성분을
  더 많이 포함한 경로를 우선한다. 내부 component query는 이 선호 경로의 한 node와 span이
  일치할 때만 유지한다. graph로 조합한 명사+조사 host는 내부 nominal component를
  검증할 때만 사용하고 token 전체의 품사 구조를 선택하는 근거로 쓰지 않는다. host 왼쪽
  경계에 정렬된 두 음절 이상의 nominal prefix는 유지하고, 한 음절 prefix와 host 내부
  양쪽 경계를 가로지르는 후보에는 선호 경로 검증을 적용한다.
- token 또는 조사 host의 왼쪽 경계에서 정확한 `MM` node 하나로 시작하고 나머지 host가
  `NNG`·`NNP`·`NNB/NNBC` node만으로 완성되는 선호 경로에서는 그 경로의 exact 명사
  component를 유지한다. 단, 한 음절 `MM`과 명사 component 하나만으로 완성되는 경로는
  같은 선두 span의 `NR` node도 있을 때만 유지한다.
  따라서 `어느/MM + 날/NNG`, `세/MM + 시/NNBC + 반/NNG + 에/JKB`의 `날`, `반`을
  유지하고, `칠/MM|NR + 월/NNBC`의 `월`도 유지한다. token 전체의 단일 품사 분석이
  경쟁하거나 `MM` 뒤의 체언 경로가 불완전하면 이 근거를 사용하지 않는다. `MM`보다 짧은
  predicate prefix node만 함께 존재하는 경우에는 완성된 `MM + 명사` 경로를 폐기하지 않는다.
  `매일/MAG` 안의 `일`, `아무/MM + 나/NP`의 대명사 `나`, `소/MM + 년/NNB`로도
  분해되는 `소년` 안의 `년`과 component 경계를 가로지르는 span은 계속 거부한다.
- 두 음절 이상의 `NNP`가 host 왼쪽 경계부터 query core 직전까지 이어지고, 한 음절 `NNB`
  core가 host 오른쪽 경계에서 끝난 뒤 유효한 조사 continuation을 소비하면 인명+의존명사
  구조로 유지한다. 이 예외는 문자열 substring이나 표제어 의미가 아니라 complete path의
  품사 경계로만 판정한다.
- 제품 graph는 source 분석 비용을 읽거나 보존하지 않는다. 비용·연결 행렬·미등록어
  모델은 별도 full morphology 진단 artifact에서 과거 판정과 결과를 비교할 때만 사용한다.
  include/exclude 비용 마진, query별 threshold와 결과별 fallback을 제품 판정에 사용하지 않는다.
- 부사의 인접 동일 token 반복, 체언·지정사·의존명사 연속 구조와 조사 host
  이형태는 typed `AdjacentTokenConstraint`로 표현한다. query 표제어나 query 품사를
  corpus 구조 선택 힌트로 주입하지 않는다.
- 현재 token에 관형사 whole 분석이 있고 다음 token이 체언으로 시작하면 관형사 구조를
  선택한다. 따라서 `새 기능`의 `새`는 관형사로 판정한다. 여기서 다음 token의 체언
  시작은 token 전체를 덮는 완전한 체언 host 또는 그 host와 조사 suffix로 증명해야 한다.
  체언 host는 여러 체언 edge와 `XPN/XSN/XR`의 조합도 허용하므로 `전 가구별로`의
  `가구 + 별 + 로`도 같은 관형사 구조에 포함한다. 다만
  우연히 체언으로도 등록된 짧은 prefix나 predicate·modifier whole 경쟁이 있는 token만으로
  판정하지 않는다. 한 음절 관형사 구조는 경쟁 NNG/NNP/NNB만 제거하고 다른 품사의
  독립 후보는 유지한다. `V+EC N`처럼 절 연결과 명사 연속 구조가 모두 가능한 배치는
  이 규칙으로 predicate 후보를 제거하지 않는다.
- `smart` 무품사 direct-particle program은 입력과 같은 표면형만 만든다. 품사를
  명시한 조사 query는 이형태 묶음을 만들 수 있지만 host 소리 조건과 완성된
  조사 연쇄를 graph 제약으로 증명해야 한다.
- `독수리가 아니라 매일 수도 있어`의 `매`·`이다`, `매일 매일 보고 싶어`의
  반복 부사, `그는 집념으로 매일을 보내고 있었다.`의 체언·조사 결합은
  각각 copular-frame, repeated-token, component path 근거로 구분한다.
- 한 window의 원문은 256 bytes, NFC 문자열은 64 Unicode scalar, graph는 중복
  제거 후 4,096 node로 제한한다. NFC 안정 경계는 원문 byte offset으로
  역매핑하고 안정되지 않은 경계는 candidate로 만들지 않는다. 원문 window가 이미
  NFC이면 normalized byte offset과 원문 상대 offset의 identity mapping을 사용하고,
  NFC가 아닌 window만 prefix 안정 경계를 계산한다. 인접 token도 NFC이면 원문 slice를
  직접 빌려 구조를 준비하고, 비 NFC token만 bounded normalized 문자열을 소유한다.
- compact morphology resource는 schema 5 container다. NFC surface index, source node의
  POS, NFC 안정 경계에 정렬된 component span과 source identity만 보존한다. left/right
  context ID, word cost, 연결 비용 행렬, unknown model과 원본 expression 문자열은 싣지 않는다.
  국소 graph를 준비할 때는 검증된 resource의 POS 문자열과 component를 빌려 쓰며 token마다
  같은 문자열을 다시 소유하지 않는다.
  loader는 schema, source SHA-256, section length·digest, UTF-8, group·analysis·component
  offset과 span 범위를 모두 검증한 뒤 내용을 노출한다.
- token 선두의 ASCII 숫자 연속은 바로 뒤의 완전한 source 분석이 `NNB`, `NNBC` 또는 `NR`이고
  나머지가 없거나 완성된 조사 연쇄일 때만 수량·단위 graph prefix로 사용한다. 이 경로는
  정렬된 단위 span과 같은 의존명사·수사 pattern만 지원하며 일반 unknown node나 임의의 숫자+명사
  결합을 열지 않는다.
- ASCII 숫자와 `NNB/NNBC/NR` 단위 뒤에 정확한 `NNB/NNBC` 의존명사 node 하나와 선택적
  조사 연쇄가 이어지면 단위와 의존명사 tail을 같은 완성 경로로 유지한다. 따라서
  `1년간/1년간의`의 `년`과 `간`을 지원한다. Tail이 일반 `NNG/NNP`에만 해당하거나 두 node
  사이를 가로지르면 이 경로를 사용하지 않으므로 `197명사`의 `명`과 `사`는 계속 거부한다.
  같은 범위를 더 긴 단일 단위와 짧은 단위+의존명사 tail이 모두 완성하면 더 긴 단일 단위를
  선택한다. 따라서 `10시간`을 `10시+간`으로 바꾸지 않고 `시간` 단위로 유지한다.
- 한글 수사 연쇄는 token 왼쪽부터 완성된 source 분석이 `NR` 둘 이상 뒤 선택적 `NNB/NNBC`와
  조사 연쇄로 끝나거나, `NR` 하나 이상 뒤 `NNB/NNBC`와 선택적 조사 연쇄로 끝날 때만 별도
  typed 구조로 사용한다. 이 경로는 정렬된 `NR` span과 같은 수사 pattern만 지원하며 중간이나
  끝의 일반 명사, unknown node와 불완전한 나머지를 허용하지 않는다.
- ASCII 숫자 뒤의 한글 수사 연쇄는 `NR` 하나 이상과 `NNB/NNBC` 단위가 차례로 이어지고
  나머지가 없거나 완성된 조사 연쇄일 때만 별도 typed 구조로 사용한다. 이 경로는 정렬된
  `NR` span과 같은 수사 pattern만 지원한다. `NR` 없이 시작하는 단위, 끝의 일반 명사·고유
  명사, unknown node와 불완전한 나머지는 허용하지 않는다.
- CLI의 기본 boundary는 `smart`다. resource를 필요로 선언한 program이 있으면
  compact artifact를 한 번 검증하고, 누락·손상·schema·source mismatch를 초기화
  오류로 보고한다. 기존 boundary 판정으로 fallback하지 않는다.
- compact와 full morphology resource는 source identity와 비용을 제거한 structural projection의
  exact/common-prefix hit, POS와 정렬 component span이 일치해야 한다. full artifact의 비용 경로는
  별도 진단으로 기록하되 compact 판정과의 일치 여부를 제품 gate로 사용하지 않는다.
- 제품 matcher와 benchmark evaluator의 candidate coverage는 100%여야 한다. 고정 test의
  TP를 줄이거나 FP를 늘리지 않고, dev precision 99.00% 이상·revised hard-negative
  신규 FP 0·FN 비증가를 전환 게이트로 삼는다.
- `SurfaceBranch`, `BranchVerifier`, `ContextRequirement`, 수동 lexical-context surface registry,
  exact-component 1,500 비용 마진과 기존 verifier fallback은 제품 query·matcher 경로에
  존재하지 않는다.

### 0.7 Rust 라이브러리와 WASM 대상

- CLI의 자동 resource 해석과 달리 Rust 라이브러리와 npm binding은 filesystem, URL 또는 package
  asset 위치를 추정하지 않는다. caller가 component 기능을 사용할 때만 bytes를 명시적으로 전달한다.
- `kfind` 파사드 crate는 `ResourceBundle { full_pos, enriched_predicates, component }`와
  `Engine::with_resources(resources)`를 전체 사전 profile의 기본 생성 API로 제공한다. full POS binary와
  enriched predicate UTF-8 TSV는 생성 중 lexicon에 병합하고, component bytes는 compact resource로
  검증해 engine이 소유한다. 각 필드는 선택 사항이며 빈 bundle은 `Engine::new()`와 같은 profile이다.
- 기존 `with_full_pos`, `with_component_resource`, `with_full_pos_and_component` 생성자는 1.x 호환
  API로 유지하되 같은 bundle 생성 경로에 위임한다.
- 1.0의 안정 Rust facade는 `Engine`, `Matcher`, `ResourceBundle`, compile option·오류와 match
  provenance 타입을 crate root에 둔다. 이 타입의 공개 method, enum variant와 field는 1.x 호환
  계약이다.
- caller-configured `Lexicons`, `QueryPlan`과 matcher의 plan 접근은 `kfind::expert` 아래에만 둔다.
  expert API는 계획 IR과 사전 조립 실험을 위한 것으로 1.x 안정 facade 계약에 포함하지 않는다.
  root의 engine 생성·검색 경로는 expert 타입을 인자나 반환값으로 노출하지 않는다.
- `kfind-data`, `kfind-morph`, `kfind-query`, `kfind-matcher`, `kfind-search`, `kfind-testkit`은 workspace
  내부 crate이며 crates.io 배포 대상이 아니다. 공개 Rust 소비자는 `kfind` facade만 사용한다.
- component resource는 생성 이후 first-use에 자동 fetch·load하지 않는다. 검증된 resource는 engine이
  소유하고 여러 matcher에서 재사용하며 query compile마다 다시 decode하지 않는다. resource가 없는
  engine에서 source 또는 runtime component capability가 필요한 smart plan을
  compile하면 명시적 `ComponentResourceRequired` 오류를 반환하고 기존 경계 판정으로
  fallback하지 않는다.
- component resource decoder는 resource를 공개하기 전에 모든 section digest와 payload 구조를
  검증하고 header의 package version이 현재 binary/library version과 정확히 같은지 확인한다.
  Native에서는 큰 index와 payload section의 digest를 서로 다른 thread에서 검증할 수 있고,
  같은 큰 resource의 payload 구조 검증을 section digest 검증과 겹쳐 수행할 수 있다. Thread를
  만들 수 없으면 순차 검증으로 돌아가고 WASM은 순차 검증한다. 어느 경로도 digest나 payload
  구조 검증을 생략하거나 두 검증이 모두 끝나기 전 resource를 engine 상태에 설치하지 않는다.
  병렬 경로에서 두 검증이 모두 실패하면 section digest 오류를 먼저 반환한다.
- section SHA-256은 지원 CPU에서 runtime detection으로 hardware backend를 사용하고, 사용할 수
  없으면 target-compatible backend로 돌아간다. Backend와 무관하게 같은 digest를 계산하며 검증
  범위와 오류 계약을 바꾸지 않는다.
- 같은 fail-fast 계약은 저수준 `MorphMatcher` 생성자에도 적용한다. resource가 필요한 plan을
  `MorphMatcher::new`로 만들면 `MorphMatcherBuildError::ComponentResourceRequired`를 반환하며,
  resource 또는 evaluator를 받는 생성자만 해당 plan을 초기화할 수 있다.
- 생성 후 `Engine::load_component_resource(component_resource)`와 JavaScript
  `loadComponentResource(componentResource)`로 resource를 명시적으로 초기화하거나 교체할 수 있다.
  새 bytes를 모두 검증한 뒤에만 상태를 교체하며 실패하면 기존에 검증된 resource를 유지한다.
- engine은 full POS, enriched predicate와 component resource의 초기화 여부를 각각 getter로 노출한다.
  resource가 필요 없는 literal, `token`, `any`와 boundary-only plan은 component가 없는
  engine에서 그대로 compile한다.
- 라이브러리 matcher는 UTF-8 byte slice에서 겹치지 않는 match와 형태 분석 provenance를
  반환한다. 파일 순회, 인코딩 판별, 출력 형식과 CLI locale 처리는 라이브러리 API에
  포함하지 않는다.
- `kfind`, `kfind-wasm`, `kfind-data`, `kfind-morph`, `kfind-query`, `kfind-matcher`는
  Rust 1.97에서 `wasm32-unknown-unknown` 대상으로 빌드되어야 한다.
- `kfind-wasm`은 `wasm-bindgen` JavaScript glue와 TypeScript declaration을 생성한다.
  npm package metadata와 게시 계약은 0.8절을 따른다.
- JavaScript API는 `Kfind.withResources({ fullPos?, enrichedPredicates?, component? })`를 전체 사전
  profile의 기본 생성 API로 제공한다. binary resource는 `Uint8Array`, enriched predicate TSV는
  JavaScript string이다. `new Kfind(componentResource?)`와
  `Kfind.withFullPos(fullPos, componentResource?)`는 같은 bundle 경로에 위임하는 호환 API다.
  재사용 가능한 `Matcher`를 만드는 `compile`, 수동 `loadComponentResource`, UTF-16 JavaScript
  문자열을 검색하는 `findAll`을 제공한다.
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
- compact component artifact는 `assets/morphology-component-compact.kfc`, enriched predicate TSV는
  `assets/predicates.enriched.tsv` 정적 파일로 WASM 산출물과 분리해 게시한다. 각 외부 데이터의
  license notice도 package에 포함한다. 사용자는 필요한 파일을 배포물에 복사하거나 별도 호스트에
  올릴 수 있으며 npm binding은 특정 호스팅 URL을 고정하지 않는다. full POS binary는 크기와 배포
  profile이 다르므로 npm package에 포함하지 않지만 같은 `withResources` 입력으로 전달할 수 있다.
- package build는 고정 source와 checksum으로 정적 asset을 생성한다. `npm pack --dry-run`은
  asset 포함과 SHA-256을 검증하고 WASM binary에 compact container magic 또는 artifact bytes가
  포함되지 않았음을 확인한다.
- npm `prepack`은 같은 checkout의 Cargo/package version을 확인하고 component를 다시 생성한 뒤
  Node smoke·TypeScript·asset 검증을 통과해야만 pack/publish를 허용한다. Tag release workflow는
  이 검증이 끝난 동일 산출물을 npm registry에 게시한다. Prerelease version은 `next`, stable
  version은 `latest` dist-tag를 사용한다.
- npm 산출물은 브라우저 bundler용 release package로 생성한다. 별도의 Node target
  산출물로 같은 공개 API를 smoke test하고 `npm pack --dry-run`으로 게시 파일과 metadata를
  검증한다.
- npm package 검증은 package version과 Cargo version의 일치, 두 정적 asset과 license notice,
  TypeScript declaration의 optional resource bundle, enriched 분석 활성화 여부, resource 없는
  non-component compile, resource 없는 component smart 오류, JavaScript 초기화 오류, component
  positive/crossing negative와 UTF-16 offset 계약을 확인한다.
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

형태 구조가 같은 동음이의어와 동형이의어는 문맥 의미로 구분하지 않는다. 한
표제어에서 생성 가능한 표면형이면 모두 검색 결과로 인정한다. 다만 whole/component
분해, 품사 또는 인접 문장 성분 배치가 다른 경우에는 bounded 구조 근거로 구분한다.

예:

```text
검색어: 걷다

길을 걸어 갔다.     match
전화를 걸어 봤다.   match
```

두 번째 결과는 의미상 `걸다`지만, 문맥 판별은 이 제품의 범위가 아니다.

## 3. 핵심 구현 계약

### 3.1 검색 앵커와 후보 판정을 분리한다

완성된 표면형 문자열을 전부 나열한 구조를 유일한 중간 표현으로 쓰지 않는다. 표면형 수가 늘어날수록 메모리와 matcher 구성 시간이 증가하고, `걸었습니다`, `걸었지만`, `걸으셨다` 같은 연쇄 어미를 모두 전개하기 어렵다.

대신 query compiler가 검색 앵커와 후보 열거·판정 제약을 하나의 실행 IR로 만든다.

```rust
pub struct CandidateProgram {
    pub anchor: Box<[u8]>,
    pub core_mapping: CoreMapping,
    pub consumption: CandidateConsumption,
    pub decision: CandidateDecision,
    pub origins: SmallVec<[Origin; 2]>,
}
```

`걸었`을 앵커로 찾은 뒤 program의 continuation 제약이 `습니다`, `지만`, `는데`
등을 포함한 token graph path를 확인한다.

어미와 조사 continuation은 쿼리마다 복제하지 않는다. 빌드 시 생성한 전역 suffix DFA
또는 trie를 공유하고, 각 pattern은 시작 상태만 참조한다. Aho-Corasick에는 완성
활용형 전체가 아니라 고유 앵커만 등록한다.

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
- 문맥 의미 기반 동음이의어 구분
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
    pub programs: Vec<CandidateProgram>,
    pub boundary: BoundaryPolicy,
}

pub struct CandidateProgram {
    pub anchor: Vec<u8>,
    pub core_mapping: CoreMapping,
    pub consumption: CandidateConsumption,
    pub decision: CandidateDecision,
    pub origins: Vec<Origin>,
}

pub enum CandidateConsumption {
    Anchor,
    PredicateContinuation { /* DFA state, POS, rule vocabulary, left context */ },
    NominalParticleChain { /* allowed and blocked rule vocabulary */ },
    DirectParticleHost { /* particle rule */ },
}

pub enum CandidateDecision {
    Boundary(BoundaryProof),
    Structural(StructuralConstraint),
}

pub struct StructuralConstraint {
    pub patterns: Vec<QueryMorphPattern>,
    pub boundary: BoundaryProof,
}

pub struct QueryMorphPattern {
    pub lexical_form: Box<str>,
    pub fine_pos: DataFinePos,
    pub continuation: MorphContinuation,
    pub component_capability: ComponentCapability,
    pub adjacent: Vec<AdjacentTokenConstraint>,
}

pub struct Origin {
    pub analysis_index: u16,
    pub rule_path: Vec<RuleId>,
}
```

- `CandidateProgram`은 anchor 탐색·core 투영·후보 범위 열거·anchor 이후 소비·판정 제약을
  한번만 표현하는 query-owned 실행 IR이다. `CandidateConsumption`은 실제 token span을 만드는
  continuation과 rule vocabulary만 선언한다. matcher와 품질 검증기는 같은 program을 실행하며,
  별도 branch를 재구성하거나 consumption 종류에서 `extent`를 추론하지 않는다.
- exact 후보는 `Anchor`, 용언 연속 후보는 `SurroundingToken`, 조사가 없을 수도 있는
  체언은 `AnchorAndSurroundingToken`을 사용한다. 모든 후보는
  `core ⊆ anchor ⊆ consumed ⊆ token` 불변식을 만족한다.
- `QueryMorphPattern`은 표면형 예외 목록이 아니라 어휘·세부 품사·continuation DFA·component
  capability·인접 token 제약을 선언한다. 여러 분석이 같은 anchor를 공유하면 pattern
  합집과 모든 `Origin`을 보존한다.
- `Boundary`는 literal, `token`, `any` 및 구조 판정이 필요 없는 경로에만 사용한다.
  `Structural`은 bounded token graph에서 구조적으로 다른 경쟁 경로를 평가하되,
  같은 structural signature 안의 어휘 의미 차이는 추가로 열거하지 않는다.
- `BranchVerifier`, `ContextRequirement`, 수동 lexical-context surface registry,
  exact-component 비용 마진과 예외 fallback은 query·matcher 실행 경로에 존재하지 않는다.

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

동일 branch가 여러 분석에서 생성되면 origins를 합친다. compiler는 정규화된 anchor로
branch 후보를 먼저 묶고 같은 anchor 안에서 core 투영, consumption, boundary와 decision이
같은지 비교한다. 여러 branch가 공유하는 rule vocabulary 전체를 branch마다 다시 hash하지
않으며, 최초 생성 순서와 origin 정렬은 기존 plan 계약대로 보존한다.

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
-고, -고는/-곤, -지, -게, -다, -도록
-는, -(으)ㄴ, -(으)ㄹ
-아/-어, -아서/-어서
-았/-었, -았을/-었을, -았/었느냐(는), -겠, -시. 존대 선어말어미는 받침 어간뿐 아니라 모음·ㄹ·불규칙 어간의 올바른 교체형에도 결합한다.
-아요/-어요와 -았어요/-었어요
-면/으면, -며/으며, -니/으니, -니까/으니까, -니까는/으니까는, -니깐/으니깐,
  -던, -더니, -더라도, -자, -자고, -느냐, -려고/으려고, -려는/으려는,
  -리라/으리라, -리라고/으리라고
-아/어가고, -아/어가야
-ㅂ니다/습니다, -(으)세요, -(으)ㅂ시다, -(으)셨고, -(으)셨던
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

현대 표준어 어미 coverage는 고정 예문만으로 선언하지 않는다. pinned 한국어기초사전,
표준국어대사전과 우리말샘 snapshot의 `어미` 표제어를 source ID와 함께 정규화한 audit를
유지한다. 제품 필수 집합은 한국어기초사전과 표준국어대사전의 현대 일반어이며, 방언·옛말·
북한어는 catalog에 남기되 기본 generator의 필수 집합에서는 분리한다. snapshot은 배포물에
포함하지 않고 audit 결과와 재현 절차만 version control에 둔다.

현대 표준어 조사 coverage도 같은 pinned snapshot의 `조사` 표제어를 source ID와 함께
정규화한 audit로 관리한다. 이 catalog는 원자 조사 어휘의 존재와 현재 runtime rule의 표면형
coverage를 검증하는 근거다. 조사 표제어가 있다고 해서 임의의 앞말이나 다른 조사 뒤에 붙일
수 있는 것은 아니며, 사전 정의·예문에서 결합 문자열을 추출해 runtime 규칙으로 승격하지 않는다.
`까지도`처럼 여러 조사가 결합한 특정 표면형을 원자 조사로 추가하지 않는다.
한국어기초사전의 구조화된 문법 주석과 표준국어대사전의 `grammar_info`가 같은 조사 표면의
앞말 품사를 함께 지지하면 조사 host coverage의 audit 근거로 사용할 수 있다. 자유 서술 정의와
용례는 이 판정에 사용하지 않는다. Runtime 승격에는 두 사전의 일치와 별도 문법 검토가 모두
필요하다.

### 9.3 공통 규칙

다음은 사전 class가 아니라 환경 규칙으로 처리한다.

- 받침 유무에 따른 `은/는`, `이/가`, `을/를`, `과/와`
- 접속 조사 `이면/면`은 `이/가`와 같은 받침 조건을 사용하되 조사 연쇄의 terminal로 처리한다.
- `로/으로`, `로서/으로서`, `로써/으로써`의 ㄹ 받침 예외
- 조사 연쇄는 optional `들`을 포함해 최대 네 규칙을 순회하며,
  schema 2 `data/rules/particles.toml`의 `role`, `hosts`, `next`를 따른다. 첫 규칙은 실제
  앞말 종류가 `hosts`에 있고 해당 결합이 허용하는 role이어야 한다. 이후 규칙은 앞말 host를
  다시 검사하지 않고 직전 규칙의 `next` 전이와 허용 role을 검사한다. 따라서
  `까지 → 도/만/은·는` 전이는 `까지도`·`까지만`·`까지는` 계열을 함께 설명하고,
  `는 → 커녕`은 `는커녕`을 설명한다. 목록에 없는 첫 결합·역순과 최대 길이를 넘는 연쇄는
  거부한다.
- particle graph는 순환이 없어야 한다. graph 자체에 네 단계보다 긴 경로가 있어도 runtime의
  네 규칙 상한으로 제한하며, 가능한 모든 조사 연쇄 문자열을 build 시 전개하지 않는다.
- pinned 국립국어원 조사 catalog에서 한국어기초사전과 표준국어대사전이 함께 지지하는
  체언 부착 조사 중 `께서`, `같이`, `대로`, `더러`, `마다`, `만큼`, `밖에`, `보고`, `보다`,
  `뿐`, `처럼`, `커녕`, `으로서/로서`, `으로써/로써`는 독립 원자 규칙으로 유지한다.
  `이나/나`, `이나마/나마`,
  `이라도/라도`, `이랑/랑`은 받침 조건을 가진 이형태 규칙으로 유지한다. `은커녕`·`는커녕`은
  topic 뒤 `커녕` 전이로 만들며 결합 표면형을 별도 원자로 복제하지 않는다. 앞 체언의 마지막
  음절을 바꾸는 축약형 `ㄴ커녕`은 suffix verifier가 아니라 별도 contraction 후보로 남긴다.
  이 원자들 뒤의 추가 조사는 각 규칙의 `next`로만 허용한다. `으로서/로서`와
  `으로써/로써`는 `으로/로 + 서/써`로 분해하지 않고 각각 하나의 격조사로 소비하며,
  주제·첨가·한정 보조사는 graph 전이로만 뒤따른다.
- 조사 verifier는 체언 부착 형태와 받침 조건을 검증한다. `께서`·`더러`·`보고`의 유정성,
  `밖에` 뒤 부정 표현처럼 어휘 의미나 문장 오른쪽 문맥이 필요한 선택 제약은 판정하지 않는다.
  이 한계 때문에 원자 조사를 누락시키거나 임의의 품사 추측으로 대체하지 않는다.
- 한 어절이 둘 이상의 `체언 + 조사 연쇄` 완성 경로를 가지면 가장 긴 체언 host만 남기지 않는다.
  질의 체언과 같은 span에서 시작해 token 끝까지 조사만으로 이어지는 각 완성 경로를 모두 구조
  후보로 유지한다. `후+에+도`와 `후에+도`처럼 의미 문맥 없이는 고를 수 없는 동형 경로는
  의미 중의성 non-goal에 따라 함께 허용하되, `매+일+을`처럼 질의 체언 뒤가 조사만으로
  완성되지 않는 내부 substring은 허용하지 않는다.
- 명시적 체언 질의의 표면이 token 왼쪽 경계에 있고, 그 뒤의 비어 있지 않은 suffix를 조사
  graph가 token 끝까지 완전히 소비하면 질의의 품사 지정과 조사 경로 자체를 bounded 구조
  증거로 인정한다. 이 경로는 component resource에 없는 새 고유명사·복합명사에도 적용하지만,
  품사를 지정하지 않은 literal fallback, token 내부 substring, 조사 외 suffix에는 적용하지
  않는다.
- 전체 token이 하나 이상의 명사 node와 선택적 조사 node로 완성되고, 체언 질의 span 자체도
  그 명사 경로의 연속된 두 node 이상으로 완성되면 복합명사 subpath로 인정한다. 질의는 token
  처음이나 내부에서 시작할 수 있지만 node 경계를 정확히 따라야 한다. 이 규칙은
  `경영+전략+시스템`의 `경영전략`, `선박+회사+측+에서는`의 `회사측`을 포함하며,
  한 node뿐인 내부 span이나 뒤가 용언 파생·어미 경로인 token은 포함하지 않는다.
- `-(으)면`, `-(으)며`, `-(으)ㄴ`, `-(으)ㄹ`
- 일반 용언의 이유 연결형 `-(으)니`는 자음 어간에 `으`를 삽입하고 ㄹ 받침 어간의 ㄹ을 탈락시킨다.
- 일반 용언의 양보 연결형 `-더라도`는 어휘적 교체 없이 사전 어간에 직접 결합하고 token 경계에서 끝난다.
- 일반 용언의 전망 종결형 `-(으)리라`와 인용 연쇄 `-(으)리라고`는 기존 불규칙 교체를
  적용한 어간 뒤에서만 완료된 token으로 허용한다. `리라`는 그 경계에서 끝나며 뒤따르는
  임의 suffix를 소비하지 않는다.
- 의도 연결형 `-(으)려고`는 동작 용언에만 결합하고, 기존 불규칙 교체 뒤의 모음형 어간을 사용한다.
- 의도 관형형 `-(으)려는`, 회상 관형형 `-던`, 회상 연결형 `-더니`, 목적·결과 연결형
  `-도록`, 의문 종결형 `-느냐`, 청유형 `-자`와 인용형 `-자고`는 동작 용언의 bounded
  terminal 또는 한 단계 continuation으로 생성한다. `-고는`의 준말 `-곤`도 같은
  connective provenance를 유지한다.
- 존대 경로는 `-(으)세요`, `-(으)셨고`, `-(으)셨던`을 소비한다. 청유형
  `-(으)ㅂ시다`는 모음 어간에 `-ㅂ시다`, 자음 어간과 불규칙 교체형에 `-읍시다`,
  ㄹ 받침 어간에는 ㄹ 탈락 뒤 `-ㅂ시다`를 결합한다.
- 진행 방향 보조 용언 `-아/어가다`는 `-아/어` program 뒤의 `가고`, `가야`만 continuation으로 소비한다. `가` 자체나 목록 밖 후속 어미는 허용하지 않는다.
- 과거 `-았/었` program은 의문 종결형 `-느냐`와 이 종결형에 직접 붙는 주제 보조사 `는`까지 소비한다. 다른 조사나 추가 어미는 허용하지 않는다.
- 상태 용언의 `-다` 현재 평서형은 동작 용언의 `-ㄴ다/는다`와 같은 제한된 인용·회상·조건
  continuation을 소비한다. 동작 용언의 사전형, 지정사와 부정 지정사 `아니다`에는 이 전이를
  적용하지 않는다.
- `-기` 명사형은 어휘적 교체 없이 사전 어간에 직접 결합하고, 이 규칙이 만든 terminal
  predicate program만 nominal particle consumption으로 전이한다. consumption은 `기`를 모음 끝 host로
  판정해 `가`, `를`, `는`, `와`, `로` 등의 올바른 이형태와 `data/rules/particles.toml`의
  bounded 조사 연쇄만 소비한다.
- `-ㅁ/음` 명사형은 모음 또는 ㄹ 받침 어간에 `-ㅁ`, 그 밖의 자음 어간에 `-음`을 결합한다.
  ㄷ·ㅅ·ㅂ·ㅎ 불규칙은 사전 alternation을 적용해 `걸음`, `지음`, `도움`, `빨감`을 만들고,
  르·러·하·우 불규칙과 지정사는 자음 앞의 사전 어간에 `-ㅁ`을 결합한다. `-기`와
  `-ㅁ/음`이 만든 terminal predicate program만 nominal particle consumption으로 전이한다.
  다른 종결형·연결형 program은 이 전이를 사용하지 않는다. `보 + ㅁ → 봄`,
  `이르 + ㅁ → 이름`처럼 명사형 종성이 어간 마지막 음절에 합성되어 anchor와 core의 byte span이
  같아져도, 생성 provenance가 `-ㅁ/음` 명사형이면 같은 명사형·조사 구조로 판정한다.
- ㄹ 받침 뒤 특정 자음 어미에서의 ㄹ 탈락
- 어간 말음 `ㅡ`와 `-아/-어` 결합
- 모음 축약과 준말. `ㅕ` 말음 규칙 어간은 `-어`의 축약형도 보존한다 (`켜어`, `켜`).
- 자음 어미의 종성 결합

명사형 뒤의 유효한 조사 연쇄는 predicate token의 일부로 소비한다. 따라서 `걷다`는 `걷기`,
`걷기 운동`, `걷기가`, `걷기를`, `걷기에서도`, `걸음`, `걸음이`, `걸음을`, `걸음으로`를
찾는다. `걷기이`, `걷기을`, `걷기으로`, `걸음가`, `걸음를`, `걸음로`와 case 조사 두 개를
잇는 `걷기가를`, `걸음이를`은 `smart`와 `token`에서 거부한다. `any`는 기존 부분 문자열
candidate를 제거하지 않지만 유효한 조사 연쇄가 있으면 그 끝까지 token span을 확장한다.
query provenance에는 `ending.nominalizer-gi` 또는 `ending.nominalizer` 뒤에 소비한 조사 rule
path를 순서대로 남긴다.

### 9.4 어휘 사전이 필요한 교체

다음은 철자만으로 안정적으로 판별하지 않는다.

- ㄷ 불규칙과 ㄷ 규칙
- ㅂ 불규칙과 ㅂ 규칙
- ㅅ 불규칙과 ㅅ 규칙
- ㅎ 불규칙과 규칙형
- 르 불규칙과 러 불규칙
- 기타 보충법과 개별 예외
- `아니다`처럼 일반적인 `-이어 → -여` 축약을 허용하지 않는 개별 어휘 제약

보조 동사 `말다`의 금지 명령형 `마라`와 보조 동사 `달다`의 요청 명령형 `다오`는
`VX` 표면형 override로만 생성한다. 두 override는 추가 어미를 소비하지 않는 terminal
branch다. 같은 표제어의 일반 동사 `말다`, `달다`는 별도의 `VV Regular + RIEUL_DROP`
분석으로 보존해 규칙 활용과 보조 동사 예외의 합집합을 만든다. `마라`는 `말다 VX
Regular + RIEUL_DROP`의 `ending.imperative-ra` override다. `다오`는 `달다 VX
Suppletive`의 `lexical.suppletive` override이며, 이 분석은 생산적인 ending을 갖지 않는다.

아주낮춤 명령형 `-거라`는 동작 동사의 사전형 어간에 직접 붙이는
`ending.imperative-geora` terminal branch다. 모음 어미 앞 불규칙 교체를 적용하지 않아
`가거라`, `먹거라`, `걷거라`를 생성하며 형용사에는 적용하지 않는다. `-너라`는 `오다`와
`오다`로 끝나는 동작 동사에만 붙이는 `ending.imperative-neora` terminal branch다. 어간이
`오`로 끝나는지 확인해 `오너라`, `들어오너라`를 생성하고 `가너라`는 만들지 않는다.
`오다`에는 일반 `-거라`도 적용하므로 `오거라`와 `오너라`를 모두 보존한다.

### 9.5 필수 활용 범위

| 분류 | 예 | 기대 표면형 |
|---|---|---|
| 규칙 자음 어간 | 먹다 | 먹어, 먹었다, 먹는, 먹은, 먹을 |
| 규칙 모음 어간 | 가다 | 가, 갔다, 가는, 간, 갈 |
| ㅏ/ㅓ 축약 | 보다 | 보아, 봐, 보았다, 봤다 |
| ㅚ/ㅣ 계열 축약 | 되다 | 되어, 돼, 되었다, 됐다 |
| ㄷ 불규칙 | 걷다, 듣다, 싣다 | 걸어, 들어, 실어, 걸음, 들음, 실음 |
| ㅅ 불규칙 | 짓다, 낫다, 잇다 | 지어, 나아, 이어, 지음, 나음, 이음 |
| ㅂ 불규칙 | 돕다, 눕다, 아름답다 | 도와, 누워, 아름다워, 도움, 누움, 아름다움 |
| ㅎ 불규칙 | 파랗다, 그렇다, 어떻다, 이렇다, 커다랗다 | 파래, 파란, 그래, 그런, 어떤, 이런, 커다란, 파람, 그럼 |
| 르 불규칙 | 빠르다, 부르다, 모르다 | 빨라, 불러, 몰라, 빠름, 부름, 모름 |
| 러 불규칙 | 푸르다, 이르다 일부 | 푸르러, 푸름, 이름 |
| ㅡ 탈락 | 쓰다, 크다, 예쁘다 | 써, 커, 예뻐 |
| 우 불규칙 | 푸다 | 퍼, 품 |
| 하다 | 하다, 검증하다 | 하여, 해, 하였다, 했다, 함, 검증하여, 검증해, 검증하였다, 검증했다, 검증함 |
| ㄹ 탈락 | 살다, 알다, 만들다 | 사는, 압니다, 만듭니다, 삶, 앎, 만듦 |
| 진행 방향 보조 용언 | 망하다, 만들다 | 망해가고, 만들어가야 |
| 개별 보조 용언 명령형 | 말다, 달다 | 마라, 다오 |
| 아주낮춤 명령형 | 가다, 먹다, 걷다, 오다, 들어오다 | 가거라, 먹거라, 걷거라, 오거라, 오너라, 들어오너라 |
| 회상·청유·의도·존대 | 걷다 | 걷던, 걷더니, 걷자, 걷자고, 걷곤, 걷느냐, 걷도록, 걸으려는, 걸으셨고, 걸으셨던, 걸으세요, 걸읍시다 |
| 과거 의문 종결 | 하다, 먹다 | 했느냐는, 먹었느냐 |
| 지정사 | 이다 | 이고, 이어, 여서, 인, 일, 임, 입니다, 이라고, 이라는, 이지, 이며 |
| 부정 지정사 | 아니다 | 아니고, 아니어서, 아니라, 아닌, 아닐 |

## 10. 품사별 컴파일 규칙

### 10.1 체언

체언은 모든 완성형을 미리 생성하지 않는다.

```text
anchor: 사용자
right consumption:
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
백이면
공부면
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
누구 + 가 → 누가
저 + 의 → 저의, 제
이거/그거/저거 + 는 → 이건/그건/저건
```

표제어별 축약은 override로 명시한다. 주격 override는 기본 조사 결합을 교체하지만, 속격 override는
완전형과 축약형이 모두 표준이므로 기본 결합을 보존하는 alias로 추가한다.
대명사 표제어가 `거`로 끝날 때의 주제 보조사 축약은
`kind=nominal-particle-compose` contraction 하나로 합성한다. 완전형 `그거는`을 보존하고 축약형
`그건`을 alias로 추가하며, 품사가 대명사가 아니거나 `거`로 끝나지 않는 표제어에는 적용하지
않는다.
미지원 항목은 사양의 known limitation에 기록한다.

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

기본 `inflection`은 사전에 부사로 분석된 표면 뒤에 규칙 데이터가 허용한 보조사 연쇄를
소비한다. 이 결합은 새 품사를 만드는 파생이 아니므로 `derivation`에 한정하지 않는다.
`literal`은 입력 표면만 검색하며, 부사 뒤 격조사는 허용하지 않는다.
첫 조사는 `role=auxiliary`이면서 `hosts`에 `adverb`가 있어야 한다. 두 번째 이후 조사는
보조사 role과 `next` 전이만 검사하므로 특정 결합 표면을 별도 목록으로 만들지 않는다.
부사 표면 전체가 `체언 + 격조사`로도 분석되더라도 쿼리의 부사 분석과 허용 보조사 연쇄가
완전하면 부사 구조를 보존한다. 이 동형 구조의 문맥 의미 판별은 비범위다.
반복 token 구조를 사용하는 `smart` 부사 program은 surface registry 대신
`AdjacentTokenConstraint::RepeatedToken`과 세부 품사 pattern을 선언한다.

```text
빨리
빨리도
잘만
실제로는
혹시나
실제로는커녕
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

각 program에서 가능한 가장 긴 고정 바이트열을 앵커로 선택한다.

우선순위:

1. 어간 교체 이후 첫 어미까지 포함한 문자열
2. 어간 전체
3. 짧은 어간이면 다음 고정 요소와 결합
4. 한 음절 앵커는 boundary decision 없이는 허용하지 않음

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
고유 anchor 1개: Box에 보관한 memchr::memmem::Finder의 owned variant
고유 anchor 2개 이상, 짧은 1회성 입력: owned Finder 집합의 build-free overlapping search
고유 anchor 2개 이상, 누적 검색량이 큰 입력: Aho-Corasick standard match kind의 overlapping search
```

단일 앵커 Finder는 `Finder::new(needle).into_owned()`로 구성하고 platform별 Finder 내부 크기가
`AnchorEngine` 전체 크기를 키우지 않도록 Box에 보관한다. 다중 앵커도 처음에는 owned Finder를
재사용해 각 pattern의 다음 hit를 병합한다. Hit 순서는 Aho-Corasick standard overlapping과 같은
`(end, start)` 순서를 보존한다.

다중 앵커 엔진은 검색한 input bytes와 anchor 수의 곱으로 직접 검색량을 누적한다. 정해진
work threshold를 넘을 때만 Aho-Corasick을 한 번 구성하고 이후 입력에서 재사용한다. Automaton
구성이 실패하거나 Finder 집합과 automaton의 합산 예상 메모리가 matcher 제한을 넘으면 Finder
경로를 계속 사용한다. 따라서 짧은 문장 한 번을 검색하기 전에 automaton을 선구축하지 않으며,
대규모 text의 선형 다중 문자열 scan은 유지한다. 후보가 겹칠 수 있으므로 두 경로 모두 모든
overlapping hit를 내고, 검증 후 가장 왼쪽의 가장 긴 token span을 선택한다.

### 11.3 program 제한

기본 제한:

```text
쿼리 길이: 최대 256 Unicode scalar
atom 수: 최대 32
atom당 분석 수: 최대 32
전체 candidate program 수: 최대 4096
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
  → program consumption으로 조사·어미 소비
  → 오른쪽 경계 검사
  → 필요하면 structural decision 실행
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

잘못된 UTF-8이 섞인 파일은 바이트 검색 자체는 가능하지만, 한국어 program 판정은 유효 UTF-8 구간에서만 수행한다.

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
유지한 채 새로 노출된 행과 상태 행만 갱신한다. Frame 간격은 content viewport 8,192 cells마다
16 ms씩 늘리되 48 ms를 넘지 않는다. 따라서 73×316 terminal의 72×316 content viewport는
48 ms 간격을 사용하며, 반복 입력을 합쳐도 최종 이동 offset은 같다. 검색 중 종료하면 결과 출력과
남은 검색을 중단한다.

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
full POS 경로와 제품 compact component 경로는 Viterbi 분석, 비용 행렬과 미등록어 처리를
사용하지 않는다. 비용 기반 비교가 필요한 진단 도구만 별도 full morphology artifact를 읽는다.

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
수동 core 예외는 이 자동 승격 조건을 바꾸지 않는다. 해당 항목은 고정 snapshot의 source record id,
선택 이유와 fixture 결과를 benchmark 보고서에 남기며, core 중복으로 바뀐 상태를 생성 report와
통계에 반영한다.

core와 완전히 같은 분석 및 `derivations.toml`의 생산 접미 규칙으로 이미 생성되는 분석은 enriched
출력에서 제외하고 report에 중복 상태로 남긴다. `UToEo`처럼 독립 사전 합의가 있어도 이미 core에
있는 유형은 신규 행을 만들지 않는다. 승격 건수와 분류별 대조군·중복·review 건수는 생성 통계에
기록한다.

importer의 원시 레코드 grain은 `(source, source_id, raw_homonym, lemma, fine_pos)`다. 동형어
식별자를 제거한 `(lemma, fine_pos)`는 집계 키로만 사용하며, 서로 다른 source record에서
확인된 복수 alternation은 충돌로 간주하지 않고 합집합으로 보존한다. redirect, 비표준어,
방언과 옛말은 자동 승격하지 않는다.

구조화된 사전 표면형은 기존 enriched predicate TSV 안의 `SurfaceOnly` 분석으로 저장한다.
별도 전체 활용형 사전이나 런타임 문장 분석기는 추가하지 않는다. `SurfaceOnly`는 같은 품사의
core·enriched 분석이나 full POS fallback을 가리지 않으며, 기본형과 사전에 기록된 정확한
표면형만 만든다. provenance-only rule id `lexical.dictionary-conjugation`과
`lexical.dictionary-related-adverb`는 rule registry의 생산 규칙이 아니며 이 분석에서만
허용한다.

사전 활용형은 한국어기초사전과 표준국어대사전의 `일반어` record가 같은
`(lemma, fine_pos, surface)`를 지지할 때만 후보로 삼는다. core, 자동 승격된 alternation과
품사가 확인된 `하다`, `스럽다`, `답다`, `롭다`의 생산 규칙으로 이미 생성되는 surface는
저장하지 않는다. 남은 surface만 `lexical.dictionary-conjugation`으로 기록하며
`inflection`과 `derivation`에서 사용할 수 있다.

한국어기초사전 `RelatedForm`은 source가 동사·형용사이고 target이 부사이며, 양쪽 entry가 서로의
ID를 가리키고 각 `writtenForm`이 참조한 entry의 표제어와 일치하는 `파생어` 관계만 사용한다.
이 surface는 `lexical.dictionary-related-adverb`로 기록하고 `--expand derivation`에서만 연다.
예문과 정의에서 문자열을 추출하지 않는다.

생성기는 surface-only 행 수가 512개를 넘거나 배포 `predicates.tsv`가 64 KiB를 넘으면 실패한다.
source snapshot 갱신으로 이 한도를 넘으면 중복 생성 규칙과 분류 누락을 먼저 해소하고, 한도 변경은
별도 성능·배포 크기 검토로 결정한다. report에는 생략된 생성형, 배포 surface-only 활용형·파생형,
source record ID와 artifact byte 수를 구분해 기록한다.

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
    Hangul operations, lexicon, endings, alternations, consumption

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

reference backend는 production anchor 계획과 결과 타입만 공유한다. 서술어 continuation과 조사 연쇄 판정은 production consumption을 호출하지 않고 별도 순회 구현으로 계산해 동일 결함을 공유하지 않게 한다.

### 19.1 최적화 엔진과 참조 엔진을 분리한다

프로덕션 엔진은 candidate program의 앵커, consumption과 decision을 사용한다.

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

query: 걷다, 걸다
text: 그는 걸었고 계속 말했다.
expected: 두 query 모두 match

query: n:매
text: 매일 보고 싶어.
expected: no match

query: adv:매일
text: 독수리가 아니라 매일 수도 있어.
expected: no match
```

`걷다`/`걸다` constructed stress fixture는 다음 계약을 한 문단에서 함께 검증한다.

- `걸었다며`, `걸어온`, `걸어가십니까`, `걸어서`, `걸었잖소`, `걸었고`,
  `걸었는데도`, `걸어오다가`, `걸어왔던`, `걸어갔다`처럼 두 표제어가 만드는 동형
  활용 17개 span은 두 query에 모두 매칭한다.
- `걷던`, `걷자고`, `걷곤`, `걷더니`, `걷자`, `걷느냐`, `걷도록`, `걸으려는`,
  `걸으셨고`, `걸으셨던`, `걸으세요`, `걸읍시다`와 `-기/-음` 명사형 및 정렬된
  compound component는 `v:걷다`에만 매칭한다.
- `걸고` 3개와 `건` 1개는 `v:걸다`에만 매칭한다.
- `걸인`, `걸걸한`, `막걸리`, 의존명사 `걸`, `걷히자`, `걸려`, `걸터앉았다`처럼
  다른 품사 또는 별도 표제어인 token은 어느 query에도 매칭하지 않는다.
- fixture의 논리적 결과는 `v:걷다` 97개, `v:걸다` 21개 span이다. 출력 surface가
  보조용언이나 후속 어미 전부를 소비하지 않아도 같은 시작 위치의 한 match로 센다.

### 19.3 속성 테스트

- 음절 분해 후 조합하면 원래 음절과 같음
- 유효한 종성 교체 결과는 다시 분해 가능
- program consumption은 bounded 후보 범위 밖을 읽지 않음
- 동일 span의 origin 병합은 순서와 무관
- phrase join 결과는 atom 순서를 항상 보존

### 19.4 fuzz

target과 경계:

| target | 경계 |
| --- | --- |
| `query_lexer` | 잘못된 UTF-8을 포함한 임의 query, 매우 긴 combining sequence, lexer와 compile limit |
| `matcher_bytes` | 임의 byte 입력의 anchor 탐색, suffix consumption, match span 범위 |
| `matcher_plan` | 임의 query와 큰 phrase gap의 compile·matcher build, component resource 누락 오류 |
| `user_lexicon` | malformed 사용자 사전 TOML의 구문·의미 검증 |
| `json_output` | 임의 byte line과 검증된 match metadata의 JSON Lines 직렬화 |
| `binary_detection` | 임의 위치의 최초 NUL과 NUL이 없는 입력의 binary 판별 경계 |

CI는 `nightly-2026-07-11`과 `cargo-fuzz 0.13.2`로 모든 target을 실제 실행한다. target당
`max_total_time=15`, 개별 입력 `timeout=5`, `rss_limit_mb=2048`을 적용하며 전체 job timeout은
10분이다. `scripts/run-fuzz.sh`가 target 목록과 이 예산을 단일 진입점으로 유지한다. 각 실행은
version-controlled seed만 임시 corpus로 복사해 이전 실행에서 생성된 입력과 격리한다. 반복 span과
큰 gap의 phrase, 손상 UTF-8, component resource가 필요한 plan, malformed TOML과 출력 제어 문자를
고정 seed로 시작한다. crash·panic·timeout·RSS 초과는 CI 실패다.

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

#### 19.5.1 현실 기술 코퍼스 blind fixture

UD 기반 품질 fixture와 별도로, 재배포 조건이 명확한 공개 저장소의 한국어 README, 소스 코드
주석과 기술 문서에서 짧은 원문을 고정한다. source manifest는 저장소, commit, 라이선스와
라이선스 URL, 원본 경로, 원본 파일 SHA-256을 기록한다. case는 source path와 line 범위,
artifact type, query, 기대 품사, 원문, 기대 여부와 positive의 UTF-8 byte gold span을 보존한다.

fixture는 다음 slice를 모두 포함한다.

- 식별자 주변 한글
- 띄어쓰기 오류
- 한글·영문·숫자 혼합
- 동형이의어
- 복합명사 substring

원문은 NFC 정규화 후 연속 공백을 하나로 줄인 canonical text가 case 사이에서 중복되지 않아야
한다. query와 기대 span은 첫 제품 실행 전에 고정하고, 최초 보고서가 커밋된 뒤에는 제품 결과를
개선하기 위해 바꾸지 않는다. source 전사 오류나 gold 오류는 독립된 근거와 revision을 남겨
수정한다.

평가는 Agent의 `embedded + any + explicit POS`와 User의 `full-POS + smart + untagged`를 같은
fixture 순서로 실행한다. positive는 예측 span이 gold span과 겹쳐야 TP이고, negative는 문장
어디에서든 결과가 있으면 FP다. 전체와 artifact type·slice별 TP·FP·TN·FN, precision, recall,
F1과 실패 case를 version-controlled JSON과 Markdown으로 보존한다. source hash, 필수 metadata,
canonical uniqueness, gold span, 필수 artifact type·slice가 유효하지 않으면 평가를 실패시킨다.
이 fixture와 결과는 기존 UD 회귀 fixture를 대체하거나 규칙 선택에 사용하지 않는다.

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

canonical fixture는 도구 출력과 무관한 고정 seed로 생성한다. Core dev/test에는 수동 검토를
통과한 문장만 사용하며 source 이름만으로 정문임을 가정하지 않는다. 현재 후보 source는
UD Korean-Kaist다. 먼저 명사 180, 동사 120, 형용사 80, 부사 50, 대명사 30, 관형사 20,
수사 20개의 positive와 같은 source의 deterministic paired negative를 뽑아 split별 사전 검토
pool을 만든다. 검토자는 positive와 negative에 쓰인 고유 문장을 모두 확인한다. Pool은
`(source, sent_id, text)`의 정렬된 JSON line SHA-256과 문장 수로 고정하고, 제외한 문장은
sentence ID, 사유 class와 짧은 annotation으로 보존한다.

최종 fixture는 검토 pool에서 제외되지 않은 문장만 대상으로 다시 샘플링한다. 사전 검토
pool을 만든 quota도 review manifest에 보존해 pool을 재구성할 때 최종 quota 변경의 영향을
받지 않게 한다. 재샘플링은 명사 184, 동사 120, 형용사 80, 부사 50, 대명사 26, 관형사 20,
수사 20개의 positive와
negative 500개를 유지해 총 1,000개와 positive/negative 1:1 균형을 만족해야 한다. 검토 pool
밖의 새 문장으로 quota를 자동 보충하지 않는다. 검토된 문장만으로 quota를 채울 수 없으면
새 후보를 별도로 검토하고 pool digest를 갱신한 뒤 생성한다. 정렬과 샘플링은 원본 파일
순서가 아니라 case 식별자의 SHA-256 순서를 사용한다. 최종 positive는 한 문장에 최대 3개만
선택하며 상한에 도달한 문장의 다음 후보는 건너뛴다.

비문·오타가 포함된 UD Korean-KSL은 core에서 제외하고 별도 `robustness` source set으로
보존한다. Source 이름만으로 모든 문장을 오류 사례로 간주하지 않는다. Korean-KSL test split의
`Typo=Yes`·`goeswith` source signal 문장과 품사 quota를 채우는 deterministic 보충 후보로
pre-review pool을 먼저 고정하고, pool에 들어온 고유 문장을 모두 수동 검토한다. Review
manifest는 정렬된 `(source, sent_id, text)` 전체의 SHA-256과 각 문장의
`clean`·`noisy`·`source-artifact` 판정, 하나 이상의 오류 class와 짧은 annotation을 보존한다.
Source signal은 후보 수집에만 사용하며 수동 판정을 대신하지 않는다. `clean`과
`source-artifact` 문장은 Robust 품질 fixture에서 제외한다.

오류 class는 최소한 `hangul-typo`, `foreign-text-typo`, `spacing-merge`, `spacing-split`,
`nonstandard-morphology`, `nonstandard-syntax`, `repetition`을 구분한다. 여러 오류가 있는 문장은
모든 class를 기록하되 chart 집계용 primary class를 하나 고정한다. 의미 선택만 잘못되어
lemma·품사·span gold를 객관적으로 확정할 수 없는 문장은 `noisy` 판정을 보존하고 case 후보에서는
제외한다.

수동 검토에서 `noisy`로 판정한 문장만 대상으로 명사 90, 동사 60, 형용사 40, 부사 25,
대명사 15, 관형사 10, 수사 10개의 positive와 paired negative 250개씩을 같은 seed로 생성한다.
최종 500개 case는 제품이나 외부 분석기 결과를 보기 전에 query, coarse/fine POS, expected와
positive의 원문 UTF-8 byte span을 다시 수동 검토한다. Negative는 해당 lemma·품사가 문장에
없음을, 무품사 negative는 지원 품사 전체에 lemma가 없음을 확인한다. 각 case에는 오류가 gold
span에 직접 있는 `target-span`과 주변 문맥에만 있는 `context-only`를 구분한 `noise_scope`,
primary `noise_class`와 검토 annotation을 보존한다. Ambiguous gold나 annotation이 빠진 case로
quota를 자동 보충하지 않고 다음 검토 후보를 사용한다.

Core 검토에서 제외한 KAIST 문장도 별도 sentence-level robustness candidate registry에 원문,
split, sentence ID, 사유 class와 annotation을 보존한다. 이 registry의 사유 class는 corpus 정제
근거이며 query-level 제품 `noise_class` gold를 대신하지 않는다. Query, POS, expected, raw span과
noise scope를 확정하기 전에는 Robust 품질 합계에 넣지 않는다.

Robust 품질은 canonical과 분리한 같은 500-case explicit-POS fixture에서 모든 backend를
비교한다. 전체와 오류 class·scope·품사별 TP·FP·TN·FN, precision, recall, F1과 실패 case를
기록한다. Micro 전체는 동일한 자연 오류 fixture 안에서만 비교하며 natural·synthetic,
explicit-POS·untagged 또는 서로 다른 오류 class를 합쳐 단일 제품 점수나 순위를 만들지 않는다.
현재 제품 robustness가 구현되기 전의 첫 기준선은 kfind `off`와 각 외부 분석기의 고정 default
설정을 비교한다. Native robustness 기능이 있는 backend의 feature-matched 행은 같은 class,
candidate budget과 원문 span 역매핑 계약을 고정한 뒤 별도 표로 추가하며 default 행과 합치지
않는다.

같은 Robust fixture의 성능도 fresh process warm-up 1회 뒤 5회 측정한다. Embedded/full-POS
`smart`, Agent의 `embedded + any + explicit POS`, Human의 `full-POS + smart + untagged`와 고정
외부 backend에 대해 initialization, cases/s, p50·p95 latency와 peak RSS의 median/min/max를
기록한다. 품질과 성능은 같은 보고서에서 별도 표와 chart로 제시하고 canonical 합계와 섞지
않는다.

gold 후보는 CoNLL-U의 정렬된 lemma/XPOS 형태소 쌍에서 추출하고, lemma가 축약된 KAIST
어절은 `OrigLemma`를 우선 사용한다. 지원 품사에 속하고 표제어가 한글 음절로만 구성된
형태소만 포함한다. VV·VA·VX·VCP·VCN과 이에 대응하는 KAIST 용언 태그는 어간에 `다`를
붙여 사전형으로 정규화한다. 형태소 수와 XPOS 수가 끝까지 다른 어절, 접사·조사·어미,
외국어·숫자·기호는 제외한다. negative는 모든 어절의 lemma/XPOS가 정렬된 문장에서만
선택한다. 이 필터와 제외 건수는 metadata에 기록한다.

모든 도구는 동일한 `(문장, 표제어, 품사)` 존재 여부를 예측한다. positive는 예측 span이
gold 어절의 UTF-8 byte span과 겹쳐야 true positive이고, negative는 문장 어디에서든 같은
표제어·품사를 반환하면 false positive다. 도구마다 accuracy, precision, recall, F1과
TP·FP·TN·FN을 계산하고 corpus별·품사별 결과 및 실패 case를 함께 보존한다. 외부 분석기가
원문에 정렬할 수 없는 길이 0 형태소를 반환하면 검색 가능한 span 후보에서 제외한다.

고정 1,000-case 회귀 fixture와 별도로, 같은 core held-out source의 수동 검토 통과 문장에서
문장 안 검색 질의를 늘린 `query matrix` fixture를 생성한다. canonical positive가 하나 이상
있는 고유 문장을 matrix의
문장 집합으로 고정하고, 그 문장에 속한 canonical positive를 모두 보존한 뒤 정렬된 gold
후보를 문장당 최대 3개까지 추가한다. 추가 후보는 아직 선택하지 않은 coarse POS를 먼저
고르고, 같은 조건에서는 고정 seed와 source·sentence·token·morpheme·query의 SHA-256 순서로
결정한다. 같은 `(표제어, 품사)`가 문장에 두 번 이상 나타나 gold span이 하나로 정해지지 않는
후보는 추가 대상에서 제외한다. canonical positive가 문장당 3개를 넘거나 fixture의 모든
canonical positive가 matrix에 정확히 한 번 포함되지 않으면 생성을 실패한다.

각 matrix positive에는 같은 source의 gold 후보 중 대상 문장에 없는 표제어를 하나 대응시켜
동일 문장 negative를 만든다. 명시적 품사 fixture는 positive와 같은 coarse POS를 유지하고
같은 `(표제어, 품사)`가 문장에 없음을 요구한다. 무품사 fixture는 표제어가 지원 품사 전체에
걸쳐 문장에 없음을 요구한다. 한 문장 안의 negative query는 서로 달라야 하며, positive와
negative를 1:1로 유지한다. fixture에는 문장 group, `present-N`/`absent-N` slot, canonical
positive ID와 paired positive ID를 보존하고, metadata에는 문장 수, 문장당 질의 수 분포,
품사 분포, canonical coverage와 source별 case 수를 기록한다.

query matrix는 질의별 strict·계약 보정 품질과 성능을 병렬로 보고한다. 두 품질 축에는 각각
confusion matrix, precision·recall·F1과 문장별 모든 positive 회수율을 포함한다. 회수한 질의 수
분포와 slot별 품질도 strict·계약 보정 기대값을 구분해 보존한다. 질의가 문장 안에서
독립이라는 가정을 하지 않으며 두 recall의 불확실성은 각각 문장 group을 재표집하는 고정 seed
10,000회 cluster bootstrap 95% 구간으로 기록한다. 고정 test matrix는 kfind의
embedded/full-POS와 smart/token/any, 사람용 무품사 profile,
Kiwi·Lindera·MeCab-ko·KOMORAN을 모두 측정한다. 외부 결과는 matrix fixture SHA-256에 묶인
별도 version-controlled snapshot으로 보존한다. development matrix는 kfind 진단에만 사용한다.

query matrix는 기존 1,000-case 회귀선과 지표를 대체하거나 합치지 않는다. canonical 지표는
장기 회귀 판정, matrix 지표는 같은 문장 안의 질의 다양성·부분 회수·동일 문장 false positive
진단에 사용한다. 제품 규칙 선택과 unseen 검증 gate는 기존 dev/test/blind 계약을 그대로
따른다.

이 strict corpus-gold 지표는 제품의 의미 중의성 non-goal과 분리해 항상 보존한다. 버전 관리
fixture가 `contract_expected`와 `contract_reason`을 함께 선언한 경우에는 같은 예측을 제품 계약
기대값으로 다시 계산한 `contract_adjusted` 지표도 병렬로 기록한다. 이 지표의 confusion matrix는
`contract_tp`·`contract_fp`·`contract_tn`·`contract_fn`, 파생 지표는
`contract_precision_percent`·`contract_recall_percent`·`contract_f1_percent`로 명명한다.
표에서는 각각 TPᶜ·FPᶜ·TNᶜ·FNᶜ로 줄여 쓸 수 있다.
canonical·hard-negative의 contract-positive 분모는 `PNᶜ = TPᶜ + FNᶜ`로 표기하며,
recall 개선 보고서는 `PNᶜ`, `FNᶜ`와 `recallᶜ = TPᶜ / PNᶜ`를 함께 기록한다.

`contract_expected`가 없으면 strict `expected`를 그대로 사용한다. 두 값이 다르면
`expected=false`, `contract_expected=true`만 허용하고, 제품 결과를 보기 전에 고정한
`contract_reason`이 필요하다. 허용 사유는 같은 품사의 동형 활용을 의미로 구분하지 않는
`same-pos-homograph`와 source에 정렬된 내부 성분을 검색하는 `aligned-source-component`다.
제품 출력이나 외부 분석기 출력으로 annotation을 자동 생성하지 않는다. strict 지표와 계약 보정
지표를 합치거나, 계약 보정 지표만으로 정밀도 회귀가 없다고 주장하지 않는다.

query matrix의 FNᶜ를 닫는 작업은 원시 confusion matrix와 별도의 disposition 장부로
관리한다. 분류를 이유로 `expected`나 `contract_expected`를 바꾸거나 FNᶜ를 성공으로
재계산하지 않는다. 장부는 fixture SHA-256과 case ID, query·품사·gold surface, 현재 failure
cause, disposition, 근거, 사전 증거를 보존한다. 완료 상태는 원시 FNᶜ 0과 미분류 FNᶜ 0을
구분해 보고한다.

disposition은 다음 중 하나다.

1. `product-fix`: 기존 계약과 정밀도 gate를 지키는 제한된 규칙으로 회수할 수 있다.
2. `dictionary-required`: 일반화 가능한 표제어·품사·활용·관계 증거가 있어야 안전하게
   회수할 수 있다.
3. `structurally-unresolvable`: 검색할 byte span이 없거나, 동일 표면형의 분석을 문맥 의미 없이
   구분해야 하거나, source 내부 성분과 제품이 반환할 span의 대응을 결정할 수 없다.
4. `cost-prohibitive`: 회수하려면 검색 시점의 범용 형태소 분석, 큰 runtime 상태, 또는 기대
   효용에 비해 과도하게 넓은 경계 탐색이 필요하다.
5. `gold-or-adapter`: gold lemma·품사·정렬이나 외부 adapter의 정규화가 제품 계약과 맞는지
   먼저 수정·확정해야 한다.
6. `out-of-contract`: derivation, 비표준 띄어쓰기, 품사를 지정하지 않은 미등재 외래
   고유명사처럼 현재 profile의 명시적 범위 밖이다.

사전 증거는 고정한 snapshot의 구조화된 표제어·품사·활용·어휘 관계·문법 주석 필드만
사용한다. 문법 주석은 조사 host처럼 앞말 종류를 직접 선언한 경우에만 사용하며, 자유 서술
정의와 용례 문장의 단어 출현은 형태 관계의 증거로 사용하지 않는다. 자동 제품 반영에는
한국어기초사전과 표준국어대사전의 일치가 필요하고, 우리말샘 단독 기록은 audit 후보로만 남긴다.
다운로드 snapshot의 hash와 importer revision이 다르면 장부를 갱신하지 않는다. 사전으로도
표면 span, 문맥 의미, source 정렬 문제를 해결할 수 없는 case는 `dictionary-required`로
분류하지 않는다.

외부 분석기의 정규화된 결과와 성능은 test fixture SHA-256, adapter·성능 schema,
도구·사전·모델 버전과 설정에 묶인 version-controlled snapshot으로 보존한다. 기본 benchmark는 snapshot을 읽고
`kfind`만 다시 실행한다. fixture SHA-256 또는 adapter schema가 다르면 자동으로 외부 분석기를
실행하거나 오래된 결과를 사용하지 않고 refresh 명령과 함께 실패한다. 도구·사전·모델 버전과
설정은 snapshot을 명시적으로 갱신할 때만 바꾼다.

기본 benchmark 이미지는 `kfind` 측정 runner와 외부 snapshot 검증 코드만 포함한다. 외부 분석기와
전용 runner의 빌드·실행 의존성은 별도 snapshot refresh 이미지에만 포함한다. 기본 CI smoke는 기본
이미지만 빌드하며 외부 분석기 의존성을 컴파일하거나 설치하지 않는다.

`scripts/benchmark-morphology.sh`의 기본 stdout은 현재 측정 단계와 최종 JSON·Markdown 보고서
경로만 출력한다. 실행 실패와 외부 도구 진단은 stderr에 출력한다. Docker 빌드 과정과 생성한
Markdown 보고서 전문은 `KFIND_MORPH_VERBOSE=1`을 지정한 경우에만 터미널에 출력한다.

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
명사 component frame을 새로 여는 경우에도 development positive와 같은 candidate 표면형이
일반 합성어 내부에서 우연히 나타나는 hard-negative를 먼저 고정한다.
분류를 위한 추가 컴파일·검색 비용은 backend 성능에 포함하지 않는다.

규칙 개발은 Korean-Kaist·KSL dev split을 test split과 독립된 seed·fixture
SHA-256로 생성해 사용한다. test 1,000개 baseline은 변경하지 않는다. hard-negative는
도구 출력과 무관한 버전 관리 fixture로 두고 slice별 precision을 전체 품질과 분리해
보고한다. 의미 중의성 또는 정렬 source component 때문에 strict negative를 제품이 의도적으로
허용하는 hard-negative는 `contract_expected`와 사유를 명시하고 strict·계약 보정 결과에 모두
남긴다. CI smoke set은 dev fixture에서 source·품사·class별 고정 case를
deterministic하게 추출하고, 수동 벤치마크는 dev·test·hard-negative 전체를 사용한다.

명시적 품사 `smart` 형태 품질 변경은 dev strict precision 99.00% 이상과 version-controlled
hard-negative 신규 contract FP 0을 지키면서 표준 띄어쓰기 case의 FN을 늘리지 않아야 한다.
부사와 용언 사이에 필요한 공백이 빠진 `안팔아서`, `안좋습니다`, `안나와요`, `못해요` 같은
`nonstandard-spacing` case는 strict 지표와 row-level delta에 그대로 남기되 이 gate에서 제외한다.
해당 입력의 FP/FN은 별도 robust 지원을 도입할 때 해소한다. 신규 strict FP는 구현과
독립적으로 미리 고정한 `contract_expected=true` case에서만 허용한다. FN이 줄어든 후보를 우선하고,
FN이 같을 때만 FP가 줄어든 후보를 선택한다. 고정 test fixture는 규칙 선택에
사용하지 않고 FN 비증가, precision 99.00% 하한과 전체 품질 회귀만 확인한다. 무품사 fixture의
결과도 같은 변경에서 다시 측정해 불리한 변화까지 기록하되 규칙 선택이나 fixture 변경 근거로
사용하지 않는다. 최종 품질 주장은 구현 전에 source·fixture를 고정하고 기존 corpus와 문장 hash
중복이 없는 unseen 평가에서도 같은 기준을 통과해야 한다. 기본 `smart`를 변경하는 구현은 기존
hard-negative에 새 contract FP를 추가하지 않아야 하며, 이 조건을 만족하지 못하면 별도 boundary
policy로 분리한다.

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

`matcher/build_and_find_short`는 미리 compile한 다중 앵커 단일 atom plan으로 짧은 문장 하나를
검색한다. 각 iteration에서 matcher를 새로 만들어 one-shot build와 첫 검색을 함께 측정한다.
`matcher/scan_deterministic_corpus`는 같은 matcher를 충분히 큰 corpus에 재사용해 adaptive
automaton 승격 이후의 scan 회귀를 감시한다. 두 workload를 함께 비교해 짧은 입력의 build 비용을
줄이면서 대규모 scan을 희생하지 않았는지 판정한다.

`matcher/phrase_find_all`은 1,024개 line 중 4개마다 `n:길 v:걷다`가 일치하는 고정 corpus를 메모리 입력으로 사용한다. smart boundary의 component 검증에 필요한 고정 resource를 matcher 생성 시 제공한다. 전체 phrase match를 반환하는 한 번의 호출을 측정해 match 수에 따른 반복 anchor scan과 span 결합 회귀를 감시한다.

`matcher/phrase_find_all_repeated`는 같은 한 음절 literal atom 8개와 한 줄의 반복 span 128개,
큰 `max-gap`을 사용한다. 가능한 조합 수와 무관하게 bounded DP로 leftmost-longest 결과를 찾는
병적 입력 경로를 측정한다.

`matcher/phrase_input_searcher_repeated_line`은 줄바꿈 없는 한 줄에서 인접한 두 literal atom
phrase가 4,096번 반복되는 입력을 `InputSearcher`의 metadata 출력 경로로 검색한다. 한 줄의
anchor와 atom span을 한 번만 수집하는지와 match 수에 따른 반복 suffix scan 회귀를 감시한다.

`matcher/structural_repeated_long_line`은 `매일`이 16,384번 반복되는 줄바꿈 없는 UTF-8 입력을
`RepeatedToken + MAG` 구조 pattern을 가진 `smart` 부사 matcher로 검색한다. 각 candidate의
인접 token만 해독하는지와 candidate마다 전체 입력의 UTF-8을 다시 검증하는 회귀를 감시한다.

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
candidate program 수 2배 이상 증가
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

- 모든 파일 크기와 candidate program 수에 상한을 둔다.
- phrase matcher의 DP 상태는 검증된 atom span 수의 합에 비례하며, 전체 조합용 API의 중간
  partial 수에는 명시적 상한을 둔다.
- 사용자 사전 파싱 오류에는 파일명과 줄 번호를 표시한다.
- symlink 순환을 방지한다.
- 검색 결과, 검색 중 issue, 초기화 오류를 포함한 모든 사람이 읽는 출력에 escape 정책을 적용해 제어 문자가 터미널 동작을 바꾸지 않게 한다.
- JSON에는 원문 제어 문자를 정상 escape한다.
- 파일 경로가 유효 UTF-8이 아니어도 처리한다.
- broken pipe에서 panic하지 않는다.
- matcher와 constraint resolver는 unsafe 없이 구현하는 것을 기본 원칙으로 한다.

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
let engine = Engine::with_resources(ResourceBundle {
    full_pos: Some(full_pos_bytes),
    enriched_predicates: Some(enriched_predicates),
    component: Some(component_bytes),
})?;

let matcher = engine.compile("권한", &CompileOptions::default())?;
let matches = matcher.find_all("사용자권한을 확인한다.".as_bytes());
```

- `Engine::new`는 embedded lexicon만 초기화한다.
- `ResourceBundle`과 `Engine::with_resources`는 full POS, enriched predicate와 component resource를
  한 profile로 검증한다. 기존 개별 생성자는 이 경로에 위임한다.
- `load_component_resource`는 새 bytes를 모두 검증한 뒤 상태를 교체하며 실패하면 기존
  resource를 보존한다.
- `compile`은 query plan과 anchor matcher를 만들고 component resource가 필요한 plan의 누락을
  `ComponentResourceRequired`로 보고한다.
- `Matcher::find_at`과 `find_all`은 UTF-8 byte offset과 형태 provenance가 포함된
  `PhraseMatch`를 반환한다.
- root의 `PhraseMatch`, `VerifiedSpan`, `Origin`, `RuleId`와 compile option·오류는 1.x 안정
  계약이다. `QueryPlan`, candidate program·structural constraint 표현, `Lexicons`와 plan inspection은 `kfind::expert`의
  변경 가능한 저수준 API다.
- workspace 내부 crate는 게시하지 않으며 `kfind::expert` 외의 경로를 공개 API로 간주하지 않는다.
- JavaScript API는 같은 profile을 `Kfind.withResources`, 같은 수명 주기를
  `loadComponentResource`, `compile`, `Matcher.findAll`로 노출하고 offset을 UTF-16 code unit으로
  변환한다.

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
