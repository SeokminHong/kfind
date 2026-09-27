# 검증과 성능

이 문서는 [kfind 기술 사양서](kfind.md)의 현행 규범적 계약이다.

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
걷다 | 사용자 → 걷거나 사용자가 있는 span을 각각 match
걷다|사용자 → 공백을 둔 disjunction과 같은 결과
"|", \| → literal `|` match
걷다 | 사용자 검증하다 → phrase와 disjunction 혼합 오류
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

### 19.4 퍼징

target과 경계:

| target                   | 경계                                                                                             |
| ------------------------ | ------------------------------------------------------------------------------------------------ |
| `query_lexer`            | 잘못된 UTF-8을 포함한 임의 query, 매우 긴 combining sequence, lexer와 compile limit              |
| `matcher_bytes`          | 임의 byte 입력의 anchor 탐색, suffix consumption, match span 범위                                |
| `matcher_plan`           | 임의 query와 큰 phrase gap의 compile·matcher build, component resource 누락 오류                 |
| `user_lexicon`           | malformed 사용자 사전 TOML의 구문·의미 검증                                                      |
| `json_output`            | 임의 byte line과 검증된 match metadata의 JSON Lines 직렬화                                       |
| `binary_detection`       | 임의 위치의 최초 NUL과 NUL이 없는 입력의 binary 판별 경계                                        |
| `pos_resource`           | 임의 byte full POS resource의 크기·header·varint·UTF-8·NFC·정렬·누적 decode 상한                 |
| `component_resource`     | 임의 byte component resource와 임의의 유효한 소형 resource의 header·digest·payload·prefix lookup |
| `search_executor`        | 임의 byte record와 작은 channel에서 병렬 검색 결과의 bounded 수집·정렬·summary 경로              |
| `structural_preparation` | 현재 token graph와 인접 token 선택을 분리한 경로가 일괄 준비 경로와 같은 판정을 내리는지 비교    |

CI는 `nightly-2026-07-11`과 `cargo-fuzz 0.13.2`로 모든 target을 실제 실행한다. target당
`max_total_time=15`, 개별 입력 `timeout=5`, `rss_limit_mb=2048`을 적용하며 전체 job timeout은
10분이다. `scripts/run-fuzz.sh`가 target 목록과 이 예산을 단일 진입점으로 유지한다. 각 실행은
version-controlled seed만 임시 corpus로 복사해 이전 실행에서 생성된 입력과 격리한다. 반복 span과
큰 gap의 phrase, 손상 UTF-8, component resource가 필요한 plan, malformed TOML, 출력 제어 문자,
최소 유효 full POS resource, 유효한 소형 component entry와 구조 준비 경계 문맥을 고정 seed로 시작한다.
crash·panic·timeout·RSS 초과는 CI 실패다.

### 19.5 정답 corpus

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

Strict 지표는 gold와 다른 표제어·품사 결과를 false positive로 보존한다.
Contract-adjusted 지표는 같은 품사의 동형 활용처럼 bounded 구조가 같은 결과만
contract positive로 재분류한다. 품사 또는 인접 성분 배치로 구분 가능한 결과는
현재 구현이 제거하지 못해도 FPᶜ로 유지한다.

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

같은 Robust fixture의 성능도 fresh process warm-up 1회 뒤 5회 측정한다. Explicit-POS
비교는 embedded/full-POS 각각의 `any`와 `smart`, 고정 외부 backend에 대해 initialization,
cases/s, p50·p95 latency와 peak RSS의 median/min/max를 기록한다. Agent의
`embedded + any + explicit POS`와 Human의 `full-POS + smart + untagged`는 제품 workflow
비교로 별도 보존한다. 품질과 성능은 같은 보고서에서 별도 표와 chart로 제시하고 canonical
합계와 섞지 않는다.

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

