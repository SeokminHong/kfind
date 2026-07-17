# 검토된 UD 코퍼스 재샘플링과 Robust 성능

> 이 보고서의 Robust 절은 query-level gold를 만들기 전의 performance-only 기록이다. 현재
> scored fixture와 제품별 오류 입력 품질은
> [수동 검토 자연 오류 Robust 품질·성능](2026-07-17-robustness-quality.md)이 대체한다.

- 측정일: 2026-07-17
- 기준 revision: `0932120b7b94672dc17b26b2ef95adaefc5e2b25`
- 후보 revision: `66dc8304372a859eed462710e4397c21832a73f6`
- 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs, Python 3.12.13,
  Rust 1.97.0, Docker 29.6.1
- 반복: fresh process warm-up 1회 뒤 5회 측정의 중앙값과 min/max
- source manifest SHA-256:
  `19d0cb7c267ca1b20f9c0c94312c392da60079ad1696825e363266076b58bbc1`
- sentence review SHA-256:
  `4f555398ca6d30f455be3fee44228da766cbd8ab5763479c15cbe3bfc8449fed`
- canonical test fixture:
  `1497b958a6970c55bc68ff148e435a88366b650c971231c3ae40adb9d8c46572`
- explicit-POS matrix:
  `b4a7294e15b137407fffbaa90202ffeaf05598a01404b06a839931ca9563088b`
- untagged matrix:
  `136bc11ee8b3d9013089b1501339aa83275f3b5570e3df65c99cba41ccfc156e`
- development matrix:
  `0398c87744aa8136dc4bc80f9e042531a931d3f92fa177fc961bf8f77958413b`
- canonical external snapshot SHA-256:
  `28d0ade4b3725ad28c9ea0fd2399fa187c80478aed82dd85b059b26bdbd93a95`
- matrix external snapshot SHA-256:
  `a6906d6a2563153faa9f2de347d06cf73de3dd4e611ce921073728e73614c9fc`
- 기준 report SHA-256:
  `3a343c1c083daffab46a7d6890047728e7d3aa55128e1d0a0860a18f6db83c90`
- 후보 report SHA-256:
  `dbf1c951e996dea2c27334078d4eaecce404c683b19bc9b5b4158a07e0fd6b94`

## 결론

기존 scored fixture에는 UD Korean-Kaist와 Korean-KSL이 함께 들어 있었다. Korean-KSL은
학습자 문장이라 비문과 오타가 많으므로 canonical 점수에서 전부 분리했다. Korean-Kaist는
실제 샘플링 대상이 된 test 813문장과 dev 792문장을 모두 수동 검토했다. 비문·오타·source
artifact 121문장은 점수 없는 Robust registry로 옮기고, 통과한 문장에서 positive 500개와
negative 500개를 다시 샘플링했다.

새 canonical test는 1,000-case, explicit-POS query matrix는 432문장의 2,592-case다.
Matrix에서 kfind full-POS `smart`는 precision 99.36%, recall 96.45%, 문장 안 세 질의를 모두
찾은 비율 89.58%였다. Agent workflow는 recall 97.30%, Human workflow는 precision 99.68%였다.

Robust 성능은 품질과 분리했다. Annotation이 끝나지 않은 Korean-KSL에서 명시 POS 500-case와
무태그 500-case를 만들고 현재 `robustness=off`만 측정했다. 후보의 처리량은 workload에 따라
14,485.8~26,746.4 cases/s였으며 품질 점수나 backend 순위는 내지 않았다.

기준과 후보의 case별 품질·실패 목록은 완전히 같다. 성능의 최대 불리 변화는 matrix Human
처리량 -7.94%로 10% 경고선 안이다. 제품 Rust·resource·runner 입력도 두 revision 사이에
변경이 없어 회귀로 판정하지 않는다.

## 코퍼스 출처와 문장 검토

원문은 Universal Dependencies 2.18이다.

| source | split | 원문 SHA-256 | 역할 |
| --- | --- | --- | --- |
| Korean-Kaist | test | `fd94dc89afb01d1f7f340a46d567e0f27ae6903a70d7b6f650b49f7427f83b97` | 검토된 canonical |
| Korean-Kaist | dev | `a1ce2dceee65683c2df3b0cce96c83d01e5ff67756060b998fe01d8bc8ca4faa` | 검토된 development |
| Korean-KSL | test | `62574d11b83f62217494a53fd2a7cbf75b7fc3fe5df74021a91e66df65149033` | annotation-required Robust 후보 |

검토 범위는 generator가 quota를 채우기 전에 고정한 Korean-Kaist pre-review pool 전체다.
Review file은 문장 ID와 text digest를 고정하므로 원문이나 selection 순서가 바뀌면 생성이
실패한다.

