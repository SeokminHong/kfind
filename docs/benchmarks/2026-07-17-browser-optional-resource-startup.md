# 브라우저 optional resource와 direct packed full POS

- 측정일: 2026-07-17
- 최신 `origin/main`: `0932120b7b94672dc17b26b2ef95adaefc5e2b25`
- 측정 revision: `d5f68f9dca0e9673ce2d543d97fd8f18424842fc`
- 환경: macOS 26.4.1 arm64, Chrome 150.0.7871.125, Node 24.5.0,
  Rust 1.97.0, wasm-pack 0.15.0
- 반복: fresh Chrome process에서 warm-up 1회 뒤 5회 측정
- report SHA-256: `0611de71168f38765e8f945384a519e559607696c1658c8ae2ff9ccd3457a5fe`

## 범위

현재 playground의 embedded→component lazy load와, full POS·component를 병렬 fetch해
`Kfind.withResources`로 생성하는 조합을 cold·warm HTTP cache에서 측정했다. 같은 source의
614,794개 lemma를 하나의 NFC blob과 고정 폭 `offset + length + fine-POS bit mask` record로
직렬화한 direct packed prototype도 함께 측정했다.

Prototype은 benchmark feature에서만 direct index를 소유하며 제품 query engine에는 연결하지
않는다. 제품 schema 1 artifact, decoder와 기본 WASM build는 바꾸지 않았다. Loopback HTTP는
content encoding을 적용하지 않으므로 R2·WAN latency는 포함하지 않는다.

## Bundle과 artifact 크기

Production build에는 benchmark feature가 없으므로 이번 변경의 배포 bundle 증가는 0이다.
아래 benchmark build 차이는 direct packed owner, JavaScript→WASM copy probe와 benchmark binding을
모두 포함한 제품 통합 시 decoder 코드 증가량의 상한이다.

| 산출물 | production | benchmark | 차이 |
| --- | ---: | ---: | ---: |
| JavaScript raw | 21,667 B | 23,535 B | +1,868 B |
| JavaScript gzip / Brotli | 4,812 / 4,251 B | 5,040 / 4,453 B | +228 / +202 B |
| WASM raw | 1,330,944 B | 1,343,470 B | +12,526 B |
| WASM gzip / Brotli | 492,106 / 377,144 B | 497,727 / 380,380 B | +5,621 / +3,236 B |

Decoder code보다 optional artifact 증가가 크다.

| artifact | raw | gzip | Brotli | SHA-256 |
| --- | ---: | ---: | ---: | --- |
| component schema 5 | 37,103,813 B | 12,787,674 B | 9,115,244 B | `d3a7eb486eef5faa92e006dd72e5ff72b63befb4bb102013f2d19e5fc32ff00a` |
| full POS schema 1 | 4,374,941 B | 1,692,361 B | 1,328,686 B | `012a2ecfc9ee049cb48f655eb240fa2ed6fc739dfde01526078a976549246e88` |
| direct packed prototype | 12,966,700 B | 3,668,340 B | 2,695,511 B | `07ca3381f68fa8c23d752902eaf114a590db9e616930065e2704b482ffb459b7` |

Direct packed full POS는 schema 1보다 raw 196.39%, gzip 116.76%, Brotli 102.87% 크다.
Component와 합친 optional payload도 raw 20.71%, gzip 13.65%, Brotli 13.09% 늘어난다.

## 브라우저 시작 시간

아래 값은 `median [min, max]` millisecond다. `activation`은 병렬 fetch 임계 경로와 engine
초기화를 합친 값이다.

