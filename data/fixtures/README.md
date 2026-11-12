# 실제 corpus fixture

`morphology_cases.tsv`에서 `corpus.*` feature를 가진 항목은 공개 corpus의 짧은
회귀 사례입니다. 식별자는 `corpus.<source>.<split>.<id>` 형식입니다.

## 출처

- `corpus.klue.*`는 [KLUE revision `349481e`](https://huggingface.co/datasets/klue/klue/tree/349481ec73fff722f88e0453ca05c77a447d967c)의 `dp`, `ynat`, `wos` train split에서 행 번호 또는 guid로 선택했습니다. 라이선스는 CC BY-SA 4.0입니다.
- `corpus.nsmc.*`는 [NSMC commit `cc0670e`](https://github.com/e9t/nsmc/tree/cc0670e872d4ac27bfe36c87456783004b39ef6c)의 `ratings_test.txt`에서 review ID로 선택했습니다. 라이선스는 CC0 1.0입니다.

문장은 원문의 띄어쓰기와 표기를 보존합니다. `no-match` 사례는 `smart` 경계가
붙여 쓴 활용형이나 뉴스 제목의 연결 문자열을 부분 token으로 허용하지 않는지
검증합니다.

`ud_morphology_cases.tsv`는 Universal Dependencies 2.18의
[Korean-Kaist test](https://github.com/UniversalDependencies/UD_Korean-Kaist/blob/r2.18/ko_kaist-ud-test.conllu),
[Korean-GSD test](https://github.com/UniversalDependencies/UD_Korean-GSD/blob/02c343e4e1e3180069f637e68a791ec6b96dd33a/ko_gsd-ud-test.conllu)와
[Korean-PUD test](https://github.com/UniversalDependencies/UD_Korean-PUD/blob/e942f9c5e9351f37bcb8f202bb9df70f78c60d35/ko_pud-ud-test.conllu)에서
선별한 문장입니다. 사례 ID에 원문 문장 ID를 기록하고, 명백한 맞춤법과 띄어쓰기 오류를
교정한 문장의 UTF-8 범위를 사용합니다. 표준어에서 허용되는 준말과 보조 용언 붙여쓰기는
원문 형태로 검증합니다.
출처 라이선스는 [Kaist CC BY-SA 4.0](https://github.com/UniversalDependencies/UD_Korean-Kaist/blob/r2.18/LICENSE.txt),
[GSD CC BY-SA 4.0](https://github.com/UniversalDependencies/UD_Korean-GSD/blob/r2.18/LICENSE.txt),
[PUD CC BY-SA 3.0](https://github.com/UniversalDependencies/UD_Korean-PUD/blob/r2.18/LICENSE.txt)입니다.
`verify-gold`는 full POS와 보강 용언 사전을 사용해 명시적 품사와 `smart`로 표적 UTF-8
범위를 확인합니다.
양성 사례는 `any`에서도 같은 위치의 검색 여부를 확인합니다.
`exact` 사례는 두 모드에서 반환한 범위가 표적의 시작·끝 위치와 같아야 합니다.
`smart-exact`는 생략된 지정사처럼 문맥으로 복원하는 형태의 정확한 범위를 `smart`에서 확인합니다.

`walk_hang_stress.txt`는 직접 구성한 회귀 fixture입니다. 동형 동사 활용의 합집합,
생산적 어미, 복합어 안의 용언 명사형, 보조 용언 continuation과 별도 파생 표제어
제외를 함께 검증합니다. `verify-gold`는 full resource에서 `v:걷다` 97개,
`v:걸다` 21개의 논리적 시작 위치를 요구합니다.
