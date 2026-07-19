# 구조 component resource view

- 측정일: 2026-07-19
- 기준 revision: `4688e80d347bc1ed63bd8d4d183a1f1b3a433899`
- 후보 revision: `d83bcec4f32b878e9ce90b7dc5bb8329cb697643`
- Criterion 환경: macOS 26.4.1, Apple M1 Max, Rust/Cargo 1.97.0
- morphology 환경: Linux 6.12.76/linuxkit aarch64, 10 logical CPUs,
  Python 3.12.13, Rust 1.97.0

## 결론

Token graph가 resource lookup마다 `Vec<ComponentAnalysis>`와 edge마다
`Vec<ComponentPart>`를 만들던 소유 경계를 없앴다. Graph edge는 검증된 analysis record handle과
resource가 소유한 typed POS slice를 빌리고, source component가 필요한 판정만 flat record iterator를
순회한다. 기존 공개 API가 소유 값을 요구할 때만 호환 경계에서 `Vec`를 만든다.

이 변경은 cache를 추가하지 않는다. 반복 POS, 모든 analysis의 POS가 고유한 입력, source component가
있는 입력을 따로 측정했다. Graph 준비 p50/p95는 반복 POS 32.955%/32.243%, 고유 POS
45.522%/44.938%, source component 포함 18.998%/10.321% 단축됐다. 고유 POS 개선이 더 커서 반복
샘플의 높은 cache hit율을 채택 근거로 삼지 않았다.

최종 후보→기준→최종 후보 순서의 morphology 측정에서 두 후보 처리량은 embedded
37,009.1/37,000.7 cases/s, full-POS 22,463.1/22,427.3 cases/s였고 가운데 기준은
35,198.1/20,916.1 cases/s였다. 최종 후보의 peak RSS는 기준과 같거나 60 KiB 낮았다. 성능 필드를
제외한 모든 품질 결과는 byte 단위로 같다.

## 구조

기존 실행 경로는 resource의 flat binary record를 중첩 소유 값으로 바꾼 뒤 graph에 보존했다.

```text
validated component resource
  -> prefix마다 Vec<ComponentAnalysis>
  -> analysis마다 Vec<ComponentPart>
  -> Edge { positions: &[ComponentPos], components: Vec<ComponentPart> }
```

변경 뒤에는 소비자가 필요한 view만 빌린다.

```text
validated component resource
  -> POS-only consumer: group의 typed POS slice 직접 순회
  -> graph builder: ComponentAnalysisRef { resource, record }
  -> Edge { positions: &[ComponentPos], analysis: handle }
  -> component consumer: flat ComponentPart iterator
  -> compatibility API: 호출자가 소유 값을 요구할 때만 Vec materialization
```

`ComponentAnalysisRef`는 64-bit target에서 16 bytes 이하고 `Edge`는 48 bytes 이하다. 이전 edge의
56 bytes보다 작으면서, 여러 graph 알고리즘이 같은 POS record를 다시 해석하지 않도록 POS slice는
edge에 직접 둔다. Component record range와 string ID는 resource decode 때 이미 검증되며 iterator는
immutable bytes만 읽는다.

Node limit도 allocation 경계에서 적용한다. 한 시작점의 analysis 수가 limit를 넘으면 실제 개수는
끝까지 세되 edge 저장은 limit에서 중단한다. 따라서 밀집 resource가 limit 오류를 반환하기 전에
제한을 넘는 graph를 보유하지 않는다.

## Criterion과 표본 편중

양쪽 revision에 같은 benchmark source와 runner를 사용했다. Source SHA-256은
`15d3674bb9e22b2533ecf2770ac3b6138129f693b5fa0044cf38660460d010df`, runner SHA-256은
`0be1682e62c307d8641d9b05c9a6704be50e370793b2ea781eefa2110ce89e7d`다. Criterion 기본
warm-up 3초, 측정 5초, 100 sample을 사용했다. 표는 sample별 1회 시간을 정렬한 p50 midpoint와
nearest-rank p95다.

