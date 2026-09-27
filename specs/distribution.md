# 웹과 패키지 배포

이 문서는 [kfind 기술 사양서](kfind.md)의 현행 규범적 계약이다.

## 0. 제품 계약

이 절은 뒤의 일반 설명보다 우선한다.

### 0.4 Web 문서와 playground

- 문서 홈은 짧은 검색 예시 뒤에 설치·첫 검색, 결과 해석, 에이전트와 API의 읽기 경로를
  제공한다. 내부 구현 설명은 사용 경로 뒤에 두며 상세 문서로 연결한다. 기존 route와
  fragment는 유지하고 한국어·영어에서 같은 순서와 목적의 문서를 제공한다.
- 설치 안내는 native CLI와 npm 경량 CLI를 별도 실행 계약으로 구분한다. 파일 순회,
  리소스, 입력 적재, 출력 단위와 offset 비교를 설치 명령보다 먼저 제시한다. Native 전용
  명령 예시는 npm CLI에서 실행할 수 있는 것으로 설명하지 않는다.
- 결과 해석 안내는 검색 결과 없음과 실행 오류를 구분하고, 구조 검증 상한으로 제외된
  후보가 일반 검색의 불일치와 구별되지 않는 한계를 명시한다. 품사·경계·입력 경로를
  점검하는 절차를 제공하되 `any`가 모든 누락을 복구하거나 의미를 판별한다고 설명하지 않는다.
- 의미 판별의 한계는 `걷다`와 `걸다`의 활용형이 `걸어`로 겹치는 예시로 설명한다.
  `걷다` 검색에서 `길을 걸어`와 `전화를 걸어`가 모두 후보가 될 수 있으며, 두 문맥의
  의미를 구분하지 않는다는 점을 명시한다. 동의어 확장의 비범위와 이 한계를 혼동하지 않는다.
- 문서의 일반 본문은 제한된 행 길이와 본문 색상을 사용한다. 제목과 절 사이 간격은
  읽기 흐름을 유지하고, 작은 화면의 큰 제목이 첫 사용 예시를 과도하게 밀어내지 않아야 한다.
- 공개 문서와 playground의 현재 버전은 `https://kfind.pages.dev`의 정적 Cloudflare Pages site로
  배포한다. 게시한 과거 버전은 같은 origin의 `/versions/VERSION/*`에서 제공한다.
  문서는 제품 목적과 goal/non-goal, 검색 model, query 문법, 사람·에이전트 workflow, 주요 옵션,
  최신 제품·외부 benchmark를 설명하고 전체 README와 source report로 연결한다.
- site header, browser favicon, touch icon과 social card는 둥근 사각형 안의 `k/` brand mark를
  공유한다. Header에서는 mark 옆에 `kfind` 이름을 유지하고, 작은 크기의 icon은 같은 벡터
  형상을 사용해 흐려지거나 찌그러지지 않아야 한다. `k`의 기둥과 두 팔은 같은 굵기의 단일
  폐합 path로 이어져 내부 경계가 드러나지 않아야 한다. `k/`의 획 끝은 둥글지 않은 평면이고
  경로 끝점 바깥으로 돌출되지 않아야 한다. Icon metadata를 제공하는 배포 채널은 128px 이상의
  PNG 자산을 사용한다.
- 한국어 문장은 기술 개념의 관계와 동작이 바로 드러나도록 쓴다. 제품·도메인의 표준 용어와
  코드 식별자는 원문 표기를 유지하되, 영어 문장 구조를 직역하거나 일반 용어를 기계적으로
  음역하지 않는다. 조건과 결과가 한 문장에 몰리면 문장을 나눠 설명한다. 문장 뒤의 콜론으로
  목록을 도입하거나 em dash로 부연하지 않는다. 영어 번역은 한국어 어순과 문형을 기계적으로
  옮기지 않고 같은 의미를 자연스러운 영어로 서술한다. 두 언어 모두 현재 제품을 설명하는 데
  필요한 사실만 남기고 같은 내용을 헤딩, 개요, 목록에 반복하지 않는다.
- 각 문서 route는 전제와 용어를 먼저 정의하고, 원인·동작·결과와 적용 범위를 연속된 문단으로
  논증한다. 핵심 설명은 본문만 읽어도 완결되어야 하며, callout·card·도해와 단편적인 label의
  나열로 본문을 대신하지 않는다. 표, 도해와 코드 예시는 정확한 대응 관계나 실행 흐름을
  보충할 때만 사용하고 앞뒤 문단에서 해석한다.
- 문서 제목 아래에는 모든 route에 동일한 형식의 subtext를 강제하지 않는다. 본문을 읽기 전에
  필요한 전제나 범위가 있으면 별도 헤딩 없이 하나 이상의 개요 문단으로 두고, 제목과 첫 절이
  내용을 충분히 설명하면 개요를 생략한다.
- 지원·거부 범위를 설명하는 절은 규칙 이름이나 존재하지 않는 표를 가리키는 데 그치지 않는다.
  사용자가 실행할 query와 source 예시를 들어 match되는 경우와 match되지 않는 경우를 함께
  설명한다. 예시는 본문이 선언한 boundary, POS, resource와 문법 조건을 그대로 재현해야 한다.
- 문서 site는 React Router Framework Mode로 구성한다. Runtime SSR은 사용하지 않고 고정된 모든
  문서 route를 build 시점에 한국어와 영어 HTML로 각각 prerender한다. 한국어는 query가 없는 clean
  URL에서, 영어는 같은 path의 `?hl=en` URL에서 제공한다. 각 URL의 최초 response에는 해당 locale의
  본문, 고유 title·description·self-canonical URL과 Open Graph metadata가 들어 있어야 한다. Browser는
  이 HTML을 hydrate하며 이후 route 전환은 전체 페이지를 다시 요청하지 않는다. 공통 shell 안에서
  다음 경로를 제공한다.
- 검색 엔진이 site 이름과 문서 계층을 해석할 수 있도록 prerender metadata에 JSON-LD를 포함한다.
  `/`와 `/?hl=en`에는 현재 locale의 canonical URL을 가리키는 `WebSite`와 한국어·영어 alternate
  name을, 나머지 indexable route에는 Home, 해당 GNB 영역과 현재 문서를 잇는 `BreadcrumbList`를
  둔다. GNB 영역의 첫 문서가 현재 문서이면 같은 항목을 반복하지 않는다. Open Graph metadata는
  site 이름, 현재 locale과 다른 locale을 명시하고, 같은 title·description을 social card
  metadata에서도 사용한다.
