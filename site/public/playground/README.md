# 한국어 위키백과 Playground corpus

`korean-wikipedia-20231101-ko-1mib.txt`는
[`wikimedia/wikipedia`](https://huggingface.co/datasets/wikimedia/wikipedia)의
`20231101.ko` snapshot에서 Dataset Viewer 행 순서대로 추출한 1 MiB 한국어 본문이다.
각 문서의 줄 끝 공백을 제거하고, 마지막 문서는 UTF-8 문자 경계에서 잘랐다.

원문은 [CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/)에 따라
재배포한다. 각 문서의 제목과 한국어 위키백과 URL은 corpus 본문에 포함하고, 사용한 행과
변환 정보는 `korean-wikipedia-20231101-ko-1mib.sources.json`에 기록한다. 저장소의 MIT
License는 이 corpus 원문에 적용되지 않는다.

고정된 dataset revision을 확인하고 corpus와 manifest를 다시 생성한다.

```console
pnpm --dir site run generate:playground-corpus
```
