# 형태소 검색 개선 핸드오프

기준 보고서: [2026-07-12 형태소 비교 분석](2026-07-12-morphology-comparison.md)

완료한 P2 계획: [선택적 국소 형태 추론](2026-07-12-selective-morphology-plan.md)

비용 분석: [local lattice 비용 분석](2026-07-12-lattice-cost-analysis.md)

다음 계획: [recall 80% 작업 계획](2026-07-13-recall-80-plan.md)

component 근거: [smart component 검색 근거](2026-07-13-smart-component-evidence.md)

fixture SHA-256: `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff`

## 현재 상태

- P0 context 계측과 P1 packed Double-Array 선택 완료
- P2 source 분석 보존형 morphology resource schema 3 완료
- P2 bounded 어절 추출과 NFC 원문 offset mapping 재구성 완료
- P2 local lattice·N-best shadow report 완료
- P1 일반 양보 연결형 `-더라도` 완료
- P0 nominal component shadow 계측 완료

- kfind embedded profile: F1 82.94%, recall 71.00%, precision 99.72%
- 품질 순위: Kiwi 92.01% > Lindera 88.02% > kfind 82.94%
- kfind 비용: 15,664.6 cases/s, p95 0.1449 ms, peak RSS 5.0 MiB (5회 median)
- kfind 오류: FN 145, FP 1
- 가장 큰 FN 영역: 명사 71, 동사 31, 형용사 25

benchmark runner는 embedded/full-POS profile을 같은 fixture에서 비교한다. full-POS의
생산적 용언 활용을 보존하도록 수정한 뒤 test split에서 recovered 0, regressed 0이며 두
profile의 FN은 145개다.

## 2026-07-12 진행 결과

- 초기 기준 FN 147개에 `primary_cause`와 판정 근거를 자동 기록한다: `boundary-rejected` 67,
  `continuation-rejected` 2, `gold-or-adapter` 23, `lexicon-missing` 50,
  `span-mismatch` 3, `surface-missing` 2.
- dev split을 별도 고정했다. ㅂ 불규칙 형용사 `가볍다`, `무겁다`, `무섭다`, `아쉽다`,
  `쉽다`, `춥다`를 dev 근거로 보강해 embedded recall이 70.60%에서 72.00%로 올랐고 test
  baseline은 변하지 않았다.
- 5개 slice, 10개 hard negative를 별도 metric으로 기록한다. embedded는 7 TN, 3 FP다.
- 기본 성능 측정은 1회 warm-up 뒤 5회 실행하고 median/min/max를 기록한다. CI는 28개 dev
  smoke case를 실행한다.
- VCP 지정사 FP는 [homonym union 정책](2026-07-12-copula-boundary-plan.md)을 유지하고 matcher를
  변경하지 않기로 확정했다.
- full POS artifact는 632,667개 entry와 614,794개 고유 표제어를 포함한다. dev의
  `lexicon-missing`은 embedded 38건, full-POS 0건이다.
- `-며/으며` 연결형을 보강해 dev TP가 360에서 361로 늘었고 recall은 72.20%다. test와
  hard-negative 결과는 변하지 않았다.
- `하다` 계열의 비축약 `하여/하였다`를 보존해 dev의 `의하여`, `대하여`를 회복했다. dev
  TP는 361에서 363, recall은 72.20%에서 72.60%로 늘었고 용언 `continuation-rejected`는
  6건에서 4건으로 줄었다. test와 hard-negative 결과는 변하지 않았다.
- 어간에 직접 붙는 `-기` 명사형을 추가해 dev의 `무너지기`, `있기`, `살아남기`를
  회복했다. dev TP는 363에서 366, recall은 72.60%에서 73.20%로 늘었고 test TP도 FP 증가
  없이 353에서 354로 늘었다. hard-negative 결과는 변하지 않았다.
- 동작 용언의 의도 연결형 `-(으)려고`를 보강해 dev의 `꾀하려고`를 회복했다. dev TP는
  366에서 367, recall은 73.20%에서 73.40%로 늘었고 `continuation-rejected`는 1건에서
  0건이 됐다. test와 hard-negative 결과는 변하지 않았다.
