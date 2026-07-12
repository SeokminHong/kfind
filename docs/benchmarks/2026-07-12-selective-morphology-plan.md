# 선택적 국소 형태 추론 작업 계획

상태: P0·P1·P2 지정사 판별 slice·resource·bounded offset mapping 완료, lattice path 대기
적용 시점: v0.1.1 이후 실험 범위

관련 문서:

- [형태소 검색 개선 핸드오프](2026-07-12-morphology-handoff.md)
- [VCP 지정사 smart-boundary 계획](2026-07-12-copula-boundary-plan.md)
- [사전 확장 전략](../lexicon-scaling.md)
- [prefix index 비교 결과](2026-07-12-morph-index-comparison.md)

## 결정

현재의 anchor 검색과 국소 verifier를 빠른 기본 경로로 유지한다. 더 정밀한 판정은 모든
문장에 적용하지 않고, query branch가 추가 정보가 필요하다고 표시한 anchor hit에만
어절-local 형태 lattice를 실행한다.

문자열 경계와 형태 분석 선택은 별도 축으로 다룬다.

```text
boundary: 문자열상 왼쪽·오른쪽 경계
disambiguation: 가능한 형태 분석 중 검색 결과로 인정할 분석
```

v0.1.1의 homonym union과 CLI 동작은 이 계획만으로 바꾸지 않는다. 결과 필터링을 시작하기
전에 `specs/kfind.md`를 먼저 갱신하고 새 옵션·fallback·호환성 계약을 확정한다.

## 근거

- 현재 embedded profile은 17,805.0 cases/s, p95 0.1270 ms, peak RSS 4.9 MiB다.
- full-POS는 dev의 `lexicon-missing`을 0으로 줄이지만 `boundary-rejected` 97건이 남는다.
- `매일`의 어휘 내부 `일`과 `학생일`, `책일`의 VCP 관형형 후보 `일`은 anchor와 인접
  Unicode 문자만으로 구분할 수 없다. `학생일`, `책일` 전체를 사전 표제어로 가정하지 않고
  체언 host와 VCP 활용의 결합으로 다룬다.
- 전체 문장 분석은 현재 low-hit scan과 작은 RSS의 장점을 약화한다.

따라서 사전 크기 확대나 boundary 예외 추가가 아니라 후보 hit에 한정한 분석이 필요하다.

## 용어

| 용어 | 의미 |
| --- | --- |
| anchor | 빠른 byte scan에서 후보 위치를 찾는 고정 표면 문자열 |
| verifier | anchor 주변의 조사·어미·경계를 확인하는 규칙 실행기 |
| 분석 어절 | 이 계획에서 공백·문장부호로 제한하는 국소 분석 범위 |
| lattice | 한 어절의 가능한 형태소 분해를 node와 edge로 표현한 DAG |
| Viterbi | lattice에서 누적 비용이 가장 낮은 분석 경로를 찾는 동적 계획법 |
| N-best | 최적 경로 하나가 아니라 비용이 낮은 여러 경로를 보존하는 방식 |
| cost margin | 최적 경로와 후보 경로의 비용 차이 |
| shadow mode | 판정은 기록하지만 실제 검색 결과는 바꾸지 않는 검증 모드 |

## 목표 구조

```text
byte scan
  -> anchor hit
  -> 기존 morphology/boundary verifier
  -> branch별 context requirement 확인
  -> 필요한 hit만 어절 추출
  -> 제약된 local lattice와 비용 비교
  -> union 정책 또는 명시적 disambiguation 정책 적용
```

query plan에는 필요한 검증 수준을 명시한다.

```rust
enum ContextRequirement {
    None,
    HostMorphology,
    EojeolLattice,
    SentenceContext,
}
```

첫 구현은 `None`과 `EojeolLattice`만 사용한다. `SentenceContext`는 local lattice로 해결되지
않는 동형이의어가 충분히 측정된 뒤 별도 범위로 판단한다.

## 불변 조건

- anchor가 없는 파일과 줄에서는 lattice를 실행하지 않는다.
- literal과 기존 boundary-only branch는 현재 경로를 유지한다.
- 분석 범위와 node 수에 상한을 두고 상한 초과를 진단 가능하게 처리한다.
- 원문 byte offset은 NFC 정규화 뒤에도 보존한다.
- corpus 단어 denylist나 특정 fixture 전용 branch를 추가하지 않는다.
- full resource 누락 시 동작은 정책별로 명시하고 조용히 다른 판정으로 대체하지 않는다.
- worker는 immutable resource를 공유하고 lattice scratch는 worker별로 재사용한다.

## 작업 단위

### P0. 계약과 측정 기반

