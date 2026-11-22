# 동형 용언의 논항과 수량 문형

기준선은 `79d5459b`, 후보는 `3a36e648`이다. 직전 검증의 일곱 대조 입력
134행을 다시 실행했다. 기준선의 원본 오탐 39행 중 17행에서 오탐 범위가
사라졌다. 22행에서는 경쟁 분석이 문장에 드러난 조사·어미와 결합할 수 있는
근거를 대조 등록부에 적었다. 원본 기대 범위와 라벨은 보존했다.

- `-다오`에서는 `소리가 난다오`의 발생 주어와 `과일을 시장에서 산다오`의
  구매 목적어를 확인한다. 같은 목적어가 `삽니다`와 `사세요`에도 적용된다.
  `산에 간다오`의 `에`는 재배 장소에도 쓰인다.
  `부산에 산다오`에서는 구입 대상 생략과 거주처 양쪽이 가능하다.
  [나다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=62210) ·
  [날다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=61530) ·
  [사다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=49419) ·
  [살다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=69579)
- `-서야`와 `-지요/-죠`의 화면 밝기는 화면 주어와 정도 부사로 판별한다.
  `-서야`의 분비물은 `나다`의 발생 주어로, `-지요/-죠`의 식물 수량은
  존재하는 수량으로 연결한다. `-기에는/-기엔` 기준절은 시각의 적절성과 물건의
  치수를 서술하는 두 형용사 문형에 쓴다. 사람과 처소가 있는 `있다`는
  머무름과 존재가 함께 성립할 수 있다.
  [밝다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=62105) ·
  [나서다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=61183) ·
  [늦다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=64526) ·
  [국립국어원 있다 품사 설명](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=307494)
- `비나이다/비옵나이다`에서는 그릇이나 숙소 주어를 확인해 비는 동작으로 읽는다.
  `첫 글자가 가랴/가렷다`는 글자 이름 뒤의 지정사 활용이며,
  `주목할 대목은 역시 감세`는 명사 술어에 지정사가 생략된 구조다.
  [비다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=15680) ·
  [빌다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=62349) ·
  [글자](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=62490) ·
  [국립국어원 지정사 생략 설명](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=216&pageIndex=1&qna_seq=308010)

대조 회귀 33행에 어휘·어순·어미를 바꾼 문장, 경쟁 해석이 성립하는 문장,
정상 분석의 전체 반환 범위를 추가했다. 감사 질의 1,747행은 위 17행에서만
반환 범위가 바뀌었다. KAIST 2,604행과 UD 2.18 GSD·PUD에서 `난다`,
`산다`, `삽니다`, `나서야`, `있지요` 등 관련 표면으로 뽑은 44개 질의의 반환 범위는
기준선과 일치했다.

## 품질

행렬은 TP, FP, TN, FN 순서이며 비율은 precision, recall, F1 순서다.
기준선과 후보에는 동일한 최신 계약 등록부를 적용했다. 제외 사례는 KAIST 4행이며
다른 입력은 0행이다.

| 입력 | 버전 | 원본 행렬 / 비율 (%) | 계약 행렬 / 비율 (%) | 검토 |
| --- | --- | --- | --- | ---: |
| 다오 16행 | 기준선 | 8, 8, 0, 0 / 50.00, 100, 66.67 | 14, 2, 0, 0 / 87.50, 100, 93.33 | 8 |
| 다오 16행 | 후보 | 8, 6, 2, 0 / 57.14, 100, 72.73 | 14, 0, 2, 0 / 100, 100, 100 | 8 |
| 서야 16행 | 기준선 | 8, 7, 1, 0 / 53.33, 100, 69.57 | 13, 2, 1, 0 / 86.67, 100, 92.86 | 8 |
| 서야 16행 | 후보 | 8, 5, 3, 0 / 61.54, 100, 76.19 | 13, 0, 3, 0 / 100, 100, 100 | 8 |
| 지요·죠 48행 | 기준선 | 24, 18, 6, 0 / 57.14, 100, 72.73 | 34, 8, 6, 0 / 80.95, 100, 89.47 | 24 |
| 지요·죠 48행 | 후보 | 24, 10, 14, 0 / 70.59, 100, 82.76 | 34, 0, 14, 0 / 100, 100, 100 | 24 |
| 겸양 18행 | 기준선 | 9, 2, 7, 0 / 81.82, 100, 90.00 | 9, 2, 7, 0 / 81.82, 100, 90.00 | 9 |
| 겸양 18행 | 후보 | 9, 0, 9, 0 / 100, 100, 100 | 9, 0, 9, 0 / 100, 100, 100 | 9 |
| 의지 종결형 20행 | 기준선 | 10, 1, 9, 0 / 90.91, 100, 95.24 | 10, 1, 9, 0 / 90.91, 100, 95.24 | 10 |
| 의지 종결형 20행 | 후보 | 10, 0, 10, 0 / 100, 100, 100 | 10, 0, 10, 0 / 100, 100, 100 | 10 |
| 랴·렷다 8행 | 기준선 | 4, 2, 2, 0 / 66.67, 100, 80.00 | 4, 2, 2, 0 / 66.67, 100, 80.00 | 4 |
| 랴·렷다 8행 | 후보 | 4, 0, 4, 0 / 100, 100, 100 | 4, 0, 4, 0 / 100, 100, 100 | 4 |
| 지정사 조건·양보 8행 | 양쪽 | 4, 1, 3, 0 / 80.00, 100, 88.89 | 5, 0, 3, 0 / 100, 100, 100 | 4 |
| KAIST 2,604행 | 양쪽 | 1297, 2, 1300, 5 / 99.85, 99.62, 99.73 | 1296, 0, 1304, 0 / 100, 100, 100 | 25 |