- 과거 선어말어미 뒤 관형형 `-을` 전이를 추가해 dev의 `불렀을`과 띄어 쓴 `좋았을 텐대`를
  회복했다. dev TP는 367에서 369, recall은 73.40%에서 73.80%로 늘었다. 붙여 쓴
  `좋았을텐데요`는 smart boundary가 계속 거부하며 test와 hard-negative 결과는 변하지 않았다.
- 진행 방향 보조 용언 `-아/어가다`의 `가고`, `가야` 연쇄를 추가해 dev의 `망해가고`,
  `만들어가야`, `넓혀가고`를 회복했다. embedded·full-POS TP는 369에서 372,
  recall은 73.80%에서 74.40%, 동사 recall은 79.17%에서 81.67%로 늘었다. test와
  hard-negative 결과는 변하지 않았다.
- 과거형 뒤 의문 종결형 `느냐`와 주제 보조사 `는` 연쇄를 추가해 dev의 `했느냐는`을
  회복했다. embedded·full-POS TP는 372에서 373, recall은 74.40%에서 74.60%, 동사
  recall은 81.67%에서 82.50%로 늘었다. test와 hard-negative 결과는 변하지 않았다.
- 이유 연결형 `-(으)니`와 전망 인용 연쇄 `-(으)리라고`를 추가해 dev의 `바쁘니`,
  `얻으리라고`를 회복했다. embedded·full-POS TP는 373에서 375, recall은 74.60%에서
  75.00%로 늘었고 precision 99.47%를 유지했다. test와 hard-negative 결과는 변하지 않았다.
- 양보 연결형 `-더라도`를 추가해 dev와 test의 `죽더라도`를 각각 회복했다. dev TP는
  375에서 376, recall은 75.00%에서 75.20%로 늘었다. test TP는 354에서 355, recall은
  70.80%에서 71.00%, F1은 82.81%에서 82.94%로 늘었고 precision 99.72%와
  hard-negative 결과를 유지했다.
- 현재 `-기` branch는 token 경계에서 끝나므로 `걷기가`, `걷기를`처럼 명사형 뒤에 조사가
  붙은 어절은 찾지 않는다. nominalizer에서 nominal particle verifier로 전이하는 작업은
  별도 후속 범위다.
- MeCab의 문맥용 지정사 표면형 14개를 표제어 후보에서 제외했다. `보이다`는 동사·보조 동사
  분석만 보존하고, 비정규 VCP stem은 형태 생성 전에 거부한다.
- smart VCP 지정사 branch를 `EojeolLattice` 대상으로 표시했다. token·any와 literal 경로는
  대상이 아니며 기본 union 결과는 변하지 않는다.
- shadow report schema 4는 case별 raw anchor hit, verifier 통과 branch hit, local 대상,
  원문·NFC span, 포함·미포함 최저 비용, cost margin과 N-best 경로를 기록한다.
- query-side full POS와 corpus-side morphology index를 분리했다. morphology index는 같은 고정
  MeCab snapshot에서 원본 표면형·품사·좌/우 연결 ID·비용을 보존한다.
- 729,173개 표면형 비교에서 packed Double-Array trie를 P2 형식으로 선택했다. mmap peak RSS는
  28.1 MiB였고 FST보다 exact lookup은 약 6배, common-prefix 열거는 약 4.3배 빨랐다.
- morphology index container는 schema, source SHA-256, 통계, section 길이와 SHA-256을
  검증하며 손상·schema·source 불일치를 구분한다. 기본 검색 결과는 변하지 않는다.
- 지정사 판별 fixture `1e06951581c84f02a4013e8410c113337c1389d3dcc2028b322f887bb181b494`에
  canonical gold 1,601건과 surface cue 음성 1,315건을 고정했다. 비정규 `VCP=있` 1건은
  양성으로 승격하지 않고 제외 사유로 기록한다.
- kfind embedded/full-POS는 지정사 slice에서 TP 1,033, FP 78, TN 1,237, FN 568로 동일하다.
  precision은 92.98%, recall은 64.52%다.
- KSL VCP는 precision 85.43%, recall 56.57%로 가장 약하다. Kiwi는 96.28%/97.05%,
  Lindera는 89.43%/97.59%다.
