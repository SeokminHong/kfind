# 실제 코퍼스 fixture

[English](README.md)

`morphology_cases.tsv`에서 `corpus.*` feature를 가진 항목은 공개 코퍼스에서 가져온 짧은 원문 회귀 사례다. 식별자는 `corpus.<source>.<split>.<id>` 형식이다.

## 출처

- `corpus.klue.*`: [KLUE revision `349481e`](https://huggingface.co/datasets/klue/klue/tree/349481ec73fff722f88e0453ca05c77a447d967c), CC BY-SA 4.0. Hugging Face Dataset Viewer의 `dp`, `ynat`, `wos` train split에서 행 번호 또는 guid로 추출했다.
- `corpus.nsmc.*`: [NSMC commit `cc0670e`](https://github.com/e9t/nsmc/tree/cc0670e872d4ac27bfe36c87456783004b39ef6c), CC0 1.0. `ratings_test.txt`에서 review id로 추출했다.

문장은 원문의 띄어쓰기와 표기를 보존한다. `no-match` 항목은 `smart` 경계가 붙여 쓴 활용형이나 뉴스 제목의 연결 문자열을 부분 토큰으로 허용하지 않는지 검증한다.
