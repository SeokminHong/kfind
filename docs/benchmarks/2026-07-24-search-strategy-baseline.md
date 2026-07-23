# 형태 질의와 정규식 검색 기준선

- 측정일: 2026-07-24
- revision: `d28fa2790b5592710291c5a90220698065ff8999`
- report schema: 2
- fixture: 112 cases, 7 queries,
  `0a99bd90ce34169f1fc6e294525ee2d4ec782240570fd0c023acdfe609bbd914`
- raw: positive 56, negative 56
- contract-adjusted: positive 62, negative 50, reviewed 6
- performance corpus: 458,752 lines, 13.54 MiB,
  `cfd252958729588624625d58e942781f2d1326b36d1690d7c17488fb60862d40`
- timing: warm-up 2회 뒤 10회, 7개 query fresh-process batch

## 판정

이 결과는 동일한 형태 질의에서 형태·품사 판정과 수동 정규식의 coverage·경계 trade-off를
보여 주는 constructed 진단이다. Held-out 품질 benchmark나 일반적인 한국어 검색 품질의
순위로 해석하지 않는다.

Kfind any와 smart는 raw positive 56건을 모두 회수했다. Any는 strict negative에서 FP 9건,
smart는 FP 6건을 허용했다. 고정 contract review를 같은 예측에 적용하면 any는
TPᶜ/TNᶜ/FPᶜ/FNᶜ 62/47/3/0, smart는 62/50/0/0이다. Contract-adjusted는 별도 실행 모드나
후처리가 아니다.

활용형 열거 정규식은 raw FN 6건과 FP 8건, 짧은 어간 정규식은 FN 10건과 FP 34건이었다.
어간을 짧게 열면 recall은 유지되지만 경계 밖 문자열을 함께 잡는 비용이 커졌다. Smart raw
F1은 열거 정규식보다 7.20%p, 짧은 어간 정규식보다 27.27%p 높다. Any raw F1은 각각
4.84%p, 24.91%p 높다.

실행시간은 `rg`가 두 정규식에서 각각 64.94 ms와 67.63 ms로 가장 짧았다. Kfind any는
491.54 ms, smart는 2311.31 ms였다. BSD `grep`은 열거 정규식 5266.18 ms와 어간 정규식
1786.89 ms였다. 이는 13.54 MiB 단일 파일을 7개 fresh process가 각각 스캔한 batch
시간이다. 정규식 도구, pattern과 boundary에 따라 결과가 달라지므로 품질과 시간을 하나의
점수로 합치지 않는다.

## 환경

- platform: macOS 26.4.1 arm64
- CPU: Apple M1 Max, logical CPUs 10
- memory: 32 GiB
- Python: 3.12.13
- kfind: 1.0.0-rc.1, binary
  `3c09c3475d72f11771dcd153b7b62092a413552e4f7b2fd47d2c4e68a5afdff4`
- full POS:
  `012a2ecfc9ee049cb48f655eb240fa2ed6fc739dfde01526078a976549246e88`
- component:
  `e6219f8bbdf08d56a1a03f724b814952fc2050754b5f50fe6f2a3656a62feb52`
- ripgrep: 15.1.0
- grep: BSD grep 2.6.0-FreeBSD

## 전체 품질

| method           | raw TP/TN/FP/FN |          raw P/R/F1 | TPᶜ/TNᶜ/FPᶜ/FNᶜ |             Pᶜ/Rᶜ/F1ᶜ |
| ---------------- | --------------: | ------------------: | --------------: | --------------------: |
| kfind any        |       56/47/9/0 | 86.15/100.00/92.56% |       62/47/3/0 |   95.38/100.00/97.64% |
| kfind smart      |       56/50/6/0 | 90.32/100.00/94.92% |       62/50/0/0 | 100.00/100.00/100.00% |
| enumerated regex |       50/48/8/6 |  86.21/89.29/87.72% |       55/47/3/7 |    94.83/88.71/91.67% |
| stem regex       |     46/22/34/10 |  57.50/82.14/67.65% |     52/22/28/10 |    65.00/83.87/73.24% |

`rg`와 `grep`은 같은 정규식에서 matching line 집합이 같았다. 품질은 정규식 전략별로
합치고 실행시간은 도구별로 분리했다.

## Fresh-process batch 시간

| method          |     median |        min |        max |        p95 | effective MiB/s |
| --------------- | ---------: | ---------: | ---------: | ---------: | --------------: |
| kfind any       |  491.54 ms |  488.00 ms |  507.89 ms |  507.89 ms |           192.8 |
| kfind smart     | 2311.31 ms | 2283.46 ms | 2421.43 ms | 2421.43 ms |            41.0 |
| rg enumerated   |   64.94 ms |   63.03 ms |   73.39 ms |   73.39 ms |          1458.9 |
| grep enumerated | 5266.18 ms | 5235.50 ms | 5341.44 ms | 5341.44 ms |            18.0 |
| rg stem         |   67.63 ms |   63.23 ms |   73.38 ms |   73.38 ms |          1400.9 |
| grep stem       | 1786.89 ms | 1779.30 ms | 1989.68 ms | 1989.68 ms |            53.0 |

## Query별 raw F1

| query        | kfind any | kfind smart | enumerated regex | stem regex |
| ------------ | --------: | ----------: | ---------------: | ---------: |
| `v:걷다`     |    84.21% |      84.21% |           75.00% |     66.67% |
| `v:듣다`     |    84.21% |      84.21% |           77.78% |     69.57% |
| `v:돕다`     |   100.00% |     100.00% |           85.71% |     80.00% |
| `v:짓다`     |   100.00% |     100.00% |           93.33% |     66.67% |
| `v:부르다`   |    94.12% |     100.00% |           94.12% |     55.56% |
| `adj:예쁘다` |   100.00% |     100.00% |          100.00% |     80.00% |
| `v:되다`     |    88.89% |     100.00% |           88.89% |     58.82% |

## 방법

Enumerated regex는 사람이 고른 활용 표면형을 `|`로 열거한다. Stem regex는 짧은 어간
후보만 열거한다. 두 정규식에는 자동 활용 생성, 품사 판정과 token boundary가 없다.

Kfind는 같은 full POS·component resource에서 `boundary=any`와 `boundary=smart`를 각각
실행한다. 품질과 batch 시간은 독립된 행으로 보존한다.

각 시간 batch는 7개 질의를 별도 fresh process로 실행해 같은 파일을 7회 스캔한다.
Matching-line count만 계산하고 stdout은 폐기한다. 방법 순서는 round마다 순환한다.

JSON report SHA-256은
`40e3c1fcfc7b803fe48121d8e73e7e7dde2a61de70f90213890e58b43bec5021`다. JSON에는 정규식,
실제 executable·입력 경로를 포함한 전체 명령 배열, 개별 run과 case-level failure를
보존했다.

## 재현

```console
scripts/benchmark-search-baseline.sh
```