| workload | 기준 p50 / p95 | 후보 p50 / p95 | 변화 |
| --- | ---: | ---: | ---: |
| 4,032-edge source component graph 준비 | 0.8314 / 0.8443 ms | 0.6734 / 0.7571 ms | -18.998% / -10.321% |
| 4,032-edge 반복 POS graph 준비 | 0.2486 / 0.2542 ms | 0.1667 / 0.1723 ms | -32.955% / -32.243% |
| 4,032-edge 고유 POS graph 준비 | 0.2139 / 0.2178 ms | 0.1165 / 0.1199 ms | -45.522% / -44.938% |
| dense graph 생성 + nominal 선택 | 30.6895 / 31.2129 us | 22.7021 / 24.1546 us | -26.027% / -22.613% |
| 구조 후보 판정 | 2.6225 / 2.7097 us | 2.4487 / 2.5317 us | -6.625% / -6.567% |
| prepared preferred paths | 0.2083 / 0.2142 ms | 0.2176 / 0.2212 ms | +4.451% / +3.259% |
| particle suffix 12회 거부 | 0.9954 / 1.0079 us | 1.0238 / 1.0719 us | +2.850% / +6.348% |
| particle suffix 20회 거부 | 2.3605 / 2.4651 us | 2.4349 / 2.5264 us | +3.155% / +2.490% |
| 준비된 graph의 nominal 선택 | 0.3793 / 0.3920 us | 0.3964 / 0.4344 us | +4.498% / +10.816% |

준비가 끝난 밀집 graph만 반복하는 세 workload는 edge의 POS slice 때문에 2.850~4.498% 느려졌고,
nominal 선택 p95는 10.816% 불리했다. 제품 경로처럼 graph 생성과 선택을 함께 측정한 workload는
22.613~26.027% 개선됐다. Source component workload는 63-syllable token의 모든 시작 위치에서
두 분석을 만들고, 길이 2 이상 분석마다 두 개의 정렬된 source component를 둔다. 입력 SHA-256은
`99bdcc253f406a83253f16f72dbdff71616f1979c6b18422f2b4dbb2399efef7`다.

Sample JSON SHA-256:

| workload | 기준 | 후보 |
| --- | --- | --- |
| source component graph | `8a72ede9b97b682520aa043b824f1fb078767d655c6a4ee884ed513b2e17e860` | `6468b17b72e2bbacadcc3bc91ae24f7a106a05c7f74b30f6a46ce0beb81b74f9` |
| 반복 POS graph | `13d2deea1ab497c6bde726df9552a40bc6785c80c613edc7403f21b1a20ddc2c` | `7dfa6a4b81433ad2c87e01bce29cea86551030ce493189613ebf69cc00ce5ce5` |
| 고유 POS graph | `c9ac651e56241ee3167a8f5033a662534931f53ddd66e5ab4c1d71b8e49cf55d` | `8dfb0e433e794fd70b6bd83c0c2928a331aba0fb573497f2f2d7f80331717c9a` |
| dense 생성 + 선택 | `3c1deb66aa37a6d62e25b76d12c5a8068edaf212dd9c405666caba8c9bc76149` | `e25ee4dd6667bdc5cf5e72146813c422d2dc9fb287625e73fdcac11a572768fd` |
| 구조 후보 판정 | `17610bdd0e3105109adea32a469791e1eda0ef1262dd34888c18bf3fdf8dbbf5` | `b7d63ada787079c84346d42e97548f9178cd0f92c6c392b34d0f892b3b372cfc` |
| prepared preferred paths | `a4e3d5103c908ccb234e80bbce2d454cbb7170eff428f2562cee9b2bab2194ea` | `cd52ad9cc78651217fe57c2e05de9761ea15344b7ea8a989c6181e40f275bd95` |
| particle suffix 12회 | `150dcd8a17856492b625bafd7c05b9ed713040eede7ada832a2add088fe446db` | `c03d23284405c31b7d3b990582febc220020c41e0c1fcf25e30ea83a24450548` |
| particle suffix 20회 | `50ecac31e871f0f56dc4730c3973f9eb204713cfa8f910dc7d4d02961b35a056` | `05f2d27ded7d3229e348a5e126527b19798b62a8c84f465cfca8504d0e1da0e8` |
| 준비된 nominal 선택 | `cce14b19984c1489270393c93be3ecb8a1510b9bbcb69f82a1d9162a2cef0a5f` | `3466bf8b0d23adcdabf59d7a962f2c51569a199d3623ccd489ec0297fd8630ab` |

## 제품 workload와 품질

공식 morphology runner로 fresh process warm-up 1회 뒤 5회 측정했다. 최종 후보 A, 기준, 최종
후보 B를 연속 실행해 시간에 따른 시스템 변동을 감쌌다.