Site의 Canonical, query matrix와 Robust explicit-POS 비교는 각각 kfind를
`embedded + any`, `embedded + smart`, `full-POS + any`, `full-POS + smart`의 네 profile로
나눈다. 각 workload의 품질은 raw와 contract-adjusted confusion matrix를 각각 보존하고,
성능은 initialization, cases/s, p50·p95 latency와 peak RSS를 함께 보존한다. 외부 분석기 행은
같은 workload의 기존 고정 품질·성능 설정으로 함께 표시하되 kfind profile을 resource
종류만으로 합치지 않는다. 서로 다른 workload의 품질 또는 성능을 하나의 순위나 합계로
합치지 않는다. `full-POS + smart`의 고정 canonical gate는 `FPᶜ = 0`, `FNᶜ = 0`이다.

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
기대값으로 다시 계산한 `contract_adjusted` 지표도 병렬로 기록한다. query matrix 생성기는
version-controlled contract review registry를 적용하고 그 hash와 적용·제외 건수를 metadata에
기록한다. registry와 맞지 않는 case identity가 있으면 생성에 실패하며 제품 또는 외부 분석기
출력으로 annotation을 만들지 않는다. 이 지표의 confusion matrix는
`contract_tp`·`contract_fp`·`contract_tn`·`contract_fn`, 파생 지표는
`contract_precision_percent`·`contract_recall_percent`·`contract_f1_percent`로 명명한다.
표에서는 각각 TPᶜ·FPᶜ·TNᶜ·FNᶜ로 줄여 쓸 수 있다.
canonical·hard-negative의 contract-positive 분모는 `PNᶜ = TPᶜ + FNᶜ`로 표기하며,
recall 개선 보고서는 `PNᶜ`, `FNᶜ`와 `recallᶜ = TPᶜ / PNᶜ`를 함께 기록한다.

`contract_expected`가 없으면 strict `expected`를 그대로 사용한다. boolean 값이 strict와 다르면
양방향 reclassification을 허용한다. `expected=false`, `contract_expected=true`는 같은 품사의
동형 활용을 의미로 구분하지 않는 `same-pos-homograph`, 품사가 달라도 bounded 문장 구조가 같은
`structurally-indistinguishable-homograph`, source에 정렬된 내부 성분을 검색하는
`aligned-source-component`에만 사용한다. `expected=true`, `contract_expected=false`는 gold가
완성 어휘 내부의 다른 품사 span에 잘못 정렬된 `gold-alignment-error`에만 사용한다. strict
기대값을 검토 후 유지한 case는 `implementation-target`으로 기록한다.

`contract_expected=null`은 계약 평가 제외다. 현재 비문·비표준 띄어쓰기처럼 표준문 형태 검색의
입력 계약을 어긴 `nonstandard-input`만 제외 사유로 허용한다. 현재 구현 profile이 지원하지 않는
파생, 반환 span 설계가 필요한 축약, 범용 구조 판정 비용이 큰 case와 미구현 문법은 제품 목표에서
제외하지 않고 FNᶜ로 유지한다. 모든 확인·변경·제외에는 제품 결과를 보기 전에 고정한
`contract_reason`이 필요하다.

계약 confusion matrix의 `cases`는 제외하지 않은 case 수다. `reviewed_cases`,
`confirmed_cases`, `reclassified_cases`, `excluded_cases`와 사유별 건수를 함께 기록한다. 계약 문장 회수율은 제외한
질의를 분모에서 빼고 contract-positive 질의만 센다. strict 지표와 계약 보정 지표를 합치거나,
계약 보정 지표만으로 정밀도 회귀가 없다고 주장하지 않는다. 모든 제품과 외부 분석기의 품질 표와 차트는 같은 profile의
raw TP·FP·TN·FN·precision·recall과 TPᶜ·FPᶜ·TNᶜ·FNᶜ·precisionᶜ·recallᶜ를 나란히 표시한다.
FPᶜ·FNᶜ·recallᶜ는 annotation과 제외가 실제 적용된 제품 계약 값이어야 한다. Review registry가
없는 fixture에서는 raw 기대값을 그대로 사용하고 review 0건을 함께 표시한다. 외부 분석기에도
동일한 contract expectation을 적용하며 raw와 contract-adjusted 결과를 모두 비교한다.

