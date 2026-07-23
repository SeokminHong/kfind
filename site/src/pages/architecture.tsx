import type { DocumentContent } from '../components/localized-document';

import { DocumentLocale } from '../app/i18n';
import { createDocumentMeta } from '../app/metadata';
import { RoutePath } from '../app/navigation';
import { LocalizedDocument } from '../components/localized-document';

export const meta = createDocumentMeta(RoutePath.Architecture);

const sectionIds = [
  'query-and-corpus-lanes',
  'candidate-programs',
  'local-verification',
  'phrase-spans',
  'parallel-output',
  'execution-surfaces',
] as const;

const executionDiagram = `query lane
  parse → normalize → analyze → compile programs
                                         │
corpus lane                              ▼
  walk files → byte scan → anchor hit → local verify
                                         │
                                         ▼
                              source span + provenance`;

const content: Readonly<Record<DocumentLocale, DocumentContent>> = {
  [DocumentLocale.Korean]: {
    eyebrow: '기술 · 구조',
    title: '검색 엔진 구조',
    summary:
      'kfind는 형태 분석을 검색 질의와 실제 후보 주변에 한정하고, corpus 전체는 byte anchor와 bounded output 경로로 처리합니다.',
    sections: [
      {
        title: '검색 질의 경로와 corpus 경로',
        body: (
          <>
            <p>
              검색 질의 경로는 입력을 parse하고 정규화한 뒤 사전 분석과 형태
              규칙으로 후보 프로그램을 만듭니다. 각 프로그램은 고정 anchor, 원문
              core mapping, suffix consumption과 판정 제약을 가집니다. 같은
              계획으로 여러 파일을 검색하는 동안 이 과정은 반복되지 않습니다.
            </p>
            <p>
              corpus 경로는 파일을 병렬 순회하고 byte anchor를 찾습니다.
              anchor가 없는 구간에서는 형태소 분석이나 Unicode scalar 순회를
              하지 않습니다. hit가 발생한 위치에서만 UTF-8 경계, 조사·어미,
              token boundary와 선택적 component 조건을 확인합니다.
            </p>
            <pre>
              <code>{executionDiagram}</code>
            </pre>
          </>
        ),
      },
      {
        title: '후보 프로그램',
        body: (
          <>
            <p>
              모든 활용형을 완성 문자열로 열거하면 어미 연쇄에 따라 matcher
              메모리가 증가합니다. <code>CandidateProgram</code>은 고정 부분과
              가변 부분을 분리합니다. anchor는 byte 검색에 사용하고,
              consumption은 anchor 뒤의 조사·어미 상태를 소비합니다.
            </p>
            <pre>
              <code>{`CandidateProgram
├─ anchor: "걸었"
├─ core_mapping: 걷다의 원문 span 투영
├─ consumption: 어미 continuation 시작 상태
├─ decision: boundary 또는 structural constraint
└─ origins: [ㄷ 불규칙, 과거 선어말어미]`}</code>
            </pre>
            <p>
              조사와 어미 continuation은 전역 DFA 또는 trie로 공유합니다. 같은
              surface가 여러 품사 분석이나 규칙에서 만들어지면 실행 span은 한
              번만 반환하고 provenance는 합쳐서 보존합니다.
            </p>
          </>
        ),
      },
      {
        title: '국소 검증',
        body: (
          <>
            <p>
              anchor hit는 일치가 아니라 검증 후보입니다. 엔진은 byte 위치의
              UTF-8 경계, core 왼쪽 경계, suffix 상태 전이, 완성된 token의
              오른쪽 경계와 구조 제약을 순서대로 검사합니다. 필요한 조건을 모두
              통과한 후보만 원문 span이 됩니다.
            </p>
            <p>
              <code>smart</code> 경계의 구조 제약은 token 전체를 다시 분석하지
              않습니다. component resource에서 후보 token의 세부 품사와 정렬된
              형태소 span을 읽고, 검색 질의가 요구한 lexical identity와 인접
              component 관계만 판정합니다. 일반 형태소 분석기처럼 corpus의 모든
              token에 전체 분석 그래프를 만드는 비용이 없습니다.
            </p>
          </>
        ),
      },
      {
        title: '구 span 결합',
        body: (
          <>
            <p>
              구 검색은 atom의 모든 표면형을 곱해 거대한 정규식으로 만들지
              않습니다. 각 atom을 독립적으로 검증한 뒤, span 목록을 검색 질의
              순서와 <code>max-gap</code> 조건으로 결합합니다. 후보가 없는
              atom이 있으면 결합을 중단합니다.
            </p>
            <pre>
              <code>{`atom 0 spans ─┐
atom 1 spans ─┼─ ordered join ─ max-gap ─ phrase span
atom 2 spans ─┘`}</code>
            </pre>
            <p>
              <code>|</code> alternative는 하나의 논리 atom으로 컴파일합니다. 각
              alternative의 고유 anchor를 한 matcher가 함께 scan하고, 같은
              span을 만든 alternative의 provenance는 origin으로 합칩니다.
            </p>
          </>
        ),
      },
      {
        title: '병렬 순회와 출력',
        body: (
          <>
            <p>
              ignore 규칙을 이해하는 parallel walker가 파일을 worker에
              분배합니다. 각 worker는 자신의 searcher와 scratch buffer를
              사용하고, 파일 단위 결과를 capacity가 제한된 channel로 단일
              writer에 전달합니다. writer가 느리면 backpressure가 worker를
              멈추므로 결과 수에 비례해 메모리가 계속 늘지 않습니다.
            </p>
            <p>
              기본 출력은 파일 결과를 모으지 않습니다. <code>--sort path</code>
              만 결정적인 경로 순서를 위해 전체 file stream을 버퍼링합니다.
              소비자가 pipe를 닫아 발생한 broken pipe는 정상 종료로 처리합니다.
            </p>
          </>
        ),
      },
      {
        title: '실행 표면의 책임',
        body: (
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th scope="col">표면</th>
                  <th scope="col">담당 범위</th>
                  <th scope="col">리소스 정책</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>CLI</td>
                  <td>파일 순회, 입력 인코딩, locale, 출력</td>
                  <td>설치 경로에서 버전이 맞는 리소스를 탐색</td>
                </tr>
                <tr>
                  <td>Rust library</td>
                  <td>UTF-8 memory text의 query compile과 match</td>
                  <td>호출자가 resource bytes를 전달</td>
                </tr>
                <tr>
                  <td>npm / WASM</td>
                  <td>JavaScript 문자열과 UTF-16 offset</td>
                  <td>호출자가 asset bytes를 전달</td>
                </tr>
              </tbody>
            </table>
          </div>
        ),
      },
    ],
  },
  [DocumentLocale.English]: {
    eyebrow: 'TECHNICAL · ARCHITECTURE',
    title: 'Search engine architecture',
    summary:
      'kfind confines morphology to query compilation and local candidate verification while corpus-scale work remains byte-anchor scanning and bounded output.',
    sections: [
      {
        title: 'Query lane and corpus lane',
        body: (
          <>
            <p>
              The query lane parses and normalizes the input, resolves lexicon
              analyses, and compiles candidate programs. Each program carries a
              fixed anchor, source-core mapping, suffix consumption, and a
              decision constraint. The plan is reused across files.
            </p>
            <p>
              The corpus lane walks files in parallel and scans byte anchors.
              Regions without an anchor do not run morphological analysis or
              Unicode-scalar iteration. UTF-8 boundaries, particles, endings,
              token boundaries, and optional component conditions are checked
              only at an anchor hit.
            </p>
            <pre>
              <code>{executionDiagram}</code>
            </pre>
          </>
        ),
      },
      {
        title: 'Candidate programs',
        body: (
          <>
            <p>
              Enumerating every completed inflection would make matcher memory
              grow with ending chains. <code>CandidateProgram</code> separates a
              fixed anchor from variable suffix consumption.
            </p>
            <pre>
              <code>{`CandidateProgram
├─ anchor: "걸었"
├─ core_mapping: source projection for 걷다
├─ consumption: ending-continuation state
├─ decision: boundary or structural constraint
└─ origins: [ㄷ irregular, past prefinal ending]`}</code>
            </pre>
            <p>
              Particle and ending continuations share a global DFA or trie. When
              multiple analyses produce one surface span, execution deduplicates
              the span and merges provenance.
            </p>
          </>
        ),
      },
      {
        title: 'Local verification',
        body: (
          <>
            <p>
              An anchor hit is only a candidate. The engine checks its UTF-8
              boundary, the core’s left boundary, suffix-state transitions, the
              completed token’s right boundary, and structural constraints in
              that order.
            </p>
            <p>
              A <code>smart</code> structural decision reads aligned fine-POS
              components for the candidate token and verifies only the lexical
              identity and adjacency required by the query. Unlike a general
              morphological analyzer, kfind does not build a full analysis graph
              for every corpus token.
            </p>
          </>
        ),
      },
      {
        title: 'Phrase-span join',
        body: (
          <>
            <p>
              A phrase does not multiply every atom surface into a large regular
              expression. kfind verifies atoms independently, then joins their
              span lists by query order and <code>max-gap</code>. A missing atom
              stops the join.
            </p>
            <pre>
              <code>{`atom 0 spans ─┐
atom 1 spans ─┼─ ordered join ─ max-gap ─ phrase span
atom 2 spans ─┘`}</code>
            </pre>
            <p>
              <code>|</code> alternatives compile into one logical atom. One
              matcher scans their unique anchors together and merges provenance
              when alternatives produce the same span.
            </p>
          </>
        ),
      },
      {
        title: 'Parallel traversal and output',
        body: (
          <>
            <p>
              An ignore-aware parallel walker distributes files. Each worker
              owns a searcher and scratch buffer and sends file-scoped records
              through a capacity-bounded channel to one writer. Backpressure
              stops workers when output is slower than scanning.
            </p>
            <p>
              Default output never collects the entire corpus result. Only{' '}
              <code>--sort path</code> buffers file streams for deterministic
              path order. A broken pipe caused by the consumer closing is a
              normal exit.
            </p>
          </>
        ),
      },
      {
        title: 'Execution-surface responsibilities',
        body: (
          <div className="table-scroll">
            <table>
              <thead>
                <tr>
                  <th scope="col">Surface</th>
                  <th scope="col">Responsibility</th>
                  <th scope="col">Resource policy</th>
                </tr>
              </thead>
              <tbody>
                <tr>
                  <td>CLI</td>
                  <td>Filesystem, input encoding, locale, and output</td>
                  <td>Discovers version-paired installed resources</td>
                </tr>
                <tr>
                  <td>Rust library</td>
                  <td>Query compile and match over UTF-8 memory text</td>
                  <td>Caller supplies resource bytes</td>
                </tr>
                <tr>
                  <td>npm / WASM</td>
                  <td>JavaScript strings and UTF-16 offsets</td>
                  <td>Caller supplies asset bytes</td>
                </tr>
              </tbody>
            </table>
          </div>
        ),
      },
    ],
  },
};

export default function ArchitecturePage(): React.JSX.Element {
  return <LocalizedDocument content={content} sectionIds={sectionIds} />;
}
