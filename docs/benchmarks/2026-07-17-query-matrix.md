# 같은 문장의 누적 검색 누락 검증

- 측정일: 2026-07-17
- 기준 revision: `5d41e950e54a8168db2415e1ea37e8535ba6dacd`
- 후보 revision: `e08d98f3f0d87a649114b4f8f8edb163134b401d`
- 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs, Python 3.12.13,
  Rust 1.97.0, Docker 29.6.1
- 반복: fresh process warm-up 1회 뒤 5회 측정의 중앙값
- canonical test fixture:
  `933bc12197da866d2363d7df9107d4d9be89a65ddaafd73968ad5384832b21ff`
- explicit-POS matrix:
  `fbcce40b533655085ff8a4e9031559f99b54f86abe188b6ddc1d690dd44326c6`
- untagged matrix:
  `b9dd7601301fa19b35acba735a977eba7c56a0c9d67c65dee32db5c8028c71bb`
- development matrix:
  `bc67497c3dc966fb7453b238df52c6d781b1b4485d40e8a5d6a38104dcc7abed`
- external matrix snapshot SHA-256:
  `1168cde228f571de0fea687114adc597d266b5a2e7eac11784bfdf431ed1d60a`
- 기준 report SHA-256:
  `65edaf4134e7b39816245b29940057410c390ae94089fc837aa4cb7fb618fbb8`
- 후보 report SHA-256:
  `1c32f59b23cdc0dd2d72af4213efd9669ec458cc4b411deb8a71a2a36339a7d0`

## 결론

기존 canonical 1,000-case는 그대로 유지하면서, positive가 있는 468개 문장의 검색 질의를
최대 3개로 늘렸다. 모든 canonical positive를 보존하고 같은 품사의 paired negative를
대응시켜 1,401 positive와 1,401 negative, 총 2,802-case를 만들었다.

동일 explicit-POS 입력에서 matrix recall은 80.51~92.86%였다. 한 문장의 선택 질의를 모두
찾은 비율은 54.70~80.13%로 더 낮았다. 개별 query recall만으로 보이지 않던 누적 누락을
문장 단위 지표가 분리한다. Canonical은 고정 회귀선으로 유지하고 matrix는 별도 진단
workload로 사용한다.

Matrix의 strict와 contract-adjusted confusion matrix, 문장별 모든 positive 회수율과 cluster
bootstrap 구간을 함께 기록한다. 현재 matrix에는 제품 실행 전에 선언한 `contract_expected`가
없어 `reclassified=0`이며 두 결과가 같다. 이는 hard-negative의 기존 annotation을 자동으로
복사하지 않고 matrix 자체에서 사전 검토한 case만 보정한다는 계약과 일치한다.

## 동일 입력 비교

Canonical과 matrix의 positive 분모는 각각 500건과 1,401건이다. Matrix는 canonical
positive 500건을 모두 포함하지만 추가 query의 품사 구성이 달라지므로 recall 차이를 새
corpus에 대한 일반화 향상으로 해석하지 않는다.

| backend | canonical recall | matrix precision | matrix recall | 증감 | matrix F1 | 모든 질의 회수율 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| kfind embedded `smart` | 87.60% | 99.60% | 88.29% | +0.69%p | 93.61% | 68.38% |
| kfind full-POS `smart` | 94.60% | 99.62% | 92.86% | -1.74%p | 96.12% | 80.13% |
| Kiwi 0.23.2 | 85.20% | 100.00% | 87.87% | +2.67%p | 93.54% | 67.74% |
| Lindera 4.0.0 | 78.60% | 99.82% | 80.51% | +1.91%p | 89.13% | 54.70% |
| MeCab-ko 1.0.2 | 80.60% | 99.91% | 82.94% | +2.34%p | 90.64% | 59.40% |
| KOMORAN 3.3.9 | 81.20% | 99.92% | 84.73% | +3.53%p | 91.70% | 61.11% |

Recall 95% 구간은 query를 독립 표본으로 취급하지 않고 문장을 cluster로 삼아 10,000회
bootstrap했다.

| backend | recall 95% 구간 | 전부 찾은 문장 / 전체 문장 |
| --- | ---: | ---: |
| kfind embedded `smart` | 86.60~89.93% | 320 / 468 |
| kfind full-POS `smart` | 91.50~94.16% | 375 / 468 |
| Kiwi 0.23.2 | 86.18~89.58% | 317 / 468 |
| Lindera 4.0.0 | 78.29~82.67% | 256 / 468 |
| MeCab-ko 1.0.2 | 80.84~84.96% | 278 / 468 |
| KOMORAN 3.3.9 | 82.86~86.63% | 286 / 468 |

