# 명사절 안의 관형형과 대명사 범위

GSD s401의 `느낌이 나는 무늬를 쓰는 것도`에서 Smart가 명사·대명사
`나`와 동사 `날다`를 반환하던 [103, 109) 범위를 제거했다. 동사 `나다`는 원문과
`자리 잡고`로 띄어 쓴 교정문에서 같은 범위를 유지했다. 기존 원문
코퍼스와 계약 검토를 사용하고, 고정 회귀의 서술어는 `자리 잡고`로
띄어 썼다.
[GSD 교정 근거](2026-11-23-gsd-nominal-predicate-spacing-review.md)
· [국립국어원 띄어쓰기 답변](https://www.korean.go.kr/front/onlineQna/onlineQnaView.do?mn_id=&pageIndex=1&qna_seq=329023&searchCondition=&searchKeyword=)

발생 관형절의 앞 주격과 뒤 목적어를 찾고, 그 목적어를 받는 동사가
`것`, `-기`, `-(으)ㅁ`으로 명사절을 이루는지 확인했다. 바깥
서술어는 형용사, 지정사, 목적격을 취하지 않는 동사와 진행 구성을
검증했다. `것도`가 목적어인 `경고음이 나는 음악을 듣는 것도
방해했다`와 시간절 `그 기계의 소리가 나는 책을 읽는 동안
들렸다`의 대명사 반환은 보존했다.

한국어기초사전 `나다`의 감정, 감각, 신체 현상, 기억, 외양을
나타내는 여덟 뜻의 주격 용례와 기존 소리·분비물 품사 자료를
대조했다. 기본 용언 사전과 MeCab-ko-dic의 VA·VV가 겹치는 어간
10개를 확인해 표준 형용사 단독 여섯 개의 형태 판정 근거를 분리했다.
`낫다·누르다·이르다·크다`의 동사 분석을 보존했다.
같은 표면의 `날다`는 발생 주어가 감정이나 기억이고 이동 논항이
없을 때 제외했다. 물리적 용기를 뜻하는 동형 명사와 실제 비행
문맥에서는 `날다` 범위를 반환했다.
[한국어기초사전 나다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=62210)
· [좋다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=79033)
· [날다](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=61530)

중첩 명사절과 경쟁 구조 31개에서 두 빌드의 전체 반환 범위를
대조했다. 오반환 19범위를 제거하고 서로 다른 실패 경로와
대조군을 고정 회귀 17행에 남겼다. 대조 질의 입력의 SHA-256은
`6627cf5c06fff0a586d41d1ed2ffe3c8cb956bb26781f6380b18259c5668ec5c`다.

## 고정 음성과 GSD

71,941개 질의의 전체 범위를 비교했다. 고정 음성에서 s401의
명사·대명사 `나`와 동사 `날다`의 세 범위만 사라졌다.
행렬은 TP, FP, TN, FN 순서이며 지표의 단위는 %다.

| 입력·빌드 | 원본 행렬 / 정밀도·재현율·F1 | 계약 행렬 / 정밀도·재현율·F1 | FPc | FNc | 검토 / 제외 |
| --- | --- | --- | ---: | ---: | ---: |
| 고정 음성 53,308 기준선 | 0, 3,020, 50,288, 0 / 0, 0, 0 | 1,430, 1,475, 50,254, 0 / 49.23, 100, 65.97 | 1,477 | 0 | 1,589 / 149 |
| 고정 음성 53,308 후보 | 0, 3,017, 50,291, 0 / 0, 0, 0 | 1,430, 1,472, 50,257, 0 / 49.28, 100, 66.02 | 1,474 | 0 | 1,589 / 149 |
| 원문 GSD 9,335 양쪽 | 8,225, 0, 0, 1,110 / 100, 88.11, 93.68 | 8,651, 9, 97, 538 / 99.90, 94.15, 96.94 | 20 | 538 | 622 / 40 |
| 교정 GSD 9,298 양쪽 | 9,286, 0, 0, 12 / 100, 99.87, 99.94 | 9,285, 2, 11, 0 / 99.98, 100, 99.99 | 6 | 0 | 155 / 0 |

| 입력·리소스 | SHA-256 |
| --- | --- |
| 고정 음성 | `482e55d417516e8db503dbd421725fcc7affc6c6afcf5b5d42cec3bbd4d35d54` |
| 원문 GSD | `eaafbae7757dbb62c9a0ca6b370dadf0f41b53c3e3d044f21b191c2aab8e3682` |
| 교정 GSD | `66466637b6439a7ca8553e0b493d92aeb28e55394316042850f04e3d20384cf1` |
| Full POS | `47bb7a3a31d6985f0b31dab5632b0d0a07feefa7cd2d94f220ec5d5483ac735f` |
| Compact component | `f7df2acb53b3cf9c9e3070c8989552c527d75aba155aa06bb705dac50c6926d1` |
| 용언 사전 | `67e99dc49c8d74d7584b370d1ca26e2e07e30ad5dce9391a7615734fadcc668e` |

## 네이티브 성능

Apple M1 Max 32 GiB, macOS 26.6.2 arm64, Rust·Cargo 1.97.0,
Python 3.14.8에서 `be179f60`과 `f5483b40`의 release를 비교했다.
워밍업 1회 후 빌드별로 새 프로세스에서 5회씩 순서를 교대해 측정했다.
각 실행의 p95는 최근접 순위로 구했다. 표는 중앙값 [최솟값, 최댓값]이다.

```sh
cargo build --locked --release --manifest-path tools/morph-compare/runner/Cargo.toml
node tools/morph-compare/replay-gsd-audit-positives.mjs "$raw" > "$corrected"
scripts/benchmark-run.sh run --name morphology-embedded-object-mental -- \
  python3 tools/morph-compare/measure-smart.py "$input" "$out" \
  --baseline "$baseline_runner" be179f60 "$lexicon" "$component" "$predicates" \
  --candidate "$candidate_runner" f5483b40 "$lexicon" "$component" "$predicates"
python3 tools/morph-compare/score-ud-audit.py --residual "$input" "$output"
```

| 입력 | 빌드 | 초기화 (ms) | 처리량 (건/s) | p95 (ms) | 최대 RSS (KiB) |
| --- | --- | ---: | ---: | ---: | ---: |
| 고정 음성 | 기준선 | 63.674 [62.798, 64.143] | 19,240.4 [18,559.3, 19,339.5] | 0.1275 [0.1266, 0.1325] | 131,088 [131,072, 131,232] |
| 고정 음성 | 후보 | 63.252 [62.845, 63.714] | 19,210.9 [16,815.7, 19,248.7] | 0.1275 [0.1269, 0.1346] | 131,184 [131,072, 131,296] |
| 교정 GSD | 기준선 | 63.123 [62.815, 63.734] | 29,016.5 [27,872.8, 29,044.8] | 0.0742 [0.0738, 0.0767] | 83,824 [83,760, 83,936] |
| 교정 GSD | 후보 | 63.138 [62.774, 64.088] | 28,687.6 [27,192.7, 29,080.0] | 0.0744 [0.0740, 0.0783] | 83,904 [83,776, 84,192] |

고정 음성의 처리량과 p95 중앙값은 −0.15%, 0.00%, 교정 GSD는
−1.13%, +0.27%였다. 고정 음성의 후보 최저 처리량은 16,815.7건/s였다.

## WASM

wasm-pack 0.15.0과 Node 26.11.1로 기본 번들의 크기를 비교했다.
별도 Full POS와 compact component는 두 실행에 같은 파일을
장착했다. component는 37,450,453 B다. 대조 질의 31개의 NFC·NFD
반환 범위를 네이티브와 비교한 124개 검사를 모두 통과했다.

```sh
wasm-pack build crates/kfind-wasm --target bundler --out-dir "$out" --out-name kfind --release --locked
```

| 형식 | 기준선 (B) | 후보 (B) | 차이 (B) |
| --- | ---: | ---: | ---: |
| 원본 | 2,899,194 | 2,911,413 | +12,219 |
| gzip 9 | 986,562 | 989,095 | +2,533 |
| Brotli 11 | 726,450 | 730,791 | +4,341 |
