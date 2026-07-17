# 비표준·오타·띄어쓰기 입력 평가 계약

이 문서는 [robustness 실행 설계](noisy-text-robustness-plan.md)의 corpus, 품질 지표와 외부
라이브러리 비교 방법을 정의한다. 현재 canonical morphology benchmark와 제품 기준선은 그대로
유지한다.

## 평가 corpus

수동 검토를 통과한 UD Korean-Kaist fixture는 표준 형태 회귀를, Korean-KSL과 core 검토에서
제외한 문장은 annotation-required robustness 후보를 담당한다. 후보 문장은 query-level gold가
아니므로 품질 합계에 넣지 않는다. 25-case 현실 기술 코퍼스는 blind 진단을 담당한다. 현재
현실 기술 코퍼스의 User `spacing-error` 5건은 TP 1 / FN 4이지만 사례 수와 negative가 부족해
robustness threshold나 규칙 선택에 쓰지 않는다.

Korean-KSL 후보의 실행 비용은 품질과 분리한 performance-only 기준선으로 측정한다. 제품
robustness mode가 구현되기 전에는 `off`만 실행하며 명시적 품사와 무품사 500-case의
initialization, cases/s, p50·p95 latency, peak RSS만 보고한다. Query-level annotation이 없는
core 제외 문장은 이 workload에 넣지 않는다.

별도 `noisy_text` fixture는 다음 두 층으로 만든다.

### 자연 원문

- 재사용이 허용된 인터넷 원문에서 300~500건의 pilot을 먼저 만든다. 공개 열람 가능성만으로
  수집하지 않고 URL, 고정 revision, license, 원본 SHA-256과 excerpt 위치를 manifest에 둔다.
- 원문 문자열을 교정하거나 정규화해 저장하지 않는다. query, coarse/fine POS, expected,
  gold raw byte span과 오류 class를 사람이 검증한다.
- 한 source document의 사례는 split 사이에 나누지 않는다. 동일 게시자·복제 문서와 같은 rule
  family도 dev와 held-out test에 걸치지 않는다.
- pilot은 rule·비용·상한 개발에만 사용한다. held-out 자연 test는 정책과 threshold를 고정한 뒤
  한 번 측정한다.
- 자동 형태소 분석이나 모델 제안은 후보 수집에만 쓸 수 있다. gold에는 수동 검증 여부와 보조
  도구를 기록한다.

### 통제된 paired 변형

- 표준 positive에서 오류 class 하나만 deterministic하게 적용해 canonical/noisy pair를 만든다.
- 각 pair에는 같은 edit가 다른 lemma/POS 또는 비대상 span을 열지 않아야 하는 matched negative를
  둔다.
- synthetic case는 class별 coverage와 회귀 재현에 쓰고 자연 원문의 품질 점수를 대신하지 않는다.
- 자연 원문에서 관찰하지 않은 변형은 제품 규칙 채택 근거로 사용하지 않는다.

fixture row는 다음 필드를 갖는다.

```text
id, source_id, document_id, text, query, coarse_pos, fine_pos, expected,
gold_byte_start, gold_byte_end, noise_origin, noise_class, paired_case_id,
transform_id, canonical_text, annotation
```

positive의 gold span은 query의 canonical 문자열이 아니라 원문에서 해당 의미를 실현한 전체 raw
surface다. correction 때문에 길이와 공백이 달라져도 span을 보정된 문자열 기준으로 바꾸지 않는다.

## 품질 지표

모든 수치는 자연·synthetic, 오류 class, source와 품사별로 나누어 보고한다. micro 합계는 함께
기록하되 하나의 종합 점수나 전체 backend 순위를 만들지 않는다.

