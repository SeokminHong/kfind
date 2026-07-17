# 수동 검토 자연 오류 Robust 품질·성능

- 측정일: 2026-07-17
- revision: `741fb91415906841daf6ecaf4146bfa33e924c61`
- report schema: 19
- report SHA-256: `7b755afc74acf21bd0249e80f11dee663a46ff8f59df815bb1333967693bd1cf`
- image: `sha256:7ee4c0bfc43cd95f43ed9986d43074cac5136fb0f88b76420a9154e9870414ea`
- 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs, Python 3.12.13,
  Rust 1.97.0, Docker 29.6.1
- 반복: fresh process warm-up 1회 뒤 5회 측정의 median과 min/max
- Robust explicit-POS fixture:
  `6bfa1c00d1d4469742d100099eab3a4d6d0d679d2ed147cda1ca2e980a64e282`
- Robust untagged fixture:
  `8123235998e3b10d73817c99b66cb8312d76052afdee1401d7041cad6bb53e8a`
- external snapshot SHA-256:
  `426778d106830fc2c0b0c42149a01685de7a1663c0e547385aeab52b8513e186`

## 결론

표준문 canonical과 오류문 Robust를 분리했다. Canonical은 UD Korean-Kaist sampling 후보를
수동 검토해 표준 맞춤법을 확인한 문장만 사용한다. Robust는 UD Korean-KSL의 실제 오류 문장을
수동 검토하고 query·품사·원문 span gold까지 확정한 별도 500-case다. 두 점수는 합산하지
않는다.

같은 explicit-POS Robust gold에서 kfind Agent는 precision 97.89%, recall 92.80%, F1
95.28%였다. Kiwi는 F1 92.01%, Lindera 90.83%, MeCab-ko 90.35%, KOMORAN 90.11%였다.
kfind Agent는 오류 입력 recall과 F1이 가장 높았지만 false positive가 5건 있었고, 외부 네
제품은 false positive 없이 recall이 더 낮았다.

오류가 찾으려는 형태소에 직접 걸린 100건의 recall은 kfind Agent 90.00%, Kiwi 85.00%,
Lindera와 MeCab-ko 83.00%, KOMORAN 81.00%였다. 따라서 주변 문맥 오류만 통과한 결과가 아니라
목표 형태소 자체의 오류 대응 차이도 별도로 확인된다. 이 수치는 robust 복구 기능을 켠 비교가
아니라 모든 제품 기본 설정과 kfind `robustness=off`의 첫 기준선이다.

## 코퍼스와 수동 검토

원문은 Universal Dependencies 2.18 Korean-KSL test split이다. 원문 SHA-256은
`62574d11b83f62217494a53fd2a7cbf75b7fc3fe5df74021a91e66df65149033`이며 라이선스는
CC BY-SA 4.0이다.

`Typo=Yes`·`goeswith` source signal 441문장과 수사 quota 보충 4문장을 먼저 고정하고 445문장
전체를 수동 검토했다. 정상문 5개와 source artifact 1개를 제외하고 실제 오류 439문장을
확정했다. Review pool digest는
`d32afb9cd86d09012eb99ae664a831d56abfc166226bf7190ef70f737e180365`, review manifest
SHA-256은 `63166d15236566c2fe24d99ca16b928b9833f5f32bd170320902ce24137feced`다.

최종 explicit-POS 500건과 untagged 500건은 각각 250 positive·250 negative다. 선택된
query, 품사, expected, 원문 byte span과 negative 부재 여부를 다시 검토했다. Explicit-POS
case review digest는
`4aa6bd4defe2f44352bc8a7b5632d3c80826437385a632353497224be099ce45`, untagged digest는
`f77f6af2c01f21a5365c4455577c34979a1bfbb4aa0a9b43ee4e49db0e0b2b46`다.

| POS | positive | negative |
| --- | ---: | ---: |
| 명사 | 90 | 90 |
| 동사 | 60 | 60 |
| 형용사 | 40 | 40 |
| 부사 | 25 | 25 |
| 대명사 | 15 | 15 |
| 관형사 | 10 | 10 |
| 수사 | 10 | 10 |
| 합계 | 250 | 250 |

Positive 250건 중 오류가 gold span에 직접 겹치는 `target-span`은 100건, 오류가 주변 token에만
있는 `context-only`는 150건이다. Negative 250건은 모두 오류 문장을 유지한 context-only
case다. 전체 precision·recall·F1은 500건을 사용하고 target/context recall은 각 positive
분모만 사용한다.

## 제품 기본 품질 비교

kfind 행은 Agent 제품 경로인 `embedded + any + explicit POS`, 외부 행은 같은 explicit-POS
fixture에서 고정 기본 설정을 사용한다. 모든 행에서 별도 오류 교정 전처리를 사용하지 않았다.

