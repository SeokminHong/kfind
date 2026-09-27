# 형태 규칙과 검색 실행

이 문서는 [kfind 기술 사양서](kfind.md)의 현행 규범적 계약이다.

## 0. 제품 계약

이 절은 뒤의 일반 설명보다 우선한다.

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
- 현재 token graph의 여러 구조 기능이 함께 사용하는 nominal prefix, ending suffix와
  predicate-connective 경계는 token 준비 단계에서 한 번 계산한다. Attached auxiliary,
  compound predicate, nominal derivation과 copula 판정은 같은 도달성 사실을 공유하며 기능마다
  전체 edge graph를 다시 순회해 같은 상태 배열을 만들지 않는다.
- Resource loader는 검증된 POS string table을 compact typed sequence table로 한 번 변환한다.
  Token graph edge는 resource가 소유한 sequence slice를 빌려 쓰고 구조 상태기계는 raw 문자열을
  기능마다 다시 분리하거나 세부 품사를 반복 해석하지 않는다. Token 준비 경로에는 POS interning
  cache나 별도 sequence arena를 두지 않는다. Component span이 필요 없는 구조 판정은 POS-only
  resource view를 순회하며 analysis·component `Vec`를 만들지 않는다.
- 구조 근거 수집은 corpus graph 구성·경로 선택과 분리된 모듈에서 수행한다. Source 근거,
  명사형 활용 anchor 근거, runtime 복합 구조와 완결 span 근거의 우선순위를 보존하며
  같은 candidate의 지원 근거와 pattern index를 유지한다.
- 구조 판정의 내부 경계는 resource 조회, graph 인덱스, token 근거 준비, 구조별 경로 사실,
  문맥의 구조 선택과 candidate 수용으로 나눈다. Graph 계층은 query pattern이나 수용 정책에
  의존하지 않으며, 경로 사실은 준비 단계에서 계산해 선택·수용에 전달한다. 공개 resolver API,
  근거 우선순위, node 상한과 판정 결과는 이 내부 분리와 무관하게 유지한다.
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
- 구조 context의 window 추출·graph 준비·좌표 정렬 실패 또는 `Unavailable` 판정은
  후보를 제외하면서 `SearchDiagnostics`의 `structural_verification_incomplete`를 설정한다.
  이 진단은 호출자가 소유하는 검색 단위 상태이며 다른 입력·worker와 암묵적으로 공유하거나
  초기화하지 않는다. 같은 상태에 여러 검색을 누적하면 하나라도 판정 불가일 때 참을 유지한다.
  기존 matcher API는 결과 계약을 유지하고 진단을 받는 API를 추가한다.
- 네이티브 파일 검색은 파일마다 별도 진단을 사용하고 `FileSearchResult`에 위 상태를 보존한다.
  `SearchSummary`는 판정 불가가 발생한 파일 수를 집계한다. 결과가 없는 파일과 문맥·집계
  출력에서도 진단은 유지한다. 이 값이 거짓이어도 지원하지 않는 활용·의미 검색, 조기 종료나
  검색 대상 밖의 파일까지 포함한 완전성을 보장하지 않는다. 판정 불가 후보 수는 제공하지 않는다.
- 네이티브 CLI는 구조 판정 불가가 관측된 파일 경로와 `structural_verification_incomplete`
  진단을 stderr에 출력하고 종료 코드 2를 반환한다. 이미 찾은 stdout 결과는 보존한다.
  JSON Lines에 진단 record를 섞지 않으며, `--count`, `--files-with-matches`, `--quiet`에도
  같은 종료 계약을 적용한다. 조기 종료 뒤의 미검색 구간에 대한 판정은 추측하지 않는다.
- Window의 기본 제한은 원문 256 byte와 정규화 후 64 Unicode scalar이며 현재 token과
  필요한 인접 문맥에 적용한다. 입력 파일 전체의 길이 제한이 아니다.