- 홈의 title, description, `h1`과 첫 문단은 `kfind`라는 이름만 제시하지 않고 한국어 표제어와
  활용형을 찾는 검색 엔진이라는 제품 범위와 `걷다` 검색 예시를 직접 설명한다. 하위 route의
  browser title은 검색 결과만 읽어도 제품과 문서 주제를 구분할 수 있게 작성한다. 문서 안의
  navigation label과 breadcrumb 이름은 browser title보다 짧은 현재 정보 구조의 이름을 유지한다.

  ```text
  /                                      개요와 제품 범위
  /guide/getting-started                 설치와 첫 검색
  /guide/installation                    배포 방식과 실행 환경
  /guide/workflows                       사람·에이전트 검색 절차
  /guide/goals                           목표와 비목표

  /cli                                   CLI 개요
  /cli/query-syntax                      query atom과 태그 문법
  /cli/parts-of-speech                   품사 지정과 자동 판정
  /cli/expansion                         literal·inflection·derivation
  /cli/boundaries                        smart·token·any 경계
  /cli/phrases                           구 검색과 max gap
  /cli/input-output                      파일·인코딩·출력 형식
  /cli/diagnostics                       오류·종료 상태·진단
  /cli/resources                         사전·resource·사용자 설정
  /cli/recipes                           반복 가능한 검색 예시

  /agents                                에이전트 통합 개요
  /agents/workflow                       권장 검색 절차
  /agents/skills                         skill 설치와 사용
  /agents/integrations                   Codex·Claude Code·Gemini CLI
  /agents/automation                     자동화 패턴
  /agents/contract                       JSON Lines와 통합 계약

  /internals/architecture                구성 요소와 책임 경계
  /internals/pipeline                    compile·scan·verify pipeline
  /internals/query-compiler              분석과 검색 program 생성
  /internals/matcher                     anchor scan과 후보 검증
  /internals/structural-verification     compact resource 기반 판정
  /internals/resources                   resource format과 초기화
  /internals/unicode-spans               정규화와 span 좌표계
  /internals/performance                 비용 모델과 병렬 실행

  /internals/morphology                  한국어 형태 처리 개요
  /internals/morphology/parts-of-speech  세부 품사와 coarse POS
  /internals/morphology/nominals         체언과 명사 계열
  /internals/morphology/particles        조사와 이형태
  /internals/morphology/predicates       용언과 어간
  /internals/morphology/endings          선어말어미·어말어미
  /internals/morphology/irregulars       불규칙 활용
  /internals/morphology/derivation       파생과 품사 전환
  /internals/morphology/compounds        합성어와 보조용언
  /internals/morphology/contractions     축약과 영형태
  /internals/morphology/ambiguity        중의성과 경계 판정
  /internals/morphology/coverage         규칙 범위와 비목표

  /benchmarks                            평가 개요와 최신 품질·성능 결과
  /benchmarks/methodology                fixture와 측정 절차
  /benchmarks/contract                   raw·contract-adjusted 계약
  /benchmarks/canonical                  표준 맞춤법 품질
  /benchmarks/query-matrix               문법 조합 품질
  /benchmarks/robustness                 오류 문장 품질
  /benchmarks/performance                workload별 성능
  /benchmarks/comparisons                외부 분석기 비교
  /benchmarks/reproducibility            revision·입력·실행 명령
  /benchmarks/reports                    날짜별 source report

  /reference/cli                         native·npm CLI 옵션
  /reference/query-language              query 언어 문법
  /reference/pos-tags                    품사 태그
  /reference/configuration               환경 변수와 설정 파일
  /reference/user-lexicon                사용자 사전 형식
  /reference/jsonl                       JSON Lines schema
  /reference/exit-codes                  종료 코드
  /reference/errors                      오류 분류
  /reference/rust                        Rust facade API
  /reference/javascript                  JavaScript·TypeScript API
  /reference/resources                   resource 파일과 schema
  /reference/rule-ids                    provenance rule ID
  /reference/glossary                    문법·실행·평가 용어
  /reference/licenses                    코드·데이터 라이선스

  /playground                            WebAssembly 플레이그라운드
  ```

- 전역 navigation은 영어에서 `Home`, `Get Started`, `CLI`, `Agents`, `Internals`, `Benchmarks`,
  `Reference`, 한국어에서 `홈`, `시작하기`, `CLI`, `에이전트`, `내부 구조`, `벤치마크`, `명세`의
  한 단계 GNB로 구성한다. 한국어 navigation category, 문서 제목과 eyebrow에서 영어
  `Reference`를 `참조`로 직역하지 않는다. `Playground`와 GitHub는 GNB 오른쪽의 독립 action으로
  둔다. 문법 항목, 구현 단계와 개별 지표를 GNB에 직접 나열하지 않는다.
- Playground는 전역 header와 footer만 문서 route와 공유하는 별도 layout을 사용한다. 문서 GNB에는
  활성 항목을 두지 않고 `Playground` action을 현재 page로 표시하며, 문서 sidebar와 좁은 화면의
  문서 메뉴를 렌더링하지 않는다. Playground의 좁은 화면 site 메뉴는 문서 GNB와 외부 link만
  제공하고 문서 목차는 포함하지 않는다.
- 문서 route의 좌측 sidebar는 현재 GNB 영역에 속한 문서와 현재 문서의 절을 함께 표시한다.
  현재 route와 절은 접근 가능한 navigation 상태로 구분한다. 데스크톱에서는 sticky sidebar로,
  좁은 화면에서는 같은 계층을 보존하는 collapsible 문서 메뉴로 제공한다. 메뉴 trigger에는
  펼침 상태를 나타내는 chevron과 접근 가능한 expanded 상태를 함께 제공한다. 본문 순서와
  sidebar 순서는 일치해야 하며, 언어를 전환해도 route와 절 fragment는 유지한다. `Internals`의
  한국어 형태 처리는 별도 sidebar 하위 범주로 묶는다. Benchmark의 평가 개요와 최신 결과는
  `/benchmarks`에 함께 표시하고, 방법론과 역사 보고서는 별도 route로 분리한다. GNB에는 이 하위
  범주를 펼치지 않는다.
- 각 문서 route의 본문 하단에는 GNB와 sidebar의 전체 문서 순서를 기준으로 이전·다음 문서
  link를 제공한다. 첫 문서에는 다음 link만, 마지막 문서에는 이전 link만 표시하며 link label은
  현재 locale을 따른다. Playground와 정의되지 않은 경로는 이 순서에 포함하지 않는다.
- 사람이 읽는 문서 본문은 route와 locale별 MDX source로 관리한다. Route metadata, navigation과
  SEO catalog는 typed index에서 관리하되 제목 아래의 개요, 절, 문단, 목록, 표와 code 예시는
  TypeScript object에 장문 문자열로 넣지 않는다. 한국어와 영어 MDX는 같은 절 계층과 stable heading
  ID를 유지하며 build에서 route index와의 일치 여부를 검사한다. 서로 다른 canonical route가 같은
  locale의 MDX 본문을 복제하지 않는다.
