# 형태소 검색 개선 핸드오프

이 문서는 형태소 검색의 현재 제품 상태와 바로 이어갈 작업만 유지한다. 측정 과정과 완료한
작업 순서는 개별 benchmark 보고서에 둔다.

관련 문서:

- [기술 사양서](../../specs/kfind.md)
- [ㄷ·ㅅ·ㅂ·ㅎ 불규칙 enriched 용언 lexicon](2026-07-14-consonant-irregular-enriched-lexicon.md)
- [르·러 불규칙과 enriched 용언 lexicon](2026-07-14-reu-reo-enriched-lexicon.md)
- [User smart precision 품질·성능](2026-07-14-user-smart-precision.md)
- [Agent precision shadow 판정](2026-07-14-agent-precision-shadow.md)
- [`-기` 명사형 조사 continuation 품질·성능](2026-07-14-gi-particle-continuation.md)
- [`-ㅁ/음` 명사형 품질·성능](2026-07-14-mieum-nominalizer.md)
- [합성 불규칙 용언 core lexicon](2026-07-14-compound-irregular-core-lexicon.md)
- [`-거라`·`-너라` 아주낮춤 명령형](2026-07-14-geora-neora-imperatives.md)
- [국소 lattice 제품 경로 최적화](2026-07-14-local-lattice-optimization.md)
- [Development false negative 진단](2026-07-14-development-fn-diagnostics.md)
- [`ending.connective-ji` 위치 근거](2026-07-14-connective-ji-position-evidence.md)
- [명시적 품사 `-지` 오른쪽 끝 recall](2026-07-14-connective-ji-right-edge-recall.md)
- [ㅎ 불규칙 core lexicon recall](2026-07-14-h-irregular-recall.md)
- [의존명사 coarse-POS fallback recall](2026-07-14-dependent-noun-recall.md)
- [Full POS coarse noun 분석 합집합 recall](2026-07-14-full-pos-coarse-noun-recall.md)
- [`매일` 인접 문맥 판별 품질·성능](2026-07-14-contextual-maeil-disambiguation.md)
- [제한된 사전 표면형 계층](2026-07-15-dictionary-surface-lexicon.md)
- [선어말어미 뒤 `-으되` continuation](2026-07-15-eudoe-continuation.md)
- [현재 서술형 후속 형태 continuation](2026-07-15-present-declarative-continuation.md)
- [상태 용언 현재 평서형 후속 형태 continuation](2026-07-15-descriptive-declarative-continuation.md)
- [연결 어미 `-(으)니까` 계열](2026-07-15-connective-nikka.md)
- [Exact component 품사 확장](2026-07-15-exact-component-pos.md)
- [Full-POS 용언 exact component 확장](2026-07-15-predicate-exact-component.md)
- [Exact component 비용 마진](2026-07-15-exact-component-cost-margin.md)
- [형태 분석 그래프 전환 계획](morphology-analysis-graph-plan.md)
- [형태 구조 제약 resolver 계약](morphology-constraint-resolver-contract.md)
- [형태 구조 제약 resolver shadow 결과](2026-07-15-morphology-constraint-resolver.md)
- [형태 분석 그래프 schema 2 projection과 비용](2026-07-15-morphology-analysis-graph-resource.md)
- [Source provenance와 expression component shadow](2026-07-15-source-provenance-shadow.md)
- [접속 조사 `이면/면`의 명사류 결합](2026-07-15-connector-myeon-particle.md)
- [smart component 검색 근거](2026-07-13-smart-component-evidence.md)
- [copula lattice 폐기 판정](2026-07-13-copula-unseen-evaluation.md)
- [비표준·오타·띄어쓰기 입력 robustness 후속 설계](noisy-text-robustness-plan.md)
- [비표준·오타·띄어쓰기 입력 평가 계약](noisy-text-robustness-evaluation.md)
- [형태소 benchmark 사용법](README.md#morphology-comparison)

## 제품 상태

- CLI, Rust library와 WASM binding은 같은 query compiler와 matcher를 사용한다.
- 사람용 CLI 기본 경로는 full POS, enriched 용언 metadata와 `smart`다. 품사를 명시하는
  자동화 경로는 `--boundary any --embedded --json`을 사용한다.
- 명시적 품사 `smart`는 precision 99.00% 하한과 hard-negative 보호 안에서 FN을 FP보다 우선해
  줄인다. 무품사 결과는 품사 모호성을 포함한 제품 한계로 그대로 보고한다.
- `smart`의 명사·대명사·수사·관형사 branch는 문자열 token 경계 또는 compact component
  resource의 같은 fine POS를 가진 완전한 형태 component 근거가 있어야 한다. include 경로가
  최저 제외 경로보다 형태 분석 비용 1,500 이하로 높은 범위까지 인정하며 component 경계를
  가로지르는 substring과 더 큰 다른 품사의 component 내부 substring은 거부한다.
- full-POS가 로드된 `smart` 동사·형용사 branch는 predicate verifier가 활용·continuation을
  소비한 뒤 같은 fine POS의 어간 component에 같은 1,500 비용 마진을 적용해 문자열 왼쪽 경계를
  복구한다. 지정사 `PredicateLexical` 계약은 바꾸지 않는다.
- lexical context registry에 등록된 whole-token surface는 bounded 문맥 판정을 우선한다. 문맥이
  결정되지 않으면 비용 마진을 적용하지 않고 원시 component `accept`만 따른다.
- CLI는 `ExactComponent`, `PredicateLexical` 또는 `LexicalContext` branch가 있는 plan에서
  compact component resource를 자동으로 해석한다. 필요 resource의 누락·손상·schema 또는
  source 불일치는 초기화 오류이며 경계 판정으로 fallback하지 않는다.
- Rust/WASM engine은 full POS와 component bytes를 자동으로 찾지 않는다. caller가 생성자나
  load API로 명시하며, resource가 없는 component `smart` compile은 오류다.
- `smart`의 지정사 strict-subspan match는 token 전체의 exact 분석이 모두 non-predicate일 때
  해당 predicate branch만 거부한다. token 전체 match, predicate·미해석 분석, 다른 query
  branch는 유지한다.
- `smart` 무품사 조사 검색은 입력한 표면형만 사용한다. 이형태 묶음 확장은 명시적 조사 품사
  입력에서 유지하며 `token`과 `any` 계획은 바꾸지 않는다.
- `ending.nominalizer-gi`와 `ending.nominalizer` predicate branch는 nominal particle verifier로
  전이한다. `smart`와 `token`은 유효한 조사 연쇄를 token 끝까지 소비하고 잘못된 이형태와
  격조사 중복을 거부한다.
- 명시적 동사·형용사 품사의 `ending.connective-ji` branch는 오른쪽 token 경계를 유지하면서
  왼쪽 core 경계를 열어 오른쪽 끝 suffix를 복구한다. 무품사, `token`, `any`와 뒤에 문자가
  남는 candidate는 바꾸지 않는다.
- `ending.past`와 `ending.future` verifier state는 `ending.connective-eudoe`의 `으되`를
  소비한다. bare stem에는 붙이지 않으며 `치렀으되`, `하겠으되`처럼 선어말어미 뒤의 완성된
  token만 복구한다.
- 동작 용언의 현재 서술형 `-ㄴ다/-는다`와 형용사·보조 형용사의 현재 평서형 `-다` branch는
  `고`, `는`, `던`, `면`, `니`, `며`, `면서`, `는데`, `지`를 token 안에서 소비한다. 동작
  용언의 사전형, 지정사와 부정 지정사 `아니다`는 이 전이에 포함하지 않는다. bare 서술형은
  유지하고 `거나`, `든가`, `든지` 같은 종결 어미 뒤 조사와 두 번째 후속 형태 연쇄는 별도
  상태로 남긴다.
- 명사류 뒤의 접속 조사 `이면/면`은 받침 유무에 맞는 이형태만 소비하고 token을 닫는다.
  `백이면 백`, `공부면 공부`를 찾으며 `백면`, `공부이면`, `백이면도`는 거부한다.
- 연결 어미 `-니까/-으니까`, `-니까는/-으니까는`과 준말 `-니깐/-으니깐`은 받침 조건에
  맞는 이형태를 완성된 predicate token으로 소비한다. `먹니까`, `먹니깐`, `부니깐은`처럼
  잘못된 이형태나 추가 연쇄는 거부한다.
- core lexicon의 ㅎ 불규칙 예외에는 `어떻다`, `이렇다`, `커다랗다`가 포함된다. 기존 `DropH`
  generator로 `어떤`, `이런`, `커다란`을 만들며 규칙형 `어떻은`, `이렇은`, `커다랗은`은
  만들지 않는다.
- core lexicon은 `다르다 → 달라`, `이르다 → 일러/이르러`, `푸르다 → 푸르러`처럼 자주 쓰는
  르·러 불규칙과 동형어를 보존한다. full-POS 제품 경로는 국립국어원 사전 snapshot에서
  검토한 불규칙 분석 278개와 규칙형 동형어 companion 2개를 추가한다. 기존 르·러 102개에
  ㄷ·ㅅ·ㅂ·ㅎ 176개를 더하며, `푸다 → 퍼`는 core에서 유지한다. `곱다/VA`, `굽다/VV`처럼
  규칙형과 불규칙형이 독립적으로 확인된 표제어는 두 검색 branch를 모두 보존한다.
- 표준국어대사전·우리말샘의 고정 snapshot과 양방향 fixture로 검증한 `가려듣다`, `덧싣다`,
  `쏟아붓다`, `흘려듣다`는 core 불규칙 예외다. 자동 enriched 승격 조건은 유지하고 생성
  report에서는 네 항목을 `core-duplicate`로 기록한다.
- 한국어기초사전의 보조 동사 record로 검증한 `말다 → 마라`, `달다 → 다오`는 core의 개별
  terminal override다. 같은 표제어의 일반 동사 분석을 함께 보존하고 다른 표제어로의 오귀속은
  허용하지 않는다.
- 한국어기초사전의 어미 record로 검증한 `-거라`는 동작 동사의 사전형 어간에 직접 붙인다.
  `-너라`는 어간이 `오`로 끝나는 동작 동사로 제한하며 `오다`에는 두 어미를 모두 보존한다.
- 명시적 coarse `noun`의 사전 분석이 없으면 보통명사·고유명사·의존명사 fallback을 모두
  보존한다. component 판정은 corpus의 `NNBC`를 query-side `NNB`와 같은 의존명사로
  비교하며 artifact와 진단의 source tag는 바꾸지 않는다.
- 명시적 coarse `noun`에 full POS 분석이 있으면 누락된 보통명사·고유명사·의존명사
  fallback과 합집합으로 보존한다. user lexicon의 `replace = true`는 이 합집합보다 우선한다.
- `smart`의 `LexicalContext` branch는 같은 줄의 bounded 인접 token 구조로 동형이의어를
  선택한다. `아니라 매일 것`은 `매/NNG + 이/VCP + ㄹ/ETM`, 반복된 `매일 매일`은
  `매일/MAG`, `매일을`은 `매일/NNG + 을/JKO`로 판정한다. 구조가 불충분하면 기존 경계
  판정을 유지한다.
- copula 전용 lattice 분기와 shadow 계측, PUD/GSD 전용 실행 경로는 복원하지 않는다.
- 비표준 활용, 오타와 불안정한 띄어쓰기는 canonical 형태 규칙에 합치지 않는다. 현재 제품
  기본값은 그대로 두고, 별도 opt-in robustness 축과 자연 원문 fixture를 먼저 검증한다.
- 기본 morphology benchmark는 kfind 프로필만 다시 실행한다. Kiwi·Lindera·MeCab-ko·KOMORAN
  품질은 test fixture와 어댑터 schema에 묶인 저장소 스냅샷을 읽고, fixture나 고정한 비교기
  설정이 바뀔 때만 `scripts/refresh-morph-baselines.sh`로 갱신한다.
- 제품 persona 비교는 같은 explicit-POS fixture와 gold를 사용한다. Agent와 외부 분석기는
  품사를 명시하고 User는 같은 query의 품사를 제거한 `full-POS + smart`로 실행한다. 이 결과는
  동일 입력의 backend 순위가 아니라 실제 입력 조건을 반영한 비교다.
- compact component artifact는 Homebrew의 `share/kfind`와 npm의 별도 정적 asset으로
  배포한다. WASM binary에는 artifact bytes를 포함하지 않는다.
- 제품 matcher의 local lattice는 query 포함·제외별 최저 비용만 계산한다. N-best 경로는
  shadow 진단에서만 생성하며 두 경로의 판정과 최저 비용은 같다. unknown model은 component
  evaluator마다 한 번 파싱하고 Engine이 matcher 사이에서 공유한다.

## 품질 기준선

명시적 품사를 사용하는 1,000-case test의 현재 제품 결과다.

| lexicon | boundary | TP / FP / FN | precision | recall | F1 |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| embedded | smart | 436 / 0 / 64 | 100.00% | 87.2% | 93.16% |
| full-POS | smart | 466 / 0 / 34 | 100.00% | 93.2% | 96.48% |
| embedded | token | 359 / 0 / 141 | 100.00% | 71.8% | 83.59% |
| full-POS | token | 366 / 0 / 134 | 100.00% | 73.2% | 84.53% |
| embedded | any | 482 / 11 / 18 | 97.77% | 96.4% | 97.08% |
| full-POS | any | 492 / 11 / 8 | 97.81% | 98.4% | 98.11% |

development 명시적 품사 `smart`는 embedded가 TP 461 / FP 2 / FN 39, full-POS가
TP 475 / FP 2 / FN 25다.
full-POS의 기존 세부 품사 분석과 coarse `noun` fallback 합집합이 `197명이`를 복구했다.
세부 품사와 품질 계약은 [User smart precision 품질·성능](2026-07-14-user-smart-precision.md),
현재 처리량과 latency는 [Exact component 비용 마진](2026-07-15-exact-component-cost-margin.md)을
기준으로 한다.

품사를 생략하는 사람용 1,000-case fixture에서 full-POS `smart`는 TP 461, FP 0, FN 39,
precision 100.00%, recall 92.2%, F1 95.94%다. embedded `smart`는 TP 331, FP 0, FN 169다.
embedded `smart`는 기대 품사를 plan에 포함하는
비율이 48.4%이므로 사람용 기본 경로를 대신하지 않는다.

explicit-POS test fixture의 품사를 제거한 User persona도 full-POS `smart`에서 TP 461, FP 0,
FN 39, precision 100.00%, recall 92.2%, F1 95.94%다. `이다 -> 매일`은 bounded 지정사 관형형
문맥이 아니면 whole-token lexical 근거로 거부하고, determiner query `이 -> 날씨가`는 무품사
조사 이형태 확장을 제한해 제거했다. fixture·gold·지표 정의는 바꾸지 않았으며 현재 `any`는
TP 482 / FP 11 / FN 18이다.

무품사 fixture와 persona 결과는 명시적 품사 품질과 분리한다. 목표 수치를 맞추기 위한 fixture,
gold, negative 선택 변경은 허용하지 않으며 품사 모호성에서 생긴 FP와 FN도 그대로 남긴다.

## 현재 경계

- `-기`와 `-ㅁ/음` 명사형 뒤의 유효한 조사 연쇄는 predicate token의 일부다. `걷기가`,
  `걷기에서도`, `걸음이`, `걸음으로`를 찾고 `걷기이`, `걷기가를`, `걸음가`, `걸음이를`은
  `smart`와 `token`에서 거부한다. 다른 종결형·연결형은 조사 verifier로 전이하지 않는다.
- `smart` component는 명사·대명사·수사·관형사와 full-POS 일반 동사·형용사의 exact component
  span만 복구한다. include 경로 비용은 최저 제외 경로보다 최대 1,500 높을 수 있다. 용언은
  활용·continuation을 소비한 어간 core의 fine POS를 검증한다.
  `대학교`의 `학교`처럼 source 분석이 component로 증명하지 않는 substring,
  `역사과목`의 `사과`처럼 component 경계를 가로지르는 span과 `전자기견해`의 `자기`처럼 더 큰
  다른 품사의 component 내부 substring은 거부한다.
- component resource가 필요한 `smart` query의 fail-fast 동작은 호환성 계약이다. optional
  resource가 필요한 caller는 query compile 전에 resource를 준비해야 한다.
- whole-token 분석은 기본적으로 지정사 strict-subspan보다 우선한다. 예외는 직전 token이
  완전한 `VCN+EC`, 현재 token이 유일한 명사 prefix와 exact `VCP+ETM` suffix, 다음 token이
  `NNB/NNBC`로 시작하는 같은 줄의 bounded 구조다. 이때 명사 prefix와 지정사 suffix만
  선택하고 whole-token 명사·부사는 거부한다.
- NFC가 같은 인접 token에 exact `MAG` 분석이 있으면 반복된 두 token을 부사로 선택하고
  명사·명사 component branch는 거부한다. `매일을`처럼 조사까지 포함한 whole-token 명사
  분석은 그대로 명사로 선택한다.
- 인접 문맥은 같은 줄의 raw 256 bytes와 NFC scalar 64개로 제한한다. 줄을 넘거나 한도를
  초과하거나 구조가 모호하면 별도 문맥 결정을 만들지 않고 기존 `smart` 결과를 유지한다.
- Korean-Kaist·KSL dev의 실제 지정사 annotation에는 `예이다`, `생명인데`, `것인가를`처럼
  `any`에는 있고 `smart`가 제거한 gold token이 130개 있다. annotation의 split만으로
  whole-token 완전 경로의 부재를 증명하지 않으므로 제품 복구 근거로 사용하지 않는다.
- Agent precision 후보는 먼저 `embedded + any` 결과에 대한 benchmark shadow로만 측정한다.
  timed 결과와 제품 `any` 결과는 유지하고, bounded local lattice의 include/exclude 완전 경로
  존재 여부와 생성 근거를 development·hard-negative에서 분류한다.
- Agent shadow의 `include-path` 투영은 development TP를 487에서 452로 줄이면서 FP 16을
  유지했다. `include-only`는 FP를 0으로 줄이지만 TP도 8로 줄였다. 제품 matcher와 `any`
  정책은 변경하지 않는다.
- Korean-Kaist·KSL dev의 실제 지정사 token과 겹치는 `이다` candidate 1,174개는 모두 include와
  exclude 완전 경로가 함께 존재했다. 지정사 split만 가능한 최소 대조가 없으므로 문맥 복구를
  일반화하지 않는다.
- `ending.connective-ji` 오른쪽 끝은 `주다 -> 심어주지`를 복구한다. 같은 표면형 hard-negative
  `주지 스님`은 기존 FP이며, 남은 gold candidate 3건은 모두 `left-edge`다.
- full-POS `smart`의 exact component 판정은 `없다/VA -> 없지는`과 `없다/VX -> 없지요`처럼
  `ending.connective-ji` left-edge의 같은 표면 구조를 fine POS 근거로 구분한다. embedded
  `smart`와 오른쪽 token 경계는 유지한다.
- `-었-`, `-겠-` 뒤의 `으되`는 완성된 predicate token의 일부다. 이 continuation은
  `치렀으되`, `하겠으되`를 복구하고 bare stem이나 `으데`는 허용하지 않는다.
- 동작 용언의 현재 서술형과 상태 용언의 현재 평서형 뒤 `고`, `는`, `던`, `면`, `니`, `며`,
  `면서`, `는데`, `지`는 완성된 predicate token의 일부다. `받다 -> 받는다는`,
  `받들다 -> 받든다는`, `나쁘다 -> 나쁘다면`, `좋다 -> 좋다는`, `어렵다 -> 어렵다면서`와
  `영원히 함께한다던 말도`의 `한다던`을 복구하며 `말도`는 다음 명사 token으로 남긴다.
  `쓴다도`, `먹는다도`, `가다면`, `나쁘다면도`, `아니다면`은 거부한다. 부정 지정사의 올바른
  조건형은 `아니라면`이지만 현재 `아니라` branch가 terminal이라 `smart`와 `token`에서는 아직
  복구하지 않는다. 이 연쇄는 상태 용언 `-다` 전이와 분리한 후속 경계다.
- 명사류 뒤의 `이면/면`은 접속 조사다. 받침 있는 말은 `이면`, 받침 없는 말은 `면`을 쓰며
  뒤에 다른 조사를 잇지 않는다.
- `불다 -> 부니까`의 `-니까`는 `ㄹ` 탈락 뒤 완성된 predicate token의 일부다. 같은 받침
  조건을 `-니까는`, `-니깐`에도 적용하며 잘못된 `으` 이형태는 만들지 않는다.
- 남은 full-POS `smart` FN 25건은 `boundary-rejected` 12건, `surface-missing` 8건,
  `span-mismatch` 3건, `lexicon-missing` 2건이다.

## 1.0 RC 안정화

남은 FN 해소와 별도로 다음 안정화 조건을 유지한다.

1. 모든 공개 matcher 생성 경로는 plan의 resource 요구를 같은 방식으로 검사한다. 필요한
   resource가 없으면 fallback이나 부분 판정 없이 초기화 오류를 반환한다.
2. phrase 검색은 반복 token, 줄바꿈 없는 긴 입력과 큰 `max-gap`에서도 가능한 중간 조합을
   모두 만들어 메모리에 쌓지 않는다. 가장 이른 match만 필요한 경로와 전체 metadata 경로를 분리하고,
   병적 입력 benchmark와 fuzz target으로 시간·메모리 경계를 검증한다.
3. kfind 1.0의 안정 Rust API는 `Engine`, `Matcher`, compile option과 오류를 중심으로 확정한다.
   `QueryPlan`, branch·verifier 표현과 public field는 stable facade, 변경 가능한 expert API,
   내부 crate 중 하나로 분류한 뒤 공개 범위를 고정한다. 내부 crate의 배포 여부도 같은 기준으로
   명시한다.
4. 모든 fuzz target은 CI에서 target당 15초의 고정 예산과 개별 입력 timeout·RSS 상한으로 실제
   실행한다. phrase join의 반복 span·큰 gap, 손상된 UTF-8과 component resource 누락을 seed
   corpus에 포함하고 crash, panic, timeout과 RSS 초과가 없어야 한다.
5. 재배포 조건이 명확한 한국어 소스 코드 주석·README·기술 문서 snapshot으로 blind 검색 평가를
   추가한다. query와 기대 span은 규칙 개발 전에 고정하고 canonical 문장 중복을 제거하며,
   식별자 주변 한글, 띄어쓰기 오류, 한영·숫자 혼합, 동형이의어와 복합명사 substring을 별도
   slice로 보고한다. 이 결과는 기존 UD 회귀 fixture를 대체하지 않는다.

항목 5는 [현실 기술 코퍼스 blind 평가](2026-07-14-real-corpus-blind.md)로 닫았다. 고정한
25건에서 Agent는 TP 20 / FP 3 / FN 1, User는 TP 15 / FP 1 / FN 6이었다. User의 띄어쓰기
오류 slice는 recall 20%이며, 이 소표본은 제품 전체 품질 점수나 규칙 선택 근거로 사용하지 않는다.

항목 3은 stable facade와 expert API를 분리해 닫는다. `Engine`, `Matcher`, resource·compile
option·오류와 match provenance만 1.x 안정 계약으로 두고, `QueryPlan`, branch·verifier,
`Lexicons`와 plan inspection은 `kfind::expert`로 격리한다. workspace 내부 crate는 게시하지 않는다.

규칙 capability 일원화는 위 안정화 조건 뒤에 진행한다. 단순 boolean metadata가 실제 구현을
대신하지 않으므로 rule artifact에서 생성한 typed capability registry와 generator·verifier handler
coverage 검사를 함께 도입한다. 새 규칙이 없는 RC 변경에는 이 schema migration을 섞지 않는다.

## 이어갈 형태 품질 작업

source expression 관계 감사, policy-neutral component graph schema 2, 비용 독립 `TokenAnalysisGraph`, `QueryMorphPattern`과 `ConstraintResolver`를 구현했다. 전체 development, 고정 test와 hard-negative에서 세 profile을 shadow 평가했으나 `opaque`와 현재 capability의 `explicit`은 기존 positive를 보존하지 못했고 `transparent`는 새 false positive와 hard-negative 회귀를 만들었다. 형태 구조만으로 같은 표면의 다른 표제어·의미를 구분할 수 없으므로 제품 전환은 실패로 닫고 현재 matcher, lexical context registry와 1,500 비용 마진을 유지한다.

다음 형태 품질 작업은 명시적 품사 full-POS `smart`의 남은 FN 25건을 현재 제품 계약 안에서 줄이는 변경이다. surface별 예외 목록, 새 비용 임계값과 bounded context의 강제 분석 선택은 사용하지 않는다.
계측·report·runner만 바꾼 상태는 작업 완료로 보지 않는다. 규칙 조건은 development case만으로
만들지 않고 독립된 사전·문법 근거로 정의한다. development는 후보 선택에 사용하고, 고정 test와
무품사 결과는 규칙을 고정한 뒤 회귀 판정에만 사용한다.

### 기준선과 작업 집합

- development full-POS `smart`: TP 475 / FP 2 / FN 25
- explicit-POS test full-POS `smart`: TP 466 / FP 0 / FN 34
- Agent embedded `any`: TP 482 / FP 11 / FN 18
- Human full-POS `smart`: TP 461 / FP 0 / FN 39
- hard-negative: 22건, 기존 FP 4건

| 원인 | 품사별 건수 | 합계 |
| --- | --- | ---: |
| `boundary-rejected` | adjective 3, determiner 2, noun 4, numeral 2, verb 1 | 12 |
| `surface-missing` | adjective 4, noun 2, verb 2 | 8 |
| `span-mismatch` | numeral 1, verb 2 | 3 |
| `lexicon-missing` | adjective 2 | 2 |

`boundary-rejected` 12건은 다음 고정 집합이다. predicate 4건의 gold surface와 any-boundary
rule path는 [제한된 사전 표면형 계층](2026-07-15-dictionary-surface-lexicon.md)의 source
report에서 확인한다.

| 품사 | 쿼리 |
| --- | --- |
| adjective | `안되다`, `비싸다`, `있다` |
| determiner | `사`, `열` |
| noun | `도구`, `서사극`, `업자`, `년대` |
| numeral | `만`, `일` |
| verb | `지나다` |

나머지 13건은 report의 원인 이름을 바로 구현 방향으로 해석하지 않고 gold 위치와 source
annotation을 먼저 판정한다.

원문 감사에서 `열린다고`, `맞춰서`, `다립니다`, `적잖은`은 표제어 또는 품사 annotation과
충돌했고 `이 백명`, `미오씨 입니다`는 `Typo=Yes` 문장의 gold span 정렬과 충돌했다. 이 여섯
건은 제품 규칙으로 덮지 않는다. `격식있다`는 비표준 띄어쓰기 robustness 축으로 분리하고,
`상관없이`, `같이`는 derivation 경계를 유지한다. 나머지는 host+copula 또는 축약을 구분할
독립 근거가 생길 때 다시 검토한다.

| 원인 | 쿼리 / gold surface | 첫 판정 게이트 |
| --- | --- | --- |
| `lexicon-missing` | `열리다/adjective -> 열린다고` | 동사 표제어의 형용사 annotation인지 원문과 독립 사전에서 확인한다. |
| `lexicon-missing` | `격식있다/adjective -> 격식있는` | 띄어쓰기 없는 합성 용언을 일반화할 사전·component 근거가 있는지 확인한다. |
| `surface-missing` | `맞다/verb -> 맞춰서` | `맞추다` surface의 표제어 오귀속인지 annotation을 먼저 확인한다. |
| `surface-missing` | `상관없다/adjective -> 상관없이` | 검증된 파생 부사이므로 기본 `inflection`을 넓히지 않고 `derivation` 경계를 유지한다. |
| `surface-missing` | `다리/noun -> 다립니다` | 모음 끝 명사 `N + -ㅂ니다`와 `다리다/verb + -ㅂ니다`의 동형 surface를 완전한 분석 경로와 문맥으로 구분한다. |
| `surface-missing` | `이다/adjective -> 마찬가지다` | host 명사와 지정사 suffix를 분리할 수 있는지 확인한다. |
| `span-mismatch` | `백/numeral`: raw lemma `백`, gold surface `이` | `이 백명`의 token 정렬과 adapter byte span을 먼저 확인한다. |
| `surface-missing` | `같다/adjective -> 같이` | 두 사전 활용 합의나 양방향 파생 관계가 없으므로 현재 surface 계층에 넣지 않는다. |
| `surface-missing` | `거/noun -> 게` | 예문 외의 축약 규칙과 대조군 근거가 생기기 전에는 사전 행을 추가하지 않는다. |
| `surface-missing` | `적다/verb -> 적잖은` | `적지 않은` 축약의 품사·생산 범위와 반례를 확인한다. |
| `surface-missing` | `어떻다/adjective -> 어떻는` | 비표준 학습자 표면형인지 확인하고 표준 활용으로 승격하지 않는다. |
| `span-mismatch` | `이다/verb`: gold `미오씨`, 현재 match `입니다` | zero-copula 정렬인지 gold span과 adapter를 먼저 확인한다. |

### 작업 순서

1. `surface-missing`과 `lexicon-missing`은 개별 표면형을 손으로 추가하지 않는다. 고정 NIKL
   snapshot 전체를 기존 importer에 통과시키고, 두 사전 활용 합의 또는 양방향 `RelatedForm`
   조건을 만족한 후보를 먼저 만든다. 기존 생산 규칙으로 생성 가능한 항목은 데이터에서 빼고
   일반 규칙을 고친다. 남는 검증형만 용량 상한 안에서 `SurfaceOnly`로 저장한다.
2. `boundary-rejected`는 any-boundary의 core·token span, token 내 위치와 모든 rule path를
   집계한다. exact component 경로나 제한된 continuation이 동일 surface의 positive와 negative를
   구분할 때만 제품 후보로 연다. 특정 lemma, case ID나 문장 문자열을 조건으로 삼지 않는다.
3. 남은 predicate 4건은 exact component가 거부한 경로와 gold span을 대조한다. 남은
   명사·수사·관형사 8건도 whole-token 분석보다 substring을 우선하지 않는다.
4. 하나의 일반 규칙 또는 하나의 사전 생성 계약을 한 작업 단위로 구현한다. 개발 fixture와
   hard-negative를 통과한 뒤에만 고정 test, Agent, Human과 성능을 측정한다.

### 채택 조건과 중단 조건

- development는 FN 25 미만, precision 99.00% 이상이어야 한다. hard-negative 22건의 기존 FP
  4건 외 신규 FP는 0이어야 한다. 통과 후보가 여럿이면 FN이 적은 후보를 먼저 선택하고 FN이
  같을 때 FP가 적은 후보를 선택한다.
- 규칙 고정 뒤 explicit-POS test는 FN 34 이하, precision 99.00% 이상이어야 한다. Agent와 Human도
  같은 고정 fixture에서 다시 측정하고 불리한 변화를 기록한다.
- 동일 candidate surface의 positive와 negative를 구분할 독립 근거가 없으면 제품 경계를 열지
  않는다. 사전 예문 한 건이나 development case 자체는 독립 근거가 아니다.
- source annotation이 표제어·품사·span과 충돌하면 제품 FN과 분리한다. 목표 수치를 위해 fixture,
  gold, negative를 바꾸지 않는다.
- `inflection`과 `derivation`의 공개 계약을 특정 FN 때문에 합치지 않는다. 계약 변경이 필요하면
  사양과 CLI 동작을 먼저 별도 결정한다.
- Agent precision은 이 explicit-POS `smart` recall 작업 뒤에 재개한다. 기존 `include-path`와
  `include-only` 투영은 제품 후보로 재사용하지 않는다.

### NIKL surface 계층

고정 NIKL snapshot은 `${KFIND_NIKL_DOWNLOADS:-~/Downloads}`의 ZIP을 SHA-256으로 검증하고
`${KFIND_NIKL_CACHE:-${XDG_CACHE_HOME:-~/.cache}/kfind/nikl}`에 추출한다. 현재 입력은
한국어기초사전 `20260619`, 표준국어대사전 `20260705`, 우리말샘 `20260702` snapshot이다.

한국어기초사전과 표준국어대사전이 함께 지지하는 활용형은 12,888개, `(lemma, fine_pos)`는
6,073개다. core·enriched 분석과 품사 기반 생산 alternation으로 생성 가능한 surface는 배포
데이터에 복제하지 않는다. 분석 시점의 생산 alternation 선택으로 12,758개를 설명하고, 남는
활용형만 enriched TSV의 `SurfaceOnly` 분석으로 만든다.

한국어기초사전의 용언-부사 `파생어` 관계 중 source·target ID와 표면형이 양방향으로 일치하는
153개도 같은 TSV에 저장한다. 활용형은 기본 `inflection`에 포함하고, 파생 부사는
`--expand derivation`에서만 연다. 예문·정의는 추출하지 않는다. surface-only 행은 512개,
배포 enriched TSV는 64 KiB를 상한으로 둔다.

현재 development FN의 해석은 다음과 같다.

- `상관없다 -> 상관없이`는 한국어기초사전 record `16206`과 부사 record `16217`의 양방향
  `RelatedForm`이므로 derivation surface 계층에서 처리한다. 기본 `inflection` benchmark의 FN을
  없애기 위해 파생 부사를 기본 모드로 옮기지 않는다.
- `있다 -> 있는`은 한국어기초사전 `68796`, `68797`과 표준국어대사전
  `275069:275069001`, `275069:275069002`가 함께 지지하는 활용형이다. `멋있는`의 최종
  `smart` 판정은 surface 생성과 분리된 component boundary 문제다.
- `같다 -> 같이`는 두 snapshot의 활용형 합의나 한국어기초사전 양방향 `RelatedForm`이 없으므로
  corpus case만으로 surface 계층에 넣지 않는다.
- `거 -> 게`는 예문 근거뿐이므로 NIKL surface 계층에 넣지 않고 명사 축약 규칙 후보로 남긴다.
- `다리 -> 다립니다`는 모음으로 끝나는 명사 뒤의 `N + -ㅂ니다`형과
  `다리다/verb -> 다립니다`가 같은 surface다. [국립국어원 문법 자료](https://kcenter.korean.go.kr/kcenter/search/dgrammar/view.do?id=167)는
  전자를 허용하되 격식적인 문어에서는 잘 쓰지 않는다고 설명하고,
  [한국어기초사전 record 40517](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=40517)은
  후자를 동사 활용형으로 제시한다. `긴 다립니다`와 `옷을 다립니다`를 양방향 대조군으로 고정하고,
  단순 surface 행이 아니라 explicit POS, 완전한 component 경로와 bounded 문맥이 두 분석을
  구분하는 일반 규칙만 검토한다.
- `치르다 -> 치렀으되`는 Korean-Kaist 원문 분석 `치르+었+으되`와 한국어기초사전의
  `-었-`, `-겠-` 뒤 `-으되` 결합을 근거로 선어말어미 continuation에서 처리한다. surface 계층에
  개별 활용형을 추가하지 않는다.

## 재현과 검증

```console
scripts/benchmark-morphology.sh
scripts/refresh-morph-baselines.sh
scripts/benchmark-morph-index.sh
pnpm --dir packages/kfind run benchmark:startup
```

형태소 계약을 변경할 때는 다음 검증을 함께 실행한다.

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
scripts/benchmark-criterion.sh local_lattice
cargo fmt --manifest-path tools/morph-index-benchmark/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/morph-index-benchmark/Cargo.toml \
  --all-targets -- -D warnings
cargo test --locked --manifest-path tools/morph-index-benchmark/Cargo.toml
scripts/benchmark-morphology.sh
scripts/benchmark-morph-index.sh
```