| profile | 후보 A | 기준 | 후보 B | 후보 B 변화 |
| --- | ---: | ---: | ---: | ---: |
| embedded smart cases/s | 37,009.1 | 35,198.1 | 37,000.7 | +5.121% |
| embedded smart p50 | 0.0214 ms | 0.0245 ms | 0.0215 ms | -12.245% |
| embedded smart p95 | 0.0619 ms | 0.0613 ms | 0.0600 ms | -2.121% |
| full-POS smart cases/s | 22,463.1 | 20,916.1 | 22,427.3 | +7.225% |
| full-POS smart p50 | 0.0251 ms | 0.0292 ms | 0.0245 ms | -16.096% |
| full-POS smart p95 | 0.1241 ms | 0.1334 ms | 0.1257 ms | -5.772% |

후보 B의 component initialization은 embedded 24.8717 ms, full-POS 25.4025 ms로 기준
25.9729/25.4762 ms보다 4.240%/0.289% 짧았다. Component profile peak RSS는 embedded에서
40,292→40,296 KiB로 4 KiB 늘었고 full-POS는 53,472 KiB로 같았다.

100 MiB, 1,000-file 실제 CLI workload에서 Agent 처리량은 4,952.7→5,045.1 MiB/s로 1.865%
개선됐고 Human 처리량은 1,275.8→1,269.9 MiB/s로 0.466% 불리했다. Human peak RSS는
57,988→57,956 KiB로 줄었다. 이 CLI workload는 morphology query보다 scan 비중이 크므로 작은
차이를 구조 graph 성능으로 일반화하지 않는다.

성능 scalar만 재귀적으로 제외한 canonical, development, hard-negative, Human, query matrix,
Robust, shadow와 workflow 결과는 기준과 두 후보에서 같다. Projection SHA-256은
`c907ac3e75e9fe8f5fccade7feb79d235910198be53cc032403690da4a59d7fc`다. Fixture SHA-256은
`1497b958a6970c55bc68ff148e435a88366b650c971231c3ae40adb9d8c46572`, morphology resource는
`50bbaa64b06a080c7fa09c13e21090388a1c0f5109ed413546e0004ce7794f23`, component resource는
`d3a7eb486eef5faa92e006dd72e5ff72b63befb4bb102013f2d19e5fc32ff00a`다.

Morphology report SHA-256:

- 후보 A: `0c2eafb0404758dc7f84eb942907872832d7d78ac97d9568d9156bb5353d817a`
- 기준: `6bacd5a91890961b325034c5a9017ca1829c2da08224c0c42a4488bdc202067b`
- 후보 B: `91ff6059796005145662e7fc79674ff78ead8505530b47344ede47f196130065`

## 안전성

Analysis handle은 검증된 resource만 생성하며 외부에서 record 번호를 만들 수 없다. Loader는 schema,
source identity, section digest, UTF-8, string ID, group/analysis/component range와 component span을
검증한다. Iterator는 이 immutable range 밖을 읽지 않는다. Node limit 회귀 검사는 같은 시작점에
두 분석이 있어도 하나만 저장하고 `actual=2, limit=1` 오류를 반환하는지 확인한다.

`nightly-2026-07-11`, `cargo-fuzz 0.13.2`에서 10개 target을 각각 15초, 입력 timeout 5초,
RSS 상한 2 GiB로 실행했다. 모든 target이 완료됐고 crash, panic, timeout, RSS 초과와 failure
artifact는 0건이다. 변경 경로인 `component_resource`는 113,629개 입력, peak RSS 406 MiB,
`structural_preparation`은 275,587개 입력, peak RSS 585 MiB였다. 두 target 모두
`slowest_unit_time_sec=0`이었다.

## 재현

Morphology runner SHA-256은
`35eb318302ba4e16f36df735eb4a42086b0d124de19e52bbef65c0a204391fd0`, fuzz runner는
`3bba3af9906451c92e421b91cbe0c3c45092bf400e5483d7333a1ae64c1a4968`다.

```console
git switch --detach 4688e80d347bc1ed63bd8d4d183a1f1b3a433899
scripts/benchmark-criterion.sh structural_constraint
scripts/benchmark-morphology.sh

git switch --detach d83bcec4f32b878e9ce90b7dc5bb8329cb697643
scripts/benchmark-criterion.sh structural_constraint
scripts/benchmark-morphology.sh
KFIND_FUZZ_TOOLCHAIN=nightly-2026-07-11 \
KFIND_CARGO_FUZZ_VERSION=0.13.2 \
KFIND_FUZZ_SECONDS=15 \
KFIND_FUZZ_TIMEOUT_SECONDS=5 \
KFIND_FUZZ_RSS_LIMIT_MB=2048 \
scripts/run-fuzz.sh
```
