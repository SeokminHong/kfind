# 비표준·오타·띄어쓰기 입력 robustness 설계

이 문서는 인터넷 원문에서 나타나는 비표준 활용, 오타와 불안정한 띄어쓰기를 검색하는 후속
작업을 설계한다. 현재 제품 동작이나 `specs/kfind.md`의 공개 계약은 바꾸지 않는다. 각 단계가
채택되면 구현 전에 기술 사양서, CLI·Rust·WASM API와 fixture 계약을 먼저 갱신한다.

## 문제와 경계

현재 `smart`는 표준 형태와 검증 가능한 component를 정밀하게 검색한다. 이 경계는 비표준
입력까지 표준 활용 규칙으로 인정하는 방식으로 넓히지 않는다. robustness는 다음 조건을 갖는
별도 검색 축이다.

- 기본값은 `off`이며 현재 query plan, match와 raw UTF-8 byte span이 그대로 유지된다.
- `--expand`, `--boundary`, `--pos`, Unicode 정규화와 독립적으로 적용한다. `any`는 문자열
  경계 정책이지 robustness 모드가 아니다.
- 표준 분석과 비표준 복구를 같은 rule ID로 기록하지 않는다. 같은 span에서는 표준 분석을
  우선한다.
- 원문을 교정해 반환하지 않는다. 검색 결과는 원문 byte span을 보존하고 비표준 복구의 class,
  rule ID와 비용을 provenance로 남긴다.
- 문장 전체 spell checker, 범용 형태소 분석기와 anchor 없는 edit-distance scan은 non-goal이다.

`그 답이 아니다면 다시 검토한다.`의 `아니다면`은 이 축의
`nonstandard-morphology` 후보가 될 수 있다. 이는 `아니다/VCN`의 표준 활용으로 승격하지
않는다. 표준형 `아니라면`을 놓치는 문제도 이 규칙으로 가리지 않고 canonical FN으로 따로
측정한다.

## 오류 분류

각 사례와 복구 branch는 다음 class 중 하나만 주 분류로 갖는다. 둘 이상의 변형이 있는 원문은
별도 `multi-noise` 진단 slice에 두며 첫 제품 단계의 규칙 선택에는 사용하지 않는다.

| class | 범위 | 예 |
| --- | --- | --- |
| `nonstandard-morphology` | 품사·어미 구조로 제한할 수 있는 비표준 활용 | `아니다면` |
| `spacing-merge` | 있어야 할 공백이 빠진 경우 | `설치한후` |
| `spacing-split` | 형태 내부에 공백이 들어간 경우 | `확 인한다` |
| `hangul-typo` | 한글 음절·자모의 1회 누락, 삽입, 치환, 전치 | `기여워요` |
| `orthographic-confusion` | 발음·표기 유사성으로 생긴 형태 혼동 | `되/돼`, `안/않` 계열 |
| `repetition` | 음절·자모 장음화와 반복 입력 | `지이인짜` |
| `punctuation-intrusion` | 형태 내부의 제한된 문장 부호 삽입 | 강조용 점·물결표 |
| `mixed-script-noise` | 한글과 숫자·라틴 문자·emoji가 붙은 경계 불안정 | identifier 인접 원문 |

`orthographic-confusion`은 같은 표면이 정상 어휘가 될 가능성이 높으므로 문맥 근거 없이
`conservative`에 넣지 않는다. `mixed-script-noise`는 기존 경계 fixture와 겹치는 사례를 먼저
제거한 뒤 새 오류만 분류한다.

## 제품 정책 후보

공개 옵션의 최종 이름은 구현 단계에서 사양으로 확정한다. 최초 검증은 다음 세 정책으로 한다.

| 정책 | 포함 범위 | 용도 |
| --- | --- | --- |
| `off` | 현재 canonical branch만 사용 | 기본값과 회귀 기준선 |
| `conservative` | 독립 근거와 정밀도 gate를 통과한 비표준 활용, 공백 1회 변경 | opt-in 제품 후보 |
| `exploratory` | `conservative`와 한글 오타·반복·문장 부호 1회 변경 | 진단과 평가 전용 |

benchmark runner는 정책 외에 class 하나만 켜는 내부 설정을 제공한다. 제품 옵션을 오류 class의
조합식으로 먼저 노출하지 않는다. 각 규칙은 다음 근거가 모두 있어야 `conservative` 후보가 된다.

1. 문법·사전 자료 또는 서로 독립적인 자연 원문에서 반복 확인된다.
2. 같은 surface가 오답이 되는 대조군을 bounded 근거로 구분할 수 있다.
3. 개발 fixture, 자연 원문 pilot과 hard-negative gate를 통과한다.
4. 문장 문자열이나 case ID가 아니라 품사·상태·변형 class로 일반화된다.

## 실행 구조

canonical plan을 바꾸는 대신 같은 query atom에서 제한된 robust branch를 추가한다.

```text
canonical query plan
  -> canonical branches
  -> opt-in robust branches
  -> shared anchor scan
  -> bounded raw candidate window
  -> canonical or robustness verifier
  -> raw span + provenance + cost
```

- query compiler는 `RobustnessRequirement`, class, rule ID, edit cost와 필요한 verifier 상태를
  typed plan에 기록한다.