- MDX에서 쓰는 callout, lead, step, code title과 표 wrapper는 공통 문서 component library로
  제공한다. 공통 component는 locale에 종속된 본문을 내부에 복제하지 않고 MDX children을
  접근 가능한 HTML 구조로 표현한다. Callout은 본문과 같은 읽기 폭 안에서 제목과 내용을
  한 열로 배치하고, 종류별 inline-start 강조선과 절제된 배경으로 본문을 보충한다. 좁은
  화면에서도 별도 제목 열로 본문 폭을 줄이지 않으며, 내부 첫·마지막 block의 바깥 여백은
  component가 정규화한다. Route별로 같은 시각 요소를 다시 구현하지 않는다.
- Fenced code block은 build 시점에 문법 highlighting을 완료한다. 배포된 문서가 색상을 입히기
  위해 browser에서 highlighter runtime이나 grammar를 내려받지 않게 하며, 언어가 지정되지 않은
  block도 읽을 수 있는 기본 표현을 제공한다. Highlighting 결과는 본문 code block의 복사,
  가로 scroll과 접근 가능한 text 선택을 방해하지 않는다.

- 문서 site의 popup, select, collapsible과 form control은 `@base-ui/react`의 unstyled primitive로
  구성한다. 링크, label, keyboard와 pointer 동작은 해당 primitive의 접근성 의미를 유지하고,
  제품 고유 동작만 route component에서 추가한다.

- 단어장은 검색 입력, 실행 구조, resource와 품질 지표에 쓰는 핵심 용어를 한곳에서
  정의한다. 한국어 표기와 코드·영문 표기는 같은 항목에서 대응시키고, 다른 문서의 설명은
  이 정의와 모순되지 않아야 한다. 형태소 label은 별도 범주에서 NNG·VV·EP처럼 문서에
  노출되는 세부 label을 각각 정의한다.
- 각 문서 route는 단어장 용어가 본문에서 처음 등장하는 한 곳에만 tooltip과 해당 정의 link를
  제공한다. 같은 항목의 한국어·영문 별칭은 한 용어로 센다. 단, `TP`, `FP`, `TN`, `FN`, `TPᶜ`,
  `TNᶜ`, `FPᶜ`, `FNᶜ`, `POS`, `F1`과 형태소 label처럼 독립해서 읽는 영문 acronym alias는 같은
  용어의 일반 별칭이 먼저 나왔어도 acronym별 첫 등장에 별도 tooltip을 제공한다. 형태소 분석 표기
  안의 label도 code 전체를 제외하지 않고 label 자체에 tooltip을 제공한다. Tooltip은 단어장에
  notation이 있는 용어의 notation, 현재 언어의 이름과 정의를 함께 표시한다. 형태소 label과 acronym은
  `VV · 동사`, `TP · 참양성 · true positive`, `TPᶜ · 계약 조정 참양성 · contract-adjusted true
positive`처럼 code, 현재 언어의 이름, 영문 원문 순서로 표시하되 같은 이름을 중복하지 않는다.
  Tooltip은 hover와 keyboard focus로 열 수 있어야 한다. 실제 mouse pointer activation과 keyboard
  Enter activation은 기존 link 동작을 유지한다. Touch·pen pointer activation과 선행 input event가
  없는 link activation은 첫 번째에 tooltip을 열고, 같은 용어의 다음 activation에 단어장으로 이동한다.
  이 구분에 media query나 click metadata를 사용하지 않는다. 기존 link와 form control에는 중첩해서
  적용하지 않는다. MDX source의 `/reference/glossary#*` link는 일반 본문 link로 그대로 출력하지 않고
  이 page-local 첫 등장 규칙을 적용하는 tooltip trigger로 해석한다. MDX source는 acronym 예외를
  제외하고 같은 용어의 단어장 link를 한 route에 중복해서 작성하지 않는다.
- 일반 UI text는 Pretendard 기반 sans-serif stack을 사용한다. 코드, 명령, query·output label과
  기술 도해의 코드 표기는 기존 monospace stack을 유지한다. 본문 인라인 code에는 배경, border,
  radius와 최소 padding을 적용해 문장과 구분하고, code block 안에서는 이 장식을 중첩하지 않는다.
- 데스크톱 문서 본문과 하단 이전·다음 navigation은 route의 문서 길이, 표와 code block 너비에
  관계없이 같은 content column을 사용한다. 세로 scrollbar 유무로 shell과 본문 중심축이 움직이지
  않게 viewport scrollbar 영역을 안정적으로 확보한다. 표와 code block의 overflow는 content
  column을 넓히지 않고 해당 container 안에서 처리한다.
- 문서 table은 열 내부의 짧은 label·수치·단위를 줄바꿈하지 않는다. 화면보다 넓은 table은
  cell을 접는 대신 table container에서 가로로 scroll한다.
- 공통 spacing scale은 `0.25rem`, `0.5rem`, `0.75rem`, `1rem`, `1.5rem`과 section 간격
  `2.5rem`을 사용한다. 문서 카드와 playground panel은 이 scale로 padding과 gap을 제한하며,
  상태 badge와 짧은 token은 좁은 화면에서도 내용 너비만 차지한다.
- Framework Mode의 route code splitting으로 첫 문서 화면에 불필요한 페이지 코드가 포함되지 않게 한다.
  Playground의 WASM module과 선택적 component resource는 `/playground`에 들어가기 전에는
  불러오지 않는다. 문서 route 전환은 전체 페이지를 다시 요청하지 않고, 현재 경로와 제목을
  접근 가능한 navigation 상태로 표시한다.
- Build는 실제 `robots.txt`와 전체 문서 route의 한국어·영어 URL을 열거한 `sitemap.xml`을 배포한다.
  각 sitemap 항목은 `ko`, `en`, `x-default` alternate URL을 포함한다. 정의되지 않은 경로는 SPA
  fallback으로 `200`을 반환하지 않고 prerender한 `404.html`과 HTTP 404를 반환한다. Build는 모든
  indexable route의 locale별 HTML에 올바른 document language, 단일 `h1`, 고유 title·description,
  self-canonical URL, 상호 `hreflang`, social metadata와 유효한 route별 JSON-LD가 있는지 검사한다.
  Sitemap URL 집합은 두 locale의 public URL 집합과 정확히 일치해야 하며 `404.html`은 `noindex`를
  유지한다.
