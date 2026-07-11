# 독립 형태소 벤치마크

[English](README.md)

이 개발 도구는 동일한 held-out 사례를 `kfind`, Kiwi, Lindera로 처리한다. 외부 분석기와
corpus는 제품 바이너리나 기본 검색 경로에 포함되지 않는다.

fixture는 Universal Dependencies 2.18의 Korean-Kaist와 Korean-KSL test split에서
생성한다. URL, SHA-256, CC BY-SA 4.0 라이선스는 `sources.json`에 고정되어 있다. 각
source에서 품사별 250개 positive를 선택하고 같은 source의 deterministic negative를
대응시켜 총 1,000개를 만든다.

```sh
scripts/benchmark-morphology.sh
```

결과는 `target/morph-benchmark/report.json`과 `report.md`에 생성된다. 이미지를 빌드한 뒤
컨테이너는 `--network none`으로 실행된다. `scripts/compare-morphology.sh`도 같은
벤치마크를 실행한다.

직접 실행하려면 다음 명령을 사용한다.

```sh
docker build -f tools/morph-compare/Dockerfile -t kfind-morph-benchmark:local .
mkdir -p target/morph-benchmark
docker run --rm --network none \
  --user "$(id -u):$(id -g)" \
  -v "$PWD/target/morph-benchmark:/output" \
  kfind-morph-benchmark:local
```

세 도구 모두 문장 안에 gold 표제어·품사가 존재하는지 판정한다. positive는 예측 span이
gold 어절 span과 겹쳐야 하며, negative는 같은 표제어·품사를 반환하면 false positive다.
보고서는 accuracy, precision, recall, F1, source/POS별 결과, 실패 span과 초기화·처리량·
지연·peak RSS를 기록한다.

성능 수치는 각 도구를 한 번 초기화한 뒤 질의부터 결과 판정까지 처리한 제품 작업량이다.
순수 tokenizer 처리량 비교가 아니다.
