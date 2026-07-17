# 표준 띄어쓰기와 component 버전 계약

- 측정일: 2026-07-17
- 최신 `origin/main` 및 기준 revision:
  `6957f31686cfbe8adac4504644e9edf919333d32`
- 형태소 후보 revision: `cad827592a42f3751f38a1670cde799060756d10`
- 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs, Python 3.12.13,
  Rust 1.97.0, Docker 29.6.1
- 반복: fresh process warm-up 1회 뒤 5회 측정의 중앙값
- canonical test fixture:
  `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff`
- development fixture:
  `604c3a139854fcf59570392f48ab85028785f4a3561ea3c5e702f88b841f907c`
- hard-negative fixture:
  `f4d8829977ebfd061003724ee4aeb23b36dd901f6e46171c924a1f52a63f0ee5`
- full POS lexicon artifact:
  `012a2ecfc9ee049cb48f655eb240fa2ed6fc739dfde01526078a976549246e88`
- 기준 component artifact:
  `55d4f7a83c7fac278208f21c4cad2225e33768c992f0ceefa22402823fbfc4b3`
- 후보 component artifact:
  `d3a7eb486eef5faa92e006dd72e5ff72b63befb4bb102013f2d19e5fc32ff00a`
- 기준 report SHA-256:
  `998eff93db0e9b72324568f3f16fe0132119e3be1a5400f663f986a8df3887a7`
- 후보 report SHA-256:
  `008c86a91e98667c41cea8070f44b3467fa145f953f75533ff79382dac6df583`

## 판정 계약

국립국어원 한국어기초사전 raw data에서 `못` 명사 record 15307·601663, 동사 앞 부사
record 17254, `못하다` 형용사 record 29719와 동사 record 91168을 대조했다. 명사 `못`은
`못하다`의 파생 성분이 아니며, 부사 record는 띄어 쓴 `못 하다`를 용례로 든다. 반면
`공부하다` record 23394처럼 사전에 등재된 파생은 source-aligned `NNG+XSV` component를
유지한다.

제품 판정은 이 구분을 다음처럼 적용한다.

- `일을 못했다`는 한 용언 token이므로 `adv:못`과 `n:못`을 거부하고 `v:못하다`를 유지한다.
- `형보다 못하다`의 `못하다`는 비교 형용사 전체이므로 `n:못`을 거부한다.
- `못 하겠어요`는 별도 `못` token 뒤에 완성된 `하다` 활용이 와서 `adv:못`만 선택한다.
- `못 박았어요`는 명사 token과 다른 용언의 배치이므로 `n:못`을 유지한다.
- `벼를`, `색이`, `날은` 같은 표준 체언+조사와 source-backed `공부하다` 파생은 유지한다.

실제 full-POS CLI 결과는 다음과 같다. `match`는 exit code 0, `miss`는 exit code 1이다.

| 질의 | 입력 | 결과 |
| --- | --- | --- |
| `adv:못` | `일을 못했다` | miss |
| `n:못` | `일을 못했다` | miss |
| `v:못하다` | `일을 못했다` | match |
| `n:못` | `형보다 못하다` | miss |
| `v:못하다` | `형보다 못하다` | match |
| `n:못` | `못 하겠어요` | miss |
| `adv:못` | `못 하겠어요` | match |
| `n:못` | `못 박았어요` | match |
| `adv:못` | `일을 못 했다` | match |

`안나와요`처럼 표준 띄어쓰기를 벗어난 입력은 `nonstandard-spacing`으로 분류한다. 향후 별도
robust 지원에서 다루며, 현재는 strict FP/FN과 row-level delta에 남기되 표준형 회귀 gate에서
제외한다.

## 품질

Canonical과 development는 기준과 후보가 같다. Hard-negative의
`homonymous-other-pos`는 두 smart profile 모두 FP 1→0, TN 9→10이다.

| fixture/profile | 기준 TP / FP / FN | 후보 TP / FP / FN |
| --- | ---: | ---: |
| canonical embedded `smart` | 449 / 0 / 51 | 449 / 0 / 51 |
| canonical full-POS `smart` | 491 / 0 / 9 | 491 / 0 / 9 |
| development embedded `smart` | 455 / 4 / 45 | 455 / 4 / 45 |
| development full-POS `smart` | 468 / 4 / 32 | 468 / 4 / 32 |
| test matrix embedded `smart` | 1,272 / 5 / 129 | 1,271 / 5 / 130 |
| test matrix full-POS `smart` | 1,359 / 5 / 42 | 1,358 / 5 / 43 |

