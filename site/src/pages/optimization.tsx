import {
  DocumentPage,
  DocumentSection,
  PageIntro,
} from '../components/document';

export default function OptimizationPage(): React.JSX.Element {
  return (
    <DocumentPage>
      <PageIntro
        eyebrow="ENGINEERING · DESIGN & OPTIMIZATION"
        title="형태 처리 비용을 제한하는 설계"
        summary="kfind의 최적화 목표는 형태 기능을 줄이는 것이 아니라 그 비용이 발생하는 위치를 통제하는 것입니다. 형태 지식은 query compile과 anchor 주변의 bounded structural decision에 머물고, corpus 크기에 비례하는 경로는 byte scan과 streaming output으로 유지합니다."
      />

      <DocumentSection title="비용 모델">
        <p>
          검색 비용은 query plan을 만드는 비용과 corpus를 읽는 비용으로
          나뉩니다. Query 쪽에서는 사전 analysis와 candidate program 수를
          제한하고, corpus 쪽에서는 가능한 한 긴 고정 anchor로 후보 수를
          줄입니다. 여러 program이 같은 조사·어미 continuation을 사용할 때는
          consumption 상태를 공유해 완성 surface의 중복 열거를 피합니다.
        </p>
        <p>
          Resource와 결과 메모리도 실제 필요가 생길 때만 사용합니다. Component
          근거가 필요한 <code>smart</code> program이 있는 plan만 compact
          resource를 초기화하고, 기본 출력은 bounded channel을 통해
          streaming합니다. 이 네 원칙, 즉 긴 anchor, 공유 consumption, 선택적
          resource, bounded streaming은 서로 독립된 기법이 아니라 형태 기능을
          작은 계획과 국소 후보에 묶는 하나의 비용 모델을 구성합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="Anchor 선택과 matcher 구성">
        <p>
          각 program은 판정 비용을 줄일 수 있는 가장 긴 고정 byte열을 anchor로
          선택합니다. 어간 교체 뒤의 고정 부분, 어간 전체, 짧은 어간과 다음 고정
          요소를 차례로 검토하며, 한 음절 anchor는 boundary decision 없이
          허용하지 않습니다. 예를 들어 <code>걷다</code>의 규칙형과 ㄷ
          불규칙형은 서로 다른 고정 prefix를 제공하지만 suffix 상태는 공유할 수
          있습니다.
        </p>
        <pre>
          <code>{`걷다
├─ 걷고 · 걷는 · 걷지 · 걷겠
├─ 걸어 · 걸었
└─ 걸으 · 걸은 · 걸을`}</code>
        </pre>
        <p>
          Plan에 고유 anchor가 하나뿐이면 owned <code>memmem::Finder</code>를
          사용하고, 둘 이상이면 Aho-Corasick의 overlapping search를 사용합니다.
          이 선택은 query compile 시점에 확정되므로 scan loop에서 matcher 종류를
          반복해서 판단하지 않습니다. 겹친 anchor 후보는 모두 형태 검증을 거친
          뒤 leftmost-longest non-overlapping span으로 정리합니다. 따라서 긴
          anchor를 우선하는 최적화가 더 정확한 program 결과를 미리 제거하지
          않습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="계획 폭발을 오류로 처리">
        <p>
          형태 규칙을 무제한으로 합성하면 corpus scan을 시작하기 전에 compile
          latency와 matcher 메모리가 급격히 증가합니다. kfind는 입력과 중간
          표현에 다음 상한을 두어 한 query가 사용할 수 있는 비용을 공개 계약으로
          제한합니다.
        </p>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">대상</th>
                <th scope="col">상한</th>
                <th scope="col">단위</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>Query 길이</td>
                <td>256</td>
                <td>Unicode scalar</td>
              </tr>
              <tr>
                <td>Atom 수</td>
                <td>32</td>
                <td>query당</td>
              </tr>
              <tr>
                <td>Analysis 수</td>
                <td>32</td>
                <td>atom당</td>
              </tr>
              <tr>
                <td>Candidate program 수</td>
                <td>4,096</td>
                <td>plan 전체</td>
              </tr>
              <tr>
                <td>Matcher 예상 메모리</td>
                <td>64 MiB</td>
                <td>plan당</td>
              </tr>
              <tr>
                <td>Continuation 깊이</td>
                <td>4</td>
                <td>상태 전이</td>
              </tr>
            </tbody>
          </table>
        </div>
        <p>
          상한을 넘으면 일부 program을 임의로 잘라 계속 실행하지 않고 query
          compile 오류를 반환합니다. 조용한 축소는 실행에는 성공하면서 특정
          활용형만 누락하는 결과를 만들기 때문입니다. 오류와{' '}
          <code>--explain-query</code>는 제한에 도달한 항목을 드러내므로,
          호출자는 query를 나누거나 확장 수준을 좁히는 방식으로 명시적으로
          대응할 수 있습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="Resource의 지연 초기화">
        <p>
          각 <code>CandidateProgram</code>은 판정을 <code>Boundary</code> 또는{' '}
          <code>Structural</code>로 선언합니다. Structural program은 lexical
          identity, 세부 품사, continuation과 인접 token 제약을 담은{' '}
          <code>QueryMorphPattern</code>을 직접 소유합니다. Compile된 plan에
          이런 program이 하나라도 있을 때만 compact component resource를
          요구합니다. Literal, <code>token</code>, <code>any</code> plan은
          resource를 사용하지 않으며, <code>smart</code>도 구조 제약이 없으면
          파일을 찾거나 읽지 않습니다.
        </p>
        <p>
          Resource가 필요하면 engine은 최초 한 번 bytes를 decode하고 schema,
          source SHA-256, section digest, offset과 component span을 모두
          검증합니다. 검증된 resource는 engine 수명 동안 공유하며 matcher마다
          다시 decode하지 않습니다. 새 bytes를 수동으로 load할 때도 전체 검증이
          끝난 뒤에만 engine 상태를 교체하고, 실패하면 기존 resource를
          유지합니다.
        </p>
        <p>
          CLI는 설치 경로를 알고 있으므로 resource를 자동으로 탐색합니다. Rust
          library와 npm binding은 파일 시스템이나 URL을 추정하지 않고 caller가
          bytes를 전달하게 합니다. 이 경계는 지연 초기화가 실행 환경별 암묵적
          network 또는 file access로 바뀌는 것을 막습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="Scan hot path의 중복 계산 제거">
        <p>
          Scan 경로에서는 match가 없는 일반적인 경우의 비용을 먼저 줄입니다.
          Byte anchor가 없는 구간은 Unicode 처리 없이 건너뛰고, 기본 matcher는
          설명 metadata 대신 span의 byte 범위만 반환합니다. Provenance는
          JSON이나 explain 출력이 필요한 일치 줄에서만 다시 계산합니다. Query가
          NFC이면 정규화된 anchor를 원문에서 직접 찾고, canonical program이나
          suffix 소비가 필요할 때만 추가 Unicode 처리를 수행합니다.
        </p>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">경로</th>
                <th scope="col">기본 전략</th>
                <th scope="col">추가 비용이 생기는 시점</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>Anchor scan</td>
                <td>byte 단위로 scan하고 후보가 없는 구간을 건너뜀</td>
                <td>anchor hit 이후</td>
              </tr>
              <tr>
                <td>Span-only match</td>
                <td>byte 범위만 반환</td>
                <td>설명 metadata를 요청했을 때</td>
              </tr>
              <tr>
                <td>Provenance</td>
                <td>일치 줄에서만 재계산</td>
                <td>JSON 또는 explain 출력</td>
              </tr>
              <tr>
                <td>Normalization</td>
                <td>NFC anchor를 직접 검색</td>
                <td>canonical program 또는 suffix 소비</td>
              </tr>
              <tr>
                <td>Phrase</td>
                <td>atom span을 한 번 수집해 결합</td>
                <td>모든 atom이 후보를 가질 때</td>
              </tr>
            </tbody>
          </table>
        </div>
      </DocumentSection>

      <DocumentSection title="경로별 성능 검증">
        <p>
          최적화의 효과를 판정하려면 변경한 경로와 같은 workload를 측정해야
          합니다. kfind는 startup, lexicon load, query compile, filesystem walk,
          scan, verification과 output을 분리합니다. 예를 들어 component resource
          초기화 개선을 1 GiB literal scan으로 대신 증명하거나, morphology
          fixture의 cases/s를 실제 CLI corpus 처리량처럼 해석하지 않습니다.
        </p>
        <p>
          최신 main과 같은 환경·입력·빌드 설정으로 비교했을 때 다음 변화가
          발생하면 회귀 경고 대상으로 검토합니다. 이 값은 서로 다른 지표를
          하나의 점수로 합치는 기준이 아니라, 각 경로에서 추가 분석이 필요한
          변화의 경계입니다.
        </p>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">지표</th>
                <th scope="col">최신 main 대비 경고 기준</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>Query compile</td>
                <td>20% 이상 악화</td>
              </tr>
              <tr>
                <td>Scan throughput</td>
                <td>10% 이상 악화</td>
              </tr>
              <tr>
                <td>RSS</td>
                <td>20% 이상 증가</td>
              </tr>
              <tr>
                <td>Candidate program 수</td>
                <td>2배 이상 증가</td>
              </tr>
            </tbody>
          </table>
        </div>
      </DocumentSection>
    </DocumentPage>
  );
}