query matrix의 raw FN을 닫는 작업은 contract review registry와 별도의 disposition 장부로
관리한다. 장부는 fixture SHA-256과 case ID, query·품사·gold surface, 현재 failure cause,
disposition, 근거, 사전 증거를 보존한다. disposition은 raw FN의 원인을 설명할 뿐 지표를 직접
재분류하지 않는다. 계약 기대값 또는 제외 여부는 같은 근거를 사람이 검토해 제품 실행 전에
contract review registry에 선언한다. 완료 상태는 raw FN 0, 미분류 raw FN 0과 FNᶜ 0을 각각
구분해 보고한다.

disposition은 다음 중 하나다.

1. `product-fix`: 기존 계약과 정밀도 gate를 지키는 제한된 규칙으로 회수할 수 있다.
2. `dictionary-required`: 일반화 가능한 표제어·품사·활용·관계 증거가 있어야 안전하게
   회수할 수 있다.
3. `structural-redesign`: 검색할 byte span 복원, source 내부 성분 대응이나 bounded 구조 판정의
   추가 설계가 필요하지만 제품 목표에는 포함된다.
4. `gold-alignment-error`: gold lemma·품사·정렬이 완성 어휘의 실제 구조와 맞지 않아 기대값을
   바로잡아야 한다.
5. `nonstandard-input`: 현재 비문·비표준 띄어쓰기라 표준문 형태 검색의 계약 모수에서 제외한다.

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

### 19.8 형태 질의와 정규식 검색 기준선

형태 질의와 수동 정규식의 차이는 version-controlled constructed fixture로 진단한다. Fixture는
용언·형용사 7개 질의마다 positive 8개와 형태·경계 경쟁자 negative 8개를 두어 총 112개
case를 유지한다. 이 fixture는 동일 질의의 전략 차이를 설명하는 예시이며 held-out corpus,
Canonical 회귀선이나 일반적인 한국어 검색 품질을 대표하지 않는다.

각 질의는 `kfind full POS`의 명시적 품사 질의를 `boundary=any`와 `boundary=smart`로 각각
실행하고, 사람이 자주 쓰는 두 정규식 전략과 비교한다. `enumerated`는 알려진 활용 표면형을
`|`로 열거하고, `stem`은 짧은 어간 후보만 열거한다. 정규식에는 자동 활용 생성, 품사 판정과
token boundary를 추가하지 않는다. `rg`와 `grep`은 동일 정규식을 실행하고 matching line
집합이 같은지 검증한다. 따라서 품질은 두 kfind boundary와 정규식 전략별 한 행으로 집계하고,
도구별 실행 비용은 별도 성능 행으로 보고한다.

품질에는 네 전략 모두 raw TP·TN·FP·FN, precision·recall·F1과
TPᶜ·TNᶜ·FPᶜ·FNᶜ, precisionᶜ·recallᶜ·F1ᶜ를 기록한다. Contract review는 제품 실행 전에
fixture에 선언하며 `same-pos-homograph`, `structurally-indistinguishable-homograph`,
`aligned-source-component`만 허용한다. 같은 예측을 두 기대값으로 다시 평가하므로
contract-adjusted는 별도 검색 모드나 후처리가 아니다.

성능 fixture는 112개 문장을 순서대로 반복한 단일 파일이다. 한 batch에서 7개 질의를 각각
fresh process로 실행해 같은 파일을 7회 스캔하고 matching-line count만 계산한다. 데이터 준비와
품질 failure 분류는 측정 구간에서 제외한다. 같은 장비·입력·환경에서 방법 순서를 순환하며
warm-up 2회 뒤 10회 측정하고 batch wall time의 median·min·max·p95와 전체 scan byte 기준
effective MiB/s를 기록한다. 품질과 실행시간은 하나의 점수나 순위로 합치지 않는다.

