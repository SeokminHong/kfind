import type { DocumentContent } from '../components/localized-document';

import { DocumentLocale } from '../app/i18n';
import { createDocumentMeta } from '../app/metadata';
import { RoutePath } from '../app/navigation';
import { LocalizedDocument } from '../components/localized-document';

export const meta = createDocumentMeta(RoutePath.Options);

const defaultPlan = `expand=inflection
pos=auto
boundary=smart
unicode-normalization=nfc
max-gap=24`;

const content: Readonly<Record<DocumentLocale, DocumentContent>> = {
  [DocumentLocale.Korean]: {
    eyebrow: '참조 · 검색 질의',
    title: '검색 질의와 실행 옵션',
    summary:
      '확장 수준은 생성할 형태를, 품사는 적용할 문법 규칙을, 경계 정책은 원문 후보의 허용 조건을 정합니다.',
    sections: [
      {
        title: '기본 검색 계획',
        body: (
          <>
            <p>
              옵션이 없으면 활용형을 확장하고, 사전에서 품사를 자동 분석하며,
              품사별 <code>smart</code> 경계를 적용합니다. 검색 질의는 NFC로
              정규화하며 구를 이루는 atom 사이는 Unicode scalar 24개까지
              허용합니다.
            </p>
            <pre>
              <code>{defaultPlan}</code>
            </pre>
            <p>
              자동 분석은 같은 표제어에서 가능한 품사를 합집합으로 보존합니다.
              명시한 품사는 이 집합을 제한하며, 문자열 모양만 보고 동사나
              형용사를 임의로 선택하지 않습니다.
            </p>
          </>
        ),
      },
      {
        title: '형태 확장',
        body: (
          <>
            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th scope="col">값</th>
                    <th scope="col">검색 범위</th>
                    <th scope="col">예시</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td>
                      <code>literal</code>
                    </td>
                    <td>입력 문자열만 사용하며 형태 분석을 하지 않음</td>
                    <td>
                      <code>걸어 → 걸어</code>
                    </td>
                  </tr>
                  <tr>
                    <td>
                      <code>inflection</code>
                    </td>
                    <td>조사, 어미, 이형태와 불규칙 활용을 포함</td>
                    <td>
                      <code>걷다 → 걷고 · 걸어 · 걸었다</code>
                    </td>
                  </tr>
                  <tr>
                    <td>
                      <code>derivation</code>
                    </td>
                    <td>활용과 생산적 접사에 따른 파생 표제어를 포함</td>
                    <td>
                      <code>검증 → 검증하다 · 검증했다</code>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p>
              <code>--literal</code>은{' '}
              <code>--expand literal --pos literal</code>의 단축 옵션입니다.
              파생은 <code>-적</code>, <code>-하다</code>, <code>-되다</code>,{' '}
              <code>-시키다</code>, <code>-스럽다</code>, <code>-답다</code>,{' '}
              <code>-롭다</code>, <code>-화</code> 규칙을 적용합니다. 파생
              결과가 용언이면 그 표제어에 활용 규칙도 적용합니다.
            </p>
          </>
        ),
      },
      {
        title: '경계 정책',
        body: (
          <>
            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th scope="col">값</th>
                    <th scope="col">판정</th>
                    <th scope="col">용도</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td>
                      <code>smart</code>
                    </td>
                    <td>조사·어미 소비와 세부 품사 구성 요소를 국소 검증</td>
                    <td>정밀도 중심의 기본 검색</td>
                  </tr>
                  <tr>
                    <td>
                      <code>token</code>
                    </td>
                    <td>core 시작과 완성된 token 끝의 Unicode 경계를 요구</td>
                    <td>독립 token 검색</td>
                  </tr>
                  <tr>
                    <td>
                      <code>any</code>
                    </td>
                    <td>좌우 경계 없이 형태 프로그램의 부분 span을 보존</td>
                    <td>재현율 중심 자동화</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p>
              <code>n:요리</code>를 <code>중국요리</code>에서 찾을 때
              <code>smart</code>는 명사 구성 요소인 <code>요리</code>를
              허용하지만, 구성 요소 경계를 가로지르는 <code>국요</code>는
              거부합니다.
              <code>any</code>는 구조 경계를 요구하지 않습니다.
            </p>
          </>
        ),
      },
      {
        title: '품사 체계',
        body: (
          <>
            <p>
              <code>--pos auto</code>는 core lexicon, enriched 용언 metadata,
              user lexicon, 생산적 접미 패턴과 full POS lexicon의 분석을 정해진
              우선순위로 모읍니다. 명시적 품사는 다음 coarse POS 중 하나입니다.
            </p>
            <pre>
              <code>{`noun (n:)          pronoun (pro:)     numeral (num:)
verb (v:)          adjective (adj:)   determiner (det:)
adverb (adv:)      particle (j:)      interjection (intj:)
literal (lit:)`}</code>
            </pre>
            <p>
              동사와 형용사는 서로 다른 종결·연결·관형사형 어미 허용 집합을
              사용합니다. 조사는 앞말의 받침 조건에 따른 이형태를 적용합니다.
              사전에 없는 입력을 형태만 보고 용언으로 추정하지 않으며, 필요한
              경우 품사를 명시해야 합니다.
            </p>
          </>
        ),
      },
      {
        title: '정규화와 구 거리',
        body: (
          <>
            <p>
              <code>--unicode-normalization nfc</code>는 검색 질의와 후보를
              NFC로 비교합니다. <code>none</code>은 입력 byte 표현을 보존합니다.
              검색 결과의 span은 항상 원문 offset을 가리킵니다.
            </p>
            <p>
              구 검색은 각 atom을 독립적으로 찾은 뒤 원문 순서로 결합합니다.
              <code>--max-gap</code>은 앞 atom의 token 끝과 다음 atom의 token
              시작 사이에 허용할 Unicode scalar 수이며, 기본값은 24입니다. 한
              atom의 전역 <code>--pos</code>는 허용하지만 여러 atom에서는 각
              atom 태그를 사용해야 합니다.
            </p>
          </>
        ),
      },
      {
        title: '입력과 출력',
        body: (
          <>
            <p>
              파일 순회는 ignore 규칙을 따릅니다. <code>--hidden</code>,{' '}
              <code>--no-ignore</code>, <code>--glob</code>, <code>--type</code>
              으로 대상 파일을 조정하고, <code>--encoding</code>으로 UTF-8
              이외의 입력을 지정합니다.
            </p>
            <pre>
              <code>{`kfind --glob '*.md' --hidden 검증 .
kfind --encoding euc-kr 걷다 legacy.txt
kfind --json --sort path 걷다 src
kfind --explain-query --pos verb 걷다`}</code>
            </pre>
            <p>
              기본 출력은 bounded stream으로 기록합니다.{' '}
              <code>--sort path</code>는 결정적인 경로 순서를 위해 전체 파일
              결과를 버퍼링합니다.
              <code>--json</code>은 UTF-8 원문 span과 provenance를 JSON Lines로
              출력하며, <code>--explain-query</code>는 corpus를 검색하지 않고
              컴파일된 분석과 후보 프로그램을 설명합니다.
            </p>
          </>
        ),
      },
    ],
  },
  [DocumentLocale.English]: {
    eyebrow: 'REFERENCE · QUERY',
    title: 'Query and execution options',
    summary:
      'Expansion selects generated forms, POS selects grammatical rules, and the boundary policy selects admissible source candidates.',
    sections: [
      {
        title: 'Default query plan',
        body: (
          <>
            <p>
              With no options, kfind expands inflections, infers POS from its
              lexicons, applies the POS-specific <code>smart</code> boundary,
              normalizes to NFC, and permits 24 Unicode scalars between phrase
              atoms.
            </p>
            <pre>
              <code>{defaultPlan}</code>
            </pre>
            <p>
              Automatic analysis preserves the union of valid POS analyses for
              an ambiguous lemma. An explicit POS narrows that set; surface
              spelling alone never forces an unknown input into verb or
              adjective rules.
            </p>
          </>
        ),
      },
      {
        title: 'Morphological expansion',
        body: (
          <>
            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th>Value</th>
                    <th>Coverage</th>
                    <th>Example</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td>
                      <code>literal</code>
                    </td>
                    <td>Input string only</td>
                    <td>
                      <code>걸어 → 걸어</code>
                    </td>
                  </tr>
                  <tr>
                    <td>
                      <code>inflection</code>
                    </td>
                    <td>
                      Particles, endings, allomorphs, and irregular conjugation
                    </td>
                    <td>
                      <code>걷다 → 걷고 · 걸어 · 걸었다</code>
                    </td>
                  </tr>
                  <tr>
                    <td>
                      <code>derivation</code>
                    </td>
                    <td>Inflection plus productive derived lemmas</td>
                    <td>
                      <code>검증 → 검증하다 · 검증했다</code>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p>
              <code>--literal</code> is shorthand for{' '}
              <code>--expand literal --pos literal</code>. Derivation covers the
              productive suffixes <code>-적</code>, <code>-하다</code>,{' '}
              <code>-되다</code>, <code>-시키다</code>, <code>-스럽다</code>,{' '}
              <code>-답다</code>, <code>-롭다</code>, and <code>-화</code>.
              Predicate results are inflected in turn.
            </p>
          </>
        ),
      },
      {
        title: 'Boundary policy',
        body: (
          <>
            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th>Value</th>
                    <th>Decision</th>
                    <th>Use</th>
                  </tr>
                </thead>
                <tbody>
                  <tr>
                    <td>
                      <code>smart</code>
                    </td>
                    <td>
                      Locally verifies particle or ending consumption and
                      fine-POS components
                    </td>
                    <td>Precision-oriented default</td>
                  </tr>
                  <tr>
                    <td>
                      <code>token</code>
                    </td>
                    <td>
                      Requires Unicode boundaries around the completed token
                    </td>
                    <td>Independent tokens</td>
                  </tr>
                  <tr>
                    <td>
                      <code>any</code>
                    </td>
                    <td>
                      Preserves morphological substring spans without side
                      boundaries
                    </td>
                    <td>Recall-oriented automation</td>
                  </tr>
                </tbody>
              </table>
            </div>
            <p>
              For <code>n:요리</code> in <code>중국요리</code>, smart mode can
              admit <code>요리</code> as a verified noun component while
              rejecting <code>국요</code>, which crosses component boundaries.
            </p>
          </>
        ),
      },
      {
        title: 'Part-of-speech system',
        body: (
          <>
            <p>
              <code>--pos auto</code> collects analyses from the core lexicon,
              enriched predicate metadata, user lexicon, productive suffix
              patterns, and full-POS lexicon. Explicit coarse POS values are:
            </p>
            <pre>
              <code>{`noun (n:)          pronoun (pro:)     numeral (num:)
verb (v:)          adjective (adj:)   determiner (det:)
adverb (adv:)      particle (j:)      interjection (intj:)
literal (lit:)`}</code>
            </pre>
            <p>
              Verbs and adjectives use different final, connective, and
              adnominal ending sets. Particles select allomorphs by the final
              sound of the preceding nominal.
            </p>
          </>
        ),
      },
      {
        title: 'Normalization and phrase distance',
        body: (
          <>
            <p>
              <code>--unicode-normalization nfc</code> compares normalized query
              and candidate forms. <code>none</code> preserves the input
              representation. Returned spans always address the original text.
            </p>
            <p>
              Phrase atoms are matched independently and joined in source order.{' '}
              <code>--max-gap</code> limits the Unicode scalars between adjacent
              token spans and defaults to 24. Multi-atom queries use per-atom
              POS tags.
            </p>
          </>
        ),
      },
      {
        title: 'Input and output',
        body: (
          <>
            <p>
              Filesystem traversal respects ignore rules. Use{' '}
              <code>--hidden</code>, <code>--no-ignore</code>,{' '}
              <code>--glob</code>, and <code>--type</code> to select files, and
              <code>--encoding</code> for non-UTF-8 input.
            </p>
            <pre>
              <code>{`kfind --glob '*.md' --hidden 검증 .
kfind --encoding euc-kr 걷다 legacy.txt
kfind --json --sort path 걷다 src
kfind --explain-query --pos verb 걷다`}</code>
            </pre>
            <p>
              Default output is a bounded stream. <code>--sort path</code>
              buffers file results for deterministic ordering.{' '}
              <code>--json</code> emits JSON Lines with source spans and
              provenance, while <code>--explain-query</code> prints the compiled
              plan without scanning a corpus.
            </p>
          </>
        ),
      },
    ],
  },
};

export default function OptionsPage(): React.JSX.Element {
  return <LocalizedDocument content={content} />;
}