| split | 검토 문장 | 통과 | Robust 분리 | pool SHA-256 |
| --- | ---: | ---: | ---: | --- |
| test | 813 | 756 | 57 | `9ec2f1da62c94fcf392cfbc0cf701dc6a08d59ac54517c835efd85bf7fa3d1e1` |
| dev | 792 | 728 | 64 | `8e1a38476da07f6685ee7cc7f82d632a60729de114b76d8be8124ffda5a63500` |
| 합계 | 1,605 | 1,484 | 121 | — |

| 제외 사유 | 문장 |
| --- | ---: |
| 한글 오타 | 47 |
| 비표준 문법 | 21 |
| 붙여쓰기 | 19 |
| source artifact | 11 |
| 띄어쓰기 분리 | 7 |
| 외국어 표기 오타 | 6 |
| 반복 | 5 |
| 철자 혼동 | 4 |
| 문장 파편 | 1 |

121문장은 query/POS/raw span gold가 아직 없으므로 문장 단위 Robust registry
`403dfdbb315d9dee7536210bd464faaeca25ad4357e61a87731e3a64d831de88`에만 보존한다.

## Canonical 재샘플링

검토를 통과한 Korean-Kaist에서 다음 quota로 positive를 고르고 동일한 수의 deterministic
negative를 대응시켰다. Test clean pool의 서로 다른 대명사 candidate가 26개뿐이어서 대명사
quota를 30에서 26으로 낮추고 명사에 4개를 옮겼다.

| POS | positive | negative |
| --- | ---: | ---: |
| noun | 184 | 184 |
| verb | 120 | 120 |
| adjective | 80 | 80 |
| adverb | 50 | 50 |
| pronoun | 26 | 26 |
| determiner | 20 | 20 |
| numeral | 20 | 20 |
| 합계 | 500 | 500 |

| backend | precision | recall | F1 | TP | FP | TN | FN |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| kfind embedded `smart` | 99.78% | 90.20% | 94.75% | 451 | 1 | 499 | 49 |
| kfind full-POS `smart` | 99.59% | 97.00% | 98.28% | 485 | 2 | 498 | 15 |
| Kiwi 0.23.2 | 100.00% | 83.60% | 91.07% | 418 | 0 | 500 | 82 |
| Lindera 4.0.0 | 100.00% | 75.40% | 85.97% | 377 | 0 | 500 | 123 |
| MeCab-ko 1.0.2 | 100.00% | 78.00% | 87.64% | 390 | 0 | 500 | 110 |
| KOMORAN 3.3.9 | 100.00% | 78.60% | 88.02% | 393 | 0 | 500 | 107 |

Dataset 자체가 바뀌었으므로 이 수치는 이전 fixture와 행 단위 회귀 비교하지 않는다.

## Query matrix

Canonical positive 500개가 속한 432개 문장에서 정렬된 존재 질의를 문장마다 3개씩 선택하고,
각 positive와 같은 품사의 부재 질의를 대응시켰다. 총 1,296 positive와 1,296 negative다.
현재 사전 선언된 `contract_expected`가 없어 strict와 contract-adjusted 결과는 같고
`reclassified=0`이다.

| backend | precision | recall | F1 | TP | FP | TN | FN | 세 질의 모두 회수 | cluster 95% CI |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| kfind embedded `smart` | 99.41% | 90.28% | 94.62% | 1,170 | 7 | 1,289 | 126 | 73.61% | 88.66~91.90% |
| kfind full-POS `smart` | 99.36% | 96.45% | 97.89% | 1,250 | 8 | 1,288 | 46 | 89.58% | 95.45~97.45% |
| Kiwi 0.23.2 | 100.00% | 85.49% | 92.18% | 1,108 | 0 | 1,296 | 188 | 62.73% | 83.49~87.42% |
| Lindera 4.0.0 | 100.00% | 76.70% | 86.81% | 994 | 0 | 1,296 | 302 | 45.83% | 74.31~79.01% |
| MeCab-ko 1.0.2 | 100.00% | 79.94% | 88.85% | 1,036 | 0 | 1,296 | 260 | 52.55% | 77.70~82.18% |
| KOMORAN 3.3.9 | 100.00% | 82.10% | 90.17% | 1,064 | 0 | 1,296 | 232 | 54.86% | 79.94~84.18% |

95% 구간은 query를 독립 표본으로 취급하지 않고 문장을 cluster로 삼아 10,000회
bootstrap했다.

### 제품 workflow

| workflow | precision | recall | F1 | TP | FP | TN | FN | 모든 질의 회수 | cases/s | p95 | RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Agent: embedded + any + explicit POS | 98.13% | 97.30% | 97.71% | 1,261 | 24 | 1,272 | 35 | 91.90% | 26,839.2 | 0.0620 ms | 8.0 MiB |
| Human: full-POS + smart + untagged | 99.68% | 96.06% | 97.84% | 1,245 | 4 | 1,292 | 51 | 88.43% | 13,864.5 | 0.1511 ms | 57.0 MiB |

