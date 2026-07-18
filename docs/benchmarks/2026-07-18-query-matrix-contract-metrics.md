# Query matrix raw·계약 품질 교정

> 현재 제품 결과는 [사전 합의 `-이` 부사형 recall](2026-07-18-dictionary-adverbial-i.md)이
> 이어받는다. 이 문서는 contract 교정 시점의 측정값을 보존한다.

- 측정일: 2026-07-18
- 최신 `origin/main` 및 기준 revision:
  `ad7a4dd3478e7aec26ac185900b72ae9fd8cf084`
- 후보 revision: `d16c9f03865e2c1e6c51072d23531de05b470c70`
- 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs, Python 3.12.13,
  Rust 1.97.0, Docker 29.6.1
- 반복: fresh process warm-up 1회 뒤 5회 측정의 중앙값
- canonical fixture:
  `1497b958a6970c55bc68ff148e435a88366b650c971231c3ae40adb9d8c46572`
- explicit-POS matrix:
  `e862d8af010c23462ba3a9ebf4f1134275b68de5004bc60035565734f5f19999`
- contract review registry:
  `3aa7f3be5dc4a9f0c44a18c0bde4a570b790c9372271cd15eb05e149d3a3e50e`
- 기준 report SHA-256:
  `7b42f53bc37388857058046ee031832483f07dc588d191b2d16e98d02358b628`
- 후보 report SHA-256:
  `5034efe09ca15affe60a1119cd89df89bbb49f496f4086594ab7340a54f6b2ad`

## 결론

기존 query matrix에는 contract annotation이 없었다. 따라서 raw confusion matrix를
contract-adjusted 값으로 다시 출력했고, raw FP 4건과 FN 18건을 FPᶜ·FNᶜ로 잘못 불렀다.
이번 교정은 제품 결과와 독립된 version-controlled review registry를 fixture 생성 단계에
적용하고, raw와 contract 값을 별도 confusion matrix로 계산한다.

full-POS의 의도된 값은 raw `FP 4 / FN 18 / recall 98.61%`, contract
`FPᶜ 0 / FNᶜ 14 / recallᶜ 98.92%`다. FNᶜ 14건은 미구현 표준 문법이므로 향후 제품 목표다.
현재 비표준 띄어쓰기 3건만 계약 모수에서 제외한다. 비용, 현재 profile, 사전 합의 부족이나
반환 span 설계 난이도는 제외 근거가 아니다.

## raw와 contract 품질

Explicit-POS query matrix는 432문장, 2,592 case다. Raw는 source corpus gold를 보존한다.
Contract는 같은 예측에 검토 레지스트리만 적용한다.

| profile | raw precision / recall | precisionᶜ / recallᶜ | raw TP / FP / TN / FN | TPᶜ / FPᶜ / TNᶜ / FNᶜ | raw / contract 문장 완전 회수 |
| --- | ---: | ---: | ---: | ---: | ---: |
| embedded smart | 99.67% / 91.98% | 100.00% / 92.28% | 1,192 / 4 / 1,292 / 104 | 1,196 / 0 / 1,293 / 100 | 77.55% / 78.24% |
| full-POS smart | 99.69% / 98.61% | 100.00% / 98.92% | 1,278 / 4 / 1,292 / 18 | 1,282 / 0 / 1,293 / 14 | 95.83% / 96.76% |

Contract 분모는 2,589 case다. Review 22건은 구현 목표 확인 14건, 기대값 변경 5건,
비표준 입력 제외 3건이다. `FPᶜ`, `FNᶜ`, `precisionᶜ`, `recallᶜ`는 이 annotation이 실제로
적용된 값이며 raw 지표의 별칭이 아니다.

![kfind query matrix raw와 contract 품질](assets/2026-07-18-query-matrix-contract-metrics-query-matrix-quality.svg)

## 22건의 계약 판정

Raw FP 4건은 모두 제품이 맞게 찾은 계약 양성이다.

| 분류 | 건수 | case | 계약 처리 |
| --- | ---: | --- | --- |
| 문법 구조로 구분할 수 없는 동형이의 | 3 | `불과/noun`, `제/pronoun`, `만/numeral` | strict negative → contract positive |
| source 정렬 내부 성분 | 1 | `그/pronoun → 그것이야말로` | strict negative → contract positive |
| gold 정렬 오류 | 1 | `이/pronoun → 이중` | strict positive → contract negative |
| 현재 비표준 띄어쓰기 | 3 | `국경없는`, `권위있는`, `빙원옆에` | contract 분모 제외 |

나머지 raw FN 14건은 strict positive를 유지한다.