- `EojeolLattice` 대상은 1,226개 case의 1,740개 hit이다. 현재 union 결과는 유지하며 이
  baseline을 P2 lattice path 판별력 평가에 사용한다.
- `kfind-data`의 corpus-side resource를 schema 3으로 갱신했다. resource는 773,105개
  NFC 표면형, 815,725개 source 분석, 3,822×2,693 연결 비용 행렬, 모든 문자 class와
  미등록어 분석을 보존하며 SHA-256은
  `50bbaa64b06a080c7fa09c13e21090388a1c0f5109ed413546e0004ce7794f23`다.
- schema·source·section digest, payload·context 범위 검증 실패를 구분한다. CLI와 matcher는
  아직 resource를 로드하지 않으므로 union 검색 결과는 변하지 않는다.
- `AnalysisWindow`는 검증 target 주변의 Unicode token을 최대 256 raw bytes와 64 NFC
  scalar로 제한하고 원문·NFC의 안정된 byte 경계를 양방향 매핑한다. UTF-8 오류와 상한 초과는
  명시적 오류다. lattice shadow는 성능 측정 뒤에 실행되며 검색 결과를 바꾸지 않는다.
- 1,740개 lattice candidate를 모두 평가해 `accept` 1,515개, `reject` 225개를 얻었다.
  오류와 `ambiguous`는 없다.
- gold target 1,007개 중 957개를 수용하지만 non-gold target 733개 중 175개만 거절한다.
  non-gold reject 비율 23.87%로 제품 판정에는 부족해 P3는 보류한다.
- [1 GiB low-hit 보고서](2026-07-12-1gib-mixed.md)는 kfind와 rg 모두 0.0470초,
  throughput 21,787.23 MiB/s, kfind RSS 7.23 MiB로 v0.1 게이트를 통과했다.
- blind 평가는 UD Korean-GSD r2.18 test split으로 확정했다. CC BY-SA 4.0 source와 license
  digest, 전수 선택·정렬 규칙, 기존 Kaist·KSL dev/test와의 NFC 문장 중복 0건을 스펙에
  고정했다. fixture는 781개이며 SHA-256은
  `4be12e060c4bc3faf35b78bb3c9189cafb49e7c885108383c0dd1fb5aeb1b188`이다.
- manifest schema 3에서 기본 benchmark source와 blind source를 분리했다. Docker build는
  blind fixture와 metadata를 생성·검증하지만 기본 runner에는 전달하지 않는다. smoke
  benchmark는 기존 dev local-context 결과만 평가한 채 통과했다.
- [Korean-GSD blind 평가](2026-07-13-copula-blind-evaluation.md)를 최초 1회 실행했다. 중복
  제거한 candidate에서 gold accept는 127/142, non-gold reject는 97/101이다. 정상 gold
  reject가 최소 13개 남아 P3는 계속 보류한다. fixture는 regression baseline으로 전환했다.
- nominal component shadow는 경계에서 거부된 명사 branch만 별도 수집한다. exact node를
  포함한 완전 경로와 제외한 완전 경로의 최저 비용을 비교한 결과 dev 고유 accept는 embedded
  61건, full-POS 65건이다. revised hard-negative 5개 candidate는 두 profile에서 모두
  reject했다. test TP 355/FP 1/FN 145와 dev TP 376/FP 2/FN 124는 변하지 않았다.
- component report schema 6은 candidate·window 수, case별 exact path provenance와 고유
  accept/reject case 수를 VCP lattice 지표와 분리한다. component candidate가 있는데 morphology
  resource가 누락·손상·source 불일치이면 benchmark를 실패시킨다.
- component report schema 7은 case별 최저 비용 accept/reject 경로를 분류한다. embedded accept
  61개는 derivational 23, nominal compound 22, particle 8, copular 7, mixed 1이며 P1 일반 규칙
  후보는 derivational 23개다. reject evidence 16개는 positive 14개 case에 걸쳐 있다.
- compact lattice projection은 고정 source의 exact/common-prefix analysis hit와 scoring
  checksum을 full resource와 동일하게 유지한다. artifact는 47,859,711 bytes로 full의
  66.32%이며 mmap peak RSS 49.47 MiB, 초기화 138.60~139.14 ms로 배포 비용 gate를 통과했다.