| 제품 | precision | recall | F1 | TP | FP | TN | FN | target recall | context recall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| kfind Agent | 97.89% | 92.80% | 95.28% | 232 | 5 | 245 | 18 | 90.00% | 94.67% |
| Kiwi 0.23.2 | 100.00% | 85.20% | 92.01% | 213 | 0 | 250 | 37 | 85.00% | 85.33% |
| Lindera 4.0.0 | 100.00% | 83.20% | 90.83% | 208 | 0 | 250 | 42 | 83.00% | 83.33% |
| MeCab-ko 1.0.2 | 100.00% | 82.40% | 90.35% | 206 | 0 | 250 | 44 | 83.00% | 82.00% |
| KOMORAN 3.3.9 | 100.00% | 82.00% | 90.11% | 205 | 0 | 250 | 45 | 81.00% | 82.67% |

오류 class별 사례 수가 크게 다르므로 class 행을 전체 제품 순위로 해석하지 않는다.

| 제품 | 한글 오타 442 recall | 띄어쓰기 분리 50 recall | 비표준 문법 7 recall | 외국어 오타 1 recall |
| --- | ---: | ---: | ---: | ---: |
| kfind Agent | 92.38% | 93.94% | 100.00% | 100.00% |
| Kiwi 0.23.2 | 83.33% | 96.97% | 100.00% | 0.00% |
| Lindera 4.0.0 | 81.90% | 90.91% | 83.33% | 100.00% |
| MeCab-ko 1.0.2 | 81.43% | 90.91% | 83.33% | 0.00% |
| KOMORAN 3.3.9 | 80.48% | 93.94% | 83.33% | 0.00% |

## 별도 kfind 경로

같은 explicit-POS fixture의 `smart` 진단은 embedded F1 84.79%, full-POS F1 89.62%였다.
품사를 생략한 Human `full-POS + smart`는 별도 untagged fixture에서 precision 99.52%, recall
83.60%, F1 90.87%, target recall 68.00%, context recall 94.00%였다. Negative와 query 입력
계약이 다르므로 제품 기본 explicit-POS 표에 합치지 않는다.

## 성능

외부 결과는 같은 fixture에 묶인 고정 snapshot이며, 모든 행은 warm-up 1회 뒤 fresh process
5회의 중앙값이다.

| 제품 | init s | cases/s | p50 ms | p95 ms | peak RSS MiB |
| --- | ---: | ---: | ---: | ---: | ---: |
| kfind Agent | 0.0015 | 25,239.0 | 0.0246 | 0.0685 | 4.5 |
| Kiwi 0.23.2 | 1.8788 | 1,713.4 | 0.4520 | 1.3710 | 527.5 |
| Lindera 4.0.0 | 0.0293 | 24,956.2 | 0.0299 | 0.1007 | 165.4 |
| MeCab-ko 1.0.2 | 0.0003 | 11,454.7 | 0.0688 | 0.2169 | 95.4 |
| KOMORAN 3.3.9 | 1.1644 | 1,329.2 | 0.5219 | 1.8062 | 522.4 |

kfind Agent cases/s 범위는 24,584.3~25,418.7, p95 범위는 0.0670~0.0695 ms였다. 외부
분석기의 전체 min/max와 kfind smart·Human 성능은 원본 JSON과 생성 Markdown에 보존한다.

## 한계

- 이 결과는 자연 오류 문장에서 표제어·품사·span 존재를 찾는 검색 품질이다. 문장 전체의
  형태소 분석 정확도나 교정문 자연스러움을 측정하지 않는다.
- Korean-KSL 한 source의 한글 오타 비중이 442/500으로 높다. 비표준 문법 7건과 외국어 오타
  1건은 진단값이며 class별 일반화를 지지하지 않는다.
- `target-span`에는 positive만 있으므로 해당 표의 precision은 비교하지 않는다. False positive는
  전체 250 negative에서 계산한다.
- 제품별 native robust 옵션이나 별도 오타 교정기를 켠 feature-matched 비교는 후속 fixture와
  원문 span 역매핑 계약을 고정한 뒤 default 표와 분리해 측정한다.

## 재현

외부 snapshot을 만들었다.

```console
docker build -f tools/morph-compare/external/Dockerfile \
  -t kfind-morph-baseline-refresh:robust-quality .
docker run --rm \
  -v "$PWD/target/robustness-review:/input:ro" \
  -v "$PWD/tools/morph-compare/external:/output" \
  kfind-morph-baseline-refresh:robust-quality \
  --cases /input/cases.jsonl \
  --metadata /input/metadata.json \
  --output /output/robustness-baselines.json \
  --runs 5
```

최종 report를 측정했다.

```console
docker build -f tools/morph-compare/Dockerfile \
  -t kfind-morph-benchmark:robust-quality-rebased .
mkdir -p target/robust-quality-rebased
docker run --rm \
  -v "$PWD/target/robust-quality-rebased:/output" \
  kfind-morph-benchmark:robust-quality-rebased \
  --runs 5 \
  --output /output/report.json
```

Site snapshot과 chart는 같은 report에서 생성한다.

```console
python3 tools/morph-compare/export_site_snapshot.py \
  target/robust-quality-rebased/report.json \
  docs/benchmarks/site-morphology.json \
  --revision 741fb9141590
python3 tools/morph-compare/render_charts.py \
  docs/benchmarks/site-morphology.json \
  target/site-benchmark-charts
```
