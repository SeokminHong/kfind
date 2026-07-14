import { FlowDiagram } from '../components/diagram';
import { Callout, DocumentSection, PageIntro } from '../components/document';

export default function OptimizationPage(): React.JSX.Element {
  return (
    <article>
      <PageIntro
        eyebrow="ENGINEERING · DESIGN & OPTIMIZATION"
        title="형태 분석 비용을 query plan 안에서 제한하기"
        summary="kfind는 형태 기능을 줄이는 대신 실행 범위를 제한합니다. 형태 지식은 query를 compile할 때 처리하고, corpus에서는 anchor 주변만 정해진 상한 안에서 검증합니다."
      />

      <DocumentSection title="최적화 원칙">
        <div className="principle-grid">
          <article>
            <span>01</span>
            <strong>긴 고정 anchor</strong>
            <p>후보 수를 줄이는 가장 직접적인 수단입니다.</p>
          </article>
          <article>
            <span>02</span>
            <strong>공유 verifier</strong>
            <p>
              완성된 surface를 모두 열거하지 않고 suffix 상태를 재사용합니다.
            </p>
          </article>
          <article>
            <span>03</span>
            <strong>선택적 resource</strong>
            <p>
              component data가 필요한 plan에서만 해당 resource를 초기화합니다.
            </p>
          </article>
          <article>
            <span>04</span>
            <strong>bounded streaming</strong>
            <p>
              결과를 streaming해 corpus와 match 수에 비례한 메모리 증가를
              피합니다.
            </p>
          </article>
        </div>
      </DocumentSection>

      <DocumentSection title="Branch 수에 맞춘 anchor와 matcher 선택">
        <p>
          각 branch에서 바뀌지 않는 가장 긴 byte열을 anchor로 선택합니다. 어간이
          교체되는 형태는 첫 어미까지 포함한 문자열, 어간 전체, 짧은 어간과 다음
          고정 요소를 차례로 검토합니다. 한 음절 anchor는 boundary verifier가
          있을 때만 허용합니다.
        </p>
        <pre>
          <code>{`걷다
├─ 걷고 · 걷는 · 걷지 · 걷겠
├─ 걸어 · 걸었
└─ 걸으 · 걸은 · 걸을`}</code>
        </pre>
        <div className="decision-table">
          <div>
            <strong>branch 1개</strong>
            <span>owned memmem Finder를 Box에 보관해 재사용</span>
          </div>
          <div>
            <strong>branch 2개 이상</strong>
            <span>Aho-Corasick overlapping search</span>
          </div>
          <div>
            <strong>후보 겹침</strong>
            <span>검증 후 leftmost-longest 정책으로 겹치지 않게 선택</span>
          </div>
        </div>
      </DocumentSection>

      <DocumentSection title="Plan 크기가 상한을 넘으면 컴파일 실패">
        <p>
          형태 branch를 제한 없이 늘리면 corpus scan을 시작하기도 전에 compile
          latency와 matcher 메모리가 커집니다. 따라서 각 항목의 hard limit를
          공개 계약으로 정합니다. 한도를 넘으면 일부 branch를 누락하지 않고
          컴파일 오류를 반환합니다.
        </p>
        <div className="limit-grid">
          <div>
            <span>QUERY</span>
            <strong>256</strong>
            <small>Unicode scalar</small>
          </div>
          <div>
            <span>ATOMS</span>
            <strong>32</strong>
            <small>query당</small>
          </div>
          <div>
            <span>ANALYSES</span>
            <strong>32</strong>
            <small>atom당</small>
          </div>
          <div>
            <span>BRANCHES</span>
            <strong>4,096</strong>
            <small>plan 전체</small>
          </div>
          <div>
            <span>MATCHER</span>
            <strong>64 MiB</strong>
            <small>예상 메모리</small>
          </div>
          <div>
            <span>DEPTH</span>
            <strong>4</strong>
            <small>continuation</small>
          </div>
        </div>
        <Callout title="정확도 보존">
          <p>
            limit에 가까운 항목과 제외된 규칙은 <code>--explain-query</code>에서
            확인할 수 있습니다. 상한을 넘었다면 branch 일부를 임의로 잘라 결과를
            만들지 않고 컴파일 오류를 반환합니다.
          </p>
        </Callout>
      </DocumentSection>

      <DocumentSection title="Resource 지연 초기화">
        <FlowDiagram
          title="Plan에 필요한 resource만 초기화합니다"
          caption="literal, token, any plan은 compact component resource를 사용하지 않습니다. smart plan도 component branch가 없으면 해당 resource를 찾거나 읽지 않습니다."
          steps={[
            {
              label: '01 · COMPILE',
              title: '필요한 context 표시',
              description:
                '각 branch가 None, PredicateLexical, NominalComponent 중 하나를 선언합니다.',
            },
            {
              label: '02 · INSPECT',
              title: 'Resource 필요 판정',
              description:
                'component requirement가 하나라도 있는 plan에만 asset을 요구합니다.',
            },
            {
              label: '03 · VALIDATE',
              title: '최초 한 번 decode·검증',
              description:
                'schema와 source SHA-256, section digest, scoring 범위를 확인합니다.',
            },
            {
              label: '04 · REUSE',
              title: 'Engine 수명 동안 공유',
              description:
                '검증된 resource를 matcher마다 다시 decode하지 않습니다.',
            },
          ]}
        />
        <p>
          CLI는 설치 경로에서 resource를 자동으로 찾습니다. library와 npm
          binding은 resource 위치를 추정하지 않습니다. 새 bytes를 직접 load할
          때는 전체 검증이 끝난 뒤에만 engine 상태를 교체합니다. 검증에 실패하면
          기존 resource를 그대로 유지합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="Scan hot path의 중복 계산 제거">
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">경로</th>
                <th scope="col">기본 전략</th>
                <th scope="col">비싼 기능을 여는 시점</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>anchor scan</td>
                <td>byte 단위로 scan하고 후보가 없는 구간은 건너뜀</td>
                <td>anchor hit 이후</td>
              </tr>
              <tr>
                <td>span-only match</td>
                <td>바이트 범위만 반환</td>
                <td>설명 metadata 불필요</td>
              </tr>
              <tr>
                <td>provenance</td>
                <td>일치 줄에서만 재계산</td>
                <td>JSON·explain 출력</td>
              </tr>
              <tr>
                <td>normalization</td>
                <td>NFC anchor 직접 검색</td>
                <td>canonical branch를 검사하거나 suffix를 소비할 때</td>
              </tr>
              <tr>
                <td>phrase</td>
                <td>span을 한 번 수집·결합</td>
                <td>모든 atom이 후보를 가질 때</td>
              </tr>
            </tbody>
          </table>
        </div>
      </DocumentSection>

      <DocumentSection title="성능을 분리해서 측정">
        <p>
          최적화한 경로의 효과만 확인할 수 있도록 startup, lexicon load, query
          compile, filesystem walk, scan, verification, output을 따로
          측정합니다. 서로 다른 workload에서 나온 지표는 하나의 점수로 합치지
          않습니다.
        </p>
        <div
          className="metric-timeline"
          role="list"
          aria-label="성능 측정 구간"
        >
          {[
            'startup',
            'lexicon load',
            'query compile',
            'filesystem walk',
            'scan',
            'verification',
            'output',
          ].map((metric) => (
            <span key={metric} role="listitem">
              {metric}
            </span>
          ))}
        </div>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">경고 기준</th>
                <th scope="col">최신 main 대비</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>query compile</td>
                <td>20% 이상 악화</td>
              </tr>
              <tr>
                <td>scan throughput</td>
                <td>10% 이상 악화</td>
              </tr>
              <tr>
                <td>RSS</td>
                <td>20% 이상 증가</td>
              </tr>
              <tr>
                <td>branch 수</td>
                <td>2배 이상 증가</td>
              </tr>
            </tbody>
          </table>
        </div>
      </DocumentSection>
    </article>
  );
}