| 지표 | 정의 | 목적 |
| --- | --- | --- |
| noisy recall | `TP / (TP + FN)` | 비표준 positive 복구율 |
| robust-only precision | `robust-only TP / (robust-only TP + robust-only FP)` | mode를 켜서 새로 연 결과의 신뢰도 |
| over-acceptance rate | `robust-only FP / negative` | 비문 허용이 만든 과검색률 |
| canonical retention | `robust on에서도 맞은 off의 canonical TP / off canonical TP` | 정상문 회귀 방지 |
| paired recovery | `canonical과 noisy가 모두 TP인 pair / off canonical TP pair` | 같은 의미의 변형 복구 |
| exact raw-span rate | `gold와 byte span이 같은 TP / TP` | 원문 위치 보존 |
| overlap raw-span rate | `gold와 겹치는 TP / TP` | adapter 진단 |
| provenance accuracy | `class·rule·cost가 gold와 맞는 robust-only result / robust-only result` | 복구 이유의 감사 가능성 |
| candidate amplification | `robust candidate / off candidate`의 p50·p95·max | bounded 실행 확인 |

전체 precision·recall·F1과 TP·FP·TN·FN도 저장한다. 그러나 높은 canonical TP가 robust-only FP를
가리는 문제를 피하기 위해 채택 판단은 robust-only precision과 over-acceptance를 먼저 본다.
span 평가는 overlap을 품질 성공으로 보고할 수 있지만 제품 채택에는 exact raw span을 요구한다.

각 비율에는 case 수와 95% Wilson confidence interval을 함께 기록한다. `off`와 robust의 같은
case별 recall 차이는 paired bootstrap 95% confidence interval로 계산한다. 분모가 0인
robust-only precision은 100%가 아니라 `N/A`로 표시한다. class별 자연 positive가 30건보다
적으면 수치는 진단으로만 표시하고 채택 근거로 쓰지 않는다. negative는 같은 오류 family의
유사 surface, 동일 lemma의 다른 품사와 정상 띄어쓰기를 포함한다.

### 채택 gate

- `off`는 현재 fixture의 case ID, match span, query plan과 결과 checksum이 동일해야 한다.
- clean dev/test의 canonical TP를 잃지 않고 명시적 품사 `smart` precision 99.00% 하한과
  hard-negative 신규 FP 0을 유지해야 한다.
- `conservative`의 자연 held-out robust-only precision은 95% Wilson 하한이 95% 이상이어야 한다.
- 자연 held-out over-acceptance rate의 95% Wilson 상한은 1% 이하여야 한다.
- noisy recall 개선 폭의 paired bootstrap 95% 하한이 0보다 커야 하며, 보고 가능한 어느
  class에서도 유의한 하락이 없어야 한다.
- 채택된 TP의 exact raw-span rate와 provenance accuracy는 100%여야 한다.
- morphology와 100 MiB CLI workload의 p95가 같은 설정의 `off`보다 10% 이상 느려지면 회귀로
  판정한다. 초기화, cases/s와 peak RSS의 median/min/max도 별도로 기록한다.

pilot이 이 표본 수와 confidence gate를 만족하지 못하면 사례를 더 모으거나 해당 class를
`exploratory`로 남긴다. threshold를 통과시키기 위해 held-out gold, negative 또는 오류 class를
변경하지 않는다.

### `아니다면` family 측정

`그 답이 아니다면 다시 검토한다.` 한 건은 재현 seed일 뿐 품질 표본이 아니다. 다음 slice를
같이 고정한다.

| slice | 기대 결과 |
| --- | --- |
| 서로 다른 source의 `아니다면` 의도 용례 | `아니다/VCN` robust TP와 exact raw span |
| 표준 `아니라면`·`아니면` | canonical TP, robust provenance 없음 |
| 표준 형용사 `-다면` | 기존 canonical 결과와 span 유지 |
| 동작 용언, 별도 token과 더 큰 lexical token의 유사 surface | 해당 VCN rule로 새 match 없음 |
| 같은 source·주제의 matched negative | robust-only FP 없음 |

보고서는 이 family의 noisy recall, robust-only precision, over-acceptance, canonical retention과
exact raw-span rate를 독립 행으로 낸다. `아니라면`의 canonical FN은 noisy recall에 넣지 않고
기존 `surface-missing` 또는 `continuation-rejected` 분류에 남긴다. 자연 positive 30건과 전체
채택 gate를 확보하지 못하면 해당 rule은 `exploratory`에만 둔다.

