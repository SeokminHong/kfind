import type { DocumentContent } from '../components/localized-document';

import { DocumentLocale } from '../app/i18n';
import { createDocumentMeta } from '../app/metadata';
import { RoutePath } from '../app/navigation';
import { LocalizedDocument } from '../components/localized-document';

export const meta = createDocumentMeta(RoutePath.Overview);

const content: Readonly<Record<DocumentLocale, DocumentContent>> = {
  [DocumentLocale.Korean]: {
    eyebrow: '시작 · 제품 범위',
    title: 'kfind 개요',
    summary:
      'kfind는 한국어 표제어와 짧은 구를 유한한 검색 계획으로 컴파일하고, 파일이나 메모리 text에서 형태 조건을 만족하는 span을 찾는 검색 엔진입니다.',
    sections: [
      {
        title: '제품 목적',
        body: (
          <>
            <p>
              입력은 찾으려는 표제어, 선택적 품사와 검색 옵션입니다. 출력은 원문
              span과 그 span을 생성한 표제어·품사·규칙 경로입니다. 예를 들어{' '}
              <code>걷다</code>는 <code>걷고</code>, <code>걸어</code>,{' '}
              <code>걸었다</code>의 후보를 만들고, 원문에서 실제 형태와 경계
              판정을 통과한 위치만 반환합니다.
            </p>
            <p>
              corpus 전체의 형태소열, 문장 구조나 단어 의미를 반환하는 제품은
              아닙니다. 의미가 다른 <code>걸다</code>와 <code>걷다</code>가 같은
              표면형을 만들 수 있으면 두 생성 근거를 보존합니다. 최종 의미
              판별은 결과 주변 문맥을 아는 호출자가 수행합니다.
            </p>
          </>
        ),
      },
      {
        title: '검색 중심 형태 처리',
        body: (
          <>
            <p>
              일반적인 형태소 분석기는 관찰한 문장을 입력으로 받아 각 token의
              표제어와 품사를 추정합니다. kfind는 표제어와 품사를 먼저 받은 뒤,
              검색 가능한 anchor와 허용되는 조사·어미·불규칙 교체를 계산합니다.
              입력과 출력의 방향이 반대입니다.
            </p>
            <p>
              이 모델에서는 형태 처리 비용이 큰 corpus가 아니라 짧은 query에
              집중됩니다. Corpus에서는 고정 byte anchor를 먼저 찾고 그 주변만
              Unicode 경계와 국소 형태 구조로 검증합니다. 전체 문장을 분석하지
              않아도 활용형 recall, 파일 단위 streaming과 생성 근거를 한 검색
              계약 안에서 제공할 수 있습니다.
            </p>
          </>
        ),
      },
      {
        title: '한국어 문법 범위',
        body: (
          <>
            <p>
              체언은 복수 접미사와 조사 연쇄를 처리합니다. 조사는 받침 유무와
              <code>ㄹ</code> 받침을 기준으로 <code>은/는</code>,{' '}
              <code>이/가</code>, <code>으로/로</code> 같은 이형태를 고릅니다.
              용언은 규칙 활용과 ㄷ·ㅂ·ㅅ·르·러·ㅎ 불규칙, 우·오 축약,
              선어말어미와 종결·연결·관형·명사형 어미 연쇄를 지원합니다.
            </p>
            <p>
              지정사, 보조용언, 파생 접미사와 합성용언 내부 성분은 compact
              component resource가 token 안의 품사·span 연결을 증명할 때만
              <code>smart</code> 후보로 유지합니다. 규칙 목록에 없는 임의 조합과
              문맥 의미 추론은 생성하지 않습니다.
            </p>
          </>
        ),
      },
      {
        title: '사용 경로',
        body: (
          <>
            <p>
              사람이 직접 검색할 때는 품사를 생략한 <code>auto</code>와{' '}
              <code>smart</code> 경계가 기본입니다. Full POS 사전과 국소 구조
              근거를 사용해 token 내부 오탐을 제한합니다.
            </p>
            <p>
              에이전트는 각 atom의 품사를 명시하고 <code>--embedded</code>,{' '}
              <code>--boundary any</code>, <code>--json</code>을 함께
              사용합니다. 이 경로는 recall과 낮은 초기화 비용을 우선하며,
              에이전트가 span 주변을 읽어 의미상 후보를 선택합니다. CLI, Rust와
              npm WebAssembly API는 같은 query plan과 provenance 구조를
              공유합니다.
            </p>
          </>
        ),
      },
    ],
  },
  [DocumentLocale.English]: {
    eyebrow: 'START · PRODUCT SCOPE',
    title: 'kfind overview',
    summary:
      'kfind compiles Korean lemmas and short phrases into finite search plans, then finds spans that satisfy those morphology constraints in files or in-memory text.',
    sections: [
      {
        title: 'Product purpose',
        body: (
          <>
            <p>
              The input is a target lemma, an optional part of speech, and
              search options. The output is a source span together with the
              lemma, POS, and rule path that generated it. For example,{' '}
              <code>걷다</code> produces candidates for <code>걷고</code>,{' '}
              <code>걸어</code>, and <code>걸었다</code>, but returns only
              source locations that pass morphology and boundary verification.
            </p>
            <p>
              kfind does not return a complete morpheme sequence, sentence
              parse, or word sense for the corpus. When semantically different
              lemmas can produce the same surface, every valid origin is
              retained. The caller resolves meaning from the surrounding source
              context.
            </p>
          </>
        ),
      },
      {
        title: 'Search-directed morphology',
        body: (
          <>
            <p>
              A conventional morphological analyzer receives an observed
              sentence and estimates the lemma and POS of each token. kfind
              starts with the target lemma and POS, then derives searchable
              anchors and permitted particles, endings, and irregular
              substitutions. The input and output directions are reversed.
            </p>
            <p>
              Morphology cost is therefore concentrated in the short query
              rather than the large corpus. The corpus path scans fixed byte
              anchors first and applies Unicode boundaries and local morphology
              only around hits. This model combines inflection recall, streaming
              file search, and rule provenance without analyzing every sentence.
            </p>
          </>
        ),
      },
      {
        title: 'Korean grammar scope',
        body: (
          <>
            <p>
              Nominals support plural suffixes and particle chains. Particle
              allomorphs such as <code>은/는</code>, <code>이/가</code>, and{' '}
              <code>으로/로</code> are selected from the final consonant
              condition. Predicates support regular conjugation, ㄷ, ㅂ, ㅅ, 르,
              러, and ㅎ irregulars, 우 and 오 contraction, prefinal endings,
              and finite terminal, connective, adnominal, and nominalizing
              chains.
            </p>
            <p>
              Copulas, auxiliaries, derivational suffixes, and internal compound
              components remain under <code>smart</code> only when the compact
              component resource proves the POS and span path inside the token.
              Arbitrary combinations and contextual word-sense inference are not
              generated.
            </p>
          </>
        ),
      },
      {
        title: 'Usage profiles',
        body: (
          <>
            <p>
              Interactive search defaults to POS <code>auto</code> and the{' '}
              <code>smart</code> boundary. The full-POS lexicon and local
              structural evidence constrain internal-token false positives.
            </p>
            <p>
              Agents tag each atom with a POS and combine{' '}
              <code>--embedded</code>, <code>--boundary any</code>, and{' '}
              <code>--json</code>. This profile prioritizes recall and low
              initialization cost, while the agent reads surrounding spans to
              choose semantically relevant candidates. The CLI, Rust API, and
              npm WebAssembly API share the same plan and provenance model.
            </p>
          </>
        ),
      },
    ],
  },
};

export default function OverviewPage(): React.JSX.Element {
  return <LocalizedDocument content={content} />;
}