- report schema 8은 같은 component candidate의 full/compact decision, 비용, node 수와 N-best
  provenance를 비교한다. embedded test/dev/hard-negative 84/87/5건과 full-POS 123/115/8건은
  모두 일치했다. compact artifact SHA-256은
  `5fc46a151e41485dc4b4a3a931135c0f490913f2c2c908b9d87adb87a7c14efd`다.

dev 명사 FN 70개 중 64개는 사전 누락이 아니라 smart boundary 거부다. 합성어 substring
계약을 완화하면 hard-negative 정밀도와 충돌하므로 이번 어휘 보강에는 포함하지 않았다.
[명사 경계 계획](2026-07-12-nominal-boundary-plan.md)에서 별도 mode와 복합어 resource
선택지를 분리했다.

## 재현

```console
scripts/benchmark-morphology.sh
```

산출물:

- `target/morph-benchmark/report.json`: 모든 metric, 실패 case, match span
- `target/morph-benchmark/report.md`: 사람이 읽는 요약
- `docs/benchmarks/assets/morphology-quality.svg`: 품질 차트
- `docs/benchmarks/assets/morphology-performance.svg`: 성능 차트

입력 source, SHA-256, quota, seed는 `tools/morph-compare/sources.json`에 있다. Docker image
빌드 뒤 실제 평가는 `--network none`으로 실행된다.

## 작업 순서

### P0. 비교 profile을 분리한다 (완료)

목표: 사전 coverage 부족과 matcher 규칙 실패를 같은 FN으로 취급하지 않는다.

1. runner에 `embedded`와 `full-pos` kfind profile을 추가한다.
2. report의 version/profile metadata에 lexicon artifact SHA-256을 기록한다.
3. 동일 fixture에서 두 profile의 TP/FN, 초기화, RSS를 함께 출력한다.
4. full-POS에서 회복되는 case와 그대로 실패하는 case를 별도 목록으로 저장한다.

완료 조건:

- 같은 report에서 embedded/full-POS 결과가 명시적으로 구분된다.
- full-POS artifact가 없으면 조용히 embedded로 대체하지 않고 실패한다.
- profile별 fixture·case 순서가 동일하다.

### P0. VCP 지정사 false positive 정책을 확정한다 (완료)

case:

```text
query: 이다/adjective
text: 매일 아러바이트가도 있습니다.
observed span: 매일의 마지막 음절
```

현재 query-side matcher가 가진 anchor와 인접 문자만으로 `매일`의 어휘 내부 `일`과
`학생일`, `책일`의 VCP 관형형 `일`을 구분할 수 없다. 두 조합형은 사전 표제어가 아니라
체언 host와 VCP 활용의 결합이다. v0.1.1은 homonym union을 유지하므로 matcher와 기본 검색
결과를 바꾸지 않는다. 실제 UD dev corpus의 VCP/VCN 분석으로 후속 품질을 측정한다.

완료 조건:

- 정책 선택과 제약이 [VCP 지정사 계획](2026-07-12-copula-boundary-plan.md)에 기록되어 있다.
- corpus 단어 denylist와 fixture 전용 branch를 추가하지 않는다.
- 기존 VCP/VCN positive와 `--boundary any` 계약을 유지한다.

### P0. 선택적 국소 형태 추론의 계약과 측정을 고정한다 (완료)

[새 작업 계획](2026-07-12-selective-morphology-plan.md)의 P0를 수행했다.

1. `boundary`와 `disambiguation`, union 기본값과 shadow 측정 범위를 스펙에 추가한다.
2. query branch의 context requirement를 표현하되 검색 결과는 바꾸지 않는다.
3. anchor hit와 lattice 대상 수를 측정하는 counter를 추가한다.
4. `학생일`, `책일`의 조합 회귀와 corpus 기반 VCP/VCN 정상형·어휘 내부 표면형·NFC/NFD
   fixture를 고정한다.

완료 조건:

- low-hit literal의 lattice 대상은 0이다.
- `매일`, `학생일`, `책일`의 branch 근거와 dev corpus의 모든 local 대상 판정 근거가
  report에 남는다.
- 기본 CLI와 union 결과가 변하지 않는다.

### P1. FN 147개를 원인별로 분류한다 (완료)

각 case에 다음 하나의 primary cause를 부여한다.

