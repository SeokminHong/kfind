# 실제 기술 문서 검색 작업 평가

- 측정일: 2026-09-12
- 기준 코드: `bae0a005`; 후보 코드: `0768f87`; 평가 도구: `b4fb182e`
- 환경: macOS-26.6.2-arm64-arm-64bit-Mach-O, Python 3.14.7, ripgrep 15.2.0, Rust 1.97.0 release locked build
- 원본: 고정 revision의 rustlings-kr 6개 파일과 Kubernetes 문서 3개 파일
- 입력: 기존 blind fixture 25건(양성 21, 음성 4). Query·gold와 source checksum은 변경하지 않았다.
- fresh process warm-up 1회 후 5회 측정, 작업별 method 순환 순서. OS cold-cache 측정이 아니다.
- 원본 JSON은 정확한 argv, source revision·license·SHA-256, binary·resource checksum과 작업별 median/min/max를 포함한다.

## 해석

Agent는 21개 양성 작업의 정답 파일을 모두 찾았고 User는 17개, 표제어 그대로의 고정 문자열 검색은 7개를 찾았다.
정답 파일에서 다른 줄이 일치한 경우도 있으므로 파일 발견과 검토된 정답 span 발견은 별도로 본다.
Agent는 후보 105줄을 반환해 고정 문자열 검색 58줄보다 81.0% 많았다.
이 비교의 literal 기준선에는 수작업 활용형 나열이나 추가 질의가 없다. 따라서 모든 기존 검색 전략보다 우월하다는 결론은 내리지 않는다.
기준·후보의 모든 정렬된 match 결과는 같고 판정 불가 진단은 발생하지 않았다.
전체 파일의 주변 문맥을 사용하므로 excerpt 단독으로 실행한 과거 보고서와 동일한 workload가 아니다.

## 검색 작업

| method          | 정답 파일 / 양성 21 | 후보 파일 합 | 후보 줄 합 | 후보 text bytes 합 | 작업별 중앙값 합 (ms) | 판정 불가 검색 |
| --------------- | ------------------: | -----------: | ---------: | -----------------: | --------------------: | -------------: |
| rg_literal      |              7 / 21 |           17 |         58 |               4824 |               200.534 |              0 |
| baseline_agent  |             21 / 21 |           37 |        105 |               9044 |               171.440 |              0 |
| baseline_user   |             17 / 21 |           31 |         75 |               6726 |              1168.014 |              0 |
| candidate_agent |             21 / 21 |           37 |        105 |               9044 |               171.681 |              0 |
| candidate_user  |             17 / 21 |           31 |         75 |               6726 |              1164.407 |              0 |

후보량은 전체 파일에서 반환한 값이며 같은 파일이 서로 다른 작업에 등장하면 각각 센다.
모든 작업은 검색 호출 1회다. 시간 열은 25개 작업 각각의 5회 중앙값을 더한 값이며,
전체 작업 묶음을 한 번에 실행한 시간의 중앙값이 아니다. Agent 후보 시간 합은 기준 대비 +0.14%, User는 -0.31% 변했다. 이 작은 fixture의 시작 비용을 대규모 scan 처리량으로 일반화하지 않는다.

## 검토된 excerpt 품질

전체 파일의 모든 출현에는 gold가 없으므로 아래 confusion matrix는 기존에 검토된 excerpt와 gold span에서만 산출한다.
Raw와 contract-adjusted는 별도 열로 보존한다. 추가 contract review는 0건이며 두 값은 같다.

| method          | raw TP / FP / TN / FN |    raw P / R / F1 (%) | adjusted TP / FP / TN / FN | adjusted P / R / F1 (%) |
| --------------- | --------------------: | --------------------: | -------------------------: | ----------------------: |
| rg_literal      |        7 / 3 / 1 / 14 | 70.00 / 33.33 / 45.16 |             7 / 3 / 1 / 14 |   70.00 / 33.33 / 45.16 |
| baseline_agent  |        20 / 3 / 1 / 1 | 86.96 / 95.24 / 90.91 |             20 / 3 / 1 / 1 |   86.96 / 95.24 / 90.91 |
| baseline_user   |        14 / 3 / 1 / 7 | 82.35 / 66.67 / 73.68 |             14 / 3 / 1 / 7 |   82.35 / 66.67 / 73.68 |
| candidate_agent |        20 / 3 / 1 / 1 | 86.96 / 95.24 / 90.91 |             20 / 3 / 1 / 1 |   86.96 / 95.24 / 90.91 |
| candidate_user  |        14 / 3 / 1 / 7 | 82.35 / 66.67 / 73.68 |             14 / 3 / 1 / 7 |   82.35 / 66.67 / 73.68 |

## 한계

이 평가는 작은 고정 기술 문서 집합의 검색 단계만 다룬다. 신규 저장소로의 일반화, 사람의 문맥 판단 시간,
LLM 토큰 비용이나 코드 수정 완료율을 측정하지 않는다. 고정 fixture는 제품 규칙 선택에 사용하지 않는다.

## 재현

```sh
scripts/build-full-pos.sh target/full-pos
scripts/build-component-resource.sh target/component-resource
mkdir -p target/task-search/resources
cp target/full-pos/lexicon.bin target/task-search/resources/
cp target/component-resource/morphology-component-compact.kfc target/task-search/resources/
cp data/enriched/predicates.tsv target/task-search/resources/
scripts/benchmark-run.sh run --name real-task-search -- \
  python3 tools/morph-compare/real_corpus/task_search.py \
    --baseline target/review-baseline/kfind --baseline-revision bae0a005 \
    --candidate target/release/kfind --candidate-revision 0768f87 \
    --data-dir target/task-search/resources \
    --warmups 1 --runs 5 --output target/task-search/report.json
```

각 binary는 지정한 revision에서 `cargo build --release --locked -p kfind-cli --bin kfind`로 빌드한다.
