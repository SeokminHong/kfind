import { Link } from 'react-router';

import { RoutePath } from '../app/navigation';
import {
  DocumentPage,
  DocumentSection,
  PageIntro,
} from '../components/document';

export default function OverviewPage(): React.JSX.Element {
  return (
    <DocumentPage>
      <PageIntro
        eyebrow="kfind 0.3.0-rc.2 · TECHNICAL DOCUMENTATION"
        title="한국어 표제어를 검색 가능한 계획으로"
        summary="kfind는 한국어 표제어와 짧은 구를 유한한 검색 계획으로 컴파일하고, 그 계획으로 파일과 메모리의 text를 탐색하는 matcher입니다. 형태 지식은 query를 확장하고 후보를 검증하는 데 사용하며, 검색 대상 전체를 형태소 분석하거나 문장의 의미를 판별하는 데 사용하지 않습니다."
      >
        <div className="document-links">
          <Link to={RoutePath.Playground}>WebAssembly playground</Link>
          <Link to={RoutePath.GettingStarted}>설치와 첫 검색</Link>
          <a href="https://github.com/SeokminHong/kfind">소스 저장소</a>
        </div>
      </PageIntro>

      <DocumentSection title="해결하려는 문제">
        <p>
          일반 문자열 검색은 사용자가 입력한 표면형과 같은 문자열을 찾습니다.
          따라서 <code>걷다</code>로 검색하면 <code>걸어</code>,{' '}
          <code>걸었다</code>, <code>걷는</code>처럼 실제 문장에 나타나는
          활용형을 별도로 열거해야 합니다. kfind는 표제어의 품사와 활용 정보를
          이용해 이 표면형들을 하나의 검색 계획으로 표현합니다. 사용자는
          찾으려는 표제어를 입력하고, 엔진은 그 표제어에서 생성될 수 있는 후보를
          찾아냅니다.
        </p>
        <pre>
          <code>{`query lemma: 걷다
matched surfaces: 걸어 · 걸었다 · 걷는 · 걸을 · 걷기에서도`}</code>
        </pre>
        <p>
          이 기능의 범위는 검색 결과를 만드는 데 필요한 형태 관계까지입니다.
          kfind는 짧은 query를 상한이 정해진 plan으로 컴파일하고, 대규모
          text에서 anchor를 찾아 검증된 span과 품사·생성 규칙을 반환합니다. CLI,
          Rust library와 JavaScript binding은 이 결과 계약을 공유합니다. 반면
          문장 전체의 tokenization, 문맥에 따른 의미 판별, semantic search, 임의
          표면형의 완전한 역분석은 수행하지 않습니다. 형태소 분석기 자체의
          tokenizer 처리량을 높이는 것도 제품 목표가 아닙니다.
        </p>
      </DocumentSection>

      <DocumentSection title="검색 모델">
        <p>
          kfind는 형태 규칙을 짧은 query 쪽에 적용하고, 큰 corpus 쪽에서는
          가능한 한 byte 검색을 유지합니다. 먼저 query를 정규화하고 사전에서
          가능한 품사를 조회한 뒤 활용과 파생 branch를 만듭니다. 각 branch에서는
          변하지 않는 가장 긴 byte열을 anchor로 선택하고, 조사·어미와 경계
          조건은 verifier로 분리합니다. 파일을 scan할 때는 anchor가 있는 위치만
          후보로 삼고, 그 주변에서 verifier를 실행해 최종 span을 결정합니다.
        </p>
        <pre>
          <code>{`query
  → normalize and analyze
  → compile branches
  → choose anchors

corpus
  → scan anchors
  → verify local morphology and boundaries
  → return spans and provenance`}</code>
        </pre>
        <p>
          이 구조에서는 anchor가 없는 buffer에 형태 규칙을 적용할 필요가
          없습니다. query 하나를 여러 파일에 적용하더라도 형태 분석은 컴파일
          단계에서 한 번 이루어지고, corpus 크기에 비례하는 작업은 빠른 scan과
          후보 주변의 제한된 검증으로 남습니다. 따라서 검색 범위를 넓히는 규칙과
          scan 비용을 유발하는 조건을 query plan에서 직접 확인할 수 있습니다.
        </p>
      </DocumentSection>

      <DocumentSection title="사람과 에이전트의 사용 경로">
        <p>
          사람이 터미널에서 검색할 때와 에이전트가 자동화에서 검색할 때는 같은
          엔진을 사용하지만 입력 정보와 오류 비용이 다릅니다. 사람에게는 품사를
          생략할 수 있는 사용성이 중요하고, 관련 없는 결과를 직접 검토하는
          비용이 큽니다. 기본 CLI는 설치된 full POS lexicon으로 품사를 추론하고{' '}
          <code>smart</code> boundary로 precision을 우선합니다.
        </p>
        <p>
          에이전트는 결과 주변의 문맥을 자동으로 확인할 수 있으므로 recall을
          먼저 확보하는 편이 유리합니다. 모든 형태 atom에 품사를 명시하고{' '}
          <code>--boundary any --embedded --json</code>을 사용하면 사전 초기화
          비용을 줄이면서 넓은 후보와 provenance를 받을 수 있습니다. 이 경로는
          false positive를 허용하므로, 호출자는 결과 문맥을 읽고 후보를 걸러
          내는 단계를 함께 구현해야 합니다.
        </p>
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
                <td>터미널 text</td>
                <td>JSON Lines와 provenance</td>
              </tr>
            </tbody>
          </table>
        </div>
        <pre>
          <code>{`# 사람이 직접 검색
kfind 걷다 src docs

# 에이전트 자동화
kfind --embedded --boundary any --pos verb --json 걷다 src docs`}</code>
        </pre>
      </DocumentSection>

      <DocumentSection title="문서의 구성">
        <p>
          처음 사용하는 경우에는{' '}
          <Link to={RoutePath.GettingStarted}>설치와 첫 검색</Link>
          에서 CLI와 agent skill을 준비할 수 있습니다.{' '}
          <Link to={RoutePath.Options}>쿼리와 옵션</Link>은 확장 수준, 품사,
          boundary, Unicode 정규화와 phrase 거리가 서로 어떻게 결합되는지
          정의합니다.
        </p>
        <p>
          내부 원리를 이해하려면{' '}
          <Link to={RoutePath.Analysis}>형태 분석 원리</Link>
          에서 표제어가 branch와 verifier로 변환되는 과정을 먼저 읽은 뒤,{' '}
          <Link to={RoutePath.Architecture}>아키텍처</Link>에서 query compile과
          corpus scan의 접점을 확인하는 순서가 적합합니다.{' '}
          <Link to={RoutePath.Optimization}>설계와 최적화</Link>는 branch 상한,
          anchor 선택, resource 지연 초기화와 streaming이 비용을 제한하는 방식을
          설명합니다. 측정 결과와 해석 조건은{' '}
          <Link to={RoutePath.Benchmarks}>벤치마크</Link>에 분리되어 있습니다.
        </p>
      </DocumentSection>
    </DocumentPage>
  );
}
