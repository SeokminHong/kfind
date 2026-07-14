import { Link } from 'react-router';

import { RoutePath } from '../app/navigation';
import { FlowDiagram } from '../components/diagram';
import {
  Callout,
  DocumentSection,
  PageIntro,
  RouteCard,
} from '../components/document';

export default function OverviewPage(): React.JSX.Element {
  return (
    <article>
      <PageIntro
        eyebrow="kfind 0.2.1 · TECHNICAL DOCUMENTATION"
        title="한국어 표제어를 검색 가능한 계획으로"
        summary="kfind는 한국어 표제어와 짧은 구를 유한한 검색 계획으로 컴파일하는 query-directed matcher입니다. 이 계획을 이용해 파일과 메모리의 텍스트를 빠르게 탐색합니다."
      >
        <div className="document-links">
          <Link to={RoutePath.Playground}>WebAssembly playground</Link>
          <Link to={RoutePath.GettingStarted}>5분 안에 시작하기</Link>
          <a href="https://github.com/SeokminHong/kfind">소스 저장소</a>
        </div>
        <Callout title="처리 범위">
          <p>
            형태 지식은 쿼리 확장과 후보 검증에 사용합니다. 검색 대상 전체를
            형태소 분석하거나 문장의 의미를 판별하지 않습니다.
          </p>
        </Callout>
      </PageIntro>

      <DocumentSection
        title="무엇을 해결하는가"
        lead="정확한 표면형을 모두 기억하지 않아도 표제어에서 파생되는 실제 텍스트를 찾습니다."
      >
        <div className="example-pair">
          <div>
            <span>QUERY</span>
            <strong>걷다</strong>
          </div>
          <div>
            <span>MATCHED SURFACES</span>
            <p>걸어 · 걸었다 · 걷는 · 걸을 · 걷기에서도</p>
          </div>
        </div>
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">Goal</th>
                <th scope="col">Non-goal</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <td>짧은 query를 상한이 정해진 plan으로 컴파일</td>
                <td>문장 전체 형태소 분석과 tokenization</td>
              </tr>
              <tr>
                <td>anchor를 기준으로 대규모 text를 scan</td>
                <td>문맥 의미 판별과 semantic search</td>
              </tr>
              <tr>
                <td>match span, 품사, 생성 규칙 반환</td>
                <td>임의 표면형의 완전한 역분석</td>
              </tr>
              <tr>
                <td>CLI, Rust, JavaScript에서 같은 계약 유지</td>
                <td>형태소 분석기 자체의 최고 속도 경쟁</td>
              </tr>
            </tbody>
          </table>
        </div>
      </DocumentSection>

      <DocumentSection
        title="검색 모델"
        lead="형태 규칙 계산은 짧은 query를 컴파일할 때 수행합니다. 큰 corpus를 scan할 때는 문자열 anchor로 후보를 찾고 그 주변만 검증합니다."
      >
        <FlowDiagram
          title="Query-directed search pipeline"
          caption="Corpus의 모든 문장을 분석하지 않습니다. query plan으로 후보를 좁힌 뒤 필요한 위치만 검증합니다."
          steps={[
            {
              label: '01 · COMPILE',
              title: '표제어 해석',
              description:
                'query를 정규화하고 품사를 조회한 뒤 활용·파생 branch를 만듭니다.',
            },
            {
              label: '02 · ANCHOR',
              title: '고정 문자열 선택',
              description:
                '각 branch에서 바뀌지 않는 가장 긴 byte열을 anchor로 고릅니다.',
            },
            {
              label: '03 · SCAN',
              title: '후보 위치 탐색',
              description:
                '파일을 병렬 순회하며 anchor가 있는 위치만 찾습니다.',
            },
            {
              label: '04 · VERIFY',
              title: '후보 주변 형태 검증',
              description:
                '후보의 경계와 suffix를 확인하고 span과 provenance를 반환합니다.',
            },
          ]}
        />
      </DocumentSection>

      <DocumentSection
        title="두 가지 사용 경로"
        lead="사람과 자동화는 같은 엔진을 사용하지만 입력 정보와 품질 우선순위가 다릅니다."
      >
        <div className="table-scroll">
          <table>
            <thead>
              <tr>
                <th scope="col">항목</th>
                <th scope="col">사람 · precision 우선</th>
                <th scope="col">에이전트 · recall 우선</th>
              </tr>
            </thead>
            <tbody>
              <tr>
                <th scope="row">품사</th>
                <td>생략하면 full POS로 자동 추론</td>
                <td>각 atom에 명시</td>
              </tr>
              <tr>
                <th scope="row">Boundary</th>
                <td>
                  <code>smart</code>
                </td>
                <td>
                  <code>any</code>
                </td>
              </tr>
              <tr>
                <th scope="row">사전</th>
                <td>설치된 full POS를 자동으로 사용</td>
                <td>
                  <code>--embedded</code>
                </td>
              </tr>
              <tr>
                <th scope="row">출력</th>
                <td>읽기 쉬운 터미널 text</td>
                <td>JSON Lines와 provenance</td>
              </tr>
            </tbody>
          </table>
        </div>
        <pre>
          <code>{`kfind 걷다 src docs
kfind --embedded --boundary any --pos verb --json 걷다 src docs`}</code>
        </pre>
      </DocumentSection>

      <DocumentSection title="문서 지도">
        <div className="route-card-grid">
          <RouteCard
            eyebrow="REFERENCE"
            title="쿼리와 옵션"
            description="inflection, derivation, literal의 차이와 경계·품사 조합을 비교합니다."
            to={RoutePath.Options}
          />
          <RouteCard
            eyebrow="CONCEPT"
            title="형태 분석 원리"
            description="표제어 분석과 불규칙 교체를 거쳐 verifier를 만드는 과정을 설명합니다."
            to={RoutePath.Analysis}
          />
          <RouteCard
            eyebrow="INTERNALS"
            title="아키텍처"
            description="query compile 결과가 corpus scan에 사용되는 과정을 살펴봅니다."
            to={RoutePath.Architecture}
          />
          <RouteCard
            eyebrow="ENGINEERING"
            title="설계와 최적화"
            description="branch 제한, anchor 선택, resource 지연 초기화와 streaming을 다룹니다."
            to={RoutePath.Optimization}
          />
        </div>
      </DocumentSection>
    </article>
  );
}