1. `boundary`와 `disambiguation`의 책임, union 기본값과 shadow 측정 범위를 스펙에 추가한다.
2. branch에 필요한 context 수준을 표현하되 검색 결과는 바꾸지 않는다.
3. raw anchor hit, verifier 통과, lattice 대상, 고유 어절 수를 측정하는 counter를 추가한다.
4. `학생일`, `책일`은 형태 조합 회귀 fixture로 유지한다. 실제 dev corpus에서는 VCP/VCN
   지정사 분석, 어휘 내부 표면형, 한 음절 경계, NFC/NFD case를 고정한다. 조합 fixture만으로
   품질 threshold를 정하지 않는다.
5. 현재 `union` 결과와 향후 `local` 기대 결과를 fixture metadata에서 구분한다.

완료 조건:

- low-hit literal 실행의 lattice 대상이 0이다.
- `매일`, `학생일`, `책일`의 branch 근거와 dev corpus의 모든 `EojeolLattice` 대상 branch가
  report에 남는다.
- 기본 CLI와 match 결과가 변하지 않는다.
- 다음 단계의 성능 측정에 필요한 invocation count가 재현된다.

### P1. prefix 사전 자료구조 선택

1. full-POS와 같은 고정 source snapshot에서 원본 표면형·품사·좌/우 연결 ID·비용을 보존한
   별도 morphology-index benchmark fixture를 만든다. 정규화된 query 표제어 artifact와
   corpus-side 분석 artifact는 분리한다.
2. packed Double-Array trie와 FST 후보를 exact lookup, prefix 열거, 초기화, RSS로 비교한다.
3. resident core와 mmap full resource의 cold/warm 동작을 각각 측정한다.
4. 두 후보가 공유하는 container에 schema version, source digest, 통계, section digest와
   검증기를 추가한 뒤 측정 결과로 index 형식을 선택한다.

완료 조건:

- 60만 표제어 규모에서 prefix 열거 비용과 peak RSS가 보고된다.
- 손상, schema 불일치, source digest 불일치가 명시적 오류가 된다.
- 자료구조 변경만으로 query 분석 결과가 달라지지 않는다.

결과:

- 729,173개 표면형과 757,627개 분석을 같은 payload로 측정했다.
- packed Double-Array trie는 FST보다 index가 7,452,614 bytes 크지만 exact lookup은 약 6배,
  common-prefix 열거는 약 4.3배 빨랐고 peak RSS는 28.1 MiB로 게이트 안이었다.
- P2의 full morphology index는 읽기 전용 mmap Double-Array를 사용한다. FST 구현은 비교
  기준으로만 유지한다.

### P2. 어절-local lattice shadow mode

구현에 앞서 기존 품사 quota와 분리한 지정사 판별 slice를 고정한다.

1. UD 2.18 Korean-Kaist·Korean-KSL dev 원문의 canonical `JP=이`, `VCP=이`, `VCN=아니`
   gold occurrence를 모두 양성으로 보존한다.
2. 완전히 정렬된 문장 중 지정사 활용에서 고정한 surface cue를 포함하지만 같은
   표제어·품사 gold가 없는 문장을 어휘 내부 음성으로 전수 선택한다.
3. source·raw tag·positive/negative별 confusion matrix와 shadow 대상 수를 별도 report로
   기록한다. 기존 1,000건 test와 품사 quota는 변경하지 않는다.
4. 비정규 지정사 표면형은 양성으로 승격하지 않고 제외 이유와 수를 metadata에 남긴다.

현재 고정 dev 원문에서 예상되는 규모는 양성 1,601건, 음성 1,315건이다. 이 fixture 결과를
확인하기 전에는 백업 prototype의 비용 모델이나 threshold를 제품 판정에 적용하지 않는다.

지정사 판별 slice 완료 조건:

- source hash, seed, fixture digest와 source·raw tag·class별 case 수가 고정된다.
- fixture 선택이 kfind·Kiwi·Lindera 출력이나 query anchor에 의존하지 않는다.
- union 검색 결과와 shadow 대상 수가 같은 report에서 비교된다.
- 실제 검색 결과와 CLI는 변하지 않는다.

결과:

- fixture SHA-256은 `1e06951581c84f02a4013e8410c113337c1389d3dcc2028b322f887bb181b494`다.
- 양성 1,601건과 음성 1,315건을 고정했다. 비정규 `VCP=있` 1건은 제외 사유로 기록한다.
- kfind embedded/full-POS는 TP 961, FP 76, TN 1,239, FN 640으로 동일했다. precision은
  92.67%, recall은 60.02%다.
- KSL VCP는 precision 82.76%, recall 45.04%로 가장 약한 그룹이다. 비용이나 threshold를
  조정하기 전에 lattice path가 이 그룹의 양성과 음성을 구분하는지 확인한다.
- `EojeolLattice` 대상은 1,160개 case의 1,647개 hit이며 두 kfind profile에서 동일하다.
- `kfind-data`의 schema 2 resource는 729,173개 표면형, 757,627개 분석과
  3,822×2,693 연결 비용 행렬, `char.def`, `unk.def`를 보존한다.
