# 형태소 분석 비교 이미지

[English](README.md)

이 개발 도구는 동일한 fixture 문장을 `kfind`, Kiwi, Lindera로 처리한다. 외부 분석기는 제품 바이너리나 기본 검색 경로에 포함되지 않는다.

기본 실행은 출처가 고정된 `corpus.*` 사례를 사용한다.

```sh
scripts/compare-morphology.sh
```

결과는 `target/morph-compare/report.json`과 `report.md`에 생성된다. 이미지를 빌드한 뒤 컨테이너는 `--network none`으로 실행된다.

직접 실행하려면 다음 명령을 사용한다.

```sh
docker build -f tools/morph-compare/Dockerfile -t kfind-morph-compare:local .
docker run --rm --network none \
  --user "$(id -u):$(id -g)" \
  -v "$PWD/target/morph-compare:/output" \
  kfind-morph-compare:local
```

`kfind`는 fixture의 일치·불일치 기대값을 모두 검증한다. Kiwi와 Lindera는 `match` 사례에서 표제어와 호환 품사를 회수했는지를 비교하며, `no-match` 분석 결과는 점수화하지 않는다.