### Explicit-POS matrix 성능

외부 행은 갱신한 고정 snapshot이다. 처리량을 품질 또는 persona가 다른 workflow와 합친
순위로 해석하지 않는다.

| backend | initialization | cases/s | p95 | peak RSS |
| --- | ---: | ---: | ---: | ---: |
| kfind embedded `smart` | 0.040815 s | 21,335.1 | 0.0700 ms | 43.6 MiB |
| kfind full-POS `smart` | 0.079575 s | 16,250.1 | 0.1168 ms | 57.0 MiB |
| Kiwi 0.23.2 | 1.553041 s | 1,720.9 | 0.9583 ms | 533.2 MiB |
| Lindera 4.0.0 | 0.028482 s | 24,240.9 | 0.0713 ms | 201.3 MiB |
| MeCab-ko 1.0.2 | 0.000255 s | 11,292.4 | 0.1410 ms | 104.4 MiB |
| KOMORAN 3.3.9 | 1.093550 s | 1,834.0 | 0.9067 ms | 860.1 MiB |

## Robust set과 성능

Robust 자료는 두 층으로 분리했다.

- Korean-Kaist 검토 제외 121문장은 사유가 확정된 sentence registry다. Query-level gold가
  없으므로 아직 실행하지 않는다.
- Korean-KSL은 학습자 원문의 성능 비용을 보는 annotation-required 후보 pool이다. 명시 POS
  fixture `2dabb1b4bffa57f2c8e2efdf606dcd70989c6d56f46a82b27c14537369c03331`과
  무태그 fixture `a8962a1f1470d3774fd7e08049d2b5020bd7cd52b907c0329b37d6a6f2a17c18`이
  각각 250 positive와 250 negative를 가진다.

다음 값은 후보 revision의 median `[min, max]`다. `robustness=off`이며 품질 지표가 아니다.

| workload | init s | cases/s | p50 ms | p95 ms | peak RSS MiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| embedded + smart + explicit POS | 0.0409 `[0.0405, 0.0428]` | 21,190.3 `[20,895.4, 21,299.0]` | 0.0440 `[0.0433, 0.0448]` | 0.0710 `[0.0684, 0.0718]` | 40.1 `[40.1, 40.1]` |
| full-POS + smart + explicit POS | 0.0794 `[0.0785, 0.0814]` | 16,113.0 `[15,361.8, 16,341.3]` | 0.0527 `[0.0519, 0.0537]` | 0.1152 `[0.1112, 0.1253]` | 56.1 `[56.0, 57.9]` |
| Agent: embedded + any + explicit POS | 0.0015 `[0.0014, 0.0016]` | 26,746.4 `[26,171.0, 26,876.6]` | 0.0236 `[0.0235, 0.0245]` | 0.0631 `[0.0620, 0.0657]` | 4.4 `[4.4, 4.4]` |
| Human: full-POS + smart + untagged | 0.0798 `[0.0782, 0.0803]` | 14,485.8 `[14,079.6, 14,565.7]` | 0.0519 `[0.0518, 0.0535]` | 0.1387 `[0.1372, 0.1430]` | 57.0 `[56.0, 57.3]` |

| workload | init 기준 → 후보 | cases/s 기준 → 후보 | p50 기준 → 후보 | p95 기준 → 후보 | RSS 기준 → 후보 |
| --- | ---: | ---: | ---: | ---: | ---: |
| embedded smart explicit | 0.040659 → 0.040887 (+0.56%) | 21,060.8 → 21,190.3 (+0.61%) | 0.0442 → 0.0440 (-0.45%) | 0.0717 → 0.0710 (-0.98%) | 40.1 → 40.1 MiB (+0.01%) |
| full-POS smart explicit | 0.079604 → 0.079350 (-0.32%) | 16,298.1 → 16,113.0 (-1.14%) | 0.0519 → 0.0527 (+1.54%) | 0.1135 → 0.1152 (+1.50%) | 57.0 → 56.1 MiB (-1.55%) |
| Agent explicit | 0.001450 → 0.001462 (+0.83%) | 26,728.3 → 26,746.4 (+0.07%) | 0.0235 → 0.0236 (+0.43%) | 0.0622 → 0.0631 (+1.45%) | 4.4 → 4.4 MiB (-0.26%) |
| Human untagged | 0.079999 → 0.079774 (-0.28%) | 14,493.2 → 14,485.8 (-0.05%) | 0.0514 → 0.0519 (+0.97%) | 0.1386 → 0.1387 (+0.07%) | 57.1 → 57.0 MiB (-0.05%) |

