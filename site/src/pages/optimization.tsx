import type { DocumentContent } from '../components/localized-document';

import { DocumentLocale } from '../app/i18n';
import { createDocumentMeta } from '../app/metadata';
import { RoutePath } from '../app/navigation';
import { LocalizedDocument } from '../components/localized-document';

export const meta = createDocumentMeta(RoutePath.Optimization);

const limitsTable = (
  labels: readonly [string, string, string],
): React.JSX.Element => (
  <div className="table-scroll">
    <table>
      <thead>
        <tr>
          <th scope="col">{labels[0]}</th>
          <th scope="col">{labels[1]}</th>
          <th scope="col">{labels[2]}</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td>Query length</td>
          <td>256</td>
          <td>Unicode scalars</td>
        </tr>
        <tr>
          <td>Atoms</td>
          <td>32</td>
          <td>per query</td>
        </tr>
        <tr>
          <td>Analyses</td>
          <td>32</td>
          <td>per atom</td>
        </tr>
        <tr>
          <td>Candidate programs</td>
          <td>4,096</td>
          <td>per plan</td>
        </tr>
        <tr>
          <td>Estimated matcher memory</td>
          <td>64 MiB</td>
          <td>per plan</td>
        </tr>
        <tr>
          <td>Continuation depth</td>
          <td>4</td>
          <td>state transitions</td>
        </tr>
      </tbody>
    </table>
  </div>
);