## WASM과 배포 리소스

Apple M1 Max, macOS arm64, Rust/Cargo 1.97.0, wasm-pack 0.15.0의 최적화
bundler 빌드를 비교했다. 압축 크기는 Node.js 26.11.1에서 gzip level 9와
Brotli 기본 설정으로 측정했다.

| 산출물 | 기준선 bytes | 후보 bytes | 증감 bytes |
| --- | ---: | ---: | ---: |
| 최적화 WASM | 2,795,323 | 2,802,074 | +6,751 |
| WASM gzip | 944,139 | 946,158 | +2,019 |
| WASM Brotli | 695,040 | 697,031 | +1,991 |
| 외부 component | 37,206,905 | 37,227,616 | +20,711 |

추가 사전 분류 1,467행은 외부 component의 표면·분류 섹션에 저장한다.
full POS lexicon의 크기와 해시는 같다. 후보는 schema 5·6·7을 읽고,
schema 7에 들어 있는 추가 분류가 필요한 판정은 해당 분류가 있는 리소스에서 적용된다.
Node.js에서 세 schema를 각각 로드해 구매 활용과 글자의 이동·지정사를
NFC와 NFD 원문으로 검증했다.

## 네이티브 성능

Apple M1 Max, Darwin 25.6.0, Rust/Cargo 1.97.0에서 입력별 기준선과
후보를 한 번씩 워밍업한 뒤 순서를 교대하며 새 프로세스로 각각 5회 측정했다.
표는 중앙값 [최솟값, 최댓값]이다. p95에는 최근접 순위를 사용했다.

| 입력 | 버전 | 초기화 ms | 처리량 cases/s | p95 ms | 최대 RSS MiB |
| --- | --- | --- | --- | --- | --- |
| 감사 질의 | 기준선 | 61.97 [60.58, 62.46] | 30,067 [28,856, 32,374] | 0.10679 [0.09954, 0.11667] | 66.86 [66.69, 67.03] |
| 감사 질의 | 후보 | 60.82 [60.65, 61.51] | 32,385 [30,964, 32,520] | 0.10054 [0.09838, 0.10325] | 66.63 [66.58, 66.88] |
| KAIST | 기준선 | 61.25 [60.68, 61.87] | 32,216 [30,667, 32,259] | 0.08013 [0.07908, 0.08421] | 67.25 [67.16, 67.48] |
| KAIST | 후보 | 61.10 [60.78, 61.31] | 32,206 [30,876, 32,278] | 0.07992 [0.07904, 0.08221] | 67.11 [67.06, 67.19] |

감사 질의 처리량 중앙값은 +7.71%, KAIST는 -0.03%다. p95 중앙값은 각각
-5.85%, -0.26%다. 양쪽의 처리량과 p95 관측 범위는 겹친다.

```sh
CARGO_TARGET_DIR="$PWD/target" cargo build --release --locked --manifest-path tools/morph-compare/runner/Cargo.toml
KFIND_FULL_POS_LEXICON="$pos" KFIND_COMPONENT_RESOURCE="$component" \
  KFIND_ENRICHED_PREDICATES="data/enriched/predicates.tsv" \
  /usr/bin/time -l morph-benchmark-runner boundary full-pos smart "$input" "$output"
wasm-pack build crates/kfind-wasm --target bundler --out-dir "$out" --out-name kfind --release --locked
```

Full POS·보강 용언 및 감사·KAIST 입력은 [직전 측정](2026-11-22-contract-argument-roles.md#네이티브-성능)과
같다. component SHA-256은 기준선 `e9cae1337036fd935543eea1f68d439042f0b684b95b1fdccd8e9f205c27afdb`,
후보 `dae553a838a4ead05113cc037710b88844688b1ae259da703132b5c3749a6e54`다.
