# 실제 corpus fixture

`morphology_cases.tsv`에서 `corpus.*` feature를 가진 항목은 공개 corpus의 짧은
회귀 사례입니다. 식별자는 `corpus.<source>.<split>.<id>` 형식입니다.

## 출처

- `corpus.klue.*`는 [KLUE revision `349481e`](https://huggingface.co/datasets/klue/klue/tree/349481ec73fff722f88e0453ca05c77a447d967c)의 `dp`, `ynat`, `wos` train split에서 행 번호 또는 guid로 선택했습니다. 라이선스는 CC BY-SA 4.0입니다.
- `corpus.nsmc.*`는 [NSMC commit `cc0670e`](https://github.com/e9t/nsmc/tree/cc0670e872d4ac27bfe36c87456783004b39ef6c)의 `ratings_test.txt`에서 review ID로 선택했습니다. 라이선스는 CC0 1.0입니다.

문장은 원문의 띄어쓰기와 표기를 보존합니다. `no-match` 사례는 `smart` 경계가
붙여 쓴 활용형이나 뉴스 제목의 연결 문자열을 부분 token으로 허용하지 않는지
검증합니다.

`walk_hang_stress.txt`는 직접 구성한 회귀 fixture입니다. 동형 동사 활용의 합집합,
생산적 어미, 복합어 안의 용언 명사형, 보조 용언 continuation과 별도 파생 표제어
제외를 함께 검증합니다. `verify-gold`는 full resource에서 `v:걷다` 97개,
`v:걸다` 21개의 논리적 시작 위치를 요구합니다.
