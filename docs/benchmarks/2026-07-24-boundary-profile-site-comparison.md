# Boundary profile 품질·성능 비교

- 측정일: 2026-07-24
- 기준 revision: `e7e1bd54c5d880b940ede5e937d22e40a8b452d8`
- 후보 revision: `d28fa2790b5592710291c5a90220698065ff8999`
- 기준 morphology report:
  `60bcc2cfbda11aedb990c3c4c5bd22a9c7756e0adee0f419224c042514a91bfd`
- 후보 morphology report:
  `3258d172ce7098f9434150fb24d9a59fbedc73ec4dd0b84b17ca24062a68af28`
- 검색 기준선 report:
  `40e3c1fcfc7b803fe48121d8e73e7e7dde2a61de70f90213890e58b43bec5021`

## 판정

Canonical `full POS + smart`는 raw `498/500/0/2`, contract-adjusted
`498/500/0/0`을 기록했다. `전망해야` 안의 `전` 오탐을 구조 판정으로 제거해 raw FP도
1건에서 0건으로 줄었다. 남은 raw FN 2건은 표준 띄어쓰기를 따르지 않은 `권위있는`,
`국경없는`이며, 고정 contract review에서만 `nonstandard-input`으로 제외했다.

Site snapshot은 Canonical, query matrix와 Robust 각각에 kfind
`embedded/full POS × any/smart` 4개 profile과 Kiwi, Lindera, MeCab-ko, KOMORAN을 같은
순서로 보존한다. 각 workload는 F1 chart, raw·contract-adjusted confusion matrix와
초기화·처리량·p95·RSS 표를 함께 사용한다.

기준 대비 morphology 처리량의 유일한 불리한 변화는 Canonical
`embedded + smart`의 -1.73%였다. 나머지 비교 가능한 profile은 +1.82%에서 +15.99%였다.
품질과 성능을 하나의 점수로 합치지 않았다.

## 측정 환경과 입력

- morphology: Linux 6.12.76 linuxkit aarch64, logical CPU 10, memory 7.7 GiB,
  Python 3.12.13
- morphology 측정: fresh process, warm-up 1회, 5회 측정, median
- Canonical: 1,000 cases, contract 적용 fixture
  `59c4d84de5cbafd3b134bc132c2fcdfaac75c945323b6f2880ad7ffa6aae7cec`
- Canonical 원본 fixture:
  `1497b958a6970c55bc68ff148e435a88366b650c971231c3ae40adb9d8c46572`
- query matrix: 2,592 cases,
  `e862d8af010c23462ba3a9ebf4f1134275b68de5004bc60035565734f5f19999`
- Robust: 500 cases,
  `6bfa1c00d1d4469742d100099eab3a4d6d0d679d2ed147cda1ca2e980a64e282`
- full POS:
  `012a2ecfc9ee049cb48f655eb240fa2ed6fc739dfde01526078a976549246e88`
- component:
  `e6219f8bbdf08d56a1a03f724b814952fc2050754b5f50fe6f2a3656a62feb52`
- 외부 분석기: Kiwi 0.23.2, Lindera 4.0.0, MeCab-ko 1.0.2, KOMORAN 3.3.9

표의 confusion matrix 순서는 `TP/TN/FP/FN`이다. 성능 값은 각 workload의 median이며,
외부 분석기는 고정 snapshot 값이다.

## Canonical

| profile·제품 | raw | contract-adjusted | init | cases/s | p95 | RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| kfind embedded · any | 486/493/7/14 | 486/493/7/12 | 0.001978 s | 52,504.5 | 0.0512 ms | 5.5 MiB |
| kfind embedded · smart | 461/500/0/39 | 461/500/0/37 | 0.044594 s | 37,824.3 | 0.0567 ms | 41.3 MiB |
| kfind full POS · any | 497/493/7/3 | 497/493/7/1 | 0.035629 s | 41,755.7 | 0.0731 ms | 21.0 MiB |
| kfind full POS · smart | 498/500/0/2 | 498/500/0/0 | 0.077319 s | 25,162.4 | 0.1105 ms | 56.6 MiB |
| Kiwi | 418/500/0/82 | 418/500/0/80 | 1.531036 s | 1,745.6 | 0.9617 ms | 528.5 MiB |
| Lindera | 377/500/0/123 | 377/500/0/121 | 0.027312 s | 24,199.6 | 0.0697 ms | 180.5 MiB |
| MeCab-ko | 390/500/0/110 | 390/500/0/108 | 0.000261 s | 10,946.6 | 0.1489 ms | 102.8 MiB |
| KOMORAN | 393/500/0/107 | 393/500/0/105 | 1.086962 s | 1,569.8 | 1.1011 ms | 756.1 MiB |

## Query matrix

| profile·제품 | raw | contract-adjusted | init | cases/s | p95 | RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| kfind embedded · any | 1268/1272/24/28 | 1270/1272/21/26 | 0.002029 s | 52,001.7 | 0.0508 ms | 8.4 MiB |
| kfind embedded · smart | 1197/1292/4/99 | 1201/1293/0/95 | 0.042470 s | 39,565.0 | 0.0539 ms | 44.0 MiB |
| kfind full POS · any | 1291/1272/24/5 | 1293/1272/21/3 | 0.034825 s | 45,002.9 | 0.0595 ms | 21.7 MiB |
| kfind full POS · smart | 1292/1292/4/4 | 1296/1293/0/0 | 0.077484 s | 25,472.5 | 0.1007 ms | 57.4 MiB |
| Kiwi | 1108/1296/0/188 | 1107/1293/0/189 | 1.473097 s | 1,701.5 | 0.9763 ms | 532.8 MiB |
| Lindera | 994/1296/0/302 | 993/1293/0/303 | 0.028187 s | 23,457.6 | 0.0735 ms | 201.3 MiB |
| MeCab-ko | 1036/1296/0/260 | 1035/1293/0/261 | 0.000253 s | 11,348.2 | 0.1404 ms | 103.9 MiB |
| KOMORAN | 1064/1296/0/232 | 1063/1293/0/233 | 1.091099 s | 1,837.9 | 0.9018 ms | 868.7 MiB |