| profile | cache | fetch wall | engine init | full POS init | component init | activation |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| full POS schema 1 | cold | 16.93 [14.54, 17.45] | 58.51 [57.98, 59.52] | - | - | 75.65 [73.27, 75.95] |
| full POS schema 1 | warm | 4.57 [4.19, 5.21] | 58.09 [57.96, 58.52] | - | - | 62.76 [62.41, 63.19] |
| embedded + component | cold | 92.95 [89.62, 106.00] | 892.29 [889.22, 896.09] | - | - | 983.13 [979.71, 1,002.09] |
| embedded + component | warm | 30.59 [28.78, 33.25] | 895.70 [892.15, 898.79] | - | - | 926.47 [920.93, 932.04] |
| schema 1 full POS + component | cold | 94.66 [91.89, 95.63] | 945.33 [944.07, 966.10] | - | - | 1,039.93 [1,038.73, 1,057.99] |
| schema 1 full POS + component | warm | 32.07 [30.71, 34.83] | 950.27 [945.72, 952.76] | - | - | 981.41 [977.78, 986.55] |
| packed attested + component | cold | 96.28 [94.50, 106.70] | 880.75 [877.77, 909.22] | 715.16 [712.56, 737.54] | 154.75 [152.25, 159.62] | 978.30 [974.05, 1,004.26] |
| packed attested + component | warm | 33.77 [30.81, 37.90] | 876.49 [875.04, 893.86] | 712.48 [710.42, 729.00] | 152.10 [151.72, 155.26] | 910.14 [907.23, 931.76] |
| packed full validation + component | cold | 97.16 [94.55, 147.96] | 1,030.38 [1,029.99, 1,046.86] | 866.45 [865.25, 883.22] | 151.83 [151.64, 153.83] | 1,127.15 [1,124.77, 1,194.82] |
| packed full validation + component | warm | 34.76 [31.54, 36.01] | 1,028.42 [1,027.67, 1,029.96] | 864.34 [863.37, 865.61] | 152.66 [151.28, 153.36] | 1,063.18 [1,061.25, 1,065.37] |

Attested packed 조합의 warm activation은 schema 1 조합보다 7.26% 짧지만 direct packed 자체가
빠른 것은 아니다. Packed artifact의 WASM SHA-256이 먼저 실행된 뒤 component 초기화가
895.70ms에서 152.10ms로 줄었다. 측정 순서와 속도 차이에 비추어 앞선 hash loop의 tier-up이
component 검증을 데운 phase-order 효과로 판단한다. Packed full POS 자체는 712.48ms이고
schema 1 full-POS engine 전체 초기화는 58.09ms다. 같은 artifact 안전성을 유지하는 전체
record 검증 조합은 오히려 8.33% 느리다.

복사 자체는 병목이 아니다. Warm cache에서 35.39 MiB component의 `arrayBuffer` materialization은
31.48ms, JavaScript→WASM copy는 4.67ms다. 4.17 MiB full POS는 각각 3.61ms와 0.93ms다.

## 메모리

아래는 warm profile의 resource buffer 보유 시점 중앙값이다. JavaScript heap과 WASM linear
memory의 합은 process RSS가 아니라 browser 내부 loading high-water의 하한이다.

| profile | retained resource | JS heap | WASM peak | JS + WASM checkpoint |
| --- | ---: | ---: | ---: | ---: |
| embedded + component | 35.38 MiB | 36.17 MiB | 37.00 MiB | 73.17 MiB |
| schema 1 full POS + component | 39.56 MiB | 40.35 MiB | 60.94 MiB | 101.29 MiB |
| direct packed + component | 47.75 MiB | 48.55 MiB | 49.38 MiB | 97.93 MiB |

Direct packed는 decoder가 만든 lemma blob과 entry allocation을 없애 WASM peak를 18.97%
줄인다. 그러나 더 큰 network buffer가 JavaScript heap에 남아 합산 loading checkpoint는
3.32%만 줄어든다.

## 결론

Direct packed schema는 채택하지 않는다. Decoder code의 bundle 영향은 작지만 optional payload가
13% 이상 커지고, 안전한 전체 검증은 시작 시간을 8.33% 늘리며, browser loading high-water
개선도 3.32%에 그친다.

현재 frontend의 실제 병목은 lazy component 초기화다. 다음 browser 성능 작업은 artifact layout을
다시 바꾸기 전에 component SHA·payload 검증을 Web Crypto 또는 검증된 배포 attestation 경계로
옮길 수 있는지 별도 prototype으로 확인한다. 이 결정은 native full POS startup의 layout 후보를
배제하지 않으며 browser 배포 경로에 한정한다.

## 재현

```console
git switch --detach d5f68f9dca0e9673ce2d543d97fd8f18424842fc
scripts/benchmark-browser-startup.sh target/browser-startup/report.json
```
