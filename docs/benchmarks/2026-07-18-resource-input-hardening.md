# 외부 resource 입력 경계 강화

- 측정일: 2026-07-18
- 최신 `origin/main` 및 기준 revision:
  `852649780403df0ad9ffe67dd64005a6f531ae54`
- 후보 코드 revision: `75311da6dbda9efbc9378a291634c3fd2a63c6b9`
- benchmark 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0, Docker 29.6.1
- 반복: fresh process warm-up 1회 뒤 5회 측정의 중앙값과 min/max
- canonical fixture:
  `1497b958a6970c55bc68ff148e435a88366b650c971231c3ae40adb9d8c46572`
- full POS lexicon artifact:
  `012a2ecfc9ee049cb48f655eb240fa2ed6fc739dfde01526078a976549246e88`
- component artifact:
  `d3a7eb486eef5faa92e006dd72e5ff72b63befb4bb102013f2d19e5fc32ff00a`
- 100 MiB corpus:
  `7692072cb7bff9261c1fa5933bde41b27e558170818eeac6d07cabdd673815ff`
- 기준 report SHA-256:
  `f3398d6ef416cc2b4b8780c0f017e39ba47938765c44908462bc68ec03565e0a`
- 후보 확인 report SHA-256:
  `94457c657841e0f25b68924a5939ecbb364d081ecf3f5d88a03cadffdb5b1015`

## 결론

Full POS와 component binary resource는 각각 128 MiB, enriched predicate와 user lexicon
text는 각각 16 MiB로 제한했다. Native CLI는 file metadata를 먼저 검사하고 같은 file
handle에서 `limit + 1` byte까지만 읽어 metadata가 없거나 읽는 중 파일이 커지는 경우도
제한한다. 두 binary decoder는 추가 할당, digest와 payload 검증 전에 자체 상한을 다시
검사한다.

변경 경로의 모든 성능 변화는 10% 회귀 경고선 안이고 측정 범위가 겹친다. Candidate의 첫
측정에서 Human cases/s -3.27%, p95 +4.13%가 나왔으나 같은 image 확인 측정에서는 각각
+2.58%, -2.59%로 반전됐다. 제품 경로와 무관한 시스템 변동으로 판정하며 성능 향상은
주장하지 않는다. 제한된 입력의 정상 동작과 품질은 기준선과 같다.

## 퍼징

기존 6개 target에 `pos_resource`와 `component_resource`를 추가했다. 전자는 full POS의
header, varint, UTF-8, NFC, 정렬과 누적 decode 경계를 직접 실행한다. 후자는 임의 byte
resource와 encoder가 만든 유효한 소형 component resource를 함께 사용해 header, digest,
payload와 prefix lookup을 검사한다.

`nightly-2026-07-11`, `cargo-fuzz 0.13.2`에서 target당 15초, 입력당 timeout 5초,
RSS 상한 2 GiB로 8개 target을 실행했다. 총 4,337,562개 입력에서 crash, panic, timeout과
RSS 초과는 0건이었다. 신규 target 결과는 다음과 같다.

| target | 실행 입력 | 새 corpus unit | peak RSS |
| --- | ---: | ---: | ---: |
| `pos_resource` | 1,356,062 | 2,030 | 593 MiB |
| `component_resource` | 122,307 | 338 | 406 MiB |

## 품질

Canonical 1,000건, development와 hard-negative, explicit-POS query matrix 2,592건,
Robust와 Agent/Human workflow의 quality, failure record를 기준·후보·확인 report에서
대조했다. 모든 confusion matrix, case-level failure와 CLI matching line 수가 같다.
Resource 상한보다 작은 기존 artifact의 decode 결과도 같다.

## 성능

후보는 첫 결과의 변동을 확인한 두 번째 report다. `median [min, max]`이며 양수 변화는
시간·latency 증가 또는 처리량 증가를 뜻한다.

| workload / metric | 기준 | 후보 확인 | 변화 |
| --- | ---: | ---: | ---: |
| full-POS startup | 0.031593 s [0.031344, 0.031749] | 0.031876 s [0.031538, 0.033301] | +0.90% |
| embedded+component startup | 0.024352 s [0.024018, 0.025517] | 0.025267 s [0.024732, 0.025581] | +3.76% |
| full-POS+component startup | 0.055027 s [0.054653, 0.056731] | 0.056234 s [0.055487, 0.057920] | +2.19% |
| canonical full-POS cases/s | 20,143.8 [18,212.6, 20,316.4] | 20,077.0 [18,207.8, 20,265.6] | -0.33% |
| canonical full-POS p95 | 0.1303 ms [0.1288, 0.1458] | 0.1317 ms [0.1302, 0.1416] | +1.07% |
| Human cases/s | 17,978.4 [17,088.4, 18,394.7] | 18,442.9 [18,081.2, 18,503.3] | +2.58% |
| Human p95 | 0.1429 ms [0.1395, 0.1478] | 0.1392 ms [0.1382, 0.1404] | -2.59% |
| 100 MiB Agent CLI wall | 0.017094 s [0.016246, 0.018686] | 0.017923 s [0.017033, 0.020262] | +4.85% |
| 100 MiB Human CLI wall | 0.075803 s [0.074487, 0.084973] | 0.074512 s [0.073543, 0.076182] | -1.70% |

Agent CLI는 resource를 읽지 않는 `--embedded --boundary any` 대조군이다. 이 경로의
변화도 범위가 겹친다. Full-POS와 component startup peak RSS 중앙값은 각각
21,840 KiB와 53,400 KiB로 기준의 21,816 KiB, 53,400 KiB와 같다.

## 재현

```console
git switch --detach 852649780403df0ad9ffe67dd64005a6f531ae54
KFIND_MORPH_IMAGE=kfind-morph-benchmark:hardening-base-8526497 \
KFIND_MORPH_RUNS=5 \
scripts/benchmark-morphology.sh target/morph-hardening-base-8526497

git switch --detach 75311da6dbda9efbc9378a291634c3fd2a63c6b9
KFIND_MORPH_IMAGE=kfind-morph-benchmark:hardening-candidate-75311da \
KFIND_MORPH_RUNS=5 \
scripts/benchmark-morphology.sh target/morph-hardening-candidate-rerun-75311da

KFIND_FUZZ_TOOLCHAIN=nightly-2026-07-11 \
KFIND_CARGO_FUZZ_VERSION=0.13.2 \
KFIND_FUZZ_SECONDS=15 \
KFIND_FUZZ_TIMEOUT_SECONDS=5 \
KFIND_FUZZ_RSS_LIMIT_MB=2048 \
scripts/run-fuzz.sh
```