보고서는 revision, fixture와 corpus SHA-256, resource와 binary SHA-256, 정확한 명령, OS·CPU,
도구 버전, 측정 횟수와 case-level failure를 JSON과 Markdown에 보존한다. 사이트 snapshot은
승인 보고서에서 chart와 표가 소비하는 요약 필드만 export한다. 사이트는 raw·contract-adjusted
F1 차트와 TP·TN·FP·FN 원수치 표, 도구·정규식 전략별 batch 시간 차트와 정확한 통계 표를
분리해 표시하고 constructed fixture의 해석 한계를 함께 밝힌다.

## 20. 성능 사양

### 20.1 목표

기준 장비와 corpus는 벤치마크 보고서에 고정한다. 예시는 Apple Silicon의 최근 세대 장비로 두되, 결과에는 CPU, 메모리, 저장장치, OS를 반드시 기록한다.

제품 목표:

```text
단일 atom query compile p95: 0.25 ms 이하
8 atom phrase compile p95: 0.75 ms 이하
8 atom disjunction compile p95: 0.75 ms 이하
낮은 hit 비율의 scan: rg -F wall time의 1.25배 이내
낮은 hit 비율의 처리량: rg -F의 80% 이상
기본 RSS: 16 MiB 이하
corpus 크기에 비례하는 결과 버퍼링 없음
```

`rg -F`와 기능이 동일하지 않으므로 절대 우열이 아니라 I/O 경로의 성능 회귀 감시 기준으로 사용한다.

### 20.2 검색 corpus

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

query compile 목표는 lexicon을 미리 로드한 같은 analyzer를 재사용하고 다음 세 입력을 각각
`query_compile/single_atom`, `query_compile/phrase_8_atoms`와
`query_compile/disjunction_8_atoms` Criterion benchmark로 측정한다.

```text
single_atom: 걷다
phrase_8_atoms: n:사용자 n:권한 v:검증하다 adj:예쁘다 det:새 adv:빨리 n:기술 v:걷다
disjunction_8_atoms: n:사용자|n:권한|v:검증하다|adj:예쁘다|det:새|adv:빨리|n:기술|v:걷다
```

`matcher/build_and_find_short`는 미리 compile한 다중 앵커 단일 atom plan으로 짧은 문장 하나를
검색한다. 각 iteration에서 matcher를 새로 만들어 one-shot build와 첫 검색을 함께 측정한다.
`matcher/scan_deterministic_corpus`는 같은 matcher를 충분히 큰 corpus에 재사용해 adaptive
automaton 승격 이후의 scan 회귀를 감시한다. 두 workload를 함께 비교해 짧은 입력의 build 비용을
줄이면서 대규모 scan을 희생하지 않았는지 판정한다.

`matcher/disjunction_find_all`은 같은 고정 corpus를 `lit:걸어|lit:사용자는`로 검색해 모든
line에서 두 alternative 중 하나를 반환한다. Alternative별 matcher나 반복 scan으로 분리하지 않고
하나의 logical atom과 anchor engine으로 전체 corpus를 한 번 순회하는지 감시한다.

`matcher/phrase_find_all`은 1,024개 line 중 4개마다 `n:길 v:걷다`가 일치하는 고정 corpus를 메모리 입력으로 사용한다. smart boundary의 component 검증에 필요한 고정 resource를 matcher 생성 시 제공한다. 전체 phrase match를 반환하는 한 번의 호출을 측정해 match 수에 따른 반복 anchor scan과 span 결합 회귀를 감시한다.

