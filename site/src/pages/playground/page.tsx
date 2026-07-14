import { useEffect, useRef } from 'react';

import {
  DocumentPage,
  DocumentSection,
  PageIntro,
} from '../../components/document';
import { initializePlayground } from '../../playground';

export default function PlaygroundPage(): React.JSX.Element {
  const playgroundRoot = useRef<HTMLElement>(null);

  useEffect(() => {
    const root = playgroundRoot.current;

    if (root === null) {
      return;
    }

    return initializePlayground(root);
  }, []);

  return (
    <DocumentPage articleRef={playgroundRoot}>
      <PageIntro
        eyebrow="PLAYGROUND · WEBASSEMBLY"
        title="브라우저에서 검색 계획 실행하기"
        summary="현재 source에서 빌드한 kfind-wasm을 사용합니다. 입력한 query와 text는 브라우저 안에서만 처리하며 외부 분석 API로 전송하지 않습니다."
      />

      <DocumentSection title="검색 실습">
        <div className="section-title-row">
          <p>
            Query, text나 옵션을 바꾸고 검색을 실행하면 embedded lexicon으로
            query plan을 다시 컴파일합니다. 결과에는 일치한 표면형과 각 branch의
            provenance가 함께 표시됩니다.
          </p>
          <div
            className="wasm-state"
            id="wasm-status"
            role="status"
            aria-live="polite"
          >
            <span className="state-dot" />
            <span>WASM engine을 불러오는 중…</span>
          </div>
        </div>

        <div className="playground-layout">
          <form className="playground-controls" id="playground-form">
            <div className="field field-query">
              <label htmlFor="query-input">Query</label>
              <input
                id="query-input"
                name="query"
                defaultValue="걷다"
                autoComplete="off"
                aria-label="검색 query"
              />
              <p>
                atom 태그 예: <code>n:사용자 v:검증하다</code>
              </p>
            </div>

            <fieldset className="preset-fieldset">
              <legend>예시</legend>
              <div className="preset-list">
                <button type="button" data-preset="predicate">
                  용언 활용
                </button>
                <button type="button" data-preset="phrase">
                  구(句) 검색
                </button>
                <button type="button" data-preset="component">
                  합성명사 · smart
                </button>
                <button type="button" data-preset="literal">
                  Literal
                </button>
              </div>
            </fieldset>

            <div className="option-grid">
              <div className="field">
                <label htmlFor="pos-select">품사</label>
                <select id="pos-select" name="pos" defaultValue="verb">
                  <option value="auto">자동</option>
                  <option value="noun">명사</option>
                  <option value="pronoun">대명사</option>
                  <option value="numeral">수사</option>
                  <option value="verb">동사</option>
                  <option value="adjective">형용사</option>
                  <option value="determiner">관형사</option>
                  <option value="adverb">부사</option>
                  <option value="particle">조사</option>
                  <option value="interjection">감탄사</option>
                  <option value="literal">Literal</option>
                </select>
              </div>
              <div className="field">
                <label htmlFor="boundary-select">경계</label>
                <select
                  id="boundary-select"
                  name="boundary"
                  defaultValue="smart"
                >
                  <option value="smart">smart</option>
                  <option value="token">token</option>
                  <option value="any">any</option>
                </select>
              </div>
              <div className="field">
                <label htmlFor="expand-select">확장</label>
                <select
                  id="expand-select"
                  name="expand"
                  defaultValue="inflection"
                >
                  <option value="inflection">inflection</option>
                  <option value="derivation">derivation</option>
                  <option value="literal">literal</option>
                </select>
              </div>
              <div className="field">
                <label htmlFor="normalization-select">정규화</label>
                <select
                  id="normalization-select"
                  name="normalization"
                  defaultValue="nfc"
                >
                  <option value="nfc">NFC</option>
                  <option value="canonical">NFC + NFD</option>
                  <option value="none">없음</option>
                </select>
              </div>
              <div className="field field-gap">
                <label htmlFor="max-gap-input">구(句) 최대 간격</label>
                <input
                  id="max-gap-input"
                  name="maxGap"
                  type="number"
                  min="0"
                  defaultValue="24"
                  aria-label="구 최대 간격"
                />
              </div>
            </div>

            <div className="field field-text">
              <div className="field-label-row">
                <label htmlFor="text-input">검색할 텍스트</label>
                <span id="text-count">0자</span>
              </div>
              <textarea
                id="text-input"
                name="text"
                rows={8}
                aria-label="검색할 텍스트"
                defaultValue={
                  '오늘은 공원을 걸었다.\n내일도 천천히 걷고 싶다.\n산책길을 걷는 사람을 만났다.'
                }
              />
            </div>

            <button
              className="run-button"
              id="run-button"
              type="submit"
              disabled
            >
              검색 실행
            </button>

            <div className="resource-loader" id="resource-loader">
              <div>
                <strong>고급 smart 리소스</strong>
                <span id="resource-status">
                  필요한 경우 R2에서 45.6 MiB를 받습니다.
                </span>
              </div>
              <button id="resource-button" type="button" disabled>
                Component asset 불러오기
              </button>
            </div>
          </form>

          <div className="playground-output" aria-live="polite">
            <div className="output-head">
              <div>
                <p className="output-label">결과</p>
                <p id="result-summary">WASM engine을 준비하고 있습니다.</p>
              </div>
              <span className="execution-time" id="execution-time">
                — ms
              </span>
            </div>
            <div className="result-preview" id="result-preview" />

            <div className="match-section">
              <p className="output-label">Matches &amp; provenance</p>
              <ol className="match-list" id="match-list" />
            </div>

            <details className="raw-details">
              <summary>Raw match JSON</summary>
              <pre id="raw-output">[]</pre>
            </details>
          </div>
        </div>

        <p>
          기본 WASM에는 embedded lexicon만 포함되어 있습니다. <code>smart</code>{' '}
          검색이 합성명사의 component 근거를 요구하면 사용자가 고급 resource를
          명시적으로 불러와야 합니다. 이때 같은 origin의 Pages Function이 R2
          객체를 streaming하고, engine은 schema와 checksum 검증을 마친 뒤에만
          resource를 적용해 검색을 다시 실행합니다. Resource를 불러올 필요가
          없는 query는 이 network 요청과 초기화 비용을 발생시키지 않습니다.
        </p>
      </DocumentSection>
    </DocumentPage>
  );
}
