# 성씨와 지시 관형사가 경합하는 호칭 문맥

`이 씨를 안내 데스크로 모셨다`와 `이 씨를 밭에 심었다`는
`이 씨를`의 표면과 조사 구조가 같다. 목적어가 사람인지 씨앗인지에
따라 `이`가 성씨 또는 지시 관형사로 쓰인다. `군·양·님·형`도
호칭과 동형인 다른 용법을 비교했다.
[씨](https://krdict.korean.go.kr/eng/dicSearch/SearchView?ParaWordNo=58145&nation=eng) ·
[씨앗을 뜻하는 씨](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=65937) ·
[군](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=60199) ·
[양](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=72243) ·
[님](https://krdict.korean.go.kr/vie/dicSearch/SearchView?ParaWordNo=41688) ·
[형](https://krdict.korean.go.kr/kor/dicSearch/SearchView?ParaWordNo=90015)

교정 GSD 9,298행의 961문장과 음성 53,308행의 1,957문장에서
한 음절 뒤에 `씨·군·양·님·형` 또는 `모`와 이 호칭이 오는
구조를 검색했다. 호칭 뒤 조사가 오는 표기도 포함했다. 양성에서
`이 씨` 2곳, `신 씨`, `구 씨`, `이 모 씨` 각 1곳을
확인했다. 음성에서는 `이 모 씨` 1곳을 확인했다. 다섯 표면형을
비교하는 새 문장 10개에서 각각 성씨와 관형사로 질의했다.
[대조 입력](../../tools/morph-compare/honorific-noun-context-cases.jsonl)

기준선 6a38bd26의 full POS에서 첫 `이`의 정확한 범위
`0..3` 반환 여부를 채점했다. `smart`는 `씨`의 두 문장에서
양쪽 품사를 반환했고, 나머지 네 호칭 문장에서 성씨를 반환하지
않았다. 관형사 질의는 10문장 모두 반환했다. 행렬 순서는 TP, FP,
TN, FN이다. 계약 검토는 0건이다.

| 경계 | Raw 행렬 | Raw P/R/F1 (%) | 계약 보정 행렬 | 보정 P/R/F1 (%) |
| --- | --- | --- | --- | --- |
| smart | 6, 6, 4, 4 | 50.00, 60.00, 54.55 | 6, 6, 4, 4 | 50.00, 60.00, 54.55 |
| any | 10, 10, 0, 0 | 50.00, 100.00, 66.67 | 10, 10, 0, 0 | 50.00, 100.00, 66.67 |

```sh
"$runner" boundary full-pos "$boundary" tools/morph-compare/honorific-noun-context-cases.jsonl "$output"
```

검색 코드 변경이 없어 성능 측정을 생략했다.