- 현재 clean path의 문서 HTML, metadata, `robots.txt`, `sitemap.xml`과 `404.html`에는 장기 browser
  cache를 적용하지 않는다. Cloudflare Pages의 deployment invalidation, ETag와 revalidation을 사용해
  `main` 배포마다 최신 문서를 확인한다. Content hash가 filename에 포함된 `/assets/*`와 불변
  versioned archive의 파일만 `immutable` 장기 cache를 적용하며, HTML과 Pages Function을 포함하는
  broad Cache Rule은 두지 않는다. Playground의
  component resource Cache Storage는 이 문서 cache와 분리하고 아래 resource revision 계약을 따른다.
- Publish workflow는 선택한 GitHub Release tag의 source로 한국어·영어 문서, playground와 해당
  버전의 component resource를 빌드한다. 산출물은 GitHub Release의
  `kfind-site-VERSION.tar.gz`와 R2의 불변 `site/versions/VERSION` archive·index로 함께 보존한다.
  Archive index는 각 파일의 byte offset, 길이, media type과 cache policy를 기록한다. 같은 버전의
  기존 index와 archive checksum이 다르면 덮어쓰지 않고 게시를 실패한다.
- `site/versions/manifest.json`은 게시가 끝난 버전, prerelease 여부와 SemVer상 최신 버전을
  보존한다. 새 archive와 index를 모두 올린 뒤 manifest를 마지막에 교체한다. 따라서 version
  selector에는 부분 업로드된 버전이 노출되지 않는다. Stable과 RC를 모두 나열하되 RC를 stable
  또는 latest npm channel로 취급하지 않는다.
- Pages Functions Worker는 `/versions/VERSION/*`의 version과 상대 경로를 검증하고 manifest에 있는
  버전만 R2에서 제공한다. Index JSON은 명시적 크기 상한 안에서 schema를 검증한 뒤 읽고, archive
  body는 해당 파일의 byte range만 R2에서 읽어 buffering 없이 응답한다. Versioned response에는
  `X-Robots-Tag: noindex`를 넣고 HTML은 현재 locale의 `Content-Language`를 유지한다. 존재하지 않는
  버전·파일은 404, GET·HEAD 이외 method는 405로 응답하며 임의의 current asset으로 fallback하지
  않는다.
- Header의 version selector는 R2 manifest와 현재 build version을 표시한다. 최신 게시 버전은 기존
  clean path로, 다른 버전은 현재 route·locale query·fragment를 보존한
  `/versions/VERSION/*` path로 이동한다. Manifest를 불러오지 못하면 현재 build version만 표시하고
  문서 탐색을 막지 않는다. 좁은 화면에서도 언어 선택과 함께 접근 가능한 label과 select keyboard
  동작을 제공한다.
- JavaScript와 resource 참조 문서는 npm asset의 역할, `@kfind/kfind/assets` resolver, browser
  bundler와 Node.js 서버의 자체 서빙 절차를 함께 설명한다. 예제는 resource bytes를
  `Kfind.withResources`에 전달하는 초기화, HTTP content type과 `nosniff`, same-origin 또는 명시적
  CORS, cache key의 package version·content hash 포함을 보여 준다. 고정 URL은 revalidation 없이
  `immutable`로 캐시하지 않으며 package upgrade에서 JavaScript·WASM·resource를 원자적으로
  교체해야 한다.
- 문서 locale은 같은 path와 UI 구조를 유지하면서 query로 전환한다. 지원 locale은 한국어 `ko`와
  영어 `en`이며 query가 없는 URL은 한국어, `?hl=en`은 영어다. 다른 query parameter와 fragment는
  언어를 전환해도 보존한다. 영어 문서의 내부 link는 `hl=en`을 이어서 검색 engine과 사용자가 같은
  영어 문서 계층을 탐색하게 한다. Locale별 공통 UI 문구, navigation과 SEO metadata는 typed
  catalog로 분리하고 장문 route 본문도 같은 locale model을 사용해 확장할 수 있어야 한다. Catalog
  조회, interpolation과 plural 처리는 `i18next`와 `react-i18next`에 위임하고 직접 문자열을 치환하거나
  번역 key를 동적으로 조립하지 않는다.
- 선택한 locale은 `kfind-document-locale` cookie에 site 전체 path로 보존한다. 명시적인 `hl=en`
  query는 cookie보다 우선한다. Query가 없는 URL의 hydration은 한국어 SSG HTML로 시작한 뒤 browser
  cookie가 지원 값이면 같은 URL에서 해당 locale을 적용한다. Cookie가 없거나 지원하지 않는 값이면
  한국어를 유지하며 `Accept-Language`에 따른 자동 redirect는 하지 않는다. Cookie 감지와 보존은
  i18next language detector에 위임한다. Locale cookie는 UI preference일 뿐 인증·권한 판단에
  사용하지 않는다. 사용자가 locale을 선택하면 cookie를 먼저 갱신한 뒤 현재 문서와 fragment를
  유지한 채 UI와 URL을 전환하며, locale 동기화가 이전 cookie 값으로 선택을 되돌리지 않아야 한다.
- 한국어와 영어 public URL은 각각 self-canonical을 사용하고 양방향 `hreflang="ko"`,
  `hreflang="en"`과 query 없는 한국어 URL을 가리키는 `hreflang="x-default"`를 제공한다. Pages
  Function은 일반 browser와 crawler를 구분하지 않고 `?hl=en` 요청에 영어 prerender HTML을 `200`으로
  제공한다. Browser를 다른 URL로 redirect하거나 user agent에 따라 다른 문서를 제공하지 않는다.
- 한국어 문서의 헤딩은 설명 대상을 나타내는 명사구로 작성한다. 완결된 문장, 홍보 문구와
  행동을 권하는 문장을 헤딩으로 사용하지 않는다. 영어 문서의 헤딩은 영어 독자에게 자연스러운
  문형을 사용하며 명사구로 제한하지 않는다.
- 좁은 화면의 문서 navigation은 두 열 grid로 배치한다. 각 navigation group은 링크 수와 관계없이
  내용 높이를 유지하며, 같은 grid 행의 다른 group 높이에 맞춰 내부 link를 늘리지 않는다.
- 옵션 문서는 `inflection`, `derivation`, `literal`의 생성 범위와 차이, `--literal` 단축 옵션의
  충돌 규칙, boundary·POS·Unicode normalization·phrase gap의 결합을 예제와 함께 설명한다.
  분석·아키텍처·최적화 문서는 query compile부터 anchor scan, 국소 구조 판정, span·provenance
  반환까지의 흐름과 corpus 전체를 분석하지 않는 이유를 텍스트와 접근 가능한 도해로 설명한다.