const content: Readonly<Record<DocumentLocale, DocumentContent>> = {
  [DocumentLocale.Korean]: {
    eyebrow: '기술 · 비용 모델',
    title: '형태 검색의 비용 제어',
    summary:
      '형태 지식은 검색 계획과 anchor 주변의 제한된 판정에 사용하고, corpus 크기에 비례하는 경로는 byte scan과 streaming output으로 유지합니다.',
    sections: [
      {
        title: '비용의 분리',
        body: (
          <>
            <p>
              검색 비용은 검색 계획을 만드는 고정 비용과 corpus를 읽는 가변
              비용으로 나뉩니다. 계획 단계는 사전 분석과 후보 프로그램 수를
              제한합니다. scan 단계는 긴 고정 anchor로 형태 판정이 필요한 위치를
              줄입니다.
            </p>
            <p>
              여러 프로그램은 조사·어미 continuation을 공유합니다. component
              근거가 필요한 계획만 해당 리소스를 초기화하며, 결과는 capacity가
              제한된 channel로 출력합니다. 따라서 형태 규칙의 수, corpus 크기와
              결과 수가 각각 별도의 경계에서 통제됩니다.
            </p>
          </>
        ),
      },
      {
        title: 'Anchor와 matcher',
        body: (
          <>
            <p>
              각 후보 프로그램은 판정 비용을 줄일 수 있는 가장 긴 고정 byte열을
              anchor로 선택합니다. 어간 교체 뒤의 고정 부분과 다음 고정 요소를
              함께 검토하며, 한 음절 anchor에는 반드시 boundary 또는 structural
              decision이 붙습니다.
            </p>
            <pre>
              <code>{`걷다
├─ 걷고 · 걷는 · 걷지 · 걷겠
├─ 걸어 · 걸었
└─ 걸으 · 걸은 · 걸을`}</code>
            </pre>
            <p>
              고유 anchor가 하나면 <code>memmem::Finder</code>를 사용합니다.
              여러 anchor의 누적 검색량이 작으면 finder hit를 직접 병합하고,
              검색량이 커지면 Aho-Corasick automaton을 한 번 만들어
              재사용합니다. 두 경로는 같은 overlapping 후보 순서를 제공합니다.
            </p>
          </>
        ),
      },
      {
        title: '계획 상한',
        body: (
          <>
            <p>
              한 검색 질의가 compile latency와 matcher 메모리를 무제한으로
              사용하지 않도록 입력과 중간 표현에 상한을 둡니다.
            </p>
            {limitsTable(['대상', '상한', '단위'])}
            <p>
              상한을 넘으면 일부 후보를 버리고 실행하지 않습니다. query compile
              오류가 어떤 제한에 도달했는지 알리고, 호출자가 질의를 나누거나
              확장 수준을 좁히게 합니다. 이 동작은 누락된 활용형을 성공 결과로
              숨기지 않습니다.
            </p>
          </>
        ),
      },
      {
        title: '리소스 초기화',
        body: (
          <>
            <p>
              후보 프로그램의 판정은 <code>Boundary</code> 또는{' '}
              <code>Structural</code>입니다. compile된 계획에 structural
              program이 있을 때만 component resource를 읽습니다. literal,{' '}
              <code>token</code>, <code>any</code> 계획은 이 리소스를 사용하지
              않습니다.
            </p>
            <p>
              resource bytes는 schema, 릴리즈 버전, source SHA-256, section
              digest, offset과 component span 검증을 모두 통과한 뒤 engine에
              설치됩니다. 검증된 리소스는 engine 수명 동안 공유합니다. 수동
              교체가 실패하면 사용 중인 리소스를 유지합니다.
            </p>
          </>
        ),
      },
      {
        title: 'Scan 경로',
        body: (
          <>
            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th scope="col">경로</th>
                    <th scope="col">기본 동작</th>
                    <th scope="col">추가 비용의 조건</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td>Anchor scan</td>
                    <td>byte 단위 검색</td>
                    <td>anchor hit</td>
                  </tr>
                  <tr>
                    <td>Span match</td>
                    <td>원문 byte 범위만 반환</td>
                    <td>설명 정보 요청</td>
                  </tr>
                  <tr>
                    <td>Provenance</td>
                    <td>일치 줄에서 계산</td>
                    <td>JSON 또는 explain 출력</td>
                  </tr>
                  <tr>
                    <td>Normalization</td>
                    <td>NFC anchor 직접 검색</td>
                    <td>canonical mapping 또는 suffix 소비</td>
                  </tr>
                  <tr>
                    <td>Phrase</td>
                    <td>atom span을 한 번 수집</td>
                    <td>모든 atom에 후보가 있음</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p>
              겹치는 검증 후보는 core와 token span을 기준으로 leftmost-longest,
              non-overlapping 순서로 정리합니다. 설명 정보가 필요 없는 기본
              출력은 provenance 객체를 만들지 않습니다.
            </p>
          </>
        ),
      },
      {
        title: '성능 지표의 경계',
        body: (
          <p>
            startup, lexicon load, query compile, filesystem walk, scan,
            verification과 output은 별도의 workload로 측정합니다. fixture의
            cases/s는 corpus 처리량이 아니며, 1 GiB literal scan은 형태 품질이나
            component 초기화 비용을 설명하지 않습니다. 지연 시간, 처리량, RSS와
            후보 프로그램 수는 단위가 다르므로 하나의 종합 점수로 합치지
            않습니다.
          </p>
        ),
      },
    ],
  },
  [DocumentLocale.English]: {
    eyebrow: 'TECHNICAL · COST MODEL',
    title: 'Cost control for morphological search',
    summary:
      'Morphological knowledge is confined to query plans and bounded decisions around anchors, while corpus-scale work remains byte scanning and streaming output.',
    sections: [
      {
        title: 'Cost separation',
        body: (
          <>
            <p>
              Search cost splits into fixed query-plan construction and
              corpus-dependent scanning. Planning bounds lexicon analyses and
              candidate programs. Scanning uses long fixed anchors to minimize
              positions requiring morphology.
            </p>
            <p>
              Programs share particle and ending continuations. Only plans with
              component decisions initialize that resource, and output flows
              through a capacity-bounded channel. Rule count, corpus size, and
              result count are controlled at separate boundaries.
            </p>
          </>
        ),
      },
      {
        title: 'Anchors and matchers',
        body: (
          <>
            <p>
              Each candidate program selects the longest fixed byte sequence
              that reduces verification work. A one-syllable anchor always
              carries a boundary or structural decision.
            </p>
            <pre>
              <code>{`걷다
├─ 걷고 · 걷는 · 걷지 · 걷겠
├─ 걸어 · 걸었
└─ 걸으 · 걸은 · 걸을`}</code>
            </pre>
            <p>
              One unique anchor uses <code>memmem::Finder</code>. Small
              multi-anchor inputs merge finder hits directly; larger cumulative
              scans build one reusable Aho-Corasick automaton. Both paths expose
              the same overlapping candidate order.
            </p>
          </>
        ),
      },
      {
        title: 'Plan limits',
        body: (
          <>
            <p>
              Public limits prevent one query from consuming unbounded compile
              latency or matcher memory.
            </p>
            {limitsTable(['Target', 'Limit', 'Unit'])}
            <p>
              Exceeding a limit is a compile error. kfind does not silently
              truncate programs, because a successful partial plan would hide
              missing inflections.
            </p>
          </>
        ),
      },
      {
        title: 'Resource initialization',
        body: (
          <>
            <p>
              A candidate decision is either <code>Boundary</code> or{' '}
              <code>Structural</code>. The component resource is read only when
              the compiled plan contains a structural program. Literal,{' '}
              <code>token</code>, and <code>any</code> plans do not use it.
            </p>
            <p>
              Resource bytes must pass schema, release version, source SHA-256,
              section digest, offset, and component-span validation before
              installation. A failed manual replacement leaves the active
              resource intact.
            </p>
          </>
        ),
      },
      {
        title: 'Scan path',
        body: (
          <>
            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th scope="col">Path</th>
                    <th scope="col">Default</th>
                    <th scope="col">Additional-cost condition</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td>Anchor scan</td>
                    <td>Byte search</td>
                    <td>Anchor hit</td>
                  </tr>
                  <tr>
                    <td>Span match</td>
                    <td>Source byte range only</td>
                    <td>Explanation requested</td>
                  </tr>
                  <tr>
                    <td>Provenance</td>
                    <td>Computed on matching lines</td>
                    <td>JSON or explain output</td>
                  </tr>
                  <tr>
                    <td>Normalization</td>
                    <td>Direct NFC anchor search</td>
                    <td>Canonical mapping or suffix consumption</td>
                  </tr>
                  <tr>
                    <td>Phrase</td>
                    <td>Collect atom spans once</td>
                    <td>Every atom has candidates</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p>
              Verified overlaps use leftmost-longest, non-overlapping resolution
              over core and token spans. Default output does not allocate
              provenance objects.
            </p>
          </>
        ),
      },
      {
        title: 'Performance-metric boundaries',
        body: (
          <p>
            Startup, lexicon load, query compile, filesystem walk, scan,
            verification, and output are separate workloads. Fixture cases/s is
            not corpus throughput, and a 1 GiB literal scan does not measure
            morphology quality or component initialization. Latency, throughput,
            RSS, and program count retain their own units.
          </p>
        ),
      },
    ],
  },
};

export default function OptimizationPage(): React.JSX.Element {
  return <LocalizedDocument content={content} />;
}