## Robust

Robust에는 contract review가 없어 raw와 contract-adjusted가 같다.

| profile·제품 | raw | contract-adjusted | init | cases/s | p95 | RSS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| kfind embedded · any | 230/244/6/20 | 230/244/6/20 | 0.001994 s | 54,253.0 | 0.0486 ms | 4.7 MiB |
| kfind embedded · smart | 182/249/1/68 | 182/249/1/68 | 0.043785 s | 38,596.0 | 0.0545 ms | 40.4 MiB |
| kfind full POS · any | 232/244/6/18 | 232/244/6/18 | 0.036220 s | 43,249.8 | 0.0798 ms | 20.8 MiB |
| kfind full POS · smart | 201/249/1/49 | 201/249/1/49 | 0.079694 s | 23,698.2 | 0.1203 ms | 56.5 MiB |
| Kiwi | 213/250/0/37 | 213/250/0/37 | 1.878823 s | 1,713.4 | 1.3710 ms | 527.5 MiB |
| Lindera | 208/250/0/42 | 208/250/0/42 | 0.029267 s | 24,956.2 | 0.1007 ms | 165.4 MiB |
| MeCab-ko | 206/250/0/44 | 206/250/0/44 | 0.000268 s | 11,454.7 | 0.2169 ms | 95.4 MiB |
| KOMORAN | 205/250/0/45 | 205/250/0/45 | 1.164365 s | 1,329.2 | 1.8062 ms | 522.4 MiB |

## 기준 대비 처리량

| workload | profile | 기준 cases/s | 후보 cases/s | 변화 |
| --- | --- | ---: | ---: | ---: |
| Canonical | embedded · any | 50,106.0 | 52,504.5 | +4.79% |
| Canonical | embedded · smart | 38,490.6 | 37,824.3 | -1.73% |
| Canonical | full POS · any | 40,049.3 | 41,755.7 | +4.26% |
| Canonical | full POS · smart | 23,227.4 | 25,162.4 | +8.33% |
| Query matrix | embedded · any | 48,505.6 | 52,001.7 | +7.21% |
| Query matrix | embedded · smart | 36,411.8 | 39,565.0 | +8.66% |
| Query matrix | full POS · any | 40,489.1 | 45,002.9 | +11.15% |
| Query matrix | full POS · smart | 21,961.3 | 25,472.5 | +15.99% |
| Robust | embedded · smart | 37,906.0 | 38,596.0 | +1.82% |
| Robust | full POS · smart | 21,456.1 | 23,698.2 | +10.45% |

기준 harness에는 Robust full POS any 행이 없어서 해당 profile의 증감률은 계산하지 않았다.

## 검색 기준선

검색 기준선은 macOS 26.4.1 arm64, Apple M1 Max, logical CPU 10, memory 32 GiB에서
13.54 MiB 단일 파일을 질의마다 fresh process로 스캔했다. 방법 순서를 순환하고 warm-up
2회 뒤 10회 측정했다.

| 방법 | raw TP/TN/FP/FN | contract-adjusted | batch median | p95 | effective MiB/s |
| --- | ---: | ---: | ---: | ---: | ---: |
| kfind full POS · any | 56/47/9/0 | 62/47/3/0 | 491.535 ms | 507.889 ms | 192.8 |
| kfind full POS · smart | 56/50/6/0 | 62/50/0/0 | 2,311.312 ms | 2,421.429 ms | 41.0 |
| rg · 활용형 열거 | 50/48/8/6 | 55/47/3/7 | 64.943 ms | 73.394 ms | 1,458.9 |
| grep · 활용형 열거 | 50/48/8/6 | 55/47/3/7 | 5,266.180 ms | 5,341.437 ms | 18.0 |
| rg · 짧은 어간 | 46/22/34/10 | 52/22/28/10 | 67.632 ms | 73.384 ms | 1,400.9 |
| grep · 짧은 어간 | 46/22/34/10 | 52/22/28/10 | 1,786.893 ms | 1,989.683 ms | 53.0 |

## 재현

```console
git checkout e7e1bd54c5d880b940ede5e937d22e40a8b452d8
KFIND_MORPH_RUNS=5 scripts/benchmark-morphology.sh \
  target/canonical-profile-baseline

git checkout d28fa2790b5592710291c5a90220698065ff8999
KFIND_MORPH_RUNS=5 scripts/benchmark-morphology.sh \
  target/canonical-profile-candidate
scripts/benchmark-search-baseline.sh

python3 tools/morph-compare/export_site_snapshot.py \
  target/canonical-profile-candidate/report.json \
  docs/benchmarks/site-morphology.json \
  --revision d28fa2790b5592710291c5a90220698065ff8999
python3 tools/search-baseline/export_site_snapshot.py \
  target/benchmark/search-baseline/d28fa2790b5592710291c5a90220698065ff8999/report.json \
  docs/benchmarks/site-search-baseline.json
```