## 외부 라이브러리 비교

비교 task는 모든 도구에서 동일한 `(원문, query lemma, POS, gold raw byte span)` 존재 여부다.
형태소 tokenizer 자체의 전체 정확도나 교정문의 자연스러움을 kfind 검색 결과와 합치지 않는다.

### 비교 행

기존에 고정한 Kiwi, Lindera, MeCab-ko, KOMORAN adapter를 재사용하고 다음 두 표를 분리한다.

1. **default 비교**: 각 분석기의 고정 기본 설정과 `kfind off`를 실행한다. 비표준 원문을 별도
   전처리하지 않은 out-of-box 기준이다.
2. **feature-matched 비교**: 비표준 활용·띄어쓰기는 `kfind conservative`, 오타·반복은 해당
   class만 켠 `kfind exploratory`와 분석기가 공식 제공하는 가장 가까운 robustness 옵션을
   class별로 실행한다.

고정 Kiwi 0.23.2에는 공식 API의 `typos`, `typo_cost_threshold`, `normalize_coda`와
`space_tolerance`가 있다. 따라서 Kiwi는 default, typo, spacing, coda와 combined 설정을 별도
행으로 두고 preset, threshold와 허용 공백 수를 snapshot에 고정한다. 한글 오타 1회나 공백
1회처럼 kfind의 후보 예산과 같은 class에서만 feature-matched로 비교한다. 공식 근거는
[kiwipiepy API 문서](https://bab2min.github.io/kiwipiepy/)와
[kiwipiepy 저장소](https://github.com/bab2min/kiwipiepy)에 고정한다.

다른 고정 분석기는 해당 버전의 공식 문서에서 동일 class와 원문 span 동작을 확인한 native
옵션만 feature-matched 행에 넣는다. 확인하지 못한 설정은 default 행만 둔다. 사용자 사전, 별도
spell checker나 공백 교정기를 앞에 붙인 결과는 `preprocessor + backend` 행으로 분리하고
전처리 초기화·실행 시간과 span 역매핑 실패를 모두 포함한다. 이를 원래 backend의 native
robustness로 표시하지 않는다. 비표준 활용을 위해 case별 사용자 사전을 주입하는 설정도 기본
라이브러리 비교에서 제외하고 별도 custom-rule 실험으로만 기록한다.

### 공정성 계약

- 모든 설정은 같은 noisy fixture 순서, query/POS와 gold를 사용한다. backend 출력을 본 뒤 query,
  split, negative 또는 오류 class를 바꾸지 않는다.
- 버전, 모델·사전, robustness preset, threshold, thread 수, adapter schema와 container digest를
  고정한다. fixture나 설정이 바뀌면 모든 관련 snapshot을 명시적으로 갱신한다.
- 외부 분석 결과는 현재 adapter처럼 lemma/POS와 원문 byte span으로 정규화한다. correction 뒤
  원문 span을 복원할 수 없는 결과는 TP로 간주하지 않고 `span-unmapped`로 기록한다.
- 각 설정은 fresh process에서 warm-up 1회 뒤 5회 실행한다. 초기화, 전체 evaluation, cases/s,
  p50·p95 latency와 peak RSS를 같은 schema로 저장한다.
- robust 전처리와 typo model 비용은 초기화와 evaluation에서 제외하지 않는다. kfind도 query
  compile, robust candidate 생성과 검증을 모두 포함한다.
- 자연 원문과 synthetic pair, default와 feature-matched, 문법·띄어쓰기·오타 class를 서로
  합산해 하나의 순위를 만들지 않는다.

결과는 class별 noisy recall 대 over-acceptance, p95 latency와 peak RSS의 Pareto 표로 제시한다.
특정 분석기를 모두 이기는 것을 채택 gate로 두지 않는다. 같은 정밀도 영역에서의 복구율과
비용 차이를 보여 주고, 지원하지 않는 class는 0점이 아니라 `not supported`로 표시한다.
