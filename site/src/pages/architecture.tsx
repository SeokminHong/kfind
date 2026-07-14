import { FlowDiagram } from '../components/diagram';
import { Callout, DocumentSection, PageIntro } from '../components/document';

export default function ArchitecturePage(): React.JSX.Element {
  return (
    <article>
      <PageIntro
        eyebrow="INTERNALS · ARCHITECTURE"
        title="Compile과 scan을 분리한 실행 구조"
        summary="작은 query plan은 형태 지식을 포함하고, 큰 corpus 경로는 byte scan과 bounded verification에 집중합니다."
      />

      <DocumentSection title="두 데이터 흐름이 만나는 지점">
        <div
          className="architecture-lanes"
          role="group"
          aria-label="쿼리와 corpus 처리 흐름"
        >
          <div>
            <span>QUERY LANE</span>
            <ol>
              <li>query parse·normalize</li>
              <li>lexicon analysis</li>
              <li>surface branch compile</li>
              <li>anchor engine + verifier state</li>
            </ol>
          </div>
          <div>
            <span>CORPUS LANE</span>
            <ol>
              <li>ignore-aware parallel walk</li>
              <li>buffered byte search</li>
              <li>anchor hit candidate</li>
              <li>local boundary·suffix verify</li>
            </ol>
          </div>
          <strong>validated span + provenance</strong>
        </div>
        <Callout title="Corpus 전체 분석 없음">
          <p>
            후보 anchor가 없는 buffer에서는 줄별 matcher 호출, Unicode scalar
            순회와 형태 규칙 실행을 하지 않습니다.
          </p>
        </Callout>
      </DocumentSection>

      <DocumentSection title="검색 branch의 구성">
        <p>
          완성된 활용형 문자열을 모두 Aho-Corasick에 넣지 않습니다. branch는
          고정된 <code>anchor</code>, 나머지 suffix를 소비하는
          <code>verifier</code>, 원문 core span을 복원하는 mapping과
          provenance를 결합합니다.
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
          조사와 어미 continuation은 전역 DFA/trie를 공유하고 branch는 시작
          상태만 가리킵니다. 긴 고정 prefix는 scan 비용을 줄이고 공유 verifier는
          matcher 메모리 증가를 제한합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="후보 검증 단계">
        <FlowDiagram
          title="Anchor hit에서 최종 span까지"
          caption="검증을 통과한 token span만 leftmost-longest 순서로 선택하고, 동일 span의 origin은 병합합니다."
          steps={[
            {
              label: '01 · BOUNDARY',
              title: 'UTF-8·왼쪽 경계',
              description:
                'byte 위치가 문자 경계인지, 정책이 요구하는 core 시작인지 확인합니다.',
            },
            {
              label: '02 · MORPH',
              title: 'Branch verifier',
              description:
                '어간·어미 또는 조사 상태가 bounded suffix를 소비합니다.',
            },
            {
              label: '03 · TOKEN',
              title: '오른쪽 경계',
              description: '완성 token 끝과 smart component 조건을 검사합니다.',
            },
            {
              label: '04 · RESULT',
              title: 'Span·origin 병합',
              description:
                'core/token span과 모든 analysis·rule path를 결과에 보존합니다.',
            },
          ]}
        />
      </DocumentSection>

      <DocumentSection title="Phrase 결합">
        <p>
          여러 atom은 각각 검증된 span 목록을 만든 뒤 순서대로 결합합니다.
          surface 후보의 데카르트 곱을 거대한 정규식으로 만들지 않습니다.
        </p>
        <pre>
          <code>{`atom 0 spans ─┐
atom 1 spans ─┼─ two-pointer / bounded DP ─ max-gap ─ phrase span
atom 2 spans ─┘`}</code>
        </pre>
        <ul className="contract-list">
          <li>atom 순서를 유지합니다.</li>
          <li>
            앞 token 끝과 다음 token 시작 사이 Unicode scalar 수를 잽니다.
          </li>
          <li>줄을 넘는 gap은 허용하지 않습니다.</li>
          <li>anchor와 span을 한 번 수집하고 한 번 결합합니다.</li>
        </ul>
      </DocumentSection>

      <DocumentSection title="파일 검색과 bounded output">
        <div
          className="architecture-lanes"
          data-compact="true"
          role="group"
          aria-label="병렬 파일 검색과 출력 흐름"
        >
          <div>
            <span>PARALLEL WORKERS</span>
            <ol>
              <li>ignore::WalkParallel</li>
              <li>worker별 Searcher·scratch</li>
              <li>bounded per-file records</li>
            </ol>
          </div>
          <div>
            <span>SINGLE WRITER</span>
            <ol>
              <li>bounded file-stream channel</li>
              <li>파일별 연속 출력</li>
              <li>BufWriter&lt;StdoutLock&gt;</li>
            </ol>
          </div>
          <strong>corpus 크기와 무관한 기본 결과 버퍼</strong>
        </div>
        <p>
          worker는 channel capacity에서 backpressure를 받습니다. 기본 출력은
          전체 결과를 모으지 않으며, <code>--sort path</code>만 모든 file
          stream을 버퍼링해 정렬합니다. broken pipe는 정상 종료로 처리합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="CLI, Rust와 WebAssembly 경계">
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">표면</th>
                <th scope="col">담당</th>
                <th scope="col">Resource 정책</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>CLI</td>
                <td>파일 순회, 인코딩, 출력, locale</td>
                <td>설치 경로에서 full POS·component 자동 resolve</td>
              </tr>
              <tr>
                <td>Rust library</td>
                <td>메모리 UTF-8 query compile·find</td>
                <td>caller가 bytes를 명시</td>
              </tr>
              <tr>
                <td>npm / WASM</td>
                <td>JavaScript 문자열과 UTF-16 offset</td>
                <td>URL·filesystem을 추정하지 않고 caller가 bytes 전달</td>
              </tr>
            </tbody>
          </table>
        </div>
      </DocumentSection>
    </article>
  );
}