## Contract-adjusted 결과

| backend | contract precision | contract recall | contract F1 | TPᶜ | FPᶜ | TNᶜ | FNᶜ | 모든 contract 질의 회수율 | reclassified |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| kfind embedded `smart` | 99.60% | 88.29% | 93.61% | 1,237 | 5 | 1,396 | 164 | 68.38% | 0 |
| kfind full-POS `smart` | 99.62% | 92.86% | 96.12% | 1,301 | 5 | 1,396 | 100 | 80.13% | 0 |
| Kiwi 0.23.2 | 100.00% | 87.87% | 93.54% | 1,231 | 0 | 1,401 | 170 | 67.74% | 0 |
| Lindera 4.0.0 | 99.82% | 80.51% | 89.13% | 1,128 | 2 | 1,399 | 273 | 54.70% | 0 |
| MeCab-ko 1.0.2 | 99.91% | 82.94% | 90.64% | 1,162 | 1 | 1,400 | 239 | 59.40% | 0 |
| KOMORAN 3.3.9 | 99.92% | 84.73% | 91.70% | 1,187 | 1 | 1,400 | 214 | 61.11% | 0 |

## 제품 workflow

Agent는 명시 POS와 `any` 경계로 recall을 우선하고, Human은 무태그 query와 `smart` 경계로
precision을 우선한다. 입력 계약이 다르므로 두 결과를 하나의 순위로 합치지 않는다.

| workflow | precision | recall | F1 | 모든 질의 회수율 | cases/s |
| --- | ---: | ---: | ---: | ---: | ---: |
| Agent: embedded + any + explicit POS | 98.48% | 96.93% | 97.70% | 90.81% | 17,471.5 |
| Human: full-POS + smart + untagged | 99.69% | 93.15% | 96.31% | 80.56% | 11,648.6 |

| workflow | contract precision | contract recall | contract F1 | TPᶜ | FPᶜ | TNᶜ | FNᶜ | 모든 contract 질의 회수율 | reclassified |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Agent: embedded + any + explicit POS | 98.48% | 96.93% | 97.70% | 1,358 | 21 | 1,380 | 43 | 90.81% | 0 |
| Human: full-POS + smart + untagged | 99.69% | 93.15% | 96.31% | 1,305 | 4 | 1,397 | 96 | 80.56% | 0 |

## 품사별 결과

수사는 외부 분석기에서 43.75~46.88%로 가장 낮았다. kfind의 숫자 단위 구조 판정은 수사
recall을 두 profile 모두 93.75%로 높였다. Embedded의 남은 약점은 형용사 76.89%와 동사
79.74%이며, full-POS resource를 사용하면 각각 89.50%, 89.39%다.

| POS | positive | embedded | full-POS | Kiwi | Lindera | MeCab-ko | KOMORAN |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| adjective | 238 | 76.89% | 89.50% | 83.19% | 84.45% | 79.41% | 76.05% |
| adverb | 186 | 94.09% | 96.24% | 92.47% | 86.02% | 93.55% | 94.62% |
| determiner | 56 | 98.21% | 98.21% | 94.64% | 53.57% | 94.64% | 91.07% |
| noun | 502 | 94.42% | 94.42% | 91.63% | 81.27% | 82.27% | 86.85% |
| numeral | 32 | 93.75% | 93.75% | 46.88% | 43.75% | 46.88% | 46.88% |
| pronoun | 76 | 94.74% | 94.74% | 80.26% | 86.84% | 81.58% | 76.32% |
| verb | 311 | 79.74% | 89.39% | 87.46% | 80.06% | 82.32% | 86.82% |

## 성능

외부 분석기와 kfind 모두 같은 2,802-case explicit-POS workload를 fresh process로 실행했다.
처리량은 품질 또는 제품 입력 계약과 합친 순위가 아니다.

| backend | initialization | cases/s | p95 | peak RSS |
| --- | ---: | ---: | ---: | ---: |
| kfind embedded `smart` | 0.240241 s | 14,819.9 | 0.1304 ms | 43.6 MiB |
| kfind full-POS `smart` | 0.384949 s | 12,961.4 | 0.1687 ms | 82.7 MiB |
| Kiwi 0.23.2 | 1.776459 s | 1,534.7 | 1.2421 ms | 531.5 MiB |
| Lindera 4.0.0 | 0.030357 s | 19,829.6 | 0.1007 ms | 199.5 MiB |
| MeCab-ko 1.0.2 | 0.000333 s | 9,838.8 | 0.2001 ms | 104.1 MiB |
| KOMORAN 3.3.9 | 1.216210 s | 1,786.4 | 1.0587 ms | 897.6 MiB |

