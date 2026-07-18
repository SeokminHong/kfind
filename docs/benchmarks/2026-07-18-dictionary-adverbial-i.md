# 사전 합의 `-이` 부사형 recall

- 측정일: 2026-07-18
- 기준 revision: `2ef39d268ae5f2da67ebf72d47e374793c9ec01d`
- 후보 revision: `596b272d580df77a7bd0f5a552499a65bc30d88e`
- 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs, Python 3.12.13,
  Rust 1.97.0, Docker 29.6.1
- 반복: fresh process warm-up 1회 뒤 5회 측정의 중앙값과 min/max
- canonical fixture:
  `1497b958a6970c55bc68ff148e435a88366b650c971231c3ae40adb9d8c46572`
- explicit-POS matrix:
  `e862d8af010c23462ba3a9ebf4f1134275b68de5004bc60035565734f5f19999`
- contract review registry:
  `3aa7f3be5dc4a9f0c44a18c0bde4a570b790c9372271cd15eb05e149d3a3e50e`
- 기준 report SHA-256:
  `9ba8f6037ef87bd6fac5158307ac056eedbe53884118d4c112e651eb491d1ab9`
- 후보 report SHA-256:
  `72e0491e440c3b0e8ad6bced269ab1130ac7b4ddce3e871b297aaedbb2b45833`

## 결론

Query matrix full-POS `smart`의 부사형 5건을 회수해 raw FN을 18→13, FNᶜ를 14→9로
줄였다. Raw precision은 99.69%, precisionᶜ는 100.00%를 유지했고 recallᶜ는
98.92%→99.31%다. Canonical과 development에서도 FP를 늘리지 않고 각각 FN 1건과 3건을
줄였다.

후보는 token 경계만으로 승인하지 않는다. 두 기본 사전의 어휘 합의로 검색 표면을 만들고,
`smart` matcher가 해당 span 전체의 `MAG` 구조를 확인해야 한다. 따라서 `같다→같이`는
회수하지만 `친구같이`의 조사 `같이/JKB`는 거부한다.

## 사전 표면형 계층

Importer는 형용사 `-없다`·`-같다` 계열의 `어간 + 이`와 `르→ㄹ리`만 후보로 만든다.
한국어기초사전과 표준국어대사전이 원형을 형용사로, 결과 표면을 부사로 각각 독립 등재한
경우에만 `lexical.dictionary-adverbial-i`로 승격한다. 정의와 예문은 사용하지 않고 양쪽
표제어의 source record ID를 보존한다. 우리말샘은 독립 표결이 아니라 audit 증거로만 쓴다.

| source | snapshot SHA-256 |
| --- | --- |
| 한국어기초사전 2026-06-19 | `a8ab7d044d4f6341e0f217db63f38f4d18beed3e1f153130f6cb4e9494fea1d6` |
| 표준국어대사전 2026-07-05 | `880b31447146df5879c076012b21d4cc3c0c24e70fd91be7fc73f7ff7da34d52` |
| 우리말샘 2026-07-02 | `9e8807e5fade8c7b59431d1ab527fe93aafd15395001bcdde88511e8c9293b42` |

두 기본 사전이 합의한 부사형은 88개다. 이 중 76개는 기존 한국어기초사전 양방향 파생 관계와
겹쳐 기본 `inflection` 표면으로 승격했고, 나머지 77개 파생 관계는 계속 `derivation`에서만
연다. 배포 `SurfaceOnly`는 283→295행, TSV는 27,707→28,286바이트다.

현재 상한 512행·64 KiB에서 각각 217행·37,250바이트의 여유가 있고, 실제 증가는 12행·579바이트
(2.09%)다. 런타임 성능도 gate 안이므로 상한을 변경하지 않는다. Snapshot 갱신으로 상한에
도달할 때 중복과 분류 누락을 먼저 확인하고 별도 성능·배포 크기 측정으로 조정한다.

## 품질

| fixture/profile | 기준 TP / FP / TN / FN | 후보 TP / FP / TN / FN | precision | recall |
| --- | ---: | ---: | ---: | ---: |
| canonical full-POS smart | 493 / 2 / 498 / 7 | 494 / 2 / 498 / 6 | 99.60% → 99.60% | 98.60% → 98.80% |
| development full-POS smart | 482 / 3 / 497 / 18 | 485 / 3 / 497 / 15 | 99.38% → 99.39% | 96.40% → 97.00% |
| test matrix full-POS smart | 1,278 / 4 / 1,292 / 18 | 1,283 / 4 / 1,292 / 13 | 99.69% → 99.69% | 98.61% → 99.00% |
| development matrix full-POS smart | 1,220 / 4 / 1,262 / 46 | 1,230 / 4 / 1,262 / 36 | 99.67% → 99.68% | 96.37% → 97.16% |

Test matrix contract 값은 `TPᶜ/FPᶜ/TNᶜ/FNᶜ 1,282/0/1,293/14`에서
`1,287/0/1,293/9`가 됐다. 해소한 raw FN은 다음 5건이다.

| query | gold surface | case 수 |
| --- | --- | ---: |
| `없다` | `없이` | 1 |
| `똑같다` | `똑같이` | 1 |
| `같다` | `같이` | 2 |
| `다르다` | `달리` | 1 |

기존 hard-negative 38건의 예측은 모두 유지했다. 신규 `친구같이 행동했다.`는 TN이며 전체
hard-negative는 `FP 6 / TN 33`이다. 잔여 raw FN 13건은 disposition ledger와 다시 대조해
`product-fix 7`, `structural-redesign 2`, `gold-alignment-error 1`, `nonstandard-input 3`,
미분류 0건을 확인했다.

## 성능

Canonical full-POS `smart`의 같은 1,000 case를 비교했다.

| 지표 | 기준 median [min, max] | 후보 median [min, max] | 변화 |
| --- | ---: | ---: | ---: |
| initialization | 0.075389 s [0.074438, 0.077066] | 0.075432 s [0.074833, 0.077947] | +0.06% |
| cases/s | 19,788.0 [17,554.4, 20,680.4] | 19,550.8 [15,678.1, 20,770.8] | -1.20% |
| p95 | 0.1219 ms [0.1166, 0.1321] | 0.1266 ms [0.1154, 0.1317] | +3.86% |
| peak RSS | 57,780 KiB [57,728, 58,984] | 58,708 KiB [57,644, 59,500] | +1.61% |

모든 변화는 최신 `main`의 morphology 성능 gate 안이고 측정 범위가 겹친다. 전체 `MAG` 구조
검증을 경계 판정으로 낮추는 최적화는 적용하지 않았다.

## 재현

```console
KFIND_NIKL_DOWNLOADS=/Users/seokmin/Downloads \
KFIND_NIKL_CACHE=target/nikl-cache \
scripts/build-enriched-predicates.sh target/fnc-adverbial-enriched-final

scripts/benchmark-morphology.sh target/fnc-adverbial-commit-candidate

python3 tools/morph-compare/validate_fnc_dispositions.py \
  target/fnc-adverbial-commit-candidate/report.json \
  docs/benchmarks/query-matrix-fnc-dispositions.tsv
```
