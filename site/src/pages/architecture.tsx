import { DocumentSection, PageIntro } from '../components/document';

export default function ArchitecturePage(): React.JSX.Element {
  return (
    <article>
      <PageIntro
        eyebrow="INTERNALS · ARCHITECTURE"
        title="Query compile과 corpus scan의 분리"
        summary="kfind는 형태 지식을 작은 query plan에 담고, 큰 corpus를 처리하는 경로는 byte scan과 상한이 정해진 검증에 집중시킵니다. 두 경로는 anchor hit에서 만나며, 검증을 통과한 span과 생성 근거만 결과로 반환합니다."
      />

      <DocumentSection title="두 데이터 흐름의 결합">
        <p>
          Query 경로와 corpus 경로는 서로 다른 크기와 책임을 가집니다. Query
          경로는 입력을 parse하고 정규화한 뒤 lexicon analysis와 surface
          branch를 구성합니다. 각 branch에는 anchor engine이 검색할 고정
          byte열과 후보를 판정할 verifier 상태가 들어 있습니다. 이 작업은 query
          길이와 생성된 branch 수에 비례하며, 같은 plan으로 여러 파일을 검색하는
          동안 반복하지 않습니다.
        </p>
        <p>
          Corpus 경로는 ignore 규칙에 따라 파일을 병렬 순회하고 buffer에서
          anchor를 찾습니다. Anchor가 없는 buffer에서는 Unicode scalar를
          순회하거나 형태 규칙을 실행하지 않습니다. Anchor hit가 생긴 위치에서만
          UTF-8 경계, 조사·어미, token boundary와 선택적 component 조건을
          확인합니다. Query 경로가 만든 조건과 corpus의 실제 byte 위치가 이 검증
          단계에서 처음 결합됩니다.
        </p>
        <pre>
          <code>{`query lane
  parse → normalize → analyze → compile branch
                                     │
                                     ▼
corpus lane
  walk files → byte scan → anchor hit → local verify
                                     │
                                     ▼
                         validated span + provenance`}</code>
        </pre>
        <p>
          이 분리는 corpus 전체를 분석하지 않는다는 제품 범위를 구조적으로
          보장합니다. 형태 분석은 query plan을 만들거나 실제 후보를 판정할 때만
          실행되고, 후보가 없는 원문은 문자열 검색 경로에 머뭅니다.
        </p>
      </DocumentSection>

      <DocumentSection title="검색 branch의 표현">
        <p>
          모든 활용형을 완성 문자열로 열거해 Aho-Corasick에 넣으면 어미 연쇄가
          늘어날 때 branch와 matcher 메모리가 함께 증가합니다. kfind의{' '}
          <code>SurfaceBranch</code>는 고정된 <code>anchor</code>, 나머지
          suffix를 소비하는 <code>verifier</code>, 원문의 core span을 복원하는
          mapping과 생성 근거인 origins를 결합합니다.
        </p>
        <pre>
          <code>{`SurfaceBranch
├─ anchor: "걸었"
├─ verifier: past-continuation state
├─ core_mapping: 걷다 core span mapping
└─ origins: [DToL, ending.past]

shared suffix graph
└─ 습니다 | 지만 | 는데 | ...`}</code>
        </pre>
        <p>
          조사와 어미 continuation은 query마다 복제하지 않고 전역 DFA 또는
          trie를 공유합니다. Branch는 이 graph의 시작 상태만 가리킵니다. 긴 고정
          prefix는 원문에서 발생하는 후보 수를 줄이고, 공유 verifier는 같은
          suffix 규칙을 여러 표제어에 다시 사용할 수 있게 합니다. 같은 surface가
          여러 analysis에서 생성되면 실행 span은 중복하지 않되 origins는 합쳐서
          보존합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="Anchor hit의 검증 순서">
        <p>
          Anchor를 찾았다는 사실만으로 match가 성립하지는 않습니다. 먼저 hit의
          byte 위치가 UTF-8 문자 경계인지 확인하고, 선택한 boundary 정책이
          core의 왼쪽 경계를 요구하면 그 조건을 검사합니다. 다음으로 branch
          verifier가 anchor 뒤의 bounded suffix를 소비하면서 어간·어미 또는 조사
          상태 전이를 검증합니다. Verifier가 완성한 token의 오른쪽 경계와 필요한
          smart component 조건까지 통과해야 하나의 후보 span이 만들어집니다.
        </p>
        <p>
          검증된 후보가 겹치면 core와 token span을 기준으로 leftmost-longest
          non-overlapping 순서를 적용합니다. 같은 span을 설명하는 analysis와
          rule path는 하나를 선택하지 않고 병합합니다. 일반 검색 출력에 span만
          필요한 경우에는 byte 범위만 반환하고, JSON이나 explain 출력이 요청된
          줄에서만 provenance를 다시 계산합니다. 이 분리는 설명 정보의 비용이
          기본 scan 경로에 항상 포함되는 것을 막습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="Phrase span의 결합">
        <p>
          Phrase query는 각 atom의 surface 후보를 미리 곱해 하나의 거대한
          정규식으로 만들지 않습니다. 각 atom을 독립적으로 검증해 span 목록을
          만든 뒤, 그 목록들을 query 순서대로 결합합니다. 앞 atom의 token 끝과
          다음 atom의 token 시작 사이 Unicode scalar 수가 <code>max-gap</code>{' '}
          이하여야 하며, 순서가 뒤집히거나 음수인 span은 결합하지 않습니다. 기본
          검색은 줄을 가로지르는 phrase도 만들지 않습니다.
        </p>
        <pre>
          <code>{`atom 0 spans ─┐
atom 1 spans ─┼─ ordered span join ─ max-gap ─ phrase span
atom 2 spans ─┘`}</code>
        </pre>
        <p>
          이 방식은 anchor와 span을 atom마다 한 번만 수집하게 합니다. 특정
          atom에 후보가 없으면 나머지 조합을 만들 필요가 없고, 후보가 많더라도
          순서와 거리 조건을 이용해 제한된 결합만 수행할 수 있습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="병렬 파일 검색과 출력">
        <p>
          파일 시스템에서는 ignore 규칙을 이해하는 parallel walker가 파일을
          분배하고, 각 worker가 자신의 searcher와 scratch buffer를 사용합니다.
          Worker는 파일 하나의 결과를 bounded record stream으로 만들고, 제한된
          capacity의 channel을 통해 단일 writer에 전달합니다. Writer가 느리면
          channel의 backpressure가 worker를 멈추므로 결과 수가 커져도 무제한
          메모리를 사용하지 않습니다.
        </p>
        <p>
          단일 writer는 한 파일의 출력을 연속해서 <code>BufWriter</code>에 쓰며,
          소비자가 pipe를 닫아 발생한 broken pipe는 정상 종료로 처리합니다. 기본
          출력은 corpus 전체의 결과를 모으지 않습니다. 경로 정렬을 명시한{' '}
          <code>--sort path</code>만 모든 file stream을 버퍼링하므로, 결정적인
          순서가 필요한 경우와 bounded memory가 중요한 경우의 비용을 호출자가
          선택할 수 있습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="CLI, Rust와 WebAssembly의 경계">
        <p>
          Query compile과 memory text match는 세 표면에서 같은 의미를
          유지하지만, 파일 시스템과 resource 위치를 누가 책임지는지는 다릅니다.
          CLI는 설치 환경을 알고 있으므로 파일 순회, 입력 인코딩, locale과
          출력뿐 아니라 full POS 및 component resource의 자동 탐색도 담당합니다.
          Rust library와 npm binding은 호출자의 환경을 추측하지 않으며, 필요한
          resource bytes를 caller가 명시적으로 전달해야 합니다.
        </p>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">표면</th>
                <th scope="col">담당 범위</th>
                <th scope="col">Resource 정책</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>CLI</td>
                <td>파일 순회, 인코딩, 출력, locale</td>
                <td>
                  설치 경로에서 full POS, enriched 용언과 component resource를
                  자동 탐색
                </td>
              </tr>
              <tr>
                <td>Rust library</td>
                <td>메모리의 UTF-8 query compile과 find</td>
                <td>caller가 resource bytes를 전달</td>
              </tr>
              <tr>
                <td>npm / WASM</td>
                <td>JavaScript 문자열과 UTF-16 offset</td>
                <td>
                  URL이나 filesystem을 추정하지 않고 caller가 bytes를 전달
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </DocumentSection>
    </article>
  );
}
