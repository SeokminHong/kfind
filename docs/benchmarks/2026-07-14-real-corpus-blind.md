# 현실 기술 코퍼스 blind 평가

- revision: `46f9cebd18961d270361afb912806e450941ca30`
- fixture SHA-256: `ceaa7c4890b1cc9ee74dd24f79c39e7ddcb34afbb5618f7d01bd9c83aba6bd65`
- source manifest SHA-256: `2d428f0110d29971b51a80485d110c0427380144fb9bdd06cc76ff25c4462776`
- cases: 25 (positive 21, negative 4)

query와 gold span은 제품 실행 전에 고정했다. 25건의 불균형 진단 fixture이므로 제품 전체 품질 점수나 profile 순위로 해석하지 않는다.
이 평가는 기존 UD 회귀 fixture를 대체하거나 규칙 선택에 사용하지 않는다.

## 출처

| source | revision | license | files |
| --- | --- | --- | ---: |
| [rustlings-kr](https://github.com/eoncheole/rustlings-kr) | `5afb3e6` | [MIT](https://github.com/eoncheole/rustlings-kr/blob/5afb3e613b306d0803d8f2f6ab6d1c3e26c12c35/LICENSE) | 6 |
| [kubernetes-website](https://github.com/kubernetes/website) | `11ed97b` | [CC-BY-4.0](https://github.com/kubernetes/website/blob/11ed97b8bc76e86d4b5ed330ad27df67ec2b2b8b/LICENSE) | 3 |

## 전체 결과

Agent는 `embedded + any + explicit POS`, User는 `full-POS + smart + untagged`다.

| profile | TP | FP | TN | FN | precision | recall | F1 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| agent | 20 | 3 | 1 | 1 | 86.96% | 95.24% | 90.91% |
| user | 15 | 1 | 3 | 6 | 93.75% | 71.43% | 81.08% |

## 평가 slice

| profile / slice | TP | FP | TN | FN | precision | recall | F1 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| agent / compound-substring | 3 | 2 | 0 | 0 | 60.00% | 100.00% | 75.00% |
| agent / homonym | 2 | 1 | 1 | 1 | 66.67% | 66.67% | 66.67% |
| agent / identifier-adjacent | 5 | 0 | 0 | 0 | 100.00% | 100.00% | 100.00% |
| agent / mixed-script-number | 5 | 0 | 0 | 0 | 100.00% | 100.00% | 100.00% |
| agent / spacing-error | 5 | 0 | 0 | 0 | 100.00% | 100.00% | 100.00% |
| user / compound-substring | 3 | 0 | 2 | 0 | 100.00% | 100.00% | 100.00% |
| user / homonym | 2 | 1 | 1 | 1 | 66.67% | 66.67% | 66.67% |
| user / identifier-adjacent | 4 | 0 | 0 | 1 | 100.00% | 80.00% | 88.89% |
| user / mixed-script-number | 5 | 0 | 0 | 0 | 100.00% | 100.00% | 100.00% |
| user / spacing-error | 1 | 0 | 0 | 4 | 100.00% | 20.00% | 33.33% |

## 원문 유형

| profile / artifact | TP | FP | TN | FN | precision | recall | F1 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| agent / readme | 6 | 1 | 1 | 0 | 85.71% | 100.00% | 92.31% |
| agent / source-comment | 7 | 1 | 0 | 1 | 87.50% | 87.50% | 87.50% |
| agent / technical-doc | 7 | 1 | 0 | 0 | 87.50% | 100.00% | 93.33% |
| user / readme | 4 | 0 | 2 | 2 | 100.00% | 66.67% | 80.00% |
| user / source-comment | 5 | 0 | 1 | 3 | 100.00% | 62.50% | 76.92% |
| user / technical-doc | 6 | 1 | 0 | 1 | 85.71% | 85.71% | 85.71% |

## 실패 case

| profile | case | 분류 | slice | query | POS |
| --- | --- | --- | --- | --- | --- |
| agent | `homonym-03` | FN | homonym | `말다` | verb |
| agent | `homonym-05` | FP | homonym | `그` | pronoun |
| agent | `compound-substring-01` | FP | compound-substring | `국어` | noun |
| agent | `compound-substring-03` | FP | compound-substring | `열` | noun |
| user | `identifier-adjacent-03` | FN | identifier-adjacent | `제거하다` | verb |
| user | `spacing-error-01` | FN | spacing-error | `설치하다` | verb |
| user | `spacing-error-02` | FN | spacing-error | `재시작하다` | verb |
| user | `spacing-error-03` | FN | spacing-error | `추가하다` | verb |
| user | `spacing-error-05` | FN | spacing-error | `확인하다` | verb |
| user | `homonym-03` | FN | homonym | `말다` | verb |
| user | `homonym-05` | FP | homonym | `그` | pronoun |
