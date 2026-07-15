# Source provenance와 expression component shadow

## 결론

full morphology resource의 source metadata를 local lattice 경로에 연결하고 `expression` component를 canonical alignment로 해석했다. node 종류와 source component 관계만으로 현재 `ExactComponent` 비용 판정을 대체할 수 없다.

- `runtime-composed`와 `source-decomposition`은 development positive와 negative에 함께 나타났다.
- `source-explicit-component`는 development positive의 `속 -> 산속`, `기업 -> 기업주`에서 확인됐지만 hard-negative의 `학교 -> 대학교`에도 같은 구조로 나타났다.
- 기존 1,500 비용 마진이 복구한 development 5건 중 `속 -> 산속`만 제외 경로의 명시적 source component였다. `이루다 -> 이루어지지`, `빼다 -> 빼놓을`은 제외 경로에 해당 component가 없고, `비추다 -> 비춰볼`, `건전 -> 건전한`은 축약·음절 융합으로 component byte span이 불투명했다.
- 같은 scoring node가 atomic row와 inflection row에 동시에 대응하는 경우가 있어 compact node를 source row 하나로 역추정할 수 없다. graph resource는 모든 source analysis 관계를 보존해야 한다.

따라서 graph resolver는 이 충돌을 `Ambiguous`로 표현해야 한다. 복합어 component를 기본 `smart`에서 노출할지는 형태 비용이나 source 종류가 아니라 별도의 `CompoundExposure` profile 계약이다. 이 계약을 정하기 전에는 lexical context registry와 1,500 마진의 제품 동작을 제거하지 않는다.

## 구현

- diagnostic `LocalLatticeNode`에 source row를 유일하게 대조하는 left/right context ID를 보존했다.
- full/compact projection 동등성을 먼저 검증한 뒤 full resource의 `analysis_type`, `expression`과 모든 동점 source row를 shadow 경로에 연결했다.
- `kfind-data`에 source expression alignment를 추가했다.
  - `span-aligned`: component가 안정된 NFC byte span에 대응한다.
  - `fused`: canonical composition 결과는 같지만 component 경계가 한 scalar 안에 있다.
  - `unaligned`: 축약·교체 때문에 component 표면을 이은 결과가 node surface와 다르다.
  - `invalid`: expression 형식이 잘못됐다.
- shadow report는 각 include/exclude 경로를 `exact-node`, `source-explicit-component`, `opaque-expression`, `absent`로 집계한다.

이 계측은 timed evaluation 뒤에 실행하며 matcher 결과를 바꾸지 않는다.

## 품질과 구조 결과

candidate의 제품 품질은 기준선과 같았다.

| 집합 / profile | TP | FP | FN | precision | recall |
| --- | ---: | ---: | ---: | ---: | ---: |
| test full-POS `smart` | 466 | 0 | 34 | 100.00% | 93.20% |
| development full-POS `smart` | 475 | 2 | 25 | 99.58% | 95.00% |
| Human full-POS `smart` | 461 | 0 | 39 | 100.00% | 92.20% |

full-POS component candidate의 query 관계는 다음과 같다. development와 hard-negative만 구조 판정에 사용했고 고정 test는 구현을 고정한 뒤 회귀 확인에만 사용했다.

| 집합 | class | projection | exact node | explicit source component | opaque expression | absent |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| development | positive | include | 135 | 0 | 0 | 0 |
| development | positive | exclude | 0 | 2 | 46 | 91 |
| development | negative | include | 12 | 0 | 0 | 0 |
| development | negative | exclude | 0 | 0 | 2 | 10 |
| hard-negative | negative | include | 14 | 0 | 0 | 0 |
| hard-negative | negative | exclude | 0 | 1 | 2 | 12 |
| test | positive | include | 144 | 0 | 0 | 0 |
| test | positive | exclude | 0 | 5 | 62 | 79 |
| test | negative | include | 8 | 0 | 0 | 0 |
| test | negative | exclude | 0 | 0 | 3 | 5 |

full/compact projection 비교는 test 369건, development 358건, hard-negative 39건에서 모두 일치했고 mismatch는 0건이었다.

## 성능

- baseline: `a9ec8348a7a70b2883ac5fa690a3ddb5980f3d91`
- candidate: `17e0d03e4b06d900eb7da2c99b7fa5dfcd806def`
- 명령: `scripts/benchmark-morphology.sh <output-directory>`
- 환경: Docker Linux aarch64, Python 3.12.13, Rust 1.97.0, 10 logical CPUs, 7,836 MiB memory
- test fixture: `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff`
- development fixture: `604c3a139854fcf59570392f48ab85028785f4a3561ea3c5e702f88b841f907c`
- hard-negative fixture: `cb8634491cba65916c9af510c50f909eaddfd9bb89935598875e134a01cbce99`
- morphology resource: `50bbaa64b06a080c7fa09c13e21090388a1c0f5109ed413546e0004ce7794f23`
- component resource: `5fc46a151e41485dc4b4a3a931135c0f490913f2c2c908b9d87adb87a7c14efd`
- 각 revision은 fresh process warm-up 1회 뒤 5회 측정했고 대표값은 median이다.

| profile / metric | baseline median [min, max] | candidate median [min, max] | 변화 |
| --- | ---: | ---: | ---: |
| embedded init | 0.283483s [0.282557, 0.286378] | 0.283105s [0.282388, 0.287709] | -0.13% |
| embedded cases/s | 12095.2 [11862.8, 12118.6] | 12090.8 [11516.3, 12117.8] | -0.04% |
| embedded p95 | 0.1740ms [0.1711, 0.1782] | 0.1721ms [0.1682, 0.1860] | -1.09% |
| embedded RSS | 52088 KiB [52084, 52088] | 52084 KiB [52080, 52092] | -0.01% |
| full-POS init | 0.426974s [0.426552, 0.441457] | 0.425497s [0.423917, 0.436967] | -0.35% |
| full-POS cases/s | 10678.8 [10450.1, 10710.6] | 10645.0 [10541.9, 10657.3] | -0.32% |
| full-POS p95 | 0.2370ms [0.2363, 0.2408] | 0.2393ms [0.2376, 0.2420] | +0.97% |
| full-POS RSS | 94552 KiB [94548, 94564] | 94564 KiB [94548, 94564] | +0.01% |

측정 범위는 겹치며 제품 timed path의 성능 회귀는 없다. source provenance와 expression 관계 집계는 측정 구간 밖에 있다.

## 다음 결정

graph schema 구현 전에 `CompoundExposure`의 profile 계약을 정한다.

- `opaque`: whole-compound 분석이 있으면 내부 component를 기본 `smart`에서 노출하지 않는다.
- `transparent`: source가 명시한 component를 노출한다.
- `explicit`: 기본은 opaque로 두고 별도 query capability에서만 component를 노출한다.

`opaque`는 `속`, `기업` positive를 놓치고, `transparent`는 `학교 -> 대학교` hard-negative를 통과시키므로 현재 결과를 모두 보존하는 구조-only 선택지는 없다. 외부 어휘 의미 근거 없이 surface별 선택을 저장하면 lexical registry를 다른 이름으로 복원하는 것이므로 채택하지 않는다.
