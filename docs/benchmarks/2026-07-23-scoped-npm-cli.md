# Scoped npm CLI 초기화

- 측정일: 2026-07-23
- 기준 revision: `cc996439aa5eca9fbc71386440ae707b05527817`
- 후보 revision: `3cc3958970f19e2f5d7ffde30c605a64f44a4704`
- 환경: macOS 26.4, Apple M1 Max, 32 GiB, arm64, Node.js 24.5.0,
  Rust 1.97.0, wasm-pack 0.15.0
- 반복: profile별 fresh process warm-up 1회 뒤 5회 측정의 중앙값과 min/max

## 판정

후보의 Node.js WASM은 기준과 같은 1,523,921 bytes다. 전체 초기화 중앙값 변화는
embedded +2.71%, embedded와 component -0.26%, full POS -3.56%, full POS와
component +0.26%다. RSS 중앙값 변화는 모든 profile에서 ±0.52% 안이다. 10% 경고선을
넘는 회귀는 없다.

Embedded 초기화는 14.268 ms에서 14.654 ms로 늘었고 양쪽 min/max 범위가 겹치지 않는다.
이 불리한 결과를 포함해도 증가 폭은 0.386 ms다. Component 포함 profile과 full POS는
방향이 같지 않으므로 전체 Node 초기화 회귀로 일반화하지 않는다.

이 workload는 `Kfind` 생성과 선택 resource 초기화를 측정한다. 새 npm CLI의 인자 해석,
UTF-8 파일 순회와 출력은 기준 revision에 대응하는 실행 surface가 없어 baseline 비교에
포함하지 않는다. 실제 tarball의 bin, 표준 입력, 재귀 검색과 JSON Lines 동작은
`pnpm --dir packages/kfind run pack:check`로 별도 검증한다.

## 초기화와 메모리

초기화 단위는 ms, RSS 단위는 MiB다.

| profile | 기준 초기화 median [min, max] | 후보 초기화 median [min, max] | 변화 | 기준 RSS median [min, max] | 후보 RSS median [min, max] | 변화 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| embedded | 14.268 [14.195, 14.370] | 14.654 [14.527, 14.867] | +2.71% | 85.734 [85.641, 86.188] | 86.172 [84.984, 87.469] | +0.51% |
| embedded + component | 896.627 [895.334, 973.943] | 894.339 [888.422, 906.847] | -0.26% | 178.312 [177.844, 178.875] | 178.453 [178.062, 178.797] | +0.08% |
| full POS | 59.679 [57.568, 59.987] | 57.555 [57.301, 57.848] | -3.56% | 129.797 [129.719, 129.984] | 129.797 [128.750, 129.984] | 0.00% |
| full POS + component | 934.338 [932.498, 942.035] | 936.730 [932.048, 957.686] | +0.26% | 203.438 [202.297, 203.500] | 203.406 [202.969, 203.969] | -0.02% |

성능 지표에는 정답 계약을 적용하지 않는다. 이 보고서는 품질 confusion matrix를 포함하지
않으며 raw와 contract-adjusted 품질 지표를 합치거나 대체하지 않는다.

## 입력과 산출물

- Component resource: 37,103,813 bytes,
  SHA-256 `e6219f8bbdf08d56a1a03f724b814952fc2050754b5f50fe6f2a3656a62feb52`
- Full POS resource: 4,374,941 bytes,
  SHA-256 `012a2ecfc9ee049cb48f655eb240fa2ed6fc739dfde01526078a976549246e88`
- 기준 report SHA-256:
  `d20528d8041bac397d962058276cb16ee4d8a221e74b8c8510fca050ad4d44d0`
- 후보 report SHA-256:
  `89b12f0392689968746dfd3461804f2709c9aa77545eb855f064f5e2918b523c`
- Runner SHA-256:
  `574c0ffd68a3ba49772df5cd546ab78dc29687ad39032b3a53a392528c9a3016`
- Probe SHA-256:
  `9dc1d03ad0be685e2d449142a3605243d0379218e2d810a76ed97a967cb453da`

## 재현

두 revision의 별도 worktree에서 같은 resource 경로와 Node.js 24.5.0 실행 파일을 사용한다.
공식 wrapper가 저장소 공통 benchmark lock을 획득한다.

```console
PATH="$(asdf where nodejs 24.5.0)/bin:$PATH" \
KFIND_COMPONENT_RESOURCE_DIR=/absolute/path/to/component-resource \
KFIND_FULL_POS_RESOURCE_DIR=/absolute/path/to/full-pos \
scripts/benchmark-npm-startup.sh target/npm-startup/report.json
```

Wrapper는 browser bundle과 Node.js target을 release profile로 만든 뒤
`node --expose-gc`로 profile별 process를 실행한다.