| 분류 | 판정 기준 |
| --- | --- |
| lexicon-missing | expected POS 분석이 query plan에 없음 |
| surface-missing | 분석은 있으나 gold 활용형 anchor가 생성되지 않음 |
| continuation-rejected | core anchor는 있으나 ending continuation이 거부됨 |
| boundary-rejected | 형태는 맞지만 smart boundary가 거부됨 |
| span-mismatch | 같은 lemma/POS를 찾았으나 gold 어절과 겹치지 않음 |
| gold-or-adapter | 세 도구가 모두 실패하거나 corpus 정규화가 의심됨 |

우선 113개 `kfind=false, Kiwi=true, Lindera=true` case를 분류한다. 세 도구가 모두 놓친
23개는 제품 규칙보다 gold/adapter audit를 먼저 수행한다. 분류 결과는 report의 failure
레코드에 기계 판독 가능한 필드로 남긴다.

### P1. 명사 coverage를 보강한다

명사 recall은 60.56%이고 FN은 71개다. full-POS profile 결과를 본 뒤 다음 순서로 처리한다.

1. full-POS로 회복되는 일반·고유·의존 명사를 분리한다.
2. full-POS에서도 실패하는 합성 명사는 left/right smart boundary와 particle continuation을
   확인한다.
3. corpus case를 core lexicon에 개별 추가하지 않는다. core 편입 기준을 만족하는 빈도·기능어만
   별도 제안한다.

개선 목표 후보는 dev split 명사 recall 80% 이상이다. 이 값은 합의 전 release gate가 아니다.

### P1. 용언 활용 실패를 보강한다 (진행 중)

동사 FN 33개와 형용사 FN 25개를 다음 slice로 나눈다.

- 규칙 활용 / 불규칙 활용
- 보조 용언
- 합성·파생 용언
- 관형형·연결형·종결형
- 학습자 오탈자

case별 surface를 일반 규칙으로 설명할 수 있을 때만 rule fixture로 승격한다. 특정 문장이나
특정 표제어만 통과시키는 예외 branch는 추가하지 않는다. dev split 목표 후보는 동사 recall
82%, 형용사 recall 78%이며 precision 하락은 2%p 이내로 둔다.

dev의 유일한 `continuation-rejected`인 `꾀하다 → 꾀하려고`를 근거로 동작 용언의 일반 의도
연결형 `-(으)려고`를 보강했다. 형용사 직접 결합은 허용하지 않으며 test baseline과
hard-negative 10건은 그대로다.

dev의 `불렀을` 1건과 `좋았을` 2건을 근거로 과거 선어말어미 뒤 관형형 `-을` 전이를
보강했다. 띄어 쓴 2건만 회복하고 다른 boundary 규칙은 완화하지 않았다.

지정사 dev slice의 FN 639개 중 `이+ㅂ니다` 분석은 Kaist 30개, KSL 94개다. 첫 지정사
coverage 단위는 기존 `ending.polite-declarative`로 표면에 `입니다`가 남는 높임 평서형만
생성한다. `겁니다`와 `-다/-라` 축약, local filtering은 별도 범위로 두며 union precision은
P1의 2%p 하락 guardrail을 지켜야 한다.

`입니다` branch는 dev에서 gold 71개를 회복하고 음성 2개를 추가했다. 한 음성은 `미오씨
입니다`를 분리하고 VCP를 VV로 둔 source 정렬·tag 이상이고, 다른 하나는 동사
`움직입니다`다. dev 지정사 precision은 92.68%에서 92.98%, recall은 60.09%에서 64.52%로
올랐다. blind regression도 TP 149→173,
FP 68→69로 precision 71.49%, recall 53.89%다. 새 blind gold candidate 24개는 lattice가
수용하고 `보입니다` 음성 1개는 거절했다. 기본 union과 P3 보류 정책은 유지한다.

dev의 `망해가고`, `만들어가야`, `넓혀가고`를 근거로 진행 방향 `-아/어가다`를
보강했다. corpus에서 확인된 `가고`, `가야` 연쇄만 소비하고 다른 boundary 규칙은
완화하지 않았다.