## Canonical·matrix 성능 비교

기준 이미지는 `origin/main` archive에 후보의 fixture·reporting harness만 덮어써 같은 입력과
schema를 사용했다. Product build input은 archive의 것을 유지했다. Docker가 두 context에
대해 같은 image digest `f1faee3d7e3dda95afed447573c5f9f61e25b1443a3ee628f3b19dffcef8e5f6`를
생성해 제품 바이너리 동일성도 확인했다.

| workload | init 기준 → 후보 | cases/s 기준 → 후보 | p95 기준 → 후보 | RSS 기준 → 후보 |
| --- | ---: | ---: | ---: | ---: |
| canonical embedded smart | 0.041533 → 0.041249 s (-0.68%) | 20,465.5 → 20,919.4 (+2.22%) | 0.0737 → 0.0712 ms (-3.39%) | 41.0 → 41.0 MiB (+0.01%) |
| canonical full-POS smart | 0.081819 → 0.079696 s (-2.59%) | 15,398.1 → 16,113.8 (+4.65%) | 0.1215 → 0.1143 ms (-5.93%) | 57.2 → 56.3 MiB (-1.60%) |
| matrix Agent | 0.001450 → 0.001514 s (+4.41%) | 27,015.1 → 26,839.2 (-0.65%) | 0.0605 → 0.0620 ms (+2.48%) | 8.0 → 8.0 MiB (0.00%) |
| matrix Human | 0.079895 → 0.079664 s (-0.29%) | 15,061.1 → 13,864.5 (-7.94%) | 0.1415 → 0.1511 ms (+6.78%) | 57.0 → 57.0 MiB (-0.01%) |

## Development 확인

Development matrix는 424문장, 2,532-case다. Test와 절대값은 다르지만 개별 recall보다 문장
완전 회수율이 낮은 방향은 같다.

| backend | precision | recall | F1 | TP | FP | TN | FN | 모든 질의 회수 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| kfind embedded `smart` | 99.82% | 89.97% | 94.64% | 1,139 | 2 | 1,264 | 127 | 71.93% |
| kfind full-POS `smart` | 99.83% | 94.15% | 96.91% | 1,192 | 2 | 1,264 | 74 | 82.78% |

## 한계

Canonical과 matrix는 검토된 Korean-Kaist만 평가한다. Matrix는 새 문장을 추가하지 않고 같은
432개 문장의 질의를 늘린 진단 workload다. Korean-KSL 성능 fixture는 학습자 원문 전체에서
만든 annotation-required 후보라 clean 문장과 noisy 문장이 섞일 수 있다. 따라서 현재 Robust
표는 처리 비용만 보여 주며 비문·오타 복구율, robust-only precision이나 over-acceptance를
증명하지 않는다. 이 품질 지표는 query/POS/raw span과 noise class를 확정한 뒤 측정한다.

## 재현

외부 snapshot은 새 canonical과 query matrix에 맞춰 다음 명령으로 갱신했다.

```console
KFIND_MORPH_IMAGE=kfind-morph-benchmark:curated-0c54cc3 \
KFIND_MORPH_REFRESH_IMAGE=kfind-morph-baseline-refresh:curated-0c54cc3 \
scripts/refresh-morph-baselines.sh
```

후보를 측정했다.

```console
KFIND_MORPH_IMAGE=kfind-morph-benchmark:reviewed-66dc830 \
KFIND_MORPH_RUNS=5 \
scripts/benchmark-morphology.sh target/morph-reviewed-candidate-66dc830
```

기준선은 `origin/main` archive에 후보의 `tools/morph-compare/python`, Dockerfile, benchmark
entrypoint, source/review manifest와 external snapshot만 복사한 뒤 측정했다.

```console
baseline_root="$PWD/target/morph-reviewed-origin-main-source"
mkdir -p "$baseline_root"
git archive origin/main | tar -x -C "$baseline_root"
cp -R tools/morph-compare/python/. "$baseline_root/tools/morph-compare/python/"
cp tools/morph-compare/{Dockerfile,benchmark.py,sources.json,sentence-reviews.json} \
  "$baseline_root/tools/morph-compare/"
cp tools/morph-compare/external/{baselines.json,query-matrix-baselines.json} \
  "$baseline_root/tools/morph-compare/external/"
KFIND_MORPH_IMAGE=kfind-morph-benchmark:reviewed-origin-main-0932120 \
KFIND_MORPH_RUNS=5 \
scripts/benchmark-run.sh run --name morphology-reviewed-origin-main -- \
  "$baseline_root/scripts/benchmark-morphology.sh" \
  "$PWD/target/morph-reviewed-baseline-0932120"
```