Test matrix의 유일한 불리한 이동은 두 profile에 공통인
`matrix:pos:ud-korean-ksl:ARG-ENG_06_2_10_1:3`의 `안나와요`에서 부사 `안`이
`TP→FN`으로 바뀐 1건이다. 이 행은 `nonstandard-spacing` 예외다. 표준형 행의 불리한 이동과
신규 FP는 없다.

## 성능

아래는 기준과 후보의 `median [min, max]`다. 모든 불리한 중앙값 변화는 10% 경고선 안이다.
Component header의 exact-version 검증을 추가하면서도 component decode 구간은 embedded
11.99%, full-POS 조합 10.21% 줄었다. Peak RSS는 각각 40,136KiB와 53,120KiB로 같다.

| workload | metric | 기준 | 후보 | 변화 |
| --- | --- | ---: | ---: | ---: |
| canonical embedded `smart` | initialization (s) | 0.048646 [0.047315, 0.053523] | 0.045742 [0.044935, 0.047488] | -5.97% |
| canonical embedded `smart` | cases/s | 19,490.2 [18,879.5, 19,988.5] | 19,081.2 [18,418.8, 20,885.4] | -2.10% |
| canonical embedded `smart` | p95 (ms) | 0.0796 [0.0774, 0.0804] | 0.0795 [0.0726, 0.0850] | -0.13% |
| canonical full-POS `smart` | initialization (s) | 0.094837 [0.094214, 0.098978] | 0.084032 [0.083617, 0.086123] | -11.39% |
| canonical full-POS `smart` | cases/s | 14,400.2 [13,860.9, 15,033.6] | 15,994.7 [15,186.7, 16,144.1] | +11.07% |
| canonical full-POS `smart` | p95 (ms) | 0.1334 [0.1277, 0.1463] | 0.1191 [0.1164, 0.1262] | -10.72% |
| embedded+component | component initialization (s) | 0.033016 [0.029947, 0.033734] | 0.029059 [0.027726, 0.030342] | -11.99% |
| full-POS+component | component initialization (s) | 0.033243 [0.031710, 0.035302] | 0.029847 [0.029086, 0.030352] | -10.21% |

Agent는 24,634.0→26,800.6 cases/s(+8.80%), Human은
13,178.1→14,354.6 cases/s(+8.93%)다. 두 workflow의 품질은 기준과 같다.

## 배포 무결성

Component container는 schema 5로 올리고 32-byte package version field를 추가했다. 후보
artifact는 `0.3.0-rc.3`, 37,103,813 bytes다. Decoder는 source와 section digest, payload
구조, package version을 모두 확인하며 binary/library version과 정확히 다르면 초기화 오류를
반환한다. Native의 병렬 digest·payload 검증과 WASM 순차 검증은 이 확인을 생략하지 않는다.

`kfind --check-data`는 설치된 full POS와 component를 끝까지 decode한다. JSON 결과에는
`kfind_version`, full-POS schema·entry 수와 component schema·resource version·surface 수가
포함된다. Homebrew formula는 install/upgrade 뒤 이 명령을 `post_install`에서 실행하며 formula
test도 exact version을 확인한다. `head` 설치는 release resource와 version이 어긋날 수 있어
제거했다.

npm package는 `prepack`에서 WASM, version-matched component와 Node·TypeScript smoke를 다시
검증한다. Release workflow는 같은 component artifact로 package test와 dry-run pack을 끝낸 뒤
OIDC provenance로 prerelease는 `next`, stable은 `latest` tag에 publish한다. npm registry의
trusted publisher 연결은 저장소 밖 package 설정에서 별도로 완료해야 한다.

## 재현

```console
git switch --detach 6957f31686cfbe8adac4504644e9edf919333d32
KFIND_MORPH_IMAGE=kfind-morph-benchmark:standard-spacing-latest-baseline \
KFIND_MORPH_RUNS=5 \
scripts/benchmark-morphology.sh target/morph-standard-spacing-latest-baseline

git switch --detach cad827592a42f3751f38a1670cde799060756d10
KFIND_MORPH_IMAGE=kfind-morph-benchmark:standard-spacing-candidate-cad8275 \
KFIND_MORPH_RUNS=5 \
scripts/benchmark-morphology.sh target/morph-standard-spacing-candidate-cad8275

kfind --check-data --json --data-dir target/full-pos-v5
```

외부 분석기 snapshot은 fixture, adapter schema와 고정 버전·설정이 바뀌지 않아 갱신하지
않았다.
