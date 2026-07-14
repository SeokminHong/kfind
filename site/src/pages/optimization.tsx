import { FlowDiagram } from '../components/diagram';
import { Callout, DocumentSection, PageIntro } from '../components/document';

export default function OptimizationPage(): React.JSX.Element {
  return (
    <article>
      <PageIntro
        eyebrow="ENGINEERING · DESIGN & OPTIMIZATION"
        title="비용을 큰 입력이 아니라 작은 계획에 묶기"
        summary="kfind의 최적화 목표는 형태 기능을 없애는 것이 아니라, 형태 지식의 비용을 query compile과 anchor 주변의 bounded work로 제한하는 것입니다."
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
            <p>완성 surface 열거 대신 suffix 상태를 재사용합니다.</p>
          </article>
          <article>
            <span>03</span>
            <strong>선택적 resource</strong>
            <p>필요한 plan만 큰 component data를 초기화합니다.</p>
          </article>
          <article>
            <span>04</span>
            <strong>bounded streaming</strong>
            <p>corpus와 match 수에 비례하는 결과 메모리를 피합니다.</p>
          </article>
        </div>
      </DocumentSection>

      <DocumentSection title="Anchor 선택과 adaptive matcher">
        <p>
          각 branch에서 가능한 가장 긴 고정 byte열을 선택합니다. 어간 교체 이후
          첫 어미, 어간 전체, 짧은 어간과 다음 고정 요소 순으로 후보를 검토하며,
          한 음절 anchor는 경계 verifier 없이 허용하지 않습니다.
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
            <span>owned memmem Finder를 Box에 보관</span>
          </div>
          <div>
            <strong>branch 2개 이상</strong>
            <span>Aho-Corasick overlapping search</span>
          </div>
          <div>
            <strong>후보 겹침</strong>
            <span>검증 뒤 leftmost-longest non-overlapping 선택</span>
          </div>
        </div>
      </DocumentSection>

      <DocumentSection title="계획 폭발을 실패로 드러내기">
        <p>
          형태 branch를 무제한으로 늘리면 compile latency와 matcher 메모리가
          corpus scan 전에 이미 커집니다. hard limit를 공개 계약으로 두고 초과
          시 조용히 누락하지 않습니다.
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
            확인합니다. 상한 초과는 branch 일부를 임의로 잘라 품질을 숨기지 않고
            컴파일 오류로 반환합니다.
          </p>
        </Callout>
      </DocumentSection>

      <DocumentSection title="Lazy resource 초기화">
        <FlowDiagram
          title="Plan이 resource 비용을 결정합니다"
          caption="literal, token, any 또는 component branch가 없는 smart plan은 compact component resource를 찾거나 읽지 않습니다."
          steps={[
            {
              label: '01 · COMPILE',
              title: 'Context requirement',
              description:
                'branch가 None, PredicateLexical, NominalComponent 중 하나를 선언합니다.',
            },
            {
              label: '02 · INSPECT',
              title: 'Resource 필요 판정',
              description:
                'component requirement가 하나라도 있는 plan만 asset을 요구합니다.',
            },
            {
              label: '03 · VALIDATE',
              title: '한 번 decode·검증',
              description:
                'schema, source SHA-256, section digest와 scoring 범위를 확인합니다.',
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
          CLI는 설치 경로를 자동 탐색하지만 library와 npm binding은 위치를
          추정하지 않습니다. 새 bytes를 수동 load할 때는 전체 검증이 끝난 뒤에만
          상태를 교체하고, 실패하면 기존 resource를 유지합니다.
        </p>
      </DocumentSection>

      <DocumentSection title="Scan hot path의 복제 제거">
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
                <td>byte-oriented, 후보 없는 구간 skip</td>
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
                <td>canonical branch 또는 suffix 소비</td>
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
          최적화가 어느 경로를 바꿨는지 알 수 있도록 startup, lexicon load,
          query compile, filesystem walk, scan, verification, output을 나눕니다.
          서로 다른 workload의 지표를 한 점수로 합치지 않습니다.
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