- 구조 준비는 현재 token 자체에서 얻는 형태 graph와 앞뒤 token에 따른 구조 선택을 별도
  단계로 유지한다. Matcher는 전체 program이 8개 이하인 작은 plan에서 structural program의
  정규화된 anchor를 현재 token 후보로 최대 64개까지 matcher memory 상한 안에서 등록한다.
  Corpus candidate의 정규화된 현재 token이
  등록 anchor와 정확히 같을 때 graph를 최초 1회 생성해 matcher 수명 동안 재사용하며, 검색되지
  않은 anchor의 graph는 만들지 않는다. 동시 최초 접근도 하나의 graph만 게시하고 실제 graph
  메모리를 matcher 상한에서 원자적으로 예약한다. 앞뒤 token에 따른 선택과 원문·NFC span
  역매핑은 candidate마다 실행한다. 큰 plan, 다른 현재 token, 등록 개수·메모리 상한 초과와
  graph 생성 실패는 기존 bounded candidate 준비 경로를 사용하며 결과 판정은 바꾸지 않는다.
- 구조적으로 다른 경쟁 path는 인접 성분 배치로 하나를 선택할 수 있는지 판정할
  때까지 평가한다. 이때 분해·품사·인접 제약이 같은 어휘 의미 후보는 추가로
  열거하지 않는다.
- query program이 `ending.aoeo`를 거쳐 만든 축약형 뒤의 문자열은 compact resource가
  `VX` 보조용언+어미의 완전한 연쇄로 증명할 때만 같은 predicate token으로 확장한다. 한 음절 안의
  축약은 anchor와 core의 byte 끝이 같을 수 있으므로 span 길이 차이를 별도 증명 조건으로
  요구하지 않는다.
- `nominal-copula-ending-compose` program은 규칙에 선언된 축약 표면을 anchor로 사용하고
  선택적 조사 연쇄를 token 끝까지 소비한다. `smart`는 anchor 전체와 정확히 일치하는 source
  분석이 `NP + VCP + E+` 순서이며 `EC` 또는 `EF`로 끝날 때만 후보를 유지한다. 융합 때문에
  component byte span을 만들 수 없는 source expression은 이 전체 품사열로 검증하고, query
  표제어나 결과 표면을 source component span으로 추정하지 않는다.
- `copula-host-ending-compose` program은 규칙에 선언된 체언 host·어미 축약 결과를 `smart`
  지정사 query의 anchor로 사용한다. Anchor가 맞은 뒤 exact source 분석이 하나 이상의 체언 품사,
  `VCP`, 하나 이상의 `E+` 순서이고 `EC` 또는 `EF`로 끝나며 정렬된 `VCP` component span은
  없을 때만 후보를 유지한다. 반환 span은 source에 없는 VCP 부분 span을 추정하지 않고 `걸까`처럼
  양의 span을 보존하는 축약 anchor 전체로 한다. `token`과 `any`, 미완결 어미, VCP 뒤의 비어미
  성분은 이 program을 만들지 않는다.
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
- `smart` 체언 query core가 token 왼쪽 경계부터 `N+ + XSN+`의 완성된 명사 파생 경로와
  정확히 일치하고, 그 직후부터 token 끝까지 `XSV/XSA + E+`의 완성된 용언 파생·어미 경로가
  이어지면 그 체언 core를 유지한다. 명사와 `XSN`은 각각 하나 이상이어야 하며 query core의
  양쪽 끝은 source node 경계와 일치해야 한다. 따라서 `잠식/NNG + 당/XSN + 하/XSV + 기/ETN`의
  `잠식당`을 회수하지만, token 내부에서 시작하거나 `XSN` component를 가로지르거나 용언
  파생 뒤 어미가 없는 후보는 열지 않는다. 더 낮은 비용의 `잠식/NNG + 당하/XSV + 기/ETN`
  경로가 경쟁해도 비용으로 정렬된 명사 파생 경로를 제거하지 않는다.
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
- 구조 판정은 candidate가 token 끝까지 직접 소비했거나, 아래에서 정의한 source ending,
  보조사, 의존명사, 지정사 또는 합성 용언 경로가 남은 suffix 전체를 소비한 경우에만 결과를
  유지한다. Candidate 앞부분과 같은 품사의 source node가 있다는 사실이나 token 일부를 덮는
  graph path는 남은 suffix의 허가 근거가 아니다. 체언 candidate도 완성된 체언 host와 조사·지정사
  경로가 token 끝까지 이어져야 하며, 내부 component가 선택된 선호 경로에 정확히 정렬되지 않으면
  유지하지 않는다.
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
  양쪽 경계를 가로지르는 후보에는 선호 경로 검증을 적용한다. 조사 host 전체를 덮는 source
  명사 분석이 내부 component를 선언하고 같은 span·품사의 독립 atomic 분석이 없으면 그 선언을
  선호 경로와 같은 근거로 사용한다. 따라서 `물/NNG + 줄기/NNG + 는/JX`의 `물`은 유지하지만,
  독립 `산길/NNG` 분석과 경쟁하는 별도 분해의 내부 `길`은 유지하지 않는다.