- playground는 현재 source의 `kfind-wasm`을 browser용 WebAssembly로 빌드해 embedded lexicon으로
  실행한다. Query, 입력 text, expand·boundary·max gap을 바꿀 수 있고, UTF-16 span에 맞춰
  match를 강조하며 surface와 provenance를 표시한다. 명시적 품사는 별도 전역 control이 아니라
  query atom의 `n:`·`v:`·`adj:`·`lit:` 태그로만 입력하며 태그가 없는 atom은 자동 판정한다.
  Browser 사용자가 Unicode normalization을 선택하지 않도록 canonical NFC+NFD 검색을 고정
  적용한다. 문서 제목은 현재 locale의
  `플레이그라운드` 또는 `Playground`이며, 같은 이름의 하위 heading을 반복하지 않는다.
- Query와 입력 text는 검색 작업의 주 입력으로서 playground 상단의 한 input stack에 이 순서로
  인접 배치한다. 넓은 화면은 짧은 Query와 장문 text editor를 왼쪽 main pane에 두고 예시 action,
  compile option을 오른쪽 보조 panel에 둔다. 형태 구성 요소 판정 resource는 주 입력 아래, 검색
  결과 앞의 compact한 전체 너비 capability card에 둔다. Query control과 text editor는 모두
  main pane의 전체 너비를 채운다.
- 좁은 화면은 Query → text → 형태 구성 요소 판정 resource → 검색 옵션 → 결과의 인지 순서를
  우선한다. Resource card는 설정 modal 안에 숨기지 않고 역할, 크기와 현재 상태를
  항상 표시한다. Resource 사용 여부는 켜짐·꺼짐 switch로 제어한다. Switch를 켜면 resource를
  복원하거나 내려받아 기존 WASM engine에 load하고, browser 저장소에 호환되는 resource가 있으면
  playground 진입 시 복원한 뒤 켜짐을 기본값으로 표시한다. Switch를 끄면 이후 검색은 resource가
  없는 engine으로 실행한다. 예시 action과 compile option은 현재 주요 option 요약을 표시하는
  `검색 옵션` button으로 여는 modal 안에 두며 결과보다 앞에서 긴 설정 목록을 펼치지 않는다.
  Modal은 keyboard focus trap, touch scroll lock, 명시적인 닫기 control을 제공한다. 닫기 control은
  접근 가능한 label을 가진 borderless X icon button으로 표시하고, modal 안 select의 option
  popup은 modal 위에서 현재 viewport 안에 보여야 한다. Trigger 아래에 여는 select popup의 x축
  시작점은 trigger의 시작점에 맞춘다. 모든 화면에서 가로 scroll을 만들지 않는다.
- 검색 예시는 query, text와 관련 compile option을 하나의 설정으로 불러오는 action button으로
  제공한다. 예시 action과 개별 option control은 같은 input state를 갱신하고, 별도의 preset 선택
  상태를 유지하지 않는다. 기본 용언 활용 예시는 `data/fixtures/walk_hang_stress.txt`의 `걷다`와
  `걸다` 동형 활용, 합성어와 동음이의어가 섞인 회귀 문단을 `걷다`로 검색한다. 대용량 예시는
  `wikimedia/wikipedia`의 고정 `20231101.ko` snapshot 앞 500행에서 corpus revision과 source ID의
  SHA-256 hash를 기준으로 4개 bucket 중 0번에 속한 문서를 원본 순서대로 추출해 한국어 위키백과
  본문을 정확히 1 MiB로 제공한다. 원문 asset은 대용량 예시를 선택할 때만 불러오며 각 문서의 제목과
  URL, corpus revision, 추출 방법, checksum과 `CC BY-SA 3.0` 출처를 보존한다. 서로 다른 문서에
  분산된 `말하다` 활용형을 `verb + smart + inflection`으로 검색해 복수의 실제 corpus 결과를
  제공한다. Editor에는 문자 수와 UTF-8 byte 수를, 검색 결과에는 query compile과 전체 text scan을
  합친 실행 시간을 표시한다. 예시 action은 짧은 button row로 줄바꿈하며 단순 목록을 별도 card
  grid처럼 크게 그리지 않는다. Compile option은 현재 값과 설명을 확인할 수 있되 주 입력보다
  시각적으로 앞서지 않는 compact control grid로 배치한다. Site build는 추출 manifest의 byte
  수와 SHA-256으로 원문 asset을 검증한 뒤 같은 검증값을 browser loader에 주입한다.
- Playground는 query·text·option 변경을 debounce한 뒤 자동으로 검색하며 별도의 검색 실행
  button을 두지 않는다. Query label에서 지원 atom 태그와 품사를 확인할 수 있어야 하며 atom 태그
  도움말은 hover·keyboard focus와 pointer activation으로 열 수 있어야 한다. Playground가
  compile할 때 전역 POS는 항상 `auto`이며 명시적 품사는 atom 태그로만 전달한다. 검색 예시도
  명시적 품사가 필요하면 `v:걷다`, `lit:걸어`, `v:말하다`처럼 query에 태그를 포함한다. Expand
  control은 각 값의 생성 범위를 현재 선택값과 option list에서 설명한다.
- 입력 text는 CodeMirror 기반 plain-text editor에서 수정한다. 검색 span은 UTF-16 document offset을
  사용하는 decoration으로 실제 편집 text에 표시하며 별도의 highlight layer나 결과 preview를 중복해
  두지 않는다. IME composition 상태가 아닐 때 물리·소프트 키보드의 Enter와 Shift+Enter는 editor
  document에 줄바꿈을 삽입한다. Editor와 query control은 IME composition 중 search state를 갱신하지
  않고 composition이 끝난 값만 반영한다. 외부 preset 적용 외에는 editor document를 다시 쓰지 않아
  selection, caret과 undo history를 보존하고, 대용량 입력은 현재 viewport 중심으로 렌더링한다.
  Rich-text document model과 collaboration 기능은 추가하지 않는다.
- 결과 panel은 `Matches`와 `Raw JSON` tab을 제공하고 한 번에 선택한 detail만 표시한다. 기본 tab은
  사람이 읽는 surface·span·provenance 목록이며 Raw JSON은 같은 match의 전체 구조를 표시한다.
  Match 목록은 surface, span과 설명을 빠르게 훑을 수 있는 compact row로 표시하고 각 항목을 독립된
  큰 card로 확장하지 않는다. Match row의 keyboard focus ring은 scroll container에 잘리지 않도록 row
  안쪽에 표시하고, ring이 내용을 가리지 않도록 내부 여백과 둥근 모서리를 적용한다. 설명 첫 줄은
  이미 별도 표시한 match surface를 반복하지 않고 각 생성 origin의
  표제어와 생성 규칙을 `걷다 + ㄷ→ㄹ + -았/었- + -다`처럼 한국어 사전의 형태 분석 순서로 표시한다.
  생성 규칙이 없는 literal origin은 surface 대신 직접 일치임을 표시한다. 둘째 줄은 기존 provenance
  rule path를 그대로 표시한다. 좁은 화면에서는 두 줄 설명만 다음 행으로 내려 정보 순서를 보존한다.
  Match 목록은 현재 scroll viewport와 인접한 소수의 row만 DOM에 rendering하고 실제 row 높이를
  측정해 전체 scroll range를 보존한다. Match row를 활성화하면 해당 UTF-16 span을 editor에서 선택하고
  editor 내부 scroll과 문서 viewport를 그 위치로 이동한다. 반대로 editor의 match highlight를 pointer로
  활성화하면 `Matches` tab을 열고 아직 rendering되지 않은 row도 해당 index로 scroll하되 match row로
  keyboard focus를 옮기지 않는다. 입력 text가 바뀌면 새 검색 결과를 표시할 때 Match 목록의
  scroll을 처음으로 되돌린다.