최신 `origin/main`과 후보를 같은 fixture·이미지 설정으로 연속 측정했다. 처리량은 높을수록,
나머지는 낮을수록 좋다.

| workload | initialization 기준 → 후보 | cases/s 기준 → 후보 | p95 기준 → 후보 | peak RSS 기준 → 후보 |
| --- | ---: | ---: | ---: | ---: |
| embedded `smart` | 0.234727 → 0.240241 s (+2.35%) | 15,600.7 → 14,819.9 (-5.00%) | 0.1230 → 0.1304 ms (+6.02%) | 43.6 → 43.6 MiB (0.00%) |
| full-POS `smart` | 0.377576 → 0.384949 s (+1.95%) | 12,969.9 → 12,961.4 (-0.07%) | 0.1667 → 0.1687 ms (+1.20%) | 82.8 → 82.7 MiB (0.00%) |
| Agent | 0.001483 → 0.001486 s (+0.20%) | 17,232.3 → 17,471.5 (+1.39%) | 0.1250 → 0.1231 ms (-1.52%) | 8.4 → 8.3 MiB (-0.05%) |
| Human | 0.376379 → 0.377597 s (+0.32%) | 11,506.5 → 11,648.6 (+1.23%) | 0.1965 → 0.1955 ms (-0.51%) | 82.7 → 82.7 MiB (0.00%) |

최대 불리 변화는 embedded p95 +6.02%로 10% 경고선 안이다. Reporting 경로 추가가 timed
검색 구간의 품질이나 제품 코드를 바꾸지 않았고, 기준·후보의 모든 matrix prediction도 같다.

## Development 확인

별도 development matrix는 466문장, 2,782-case다. Embedded recall은 87.20%, 모든 질의
회수율은 66.31%였고 full-POS는 각각 90.44%, 73.82%였다. Test와 절대값은 다르지만 개별
recall보다 문장 완전 회수율이 낮은 방향은 같다.

| backend | contract precision | contract recall | contract F1 | TPᶜ | FPᶜ | TNᶜ | FNᶜ | 모든 contract 질의 회수율 | reclassified |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| kfind embedded `smart` | 99.43% | 87.20% | 92.91% | 1,213 | 7 | 1,384 | 178 | 66.31% | 0 |
| kfind full-POS `smart` | 99.37% | 90.44% | 94.69% | 1,258 | 8 | 1,383 | 133 | 73.82% | 0 |

## 한계

Matrix는 문장 수를 늘리지 않고 같은 468개 문장의 질의를 늘린다. 새 문장 source에 대한
일반화나 noisy text robustness를 직접 증명하지 않는다. Paired negative도 같은 품사의 명시적
부재를 보장한 통제 표본이며 실제 사용자 negative query 분포와 같지 않다. 새 corpus와
비표준 입력은 별도 fixture로 평가한다.

## 재현

```console
KFIND_MORPH_IMAGE=kfind-morph-benchmark:matrix-contract-latest-candidate \
KFIND_MORPH_RUNS=5 \
scripts/benchmark-morphology.sh target/matrix-contract-latest-candidate

mkdir -p target/matrix-contract-latest-origin-main-source
git archive --format=tar \
  --output=target/matrix-contract-latest-origin-main.tar origin/main
tar -xf target/matrix-contract-latest-origin-main.tar \
  -C target/matrix-contract-latest-origin-main-source

BASELINE_ROOT="$PWD/target/matrix-contract-latest-origin-main-source"
KFIND_MORPH_IMAGE=kfind-morph-benchmark:matrix-contract-latest-origin-main \
KFIND_MORPH_RUNS=5 \
scripts/benchmark-run.sh run --name morphology-latest-origin-main -- \
  "$BASELINE_ROOT/scripts/benchmark-morphology.sh" \
  "$PWD/target/matrix-contract-latest-origin-main"
```

두 실행은 저장한 외부 snapshot을 검증한 뒤 kfind canonical, hard-negative, explicit-POS
matrix, untagged matrix와 development matrix를 측정한다. 외부 snapshot은 fixture·adapter
schema·버전이 바뀌지 않아 갱신하지 않았다.