- 국립국어원 고정 사전에서 검토한 명사 결합 접미사 어휘는 후보를 다시 생성하지 않고 체언
  구조 검증에만 사용한다. 사전 generator는 고정 snapshot을 한 번 읽어 재사용 가능한 catalog
  candidate를 만들고, 별도 validator가 기본 사전 합의와 schema를 검사한 뒤 설치한다. 검증
  정책만 바뀌면 catalog를 다시 생성하지 않는다. 한 음절 보통명사 query가 이 어휘에 속하고 조사 host의
  마지막 source node와 정확히 일치하며, 그 앞의 체언 node부터 뒤의 조사 연쇄까지 token
  전체를 완성하고 host 전체를 덮는 단일 source edge가 없으면 선호 nominal path의
  component로 유지한다. 따라서 `책임/NNG + 하/NNG + 에서/JKB`의 `하`를 회수하지만,
  `빙원/NNG + 옆/NNG + 에/JKB`의 독립 명사, 완성 어휘의 내부 음절, 조사 node, particle을
  소비하지 않고 whole 체언 분석도 없는 내부 한 음절 후보는 열지 않는다. Whole 체언이 같은
  span을 source component로 함께 선언한 기존 exact component는 보존한다. 이 체언 검증은 용언
  program을 바꾸지 않으므로 `가다`는 `그래 네가 가.`에서 token 전체 명령형 `가`를 계속
  회수한다.
- token을 임의 품사의 edge로 끝까지 덮을 수 있다는 사실만으로 token보다 짧은 query 품사의
  runtime component를 합성하지 않는다. 특히 `NP`·`MM`·`MAG/MAJ` 내부 component는 query
  core와 같은 span의 같은 세부 품사 node가 완전한 typed path에 있어야 한다. 이 조건은 query가
  명시한 품사를 다른 체언·용언 edge의 span으로 대신 증명하지 못하게 하며, query 표면과 token
  전체가 같은 독립 후보에는 적용하지 않는다.
- whole 체언 분석과 더 짧은 runtime 체언+조사 분할이 경쟁할 때, 한 node짜리 내부 체언
  prefix는 whole 분석이 정렬해 선언한 source component이거나 실제 조사 host 전체인 경우에만
  유지한다. 두 node 이상의 복합명사 subpath와 `MM + 체언` 선호 경로는 각각 별도 typed
  규칙으로 검증한다. 독립 whole 체언의 첫 음절을 임의의 대명사·명사 component로 만들지
  않는다.