- Playground 입력은 browser 밖으로 보내지 않는다. Full POS와 35.4 MiB의 형태 구성 요소 판정
  resource는 기본 demo에 포함하지 않는다. 이 compact index는 `smart` 경계 판정에서 원문 token
  내부의 같은 품사 component span과 인접 token 구조만 확인하며, full POS 사전처럼 문장 전체를
  분석하거나 검색어를 확장하지 않는다. 사용자가 switch를 켤 때 같은 origin의 Pages Function에서
  resource를 한 번 내려받아 기존 WASM engine에 load한다. 검증된 resource response는 browser
  Cache Storage에 보관하고 호환되는 resource revision으로 playground에 다시 들어오면 network 요청
  없이 자동으로 복원해 사용을 켠다. Cache key는 생성한 component artifact의 고정 SHA-256을
  사용한다. Resource byte가 같으면 release tag, Git commit과 working tree 상태가 달라도 key를
  바꾸지 않고, resource byte가 바뀌면 build script와 site build가 함께 읽는 checksum을 갱신한다.
  Playground 진입 시 현재 key를 먼저 확인하고, 기존 site build key로 저장한 같은-origin entry도 engine의
  schema·version·digest 검증을 통과하면 현재 key로 옮긴다. 호환되지 않는 entry는 삭제한다. 검색은 이
  확인이 끝난 뒤 시작하며 resource row는 확인 중 상태와 저장소 복원 완료 상태를 구분해 처음부터
  표시한다.
- Component resource는 25 MiB 단일 값 제한이 있는 Workers KV가 아니라 `kfind-assets` R2 bucket에
  둔다. Pages Function은 `KFIND_ASSETS` binding으로 고정 object를 읽어 body를 buffering하지 않고
  stream하며 content type, ETag와 cache header를 보존한다. R2 object가 없거나 손상되면 embedded
  preview로 조용히 fallback하지 않고 playground에 오류를 표시한다. 이 R2 경로는 kfind site의
  배포 방식이며 npm 소비자의 필수 호스팅 경로가 아니다.
- `site` package는 현재 source의 WASM과 version control에 보존한 승인 benchmark snapshot에서
  D3 기반 chart를 렌더링해 prerender HTML과 정적 asset이 있는 `build/client`를 만든다. Snapshot은
  source report의 revision과 SHA-256을
  기록하며, 승인된 benchmark가 바뀌면 같은 변경에서 갱신한다. 형태 품질은 수동 검토를 통과한
  표준 맞춤법 canonical과 실제 오류 문장만 남긴 Robust를 별도 section과 chart로 표시한다.
  Query matrix chart의 인접 본문은 한 source 문장에서 여러 positive query와 같은 품사의 paired
  negative query를 만드는 fixture 구성, 질의 단위 집계와 canonical 회귀선과 분리된 진단 범위를
  설명한다. Contract-adjusted confusion matrix는 raw 약어 오른쪽 위에 `c`를 붙인
  `TPᶜ`·`FPᶜ`·`TNᶜ`·`FNᶜ`로 표기한다.
  Robust chart는 동일한 gold fixture에서 backend별 precision·recall·F1과 실행 비용을 비교하고,
  오류 class, positive/negative 분모, robustness 설정과 표준문 품질에 합산하지 않는다는 점을
  chart subtitle과 인접 본문에 명시한다. 모든 품질 chart는 제품과 외부 분석기의 raw와
  contract-adjusted precision·recall·F1을 함께 표시한다. Contract review가 없는 fixture는 두 값이
  같으며 review 0건임을 표시한다. Robust 500-case는 positive 250, negative 250으로
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
- Publish workflow는 선택한 GitHub Release의 고정 checksum formula를 `TAP_GITHUB_TOKEN`으로 tap
  branch와 PR에 반영한다. Tap CI가 모두 통과하고 macOS arm64 bottle artifact가 생성됐음을 확인한
  뒤에만 `pr-pull` label을 적용한다. `brew pr-pull` 성공과 tap `main`의 해당 version·bottle 반영을
  확인해야 Homebrew 게시를 완료한 것으로 본다.
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

### 0.8 npm 패키지

- npm package 이름은 public organization-scoped `@kfind/kfind`다. prerelease는
  `npm install @kfind/kfind@next`, 고정 버전은 `npm install @kfind/kfind@1.1.0`로
  설치한다. `wasm-pack`의 `bundler` target으로 browser bundler용 ESM JavaScript glue,
  WASM binary와 TypeScript declaration을 생성한다.
- package의 `bin`은 `kfind` 이름으로 Node.js CLI를 제공한다. Node.js 20 이상에서
  `npx @kfind/kfind QUERY [PATH ...]`와 로컬 설치 뒤 `kfind QUERY [PATH ...]`를 지원한다.
  이를 위해 게시 산출물에는 `wasm-pack`의 `nodejs` target도 별도 디렉터리에 포함한다.
  package export는 Node.js에서 이 target을, browser bundler에서 bundler target을 선택하며
  두 target은 같은 Rust source와 공개 JavaScript API에서 생성한다.
- npm CLI는 query, path와 `--expand`, `--boundary`, `--pos`, `--normalization`, `--max-gap`,
  `--literal`, `--json`을 받는다. path가 없고 stdin이 TTY면 현재 디렉터리를, stdin이 pipe면
  stdin을 검색한다. 디렉터리는 결정적인 경로 순서로 재귀 순회하며 `.git`, `node_modules`,
  `target`과 site build 산출물은 기본 제외한다. symlink는 따라가지 않는다. UTF-8 text만
  검색하고 NUL이 있거나 UTF-8 decode에 실패한 파일은 진단 뒤 건너뛴다.
- npm CLI query는 native CLI와 같은 [6절](query-language.md) 문법을 사용한다. 도움말은 공백 phrase와 `|`
  disjunction을 구분하고, shell이 `|`를 pipe로 해석하지 않도록 query 전체를 따옴표로 묶는
  예시를 제공한다.