| 향후 구현 구조 | 건수 | case |
| --- | ---: | --- |
| 표준 용언→부사 파생 | 5 | `없다→없이`, `똑같다→똑같이`, `같다→같이` 2건, `다르다→달리` |
| 피동 파생 | 1 | `밀다→밀려` |
| source 정렬 보조·합성용언 성분 | 4 | `잠식당→잠식당하기`, `가다→올라가`, `나다→생겨나`, `오다→들어와서는` |
| 대명사 축약 | 2 | `무어→무언가`, `누구→누군가가` |
| 축약 span 복원 | 1 | `이다→걸까` |
| 한 음절 source 체언 성분 | 1 | `하→책임하에서` |

`query-matrix-fnc-dispositions.tsv`는 이 18개 raw FN을 fixture identity와 다시 대조한다.
처분은 `product-fix` 12, `structural-redesign` 2, `gold-alignment-error` 1,
`nonstandard-input` 3이며 미분류 raw FN은 0건이다.

## 국립국어원 사전 증거의 역할

기존 exact lexeme audit의 고정 snapshot은 표제어·품사·명시적 관계를 판정하는 데 사용했다.
`이중`은 완성 명사이므로 내부 `이`를 대명사로 보는 gold를 교정한다. 반면 표준 파생·축약·source
성분은 사전에 직접 관계가 없거나 별도 표제어라는 이유만으로 제품 목표에서 제외하지 않는다.
사전은 문법 구조와 반환 span을 대신하지 않으며, 미구현 구조는 FNᶜ로 남긴다.

| source | snapshot SHA-256 |
| --- | --- |
| 한국어기초사전 2026-06-19 | `a8ab7d044d4f6341e0f217db63f38f4d18beed3e1f153130f6cb4e9494fea1d6` |
| 표준국어대사전 2026-07-05 | `880b31447146df5879c076012b21d4cc3c0c24e70fd91be7fc73f7ff7da34d52` |
| 우리말샘 2026-07-02 | `9e8807e5fade8c7b59431d1ab527fe93aafd15395001bcdde88511e8c9293b42` |
| importer | `48f384221a10b38bcfed4df38e262df9f35d964b` |

## 성능

제품 Rust source는 바뀌지 않았다. 기준선은 최신 `origin/main` 제품 source에 후보 평가 harness와
동일 fixture·snapshot만 얹어 측정했다. 각 값은 중앙값이며 cases/s와 p95의 대괄호는
`[min, max]`다.

| workload | revision | initialization (s) | cases/s | p95 (ms) | RSS (KiB) |
| --- | --- | ---: | ---: | ---: | ---: |
| canonical embedded | 기준 | 0.041723 | 34,301.3 [31,573.7, 35,136.7] | 0.0591 [0.0582, 0.0634] | 42,124 |
| canonical embedded | 후보 | 0.041748 | 33,769.7 [33,435.7, 34,683.8] | 0.0591 [0.0590, 0.0602] | 42,112 |
| canonical full-POS | 기준 | 0.080484 | 22,482.9 [20,871.1, 22,901.9] | 0.1071 [0.1058, 0.1164] | 58,964 |
| canonical full-POS | 후보 | 0.080972 | 22,640.4 [21,103.4, 22,863.2] | 0.1073 [0.1065, 0.1139] | 57,836 |
| matrix embedded | 기준 | 0.041766 | 34,006.3 [32,880.0, 34,160.1] | 0.0604 [0.0595, 0.0613] | 44,892 |
| matrix embedded | 후보 | 0.041958 | 34,055.3 [29,320.5, 34,715.5] | 0.0590 [0.0583, 0.0698] | 44,896 |
| matrix full-POS | 기준 | 0.080009 | 22,789.9 [22,600.3, 22,968.4] | 0.1047 [0.1030, 0.1062] | 58,472 |
| matrix full-POS | 후보 | 0.080517 | 22,499.4 [20,409.0, 23,013.1] | 0.1056 [0.1030, 0.1168] | 58,472 |

Cases/s 중앙값 변화는 canonical embedded -1.55%, canonical full-POS +0.70%, matrix embedded
+0.14%, matrix full-POS -1.27%다. Initialization·p95·RSS를 포함해 10% 회귀 경고선에 걸리는
지표는 없다.

## 재현

```console
scripts/refresh-morph-baselines.sh

KFIND_MORPH_RUNS=5 \
scripts/benchmark-morphology.sh target/query-matrix-contract-candidate-final

python3 tools/morph-compare/validate_fnc_dispositions.py \
  target/query-matrix-contract-candidate-final/report.json \
  docs/benchmarks/query-matrix-fnc-dispositions.tsv

python3 tools/morph-compare/render_charts.py \
  target/query-matrix-contract-candidate-final/report.json \
  target/query-matrix-contract-charts \
  --prefix 2026-07-18-query-matrix-contract-metrics-
```

기준선은 `origin/main` archive에 후보의 `tools/morph-compare/python`, Dockerfile, contract
registry와 외부 snapshot만 복사한 뒤 같은 5회 명령으로 측정했다.