- 일반 용언 query의 runtime component는 token 왼쪽 경계에서 시작한 용언+어미 path 또는
  `용언 + EC + VX + 선택적 어미`의 보조용언 path에 속해야 한다. token 내부에서 우연히 같은
  세부 품사의 edge가 query core부터 token 끝까지 있다는 사실만으로 독립 용언 stem을 만들지
  않는다. 완성된 체언+조사 path와 경쟁하는 임의의 compound predicate path도 내부 용언 근거로
  사용하지 않는다. 보조용언 path와 token 전체의 `MAG/MAJ` 분석이 경쟁하면 통째 부사 분석을 선택한다.
  체언 뒤 `XSV/XSA + E*`가 완성된 파생 용언 path에서는 파생 접미사 시작 span의 runtime
  체언 후보도 source가 체언 component를 정렬해 선언하지 않은 한 거부한다.
- token 왼쪽 경계부터 `용언 + EP* + EC + 용언 + E* + J*` 순서로 끝까지 이어지는 source path가
  있고, 두 번째 용언 edge 또는 source component가 query core와 같은 span·세부 품사이며 query
  program도 그 위치부터 token 끝까지 continuation을 소비하면 내부 용언 component를 유지한다.
  첫 용언의 connective 경로는 `EP` 뒤의 단일 `EC`로만 끝나며 `ETM`·`ETN`·`EF` 뒤에 다른
  `EC`를 이어 붙이지 않는다. 따라서 완성 체언+주격 조사 `친구/NNG + 가/JKS`를
  `친/VV+ETM + 구/EC + 가/VV`로 다시 조합하지 않는다.
  token 전체의 독립 용언 분석이 경쟁해도 이 완전 경로를 가리지 않는다. 이 규칙은
  `올라가`의 `가다`, `생겨나`의 `나다`, `들어와서는`의 `오다`처럼 source가 정렬한 합성·보조
  용언 tail에 적용하며, 더 큰 node의 substring이나 token 앞뒤가 불완전한 runtime 분할은
  근거로 사용하지 않는다.
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
  이 규칙으로 predicate 후보를 제거하지 않는다. 다음 token 전체에 `NNB/NNBC` 분석이
  있으면 같은 표면의 `XSN/XR` 분석은 독립 어절 경쟁으로 보지 않는다. 따라서
  `몇/MM + 년/NNB|NNBC` 구조는 `년/XSN|XR` 동형 분석이 함께 있어도 관형사 구조로
  판정한다.
- 관형사 component와 같은 token 왼쪽 경계에서 시작하는 더 긴 체언 node가 있고, 그 체언
  뒤의 `XSV/XSA + E*`가 token 끝까지 이어지는 완전한 파생 용언 path를 만들면 내부 관형사
  component를 거부한다. 따라서 `전망해야`의 `전/MM + 망/NNG + 해야/XSV+EC` 경쟁 분석은
  `전망/NNG + 해야/XSV+EC`를 선택해 `전`을 관형사로 판정하지 않는다. 이 규칙은 독립 token
  전체의 관형사 whole 근거나 다음 token의 체언 host를 소비하는 관형사 구조에는 적용하지
  않는다.
- core modifier lexicon의 exact `MM` 분석은 embedded와 full POS profile에서 같은 관형사
  whole 근거로 사용한다. 따라서 `몇`은 `지난 몇 년 동안`에서 관형사로 검색하되,
  `몇몇`의 일부인 `몇`은 token whole 근거가 아니므로 검색하지 않는다.
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
  제품 matcher는 현재 token의 원문 slice를 직접 빌려 구조를 준비한다. Public 진단 API가
  독립 수명을 요구할 때만 소유 `AnalysisWindow`로 변환한다. NFC가 아닌 window만 bounded
  normalized 문자열과 prefix 안정 경계를 소유한다. 인접 token도 같은 borrowed-or-owned
  정규화 view를 사용한다.