- npm CLI는 package의 enriched predicate를 초기화하고, compiled query가 component 구조를
  요구할 때 package의 compact component asset을 읽어 같은 query를 다시 compile한다. full POS는
  package에 포함하지 않으므로 native CLI의 full 사전 profile이 필요한 검색은 Homebrew 또는
  source build로 설치한 native CLI를 사용한다. npm binding API 자체는 resource 위치를 추정하지
  않는 계약을 유지한다.
- 기본 text 출력은 `path:line:column:surface`이며 line과 column은 1부터 시작하는 UTF-16 좌표다.
  `--json`은 match마다 path, line, column, start, end, surface와 provenance를 담은 JSON object 한
  줄을 출력한다. match가 있으면 0, 없으면 1, 사용법·초기화·I/O 오류면 2로 종료한다.
- compact component artifact는 `assets/morphology-component-compact.kfc`, enriched predicate TSV는
  `assets/predicates.enriched.tsv` 정적 파일로 WASM 산출물과 분리해 게시한다. 각 외부 데이터의
  license notice도 package에 포함한다. 사용자는 필요한 파일을 배포물에 복사하거나 별도 호스트에
  올릴 수 있으며 npm binding은 특정 호스팅 URL을 고정하지 않는다. `@kfind/kfind/assets` export는
  설치된 package와 정확히 같은 버전의 두 asset을 `new URL(relative, import.meta.url)`로 가리킨다.
  Node.js에서는 설치 package의 `file:` URL을 제공하고, 이 구문을 지원하는 browser bundler에서는
  content hash가 붙은 same-origin 정적 asset URL로 변환된다. 이 resolver module은 browser에서
  자동 fetch하거나 서버 route를 정하지 않으며, browser binding에는 caller가 해당 URL에서 읽은
  bytes를 명시적으로 전달한다. 실제로 pack한 tarball을 임시 소비자 project에 설치하고 Node.js
  서버에서 component asset 전체를 HTTP streaming하며, Vite SPA에서 두 asset을 정적 파일로
  bundling한 뒤 HTTP streaming하는 검증을 `pack:check`에 포함한다. full POS binary는 크기와 배포
  profile이 다르므로 npm package에 포함하지 않지만 같은 `withResources` 입력으로 전달할 수 있다.
- 자체 서빙 문서는 compact KFC와 enriched predicate TSV의 서로 다른 역할, resolver export와 raw
  asset subpath, SPA·Node.js 예제, HTTP header와 cache 정책, exact package version 호환성 실패를
  설명한다. Content hash 또는 package version이 URL에 포함된 asset만 장기 `immutable`로 캐시하고,
  고정 URL은 revalidation을 사용한다. 별도 origin에서 서빙하면 application origin을 명시한 CORS를
  제공한다.
- package build는 고정 source와 checksum으로 정적 asset을 생성한다. `npm pack --dry-run`은
  asset 포함과 SHA-256을 검증하고 WASM binary에 compact container magic 또는 artifact bytes가
  포함되지 않았음을 확인한다.
- npm `prepack`은 같은 checkout의 Cargo/package version을 확인하고 component를 다시 생성한 뒤
  Node smoke·TypeScript·asset 검증을 통과해야만 pack/publish를 허용한다. Publish workflow는 선택한
  GitHub Release tag를 checkout하고 이 검증이 끝난 동일 산출물을 npm registry에 게시한다.
  Prerelease version은 `next`, stable version은 `latest` dist-tag를 사용하며 prerelease를
  `latest`에 연결하지 않는다.
- Publish workflow는 npm package의 GitHub Actions trusted publisher에 등록된 `publish.yml`과
  GitHub OIDC로 게시한다. `@kfind/kfind` package 설정에는 owner `SeokminHong`, repository
  `kfind`, workflow `publish.yml`, `npm publish` 허용을 등록한다. Workflow는 장기 npm publish
  token을 사용하지 않는다. 새 version의 `npm publish --tag`로 dist-tag를 설정하고, 이미 게시된
  version의 재실행에서는 package checksum과 기대 dist-tag를 읽기 전용으로 검증한다.
- npm 산출물은 browser bundler와 Node.js용 release package로 생성한다. Node target은 같은 공개
  API와 실제 `bin` 실행을 smoke test하고 `npm pack --dry-run`으로 게시 파일, executable mode와
  metadata를 검증한다.
- npm package 검증은 package version과 Cargo version의 일치, 두 정적 asset과 license notice,
  `@kfind/kfind/assets`의 설치 package file URL과 HTTP streaming, Vite SPA의 content-hashed asset
  bundling과 HTTP streaming, TypeScript declaration의 optional resource bundle, enriched 분석
  활성화 여부, resource 없는 non-component compile, resource 없는 component smart 오류,
  JavaScript 초기화 오류, component positive/crossing negative와 UTF-16 offset 계약을 확인한다.
- 기본 CI는 npm package build, Node smoke test와 pack 검사를 실행한다.

## 17. Rust 기술 스택

| 목적           | crate                                        |
| -------------- | -------------------------------------------- |
| CLI            | `clap`, `clap_complete`, `clap_mangen`       |
| 파일 순회      | `ignore`                                     |
| 검색 I/O       | `grep-searcher`, `grep-matcher`              |
| 단일 앵커      | `memchr::memmem`                             |
| 다중 앵커      | `aho-corasick`                               |
| 바이트 문자열  | `bstr`                                       |
| Unicode 정규화 | `unicode-normalization`                      |
| 인코딩         | `encoding_rs` 또는 `grep-searcher` 연동 계층 |
| 출력           | `grep-printer`, `serde`, `serde_json`        |
| 오류           | `thiserror`                                  |
| 병렬 결과 채널 | `crossbeam-channel`                          |
| 작은 벡터      | `smallvec` 선택                              |
| 벤치마크       | `criterion`                                  |
| 속성 테스트    | `proptest`                                   |
| fuzz           | `cargo-fuzz`                                 |

`ignore::WalkParallel`이 파일 단위 병렬 처리를 담당하므로 별도 `rayon` 의존성은 기본 구조에 필요하지 않다.

`memmap2`를 직접 다루기보다 `grep-searcher`의 mmap 정책을 우선 사용한다.

## 21. Native package 배포

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
설치 후 `post_install_steps`에서 `kfind --check-data --data-dir {{pkgshare}}`를 실행한다.
실행 파일과 사전 리소스 검증에 실패하면 설치 후 검증도 실패로 처리한다.

### 21.3 Homebrew bottle 배포

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

### 21.4 Chocolatey 배포