- 비표준 활용은 canonical ending state와 분리한 data-driven registry에서 생성한다. 개별 문장을
  코드에 넣지 않고 lemma fine POS, 이전 상태와 관찰된 비표준 전이로 제한한다.
- 띄어쓰기는 원문을 전역 정규화하지 않는다. anchor 주변의 bounded window 안에서 공백 한 번의
  삽입 또는 제거만 탐색하고 변환된 scalar와 원문 byte offset의 양방향 map을 유지한다.
- 한글 오타는 표제어·품사·활용 anchor가 있는 후보에서만 한 번의 weighted edit를 허용한다.
  키보드 인접 치환, 자모 누락과 장음화는 비용표를 분리한다.
- 비표준 활용은 규칙이 만든 exact surface 조각, 띄어쓰기는 공백 양쪽의 exact 조각을 anchor로
  쓴다. 한글 오타 budget 1은 query surface를 두 exact 조각으로 나눠 적어도 하나가 보존되는
  후보만 local verifier로 보낸다. 최소 exact 조각 길이를 만족하지 못하는 짧은 query는 검증된
  confusion table의 유한 variant만 허용하거나 해당 class를 닫는다.
- canonical branch와 robust branch를 함께 실행한다. 파일 어디엔가 canonical match가 있다는
  이유로 다른 위치의 robust match를 버리지 않는다.
- 같은 raw span과 query atom이 겹치면 canonical, 낮은 edit cost, 좁은 class 순으로 deduplicate
  한다. 비용은 결과 유무를 대신하지 않고 중복 후보의 안정적인 선택에만 쓴다.
- 기존 256 raw bytes, NFC 64 scalars와 verifier node 상한을 넘지 않는다. 상한을 넘은 robust
  branch는 canonical 결과에 fallback하지 않고 해당 branch만 닫으며 진단 counter를 남긴다.
- anchor 없는 후보, 2회 이상 edit, 파일 전체 공백 제거, 무제한 variant fan-out은 허용하지 않는다.

robustness 결과의 JSON과 query 설명에는 최소한 다음 필드를 둔다. text 출력의 기본 형식은
바꾸지 않는다.

```json
{
  "robustness": {
    "class": "nonstandard-morphology",
    "rule_id": "...",
    "edit_cost": 1,
    "canonical": "아니라면"
  }
}
```

`canonical`은 설명용 해석이며 반환 span은 `아니다면`의 원문 범위다. 정확한 schema와 공개 여부는
구현 전에 Rust·CLI·WASM 호환성 계약으로 확정한다.

## 평가 계약

corpus, 품질·성능 gate와 외부 라이브러리 비교는
[비표준·오타·띄어쓰기 입력 평가 계약](noisy-text-robustness-evaluation.md)을 따른다. 현재 UD
기준선과 25-case 현실 기술 코퍼스를 규칙 선택용 noisy fixture로 재해석하지 않는다.

## 작업 단계

각 단계는 하나의 PR과 독립된 채택 판단으로 닫는다.

1. **corpus와 runner**: license manifest, 자연 원문 pilot, paired fixture, schema validator,
   default 외부 snapshot과 품질 보고서를 만든다. matcher는 바꾸지 않는다.
2. **정책과 shadow**: typed robustness requirement, rule provenance, candidate·cap counter를 만들되
   제품 결과에는 반영하지 않는다. `off` 동등성과 candidate amplification을 검증한다.
3. **띄어쓰기**: bounded window에서 공백 한 번의 삽입·제거와 raw offset map을 구현한다.
   기존 현실 기술 코퍼스의 spacing slice는 회귀 seed로만 사용한다.
4. **비표준 활용**: fine POS와 ending state로 제한한 registry를 만든다. `아니다면` family는
   `아니라면` canonical FN과 대조군을 포함한 첫 pilot으로 평가한다.
5. **한글 오타와 반복**: 한 번의 weighted edit를 `exploratory`에 추가한다. 키보드·자모·장음화
   비용과 결과를 분리한다.
6. **제품 채택**: held-out 자연 test와 외부 feature-matched 비교를 실행한다. gate를 통과한
   class만 `conservative` opt-in으로 문서화하고 기본값 변경은 별도 결정으로 남긴다.

각 구현 PR은 `specs/kfind.md`, CLI help·man, Rust·WASM API, fixture와 benchmark 보고서의 변경
필요를 함께 검토한다. `conservative`를 사람용 기본값으로 승격하는 작업은 이 계획의 자동
후속이 아니며 별도 사용자 결정과 clean/noisy 양쪽 제품 benchmark가 필요하다.

## 중단 조건

- 같은 surface의 positive와 negative를 bounded 품사·형태·문맥 근거로 구분하지 못한다.
- anchor 없이 파일 전체 edit-distance나 공백 제거를 해야만 recall이 오른다.
- 원문 byte span을 정확히 복원할 수 없다.
- 규칙이 특정 source 문자열, case ID 또는 held-out 결과에 의존한다.
- clean precision, hard-negative, robust-only precision, over-acceptance 또는 성능 gate 중 하나를
  통과하지 못한다.
- 자연 원문의 라이선스, revision이나 원본 checksum을 고정할 수 없다.

이 경우 해당 class는 `exploratory` 진단에 남기거나 작업을 종료한다. canonical 형태 규칙이나
기본 `smart` 경계를 우회해 결과만 맞추지 않는다.