- compact morphology resource는 schema 5 container다. NFC surface index, source node의
  POS, NFC 안정 경계에 정렬된 component span과 source identity만 보존한다. left/right
  context ID, word cost, 연결 비용 행렬, unknown model과 원본 expression 문자열은 싣지 않는다.
  loader는 검증된 string ID마다 구조 판정용 typed POS sequence를 compact code로 보유한다.
  국소 graph를 준비할 때는 resource의 POS 문자열, typed sequence와 component를 빌려 쓰며 token마다
  이를 다시 소유하거나 변환하지 않는다. POS-only prefix 순회와 component materialization 경로는
  분리해 suffix·인접 token 판정이 쓰지 않는 component를 decode하거나 할당하지 않는다.
  Token graph는 검증된 analysis record handle을 보존하고 component span을 resource iterator로
  순회한다. 공개 호환 API가 소유 `Vec`를 요구할 때만 component를 materialize하며 graph edge마다
  같은 record의 component 배열을 다시 할당하지 않는다.
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
lexicon rule에서 투영한 전체 rule vocabulary와 조사 allomorph·전이·host별 rule 집합은
analyzer를 만들 때 한 번 물질화하고 immutable `Arc`로 plan과 matcher가 공유한다. query
compile과 matcher build는 이 전역 rule 집합과 조사 graph를 다시 순회하거나 allomorph
문자열을 깊은 복제하지 않는다.
본용언과 보조용언처럼 활용 실행 class가 같은 분석은 anchor·continuation program을 공유하되,
각 program은 허용하는 source predicate 품사 집합을 별도로 보존한다. 따라서 `VV/VX`와
`VA/VX`의 중복 활용 program은 합칠 수 있지만 고정 형태소 자원의 exact path와 whole-token
충돌은 합쳐진 집합의 모든 source 품사를 검사해야 하며 `VV`, `VA`, `VX`를 서로 바꾸지 않는다.

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
두 기본 사전이 독립적으로 확인한 형용사 `-이` 부사형은 생산 규칙이 아니라 사전 표면형으로
취급하므로 기본 `inflection`에도 포함한다. 사전에 없는 `-이` 후보를 생산적으로 확장하지 않는다.
두 기본 사전이 원형·파생 동사를 모두 일반어로 등재하고 한국어기초사전의 구조화 관계가
`-이-/-히-/-리-/-기-` 피·사동 파생을 직접 가리키는 경우도 기본 `inflection`에 포함한다. 이는
모든 동사에 접미사를 생산적으로 붙이거나 피동·사동의 의미 역할을 추측하는 규칙이 아니라
사전이 확인한 voice lemma 관계다.

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