- 고정 source에서 생성한 `morphology.bin` SHA-256은
  `c9aae9746c29a2848d4e5bff3b15d81601f795ba4d65cd893a7eefe9a2490ca6`다.
- loader는 schema, source digest, section digest, payload offset·record, context ID·행렬 크기를
  검증한다. 이 resource는 아직 CLI와 matcher에 연결하지 않았다.
- `AnalysisWindow`는 target을 포함하는 Unicode token 범위를 원문 256 bytes, NFC 64 scalar
  안에서 추출한다. 잘못된 범위·UTF-8·상한 초과를 구분하고 상한을 넘는 token은 전체를
  순회하지 않고 거부한다.
- NFC의 안정된 경계는 원문 절대 byte span과 양방향으로 변환한다. 한글 분해형과 임의
  Unicode 문자열의 안정 경계 round-trip을 검증했으며 검색 결과는 바꾸지 않는다.

그 뒤 lattice shadow를 다음 순서로 구현한다.

1. anchor를 포함한 bounded 어절과 원문 offset mapping을 추출한다.
2. 사전 node, 조사·어미 node, 미등록어 node와 연결 비용을 구성한다.
3. query 분석을 포함하는 최저 비용과 포함하지 않는 최저 비용을 함께 계산한다.
4. N-best와 cost margin을 report에 기록하되 검색 결과는 필터링하지 않는다.
5. worker별 scratch와 소형 어절 cache로 반복 할당과 lock을 제거한다.

완료 조건:

- dev corpus의 `EojeolLattice` 후보를 source·gold 형태 분석·선택 경로별로 집계한다.
- 정상 VCP/VCN·조사·활용 case의 가능한 경로가 보존되며, 특정 조합 어절에 맞춘 비용이나
  사전 항목을 추가하지 않는다.
- `학생일`, `책일`의 VCP 관형형 경로를 형태 조합 회귀로 보존한다.
- accept/reject threshold와 품질 기준은 이름을 정해 만든 소수 예시가 아니라 dev split에서
  정하고, 별도 blind source에서 확인한다.
- 상한 초과와 resource 누락이 report에 드러난다.
- 아래 성능 게이트를 통과한다.

### P3. 명시적 local disambiguation

1. shadow 결과와 blind 평가를 근거로 스펙과 CLI를 먼저 갱신한다.
2. `union`을 호환 기본값으로 유지하고 `local`을 명시적 정책으로 노출한다.
3. cost margin 안의 복수 분석을 보존하고 JSON/explain에 선택 근거를 출력한다.
4. local 정책의 resource 누락·손상·상한 초과 fallback을 고정한다.

기본 정책 변경은 별도 결정으로 남긴다.

### P4. 선택적 문맥 점수

어절-local 분석으로 구분할 수 없는 같은 품사의 동형 표면형만 수집한다. 규모와 사용자
가치를 확인한 뒤 POS n-gram 또는 경량 skip-bigram을 모호한 case에만 적용할지 판단한다.
큰 신경망과 전체 문장 상시 분석은 이 계획의 범위가 아니다.

## 검증 게이트

다음 값은 prototype 판정을 위한 후보이며 제품 스펙으로 자동 승격하지 않는다.

| 항목 | 게이트 |
| --- | --- |
| low-hit literal lattice 호출 | 0 |
| 1 GiB low-hit wall time | 현재 기준 대비 회귀 5% 이내 |
| morphology 처리량 | 15,100 cases/s 이상 |
| morphology p95 | 0.159 ms 이하 |
| peak RSS | 40 MiB 이하 |
| VCP/VCN local 품질 | `학생일`·`책일` 조합 회귀와 dev confusion matrix 보고, threshold는 blind source 확인 전 미적용 |
| union 호환성 | 기존 fixture 결과 불변 |

품질 threshold와 비용 모델은 dev split에서 정한다. 기존 test split은 regression baseline으로만
사용하고, 기본 정책 변경 전에는 별도 blind source와 확장된 hard-negative를 확인한다.

## 다음 구현 범위

P2는 다음 무결한 작업 단위로 나눈다.

1. 지정사 판별 fixture 생성과 metadata 검증을 추가한다. (완료)
2. fixture를 benchmark report에 연결하고 source·raw tag·class별 baseline을 기록한다. (완료)
3. 백업 branch의 morphology resource 생성·검증을 최신 `main` 위에 재구성한다. (완료)
4. bounded 어절 추출과 NFC 원문 offset mapping을 별도 작업 단위로 재구성한다. (완료)
5. lattice path와 N-best shadow report를 연결한다.
6. 성능·품질 게이트를 통과한 뒤 P3 진행 여부를 결정한다.

각 단위는 독립적으로 포맷·lint·workspace test를 통과한 뒤 커밋한다. 백업 branch
`codex/morph-lattice-shadow-backup-20260712-203332`는 prototype 참고 자료로만 사용하고,
PR #26 이전 fixture 의미를 최신 `main`에 그대로 되살리지 않는다.