`matcher/phrase_find_all_repeated`는 같은 한 음절 literal atom 8개와 한 줄의 반복 span 128개,
큰 `max-gap`을 사용한다. 가능한 조합 수와 무관하게 bounded DP로 leftmost-longest 결과를 찾는
병적 입력 경로를 측정한다. Phrase 선택은 candidate endpoint의 byte offset, Unicode scalar 수와
line-break 수를 한 번 인덱싱하고, successor 비교마다 원문이나 endpoint index를 다시 탐색하지
않는다.

`matcher/phrase_input_searcher_repeated_line`은 줄바꿈 없는 한 줄에서 인접한 두 literal atom
phrase가 4,096번 반복되는 입력을 `InputSearcher`의 metadata 출력 경로로 검색한다. 한 줄의
anchor와 atom span을 한 번만 수집하는지와 match 수에 따른 반복 suffix scan 회귀를 감시한다.
같은 입력의 `matcher/phrase_input_searcher_repeated_line_exists`는 metadata를 수집하지 않는
summary 경로를 측정해 line 평가 전달 비용이 존재 판정 경로를 악화시키지 않는지 감시한다.
`matcher/phrase_input_searcher_missing_atom_long_line`은 `lit:가 lit:나` 중 첫 atom만 반복되는
1 MiB 단일 줄을 summary 경로로 검색한다. 결과가 불가능한 줄에서 모든 atom의 raw anchor coverage를
먼저 판정해 verifier와 atom span 적재를 건너뛰는지 감시한다. 이 workload의 wall time과 maximum
RSS를 함께 비교하며, 모든 atom이 존재하는 줄의 phrase 선택 메모리 상한을 증명하는 근거로는
사용하지 않는다.
`matcher/phrase_input_searcher_sparse_tail_long_line`은 1 MiB 단일 줄의 끝에 둘째 atom을 한 번 넣어
raw coverage prefilter를 통과시키고 실제 match 하나를 만든다. Max-gap 밖의 첫 atom candidate를
active state에서 제거해 줄 전체 검증 span을 적재하지 않는지 wall time과 maximum RSS로 확인한다.

`matcher/context_repeated_long_line`은 `매일`이 16,384번 반복되는 줄바꿈 없는 UTF-8 입력을
`RepeatedToken + MAG` 구조 pattern을 가진 `smart` 부사 matcher로 검색한다. 각 candidate의
인접 token만 해독하는지와 candidate마다 전체 입력의 UTF-8을 다시 검증하는 회귀를 감시한다.
`matcher/context_alternating_spacing_long_line`은 같은 token 사이의 공백을 1바이트와 2바이트로
교대해 문맥 형태가 둘인 경로의 비용을 감시한다. 이 두 workload는 준비 context cache가 warm
hit인 반복 표본이므로 cache miss 비용의 근거로 사용하지 않는다.
`matcher/context_constant_neighbors_long_line`과
`matcher/context_unique_neighbors_long_line`은 byte 수와 match 수가 같은 `가 매일 나` 형태의
입력을 사용한다. 전자는 같은 앞뒤 token을 반복하고 후자는 매 candidate의 앞뒤 한글 token 쌍을
바꿔 raw context를 모두 고유하게 만든다. 두 결과를 함께 비교해 cache hit 편중과 miss 비용을
보고한다.
`matcher/context_unique_current_long_line`은 anchor 뒤에 서로 다른 한글 token suffix를 붙여
현재 token 자체를 모두 다르게 만들고 구조 후보를 모두 거부한다. Query에서 확정한 exact
whole-token graph의 이득을 고유 인접 token과 함께 확인하되, 이 miss 대조군의 비용이나 matcher
생성·첫 검색 비용을 악화시키지 않는지 별도로 판정한다.
단일 atom `find_all` 구조 검색은 같은 호출 안에서 동일한 bounded raw context의 준비된 구조 분석을
재사용할 수 있다. Cache key는 raw context bytes, 해당 context 안의 current token 상대 span,
node limit과 nominal-copula 포함 여부를 모두 구분하고 hash collision은 원본 값 비교로 확인한다.
각 일치의 raw·NFC span mapping은 다시 계산하며, 준비 context cache는 256개로 제한한다. 반복
context와 고유 context benchmark를 함께 사용해 내용 반복에 편중된 결과를 제품 성능으로
일반화하지 않는다.