| 분류                  | 예                                       | 기대 표면형                                                                                        |
| --------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------- |
| 규칙 자음 어간        | 먹다                                     | 먹어, 먹었다, 먹는, 먹은, 먹을                                                                     |
| 규칙 모음 어간        | 가다                                     | 가, 갔다, 가는, 간, 갈                                                                             |
| ㅏ/ㅓ 축약            | 보다                                     | 보아, 봐, 보았다, 봤다                                                                             |
| ㅚ/ㅣ 계열 축약       | 되다                                     | 되어, 돼, 되었다, 됐다                                                                             |
| ㄷ 불규칙             | 걷다, 듣다, 싣다                         | 걸어, 들어, 실어, 걸음, 들음, 실음                                                                 |
| ㅅ 불규칙             | 짓다, 낫다, 잇다                         | 지어, 나아, 이어, 지음, 나음, 이음                                                                 |
| ㅂ 불규칙             | 돕다, 눕다, 아름답다                     | 도와, 누워, 아름다워, 도움, 누움, 아름다움                                                         |
| ㅎ 불규칙             | 파랗다, 그렇다, 어떻다, 이렇다, 커다랗다 | 파래, 파란, 그래, 그런, 어떤, 이런, 커다란, 파람, 그럼                                             |
| 르 불규칙             | 빠르다, 부르다, 모르다                   | 빨라, 불러, 몰라, 빠름, 부름, 모름                                                                 |
| 러 불규칙             | 푸르다, 이르다 일부                      | 푸르러, 푸름, 이름                                                                                 |
| ㅡ 탈락               | 쓰다, 크다, 예쁘다                       | 써, 커, 예뻐                                                                                       |
| 우 불규칙             | 푸다                                     | 퍼, 품                                                                                             |
| 하다                  | 하다, 검증하다                           | 하여, 해, 하였다, 했다, 함, 검증하여, 검증해, 검증하였다, 검증했다, 검증함                         |
| ㄹ 탈락               | 살다, 알다, 만들다                       | 사는, 압니다, 만듭니다, 삶, 앎, 만듦                                                               |
| 진행 방향 보조 용언   | 망하다, 만들다                           | 망해가고, 만들어가야                                                                               |
| 개별 보조 용언 명령형 | 말다, 달다                               | 마라, 다오                                                                                         |
| 아주낮춤 명령형       | 가다, 먹다, 걷다, 오다, 들어오다         | 가거라, 먹거라, 걷거라, 오거라, 오너라, 들어오너라                                                 |
| 회상·청유·의도·존대   | 걷다                                     | 걷던, 걷더니, 걷자, 걷자고, 걷곤, 걷느냐, 걷도록, 걸으려는, 걸으셨고, 걸으셨던, 걸으세요, 걸읍시다 |
| 과거 의문 종결        | 하다, 먹다                               | 했느냐는, 먹었느냐                                                                                 |
| 지정사                | 이다                                     | 이고, 이어, 여서, 인, 일, 임, 입니다, 이라고, 이라는, 이지, 이며                                   |
| 부정 지정사           | 아니다                                   | 아니고, 아니어서, 아니라, 아닌, 아닐                                                               |

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
`누구·무어·무엇 + 이(VCP) + -ㄴ가(EC/EF)`의 축약은
`kind=nominal-copula-ending-compose`의 표제어별 표면 대응으로 제한한다. `token`과 `any`는
선언된 표면과 선택적 조사 연쇄만 소비한다. `smart`는 축약 표면 전체에 `NP + VCP + E+`가
있고 마지막 품사가 `EC` 또는 `EF`인 source 분석을 추가로 요구한다. 별도 등재된 `누군가`와
`무언가`를 원 표제어의 사전 alias로 간주하거나, 같은 음운 모양을 임의의 대명사에 생산적으로
적용하지 않는다.
`것일까 → 걸까`처럼 `이다`의 어간 자체가 표면에서 소실된 축약은
`kind=copula-host-ending-compose`의 문법 대응으로 제한한다. `smart`는 선언된 축약 anchor가
맞은 뒤 exact source의 `체언 + VCP + EC/EF` 구조와 VCP component span 소실을 검증하며,
결과는 축약 anchor 전체다. 어절 전체를 무조건 지정사 candidate로 열거하지 않는다.
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

metadata가 필요 없는 검색에서 `grep_matcher::Matcher::find_at`은 다음 검증된 token span의 바이트
범위만 반환하며 origin과 rule path를 복제하거나 병합하지 않는다. metadata가 필요한 line-local
검색은 후보 줄의 일괄 평가 결과에서 첫 span을 `grep-searcher`에 반환하고, 같은 결과를 sink에 한 번
전달한다. 같은 줄의 anchor 탐색, atom span 수집과 phrase 선택을 sink에서 다시 실행하지 않는다.
LF를 포함하는 multi-line 계획은 줄 단위 전달을 사용할 수 없으므로 buffer match 뒤 metadata를
재계산한다.

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
Line-local phrase plan은 같은 물리적 줄에 모든 atom index의 raw anchor가 하나 이상 있을 때만
그 줄을 검증 후보로 전달한다. 이 단계는 형태·경계 의미를 확정하지 않는 false-positive 허용
prefilter이며, atom 하나라도 raw anchor가 없는 줄에서는 검증된 span 목록을 만들지 않는다.
Disjunction plan은 alternative anchor가 하나라도 있는 줄을 후보로 전달한다.

