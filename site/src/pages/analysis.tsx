import type { DocumentContent } from '../components/localized-document';

import { DocumentLocale } from '../app/i18n';
import { createDocumentMeta } from '../app/metadata';
import { RoutePath } from '../app/navigation';
import { LocalizedDocument } from '../components/localized-document';

export const meta = createDocumentMeta(RoutePath.Analysis);

const sectionIds = [
  'analysis-direction',
  'lexicon-layers',
  'particles-and-allomorphs',
  'endings',
  'irregulars-and-contractions',
  'derivation-and-compounds',
  'structural-verification',
] as const;

const content: Readonly<Record<DocumentLocale, DocumentContent>> = {
  [DocumentLocale.Korean]: {
    eyebrow: '내부 원리 · 형태',
    title: '형태 분석',
    summary:
      'kfind의 형태 처리는 표제어에서 검색 조건을 만드는 정방향 생성과 원문 후보를 검증하는 국소 판정으로 구성됩니다.',
    sections: [
      {
        title: '분석 방향',
        body: (
          <>
            <p>
              Query atom 하나는 표제어와 선택적 품사를 가집니다. 사전 조회는 이
              입력을 설명할 수 있는 분석을 만들고, 각 분석은 anchor, core span,
              조사·어미 소비 상태, 경계 조건과 생성 근거를 가진{' '}
              <code>CandidateProgram</code>으로 컴파일됩니다.
            </p>
            <p>
              Corpus에서는 anchor가 있는 위치만 program으로 검증합니다. 이
              방향은 관찰한 모든 token을 분석하는 작업과 다릅니다. kfind가
              반환하는 정보는 전체 문장의 형태소열이 아니라 query에서 도달
              가능한 표면형의 span과 provenance입니다.
            </p>
          </>
        ),
      },
      {
        title: '사전 계층',
        body: (
          <>
            <p>
              Core lexicon은 핵심 불규칙, 품사 중의성, 기능어와 표면형 예외를
              담습니다. Enriched 계층은 고정된 국립국어원 사전 snapshot이 함께
              지지하는 활용·파생 관계만 보존합니다. Full POS lexicon은 자동 품사
              후보와 세부 품사를 제공하고, user lexicon은 프로젝트 용어와 교체
              규칙을 추가합니다.
            </p>
            <p>
              같은 표제어에 여러 분석이 있으면 하나를 임의로 선택하지 않습니다.
              실행 조건이 같으면 program을 합치되 모든 품사와 규칙 provenance를
              유지합니다. 사용자가 coarse POS를 명시하면 noun, verb 같은 범주가
              포함하는 세부 품사의 누락도 fallback 분석으로 채웁니다.
            </p>
          </>
        ),
      },
      {
        title: '조사와 이형태',
        body: (
          <>
            <p>
              체언은 표면형마다 모든 조사 조합을 열거하지 않습니다. Core를 찾은
              뒤 verifier가 남은 suffix를 조사 전이로 읽습니다. 받침이 없으면{' '}
              <code>는·가·를·로</code>, 받침이 있으면 <code>은·이·을·으로</code>
              를 선택합니다. <code>ㄹ</code> 받침 뒤의 <code>로</code>처럼 별도
              음운 조건도 포함합니다.
            </p>
            <pre>
              <code>{`학교 + 에서 + 는  → 학교에서는\n길 + 으로       → 길로\n집 + 로         → 거부`}</code>
            </pre>
            <p>
              조사 연쇄는 등록된 전이만 허용합니다. 완성된 조사 뒤에 임의
              문자열을 이어 붙이지 않으며 <code>smart</code>와{' '}
              <code>token</code>은 소비가 끝난 token 경계를 확인합니다.
            </p>
          </>
        ),
      },
      {
        title: '어미와 선어말어미',
        body: (
          <>
            <p>
              용언 program은 어간 교체와 어미 연쇄를 분리합니다. 선어말어미는
              종결·연결 어미 앞에서 시제, 높임과 추측을 나타냅니다. 예를 들어{' '}
              <code>먹었겠지만</code>은{' '}
              <code>먹/VV + 었/EP + 겠/EP + 지만/EC</code>
              으로 분석됩니다. kfind는 <code>었</code>과 <code>겠</code>을 typed
              consumption 상태로 보존한 뒤 허용된 <code>지만</code>으로 token을
              닫습니다.
            </p>
            <p>
              현재 평서형 <code>-ㄴ다/-는다</code>, 관형형{' '}
              <code>-는/-ㄴ/-을</code>, 명사형 <code>-기/-ㅁ</code>, 연결형{' '}
              <code>-고/-면/-니/-려고</code>와 지정된 후속 어미를 지원합니다.
              문법적으로 가능해 보이더라도 규칙 데이터에 없는 연쇄는 만들지
              않습니다.
            </p>
          </>
        ),
      },
      {
        title: '불규칙 활용과 축약',
        body: (
          <>
            <p>
              사전 entry는 적용할 교체 분류를 제공하고 generator가 어미의 음운
              환경과 결합합니다. <code>걷다</code>의 ㄷ 불규칙은 모음 시작 어미
              앞에서 <code>걷 + 어 → 걸어</code>를 만들고, <code>돕다</code>의
              ㅂ 불규칙은 <code>돕 + 아 → 도와</code>를 만듭니다.{' '}
              <code>모르다 → 몰라</code>, <code>낫다 → 나아</code>,{' '}
              <code>파랗다 → 파란</code>도 같은 방식으로 어휘 분류와 공통 어미
              규칙을 합성합니다.
            </p>
            <p>
              <code>보아 → 봐</code>, <code>주어 → 줘</code> 같은 축약은 한글
              음절의 초성·중성·종성을 분해한 뒤 허용된 결합으로 다시 조합합니다.
              규칙형과 불규칙형 분석이 모두 가능한 표제어는 두 program을 보존해
              recall을 유지합니다.
            </p>
          </>
        ),
      },
      {
        title: '파생과 복합 구조',
        body: (
          <>
            <p>
              <code>inflection</code>은 품사를 유지하는 활용을,{' '}
              <code>derivation</code>은 등록된 접미사로 새 품사를 만드는
              표면까지 포함합니다. <code>안정 + 하 + 다</code>,{' '}
              <code>잠식 + 당하 + 기</code>처럼 명사·어근과 파생 접미사가
              이어지는 경로는 source component의 품사와 span이 완성될 때만
              승인합니다.
            </p>
            <p>
              보조용언과 합성용언 내부 검색도 같은 제약을 사용합니다.{' '}
              <code>들어가다</code>의 <code>가다</code>는 선행 용언과 연결
              어미가 token 왼쪽부터 이어져야 하지만, <code>친구가</code>의 조사{' '}
              <code>가</code>는 동사 후보가 될 수 없습니다.
            </p>
            <p>
              생성 program이 남긴 보조용언 연쇄를 resource로 보완할 때는 연결
              어미 바깥의 <code>VX + E*</code> 경로가 완성되어야 합니다. 연결
              어미가 core 바깥에 있거나, 축약된 core가 용언으로만 분석되거나,
              token 전체의 정확한 분석이 <code>용언 + EC + VX + E+</code>이거나,
              결과 변화를 나타내는 <code>-아/어지다</code> 계열일 때 source
              경로를 사용합니다. <code>빼놓을</code>, <code>생겨났던</code>,{' '}
              <code>극심해지겠지만</code>은 유지하지만, 중의적인 <code>해</code>
              만으로 <code>해가며</code>를 확장하지 않습니다.
            </p>
          </>
        ),
      },
      {
        title: '국소 구조 판정',
        body: (
          <>
            <p>
              <code>smart</code>는 anchor 주변의 제한된 token만 compact
              component resource로 해독합니다. Query가 요구하는 품사열,
              component span, continuation과 인접 token 조건을{' '}
              <code>StructuralConstraint</code>로 비교합니다. 의미가 아니라 관찰
              가능한 형태 구조만 사용합니다.
            </p>
            <p>
              일반 분석기처럼 corpus 전체 tokenization을 만들지 않으므로 큰
              파일의 scan 경로는 byte 검색으로 유지됩니다. 동시에 source가
              증명한 내부 성분과 조사·어미 경계를 사용해 단순 substring 검색보다
              정밀한 후보를 반환합니다.
            </p>
            <p>
              조사로 끝나는 체언에서는 정확한 전체 체언 경로를 먼저 선택합니다.{' '}
              <code>산길을</code>의 대안 분해에만 있는 <code>길</code>은
              거부하지만, 전체 분석 자체가 선언한 <code>자본주의</code>의{' '}
              <code>주의</code> component는 유지합니다.
            </p>
          </>
        ),
      },
    ],
  },
  [DocumentLocale.English]: {
    eyebrow: 'INTERNALS · MORPHOLOGY',
    title: 'Morphology',
    summary:
      'kfind combines forward generation from a target lemma with local structural verification of source candidates.',
    sections: [
      {
        title: 'Analysis direction',
        body: (
          <>
            <p>
              A query atom contains a lemma and an optional part of speech.
              Lexicon lookup produces compatible analyses. Each analysis
              compiles into a <code>CandidateProgram</code> containing an
              anchor, core span, particle and ending consumption, boundary
              conditions, and provenance.
            </p>
            <p>
              Only source positions containing an anchor are verified against
              the program. This differs from analyzing every observed token. The
              result is a span reachable from the query plus its origin, not a
              complete morpheme sequence for the sentence.
            </p>
          </>
        ),
      },
      {
        title: 'Lexicon layers',
        body: (
          <>
            <p>
              The core lexicon stores essential irregulars, POS ambiguity,
              function words, and surface exceptions. The enriched layer
              contains inflection and derivation relations supported by pinned
              Korean dictionary snapshots. The full-POS lexicon supplies
              automatic and fine-grained POS candidates, while the user lexicon
              adds project terms and replacement rules.
            </p>
            <p>
              Multiple analyses for one lemma are preserved. Programs with
              identical execution conditions may be merged, but every POS and
              rule origin remains in provenance. An explicit coarse POS also
              receives fallback analyses for missing fine-grained categories.
            </p>
          </>
        ),
      },
      {
        title: 'Particles and allomorphs',
        body: (
          <>
            <p>
              Nominals do not enumerate every particle chain as a complete
              surface. After a core hit, a verifier reads the remaining suffix
              through a particle transition table. Vowel-final hosts select{' '}
              <code>는·가·를·로</code>, consonant-final hosts select{' '}
              <code>은·이·을·으로</code>, with a dedicated condition for{' '}
              <code>로</code> after final ㄹ.
            </p>
            <pre>
              <code>{`학교 + 에서 + 는  → 학교에서는\n길 + 으로       → 길로\n집 + 로         → rejected`}</code>
            </pre>
            <p>
              Only registered particle transitions are accepted.{' '}
              <code>smart</code> and <code>token</code> verify the token
              boundary after consumption.
            </p>
          </>
        ),
      },
      {
        title: 'Endings and prefinal endings',
        body: (
          <>
            <p>
              Predicate programs separate stem substitution from ending chains.
              Prefinal endings express tense, honorific meaning, and modality
              before connective or terminal endings. <code>먹었겠지만</code>,
              for example, is <code>먹/VV + 었/EP + 겠/EP + 지만/EC</code>.
              kfind represents <code>었</code> and <code>겠</code> as typed
              consumption states and closes the token with the permitted{' '}
              <code>지만</code> transition.
            </p>
            <p>
              The rule catalog includes present declaratives, adnominal and
              nominalizing endings, connectives, and bounded continuations. A
              chain that is absent from the versioned rule data is not inferred
              merely because it appears grammatically plausible.
            </p>
          </>
        ),
      },
      {
        title: 'Irregular conjugation and contraction',
        body: (
          <>
            <p>
              A lexicon entry selects a substitution class, and the generator
              combines it with the phonological environment of an ending. ㄷ
              irregular <code>걷다</code> yields <code>걷 + 어 → 걸어</code>; ㅂ
              irregular <code>돕다</code> yields <code>돕 + 아 → 도와</code>.
              The same composition handles <code>모르다 → 몰라</code>,{' '}
              <code>낫다 → 나아</code>, and <code>파랗다 → 파란</code>.
            </p>
            <p>
              Contractions such as <code>보아 → 봐</code> and{' '}
              <code>주어 → 줘</code> decompose and recombine Hangul syllable
              components under explicit rules. When both regular and irregular
              analyses are valid, both programs are retained for recall.
            </p>
          </>
        ),
      },
      {
        title: 'Derivation and compound structure',
        body: (
          <>
            <p>
              <code>inflection</code> preserves POS, while{' '}
              <code>derivation</code> includes registered suffixes that create a
              new POS. Paths such as <code>안정 + 하 + 다</code> and{' '}
              <code>잠식 + 당하 + 기</code> are accepted only when source
              components prove a complete POS and span sequence.
            </p>
            <p>
              Auxiliary and compound-internal search uses the same constraint.
              Internal <code>가다</code> in <code>들어가다</code> requires a
              preceding predicate and connective ending, while particle{' '}
              <code>가</code> in <code>친구가</code> cannot become a verb
              candidate.
            </p>
            <p>
              When the resource completes an auxiliary chain left by a generated
              program, the <code>VX + E*</code> path after the connective must
              be complete. The source path is used when the connective lies
              outside the core, every exact analysis of the contracted core is a
              predicate, the complete token has an exact{' '}
              <code>predicate + EC + VX + E+</code> analysis, or the suffix is
              the resultative <code>-아/어지다</code> family. This retains{' '}
              <code>빼놓을</code>, <code>생겨났던</code>, and{' '}
              <code>극심해지겠지만</code> without expanding <code>해가며</code>{' '}
              from the ambiguous <code>해</code> surface alone.
            </p>
          </>
        ),
      },
      {
        title: 'Local structural verification',
        body: (
          <>
            <p>
              <code>smart</code> decodes only bounded tokens around an anchor
              from the compact component resource. A{' '}
              <code>StructuralConstraint</code> compares the POS sequence,
              component spans, continuations, and neighboring-token conditions
              required by the query. It uses observable morphology, not semantic
              interpretation.
            </p>
            <p>
              Because no corpus-wide tokenization is built, large-file scanning
              stays on the byte-search path. Source-proven internal components
              and particle-ending boundaries still make the result more precise
              than a plain substring search.
            </p>
            <p>
              For a nominal followed by particles, the exact whole-nominal path
              is selected first. <code>길</code> found only in an alternative
              decomposition of <code>산길을</code> is rejected, while the{' '}
              <code>주의</code> component declared by the whole analysis of{' '}
              <code>자본주의</code> is retained.
            </p>
          </>
        ),
      },
    ],
  },
};

export default function AnalysisPage(): React.JSX.Element {
  return <LocalizedDocument content={content} sectionIds={sectionIds} />;
}