`local_lattice/component_decision`은 고정 component fixture를 한 번 초기화한 뒤 accept, reject와
ambiguous 입력을 순환하며 제품용 component 판정만 측정한다. `local_lattice/component_report`는
같은 입력에서 진단 경로 생성 비용을 별도로 측정한다. 구현 변경 전후를 같은 build profile과
Criterion 설정으로 비교하고, 제품 판정 p95가 10% 이상 악화되면 회귀로 본다. 이 microbenchmark는
1,000-case morphology 품질·성능 보고서를 대체하지 않는다.

`structural_constraint/prepare_dense_token_graph`는 63개 음절 token의 모든 접두 surface에 두
분석을 등록해 node 상한 바로 아래인 4,032개 edge를 만든다. 매 iteration에서 token graph를 새로
준비해 matcher나 context cache가 개입하지 않게 하고, 구조 상태기계가 시작 위치별 edge index를
공유해 `token byte 수 × 전체 edge 수` 반복 scan으로 돌아가지 않는지 감시한다. 공통 nominal
prefix, ending suffix와 predicate-connective 경계도 token 준비당 한 번만 계산한다. 일반적인 짧은
구조 판정과 morphology 제품 workload를 함께 측정해 최악 입력 개선을 일반 입력 회귀와 바꾸지
않는다.

`structural_constraint/prepare_dense_unique_pos_token_graph`는 같은 edge 수를 유지하되 모든 분석의
POS 문자열을 고유하게 만든 대조군을 측정한다. Resource decode는 측정 밖에 두고 반복 POS workload와
함께 비교해 token 준비 비용 개선이 문자열 반복이나 비정상적으로 높은 cache·intern hit율에 의존하지
않는지 확인한다. Resource decode 비용과 상주 메모리는 morphology startup 측정으로 따로 검증한다.

`structural_constraint/resolve_dense_preferred_paths`는 같은 63개 음절에 단일 음절 particle 분석을
더해 node 상한 바로 아래인 4,095개 edge와 다수의 동일 비용 명사 경로를 만든다. 준비된 token
graph에서 서로 다른 16개 component 후보를 매 iteration 순환해 최소 unit 경로 판정만 측정한다.
후보별 임시 graph나 unit 전체 재검색을 줄이는 변경은 이 workload와 graph 준비 비용을 함께
보고해, 준비 비용이나 보존 메모리를 후보 판정 개선과 맞바꾸지 않는다.

`structural_constraint/reject_ambiguous_particle_suffix_*`는 모든 접두 surface가 particle인 반복
suffix 뒤에 미등록 문자를 붙여, 완성 경로가 없는 입력의 탐색 비용을 측정한다. 12개와 20개 반복
입력을 분리해 크기 증가에 따른 비용도 비교한다. 같은 suffix 위치를 재귀적으로 다시 탐색하거나
호출 stack을 입력 분기 수만큼 늘리지 않아야 하며, 결과 cache 없이 매 iteration 실제 거부 판정을
수행한다.

`structural_constraint/select_dense_nominal_particle_facts`는 모든 접두 surface가 nominal과
particle 분석을 함께 갖는 token에서 준비된 token graph를 공유하고 구조 선택만 다시 수행한다.
`structural_constraint/prepare_dense_nominal_particle_context`는 같은 입력의 graph 생성과 구조 선택을
모두 수행한다. 두 workload를 함께 비교해 선택 자료구조의 반복 사용 이득과 생성 비용을 분리하며,
공유 graph 표본만으로 전체 준비 성능을 일반화하지 않는다.

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