### 12.4 phrase 결합

검증된 span을 위치순 candidate stream으로 만들면서 순서 결합한다.

```text
anchor hits
  → token.start 순 candidate group
  → atom별 active prefix state
  → 순서 유지
  → max-gap 검사
  → settled leftmost-longest match
```

표면형 후보들의 데카르트 곱을 정규식으로 만들지 않는다.

제품 matcher는 가능한 atom 조합이나 줄 전체의 검증 span을 미리 만들지 않는다. Anchor hit의
끝 위치와 plan의 최대 anchor 길이를 이용해 더 이른 `token.start`가 나올 수 없는 candidate group만
순서대로 확정하고, 같은 atom의 동일 core·token span은 이 단계에서 병합한다. 각 atom layer는
다음 atom과 연결될 수 있는 max-gap 범위의 prefix state만 유지하며, 현재 non-overlap cursor
구간의 동일한 token end에서는 leftmost-longest tie-break상 우선하는 prefix 하나만 남긴다.
범위를 벗어나거나 줄을 건넌 state는 즉시 제거한다. 가장 이른 시작점의 연장 가능한 state가
없어졌을 때만 완성 match를 확정한다. 완성 match 때문에 cursor가 전진하면 이미 지나간 후보 중
새 cursor 이후에 시작하는 제한된 구간만 재생해, 이전 match와 겹쳐 우선순위에서 밀렸던 다음
prefix를 복구한다. 재생 구간의 시작은 대기 중인 가장 이른 완성 match의 최종 end를 따라
전진하며, 완성 match가 없으면 별도 후보 이력을 유지하지 않는다.

Active DP와 재생 이력 메모리는 줄 전체 candidate 수가 아니라 query atom 수, max-gap 안의 서로
다른 candidate endpoint 수, 대기 중인 match 이후의 제한된 phrase 도달 범위에 비례해야 한다.
사용자가 지정한 max-gap은 이 runtime 작업 범위에도 직접 반영된다.
`find_span_at`은 가장 이른 leftmost-longest 결과 하나만 복원하고, `find_all_with_meta`는 선택된
non-overlapping 결과의 atom metadata만 복원한다. match 하나를 반환할 때마다 남은 전체 입력의
anchor를 다시 검색하지 않는다. `InputSearcher`가 한 줄의 metadata를 수집하는 경로도 같은 stream을
사용하며, 65,536개 상한은 결과를 모두 만든 뒤가 아니라 선택 중에 적용한다.

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

정렬 옵션은 검색 전에 대상 경로와 탐색 오류만 수집해 path 순서를 확정한다. 정렬된 경로는 제한된 work queue로 worker에 분배하고, writer에는 각 경로의 bounded record stream을 path 순서로 먼저 전달한다. writer가 현재 경로를 비우는 동안 뒷 경로 worker는 per-file channel의 backpressure를 받는다. 따라서 결과 record 메모리는 전체 match 수가 아니라 worker 수와 channel capacity에 의해 제한되어야 하며, 경로 수집 메모리만 검색 대상 수에 비례할 수 있다. 전체 결과를 `Vec`에 모은 뒤 정렬하는 구현은 허용하지 않는다. 정렬 순서와 파일별 연속 block은 기존 계약과 같아야 하며, worker panic과 검색 오류도 해당 path 위치의 event로 변환한다. 경로 탐색을 끝내기 전에는 첫 결과를 출력하지 않으며, 경로 정렬과 병렬 worker 조정 비용으로 기본 unsorted stream보다 처리량이 낮아질 수 있음을 도움말에 명시한다.

broken pipe는 정상 종료로 처리한다.