dev의 `했느냐는` 원문 분석 `하+었+느냐+는` (`pvg+ep+ef+jxt`)을 근거로 과거형 뒤
의문 종결형 `느냐`를 보강했다. bare `느냐`와 주제 보조사가 붙은 `느냐는` 두 연쇄만
소비하고 다른 조사나 추가 어미는 허용하지 않았다.

full-POS dev의 `바쁘니`, `얻으리라고`를 근거로 일반 `-(으)니`와 제한된
`-(으)리라고` 연쇄를 추가했다. core lexicon에 표제어를 추가하지 않았고, 후속
어미를 소비하지 않는 token 경계와 ㄹ 탈락·불규칙 교체를 fixture로 고정했다.

dev의 `죽더라도`를 근거로 일반 양보 연결형 `-더라도`를 보강했다. 어휘적 교체나 후속 어미
없이 사전 어간에 직접 붙는 token 완성형만 허용해 test의 동일 일반형도 회복했고 precision과
hard-negative 결과는 변하지 않았다.

### P2. benchmark의 판별력을 높인다 (완료)

현재 negative는 쉬워 precision 차이가 거의 나타나지 않는다. 도구 출력과 독립적인 규칙으로
다음 hard-negative slice를 추가한다.

- 동음이의어의 다른 품사
- 합성어 내부 substring
- 잘못 붙여 쓴 앞말+용언
- 표면형은 같지만 lemma가 다른 활용
- 한 음절 query의 왼쪽·오른쪽 boundary

기존 1,000개 baseline은 유지하고 hard-negative 결과를 별도 metric으로 보고한다.

### P2. 성능 측정을 반복 가능하게 만든다 (완료)

- backend별 1회 warm-up 뒤 최소 5회 반복한다.
- median, p95와 run 간 min/max를 기록한다.
- embedded/full-POS profile의 RSS와 처리량을 분리한다.
- CI에서는 작은 smoke set, 수동 benchmark에서는 전체 set을 사용한다.

## 데이터 누수 방지

현재 test failure 목록이 문서와 JSON에 노출되었다. 이후 구현을 이 목록에 맞춰 조정하면 이
test split은 더 이상 독립 검증 집합이 아니다.

- 규칙 개발과 threshold 선택은 Kaist/KSL dev split에서 수행한다.
- 현재 test split은 regression baseline으로만 유지한다.
- 품질 개선 주장에는 아직 보지 않은 source 또는 고정 blind subset을 추가한다.
- blind 결과를 확인한 뒤 case별 예외를 추가하지 않는다.

## 변경 시 필수 검증

```console
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo fmt --manifest-path tools/morph-compare/runner/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/morph-compare/runner/Cargo.toml \
  --all-targets -- -D warnings
cargo fmt --manifest-path tools/morph-index-benchmark/Cargo.toml -- --check
cargo clippy --locked --manifest-path tools/morph-index-benchmark/Cargo.toml \
  --all-targets -- -D warnings
cargo test --locked --manifest-path tools/morph-index-benchmark/Cargo.toml
scripts/benchmark-morphology.sh
scripts/benchmark-morph-index.sh
```

report의 fixture SHA-256, source hash, case 수, class/source/POS quota가 바뀌면 의도된 dataset
변경인지 먼저 확인한다. 품질 개선은 전체 F1만 보지 말고 POS별 recall, hard-negative
precision, initialization, p95, RSS를 함께 비교한다.

## 다음 작업

1. 기본 `smart` 결과를 바꾸기 전에 CLI·Rust/WASM API, explain/JSON과 배포 resource 실패
   정책을 스펙에 확정한다.
2. 정상 지정사 gold reject 13개는 별도 P3 범위에서 기존 Kaist·KSL dev 원인을 분류한다.
3. 다음 제품 판정용 unseen source를 결과 확인 전에 고정한다.

Korean-GSD 결과에 맞춘 비용·threshold·fixture 가중치 변경은 금지한다.

## 다음 세션 시작점

component shadow, source path 분류, compact projection 배포 비용과 shadow 판정 동등성 검증은
완료됐지만 아직 검색 결과를 바꾸지 않는다. 다음 작업은 CLI·Rust/WASM API, explain/JSON과
배포 resource 실패 정책 확정이다. 이 계약 전에는 matcher에 resource loader를 연결하지 않는다.