Chocolatey package ID는 `kfind`다. x64 Windows용 portable ZIP을 tagged GitHub Release에
`kfind-windows-x86_64-VERSION.zip` 이름으로 게시한다.
`kfind.exe`는 MSVC C runtime을 정적 링크해 별도 Visual C++ Redistributable 설치 없이
실행되어야 한다. CI와 archive 생성은 PE import table에 동적 MSVC·UCRT 의존성이 없는지 검사한다.

```text
bin/kfind.exe
share/kfind/lexicon.bin
share/kfind/morphology-component-compact.kfc
share/kfind/predicates.enriched.tsv
share/kfind/*MANIFEST.toml
share/doc/kfind/LICENSES/
```

실행 파일은 `bin`의 부모를 prefix로 보고 `share/kfind`의 full POS lexicon, enriched
predicate와 compact component resource를 자동 탐색한다. Archive 생성 시
`kfind.exe --check-data --json --data-dir share/kfind`를 실행해 binary와 resource version,
schema와 source digest를 검증한다.

Chocolatey package는 version tag의 immutable archive URL과 SHA-256을
`Install-ChocolateyZipPackage`에 전달한다. 압축을 package의 `tools` 아래에 풀고
`bin/kfind.exe`의 자동 shim을 사용한다. 별도 system directory, registry와 환경 변수는
수정하지 않으며 uninstall은 package directory와 shim 제거만으로 끝난다.

Chocolatey Community Repository가 SemVer 2 prerelease를 지원하지 않으므로 stable release는
release version을 그대로 package version으로 쓰고, `MAJOR.MINOR.PATCH-rc.N` release는 N을 최소
네 자리로 zero-padding한 `MAJOR.MINOR.PATCH-rcNNNN` package version으로 매핑한다. 예를 들어
`1.1.0`의 Chocolatey package version은 `1.1.0-rc0004`다. Package filename과 registry
조회에는 package version을 사용하지만 archive URL, 실행 파일과 resource version 검증에는 release
version을 사용한다.

Release workflow는 같은 archive checksum으로 `kfind.PACKAGE_VERSION.nupkg`를 만들고 GitHub
Release에 첨부한다. Publish workflow의 Chocolatey job은 mapped package asset을 재사용하며, 과거
release에 asset이 없으면 immutable Windows archive와 현재 packaging template으로 만들고 release에
첨부한다. 이후 local install, `--version`과 `--check-data --json`을 검증한 뒤
`https://push.chocolatey.org/`로 전송한다.
Push가 일시적으로 실패하면 exact version의 공개 package를 다시 확인하고 release asset과
checksum이 같으면 성공으로 처리하며, 아직 게시되지 않았으면 제한된 횟수로 재시도한다.
Repository secret `CHOCOLATEY_API_KEY`가 없으면 게시 성공으로 처리하지 않고 실패한다.

### 21.5 릴리스 자동화

Release와 Publish는 GitHub Actions의 수동 workflow로 분리한다.
모든 외부 GitHub Action은 전체 commit SHA에 고정하며 CI가 workflow의 action ref를 검사한다.

Release workflow는 `main`에서만 실행하며 `major`, `minor`, `patch` 중 bump 종류와 prerelease 여부를
입력받는다. Bump 기준은 저장소의 최신 stable `vMAJOR.MINOR.PATCH` tag다. Stable 입력은 선택한
SemVer component를 올린 version을 만들고, prerelease 입력은 같은 core version의 기존
`vVERSION-rc.N` tag에서 가장 큰 N 다음 번호를 붙이며 없으면 `rc.1`부터 시작한다. 예를 들어 최신
stable이 `0.2.1`이고 `v1.1.0`이 있으면 `major + stable`은 `1.1.0`,
`major + prerelease`는 `1.1.0`다.

Release workflow는 계산한 version을 workspace package, lockfile, npm·site metadata, 현재 버전을
보여 주는 문서와 component resource header·checksum에 동기화한다. 고정 Rust toolchain의 source,
resource, npm과 site 검증을 통과한 변경만 release commit으로 `main`에 반영한다. Windows archive와
Chocolatey package, source, full POS, component, CLI asset과 Homebrew formula를 모두 만든 뒤 해당
commit에 annotated tag를 붙이고 GitHub Release를 생성한다. RC는 GitHub prerelease로 표시한다.
Workspace와 도구 lockfile 동기화는 component resource 생성을 포함한 모든 `--locked` 검증보다
먼저 완료하며, 기존 외부 dependency version은 갱신하지 않는다.
Cargo manifest의 버전 변경은 `[workspace.package].version`과 독립 benchmark runner의
`[package].version`에 한정한다. 같은 값이나 접두부를 가진 dependency requirement는 보존한다.
`main`의 pull request 보호 규칙을 우회하지 않는다. Version bump는 실행별 release branch에
commit하고 release PR을 만든 뒤 같은 commit의 필수 CI를 명시적으로 실행하여 squash merge한다.
이후 asset과 tag는 merge commit만 참조한다.
이 workflow는 npm, Homebrew와 Chocolatey registry에는 게시하지 않는다. 동일 workflow의 동시
실행을 직렬화하고, tag나 GitHub Release 생성 전 실패한 같은 version은 다음 실행에서 이어서
검증할 수 있어야 한다.

Publish workflow는 `v` prefix 유무와 관계없이 특정 version을 입력받고, 해당 annotated tag와
draft가 아닌 GitHub Release가 존재하며 prerelease 표시가 version suffix와 일치하는지 먼저
검증한다. 검증된 exact tag source와 release asset만 npm, Homebrew, Chocolatey와 versioned 문서
게시에 사용한다. Channel job은 서로 독립적으로 실행하되 동일 version의 이미 게시한 immutable
artifact가 같으면 재사용하고, 내용이 다르면 덮어쓰지 않고 실패한다.

Stable npm package는 `latest`, RC package는 `next` dist-tag로 게시한다. RC 게시 시 기존
`latest`는 변경하지 않으며, 게시한 RC version이 `latest`를 가리키지 않는지 확인한다. 게시한
package가 public registry에 나타날 때까지 제한된 횟수로 재조회하고 local tarball과 registry의
`dist.shasum`이 같은지 확인한다. 이후 격리된 임시 directory에서 `npx`, `pnpm dlx`와 고정 Yarn
version의 `dlx`로 실행해 각 command의 성공 종료와 마지막 version output을 검증한다. Homebrew는
formula PR의 전체 test-bot과 bottle artifact를 확인한 뒤 `pr-pull`을 실행하고 tap 반영까지 기다린다.
Chocolatey는 공개 push까지 성공해야 한다.
Versioned 문서는 같은 Publish 실행에서 GitHub Release asset과 R2에 올리고 manifest를 갱신한다.
