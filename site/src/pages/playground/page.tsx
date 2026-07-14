import type { PlaygroundController, PlaygroundInput } from '../../playground';

import { Button } from '@base-ui/react/button';
import { Collapsible } from '@base-ui/react/collapsible';
import { Field } from '@base-ui/react/field';
import { Fieldset } from '@base-ui/react/fieldset';
import { Form } from '@base-ui/react/form';
import { Input } from '@base-ui/react/input';
import { useEffect, useRef, useState } from 'react';

import {
  DocumentPage,
  DocumentSection,
  PageIntro,
} from '../../components/document';
import {
  BoundaryPolicy,
  ExpandMode,
  NormalizationMode,
  PartOfSpeech,
} from '../../kfind-wasm';
import {
  applyPlaygroundPreset,
  initializePlayground,
  initialPlaygroundInput,
  PlaygroundPresetName,
} from '../../playground';

import { SelectField } from './select-field';

const partOfSpeechOptions = [
  { label: '자동', value: PartOfSpeech.Auto },
  { label: '명사', value: PartOfSpeech.Noun },
  { label: '대명사', value: PartOfSpeech.Pronoun },
  { label: '수사', value: PartOfSpeech.Numeral },
  { label: '동사', value: PartOfSpeech.Verb },
  { label: '형용사', value: PartOfSpeech.Adjective },
  { label: '관형사', value: PartOfSpeech.Determiner },
  { label: '부사', value: PartOfSpeech.Adverb },
  { label: '조사', value: PartOfSpeech.Particle },
  { label: '감탄사', value: PartOfSpeech.Interjection },
  { label: 'Literal', value: PartOfSpeech.Literal },
];

const boundaryOptions = [
  { label: 'smart', value: BoundaryPolicy.Smart },
  { label: 'token', value: BoundaryPolicy.Token },
  { label: 'any', value: BoundaryPolicy.Any },
];

const expandOptions = [
  { label: 'inflection', value: ExpandMode.Inflection },
  { label: 'derivation', value: ExpandMode.Derivation },
  { label: 'literal', value: ExpandMode.Literal },
];

const normalizationOptions = [
  { label: 'NFC', value: NormalizationMode.Nfc },
  { label: 'NFC + NFD', value: NormalizationMode.Canonical },
  { label: '없음', value: NormalizationMode.None },
];

export default function PlaygroundPage(): React.JSX.Element {
  const playgroundRoot = useRef<HTMLElement>(null);
  const controllerRef = useRef<PlaygroundController>(null);
  const inputRef = useRef<PlaygroundInput>(initialPlaygroundInput);
  const [input, setInput] = useState(initialPlaygroundInput);
  const [isEngineReady, setIsEngineReady] = useState(false);

  useEffect(() => {
    const root = playgroundRoot.current;

    if (root === null) {
      return;
    }

    const controller = initializePlayground(
      root,
      () => inputRef.current,
      () => {
        setIsEngineReady(true);
      },
    );
    controllerRef.current = controller;

    return () => {
      controllerRef.current = null;
      controller.dispose();
    };
  }, []);

  function commitInput(nextInput: PlaygroundInput, runImmediately: boolean) {
    inputRef.current = nextInput;
    setInput(nextInput);

    if (runImmediately) {
      controllerRef.current?.run();
    } else {
      controllerRef.current?.scheduleRun();
    }
  }

  function updateInput<Key extends keyof PlaygroundInput>(
    key: Key,
    value: PlaygroundInput[Key],
  ): void {
    commitInput({ ...inputRef.current, [key]: value }, false);
  }

  function selectPreset(presetName: PlaygroundPresetName): void {
    commitInput(applyPlaygroundPreset(inputRef.current, presetName), true);
  }

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
          <Form
            className="playground-controls"
            id="playground-form"
            onFormSubmit={() => {
              controllerRef.current?.run();
            }}
          >
            <Field.Root className="field field-query" name="query">
              <Field.Label data-glossary-skip="">Query</Field.Label>
              <Input
                className="text-control"
                id="query-input"
                autoComplete="off"
                onValueChange={(value) => {
                  updateInput('query', value);
                }}
                value={input.query}
              />
              <Field.Description>
                atom 태그 예: <code>n:사용자 v:검증하다</code>
              </Field.Description>
            </Field.Root>

            <Fieldset.Root className="preset-fieldset">
              <Fieldset.Legend data-glossary-skip="">예시</Fieldset.Legend>
              <div className="preset-list">
                <Button
                  data-glossary-skip=""
                  data-preset={PlaygroundPresetName.Predicate}
                  onClick={() => {
                    selectPreset(PlaygroundPresetName.Predicate);
                  }}
                >
                  용언 활용
                </Button>
                <Button
                  data-glossary-skip=""
                  data-preset={PlaygroundPresetName.Phrase}
                  onClick={() => {
                    selectPreset(PlaygroundPresetName.Phrase);
                  }}
                >
                  구(句) 검색
                </Button>
                <Button
                  data-glossary-skip=""
                  data-preset={PlaygroundPresetName.Component}
                  onClick={() => {
                    selectPreset(PlaygroundPresetName.Component);
                  }}
                >
                  합성명사 · smart
                </Button>
                <Button
                  data-glossary-skip=""
                  data-preset={PlaygroundPresetName.Literal}
                  onClick={() => {
                    selectPreset(PlaygroundPresetName.Literal);
                  }}
                >
                  Literal
                </Button>
              </div>
            </Fieldset.Root>

            <div className="option-grid">
              <SelectField<PartOfSpeech>
                id="pos-select"
                label="품사"
                name="pos"
                onValueChange={(value) => {
                  updateInput('pos', value);
                }}
                options={partOfSpeechOptions}
                value={input.pos}
              />
              <SelectField<BoundaryPolicy>
                id="boundary-select"
                label="경계"
                name="boundary"
                onValueChange={(value) => {
                  updateInput('boundary', value);
                }}
                options={boundaryOptions}
                value={input.boundary}
              />
              <SelectField<ExpandMode>
                id="expand-select"
                label="확장"
                name="expand"
                onValueChange={(value) => {
                  updateInput('expand', value);
                }}
                options={expandOptions}
                value={input.expand}
              />
              <SelectField<NormalizationMode>
                id="normalization-select"
                label="정규화"
                name="normalization"
                onValueChange={(value) => {
                  updateInput('normalization', value);
                }}
                options={normalizationOptions}
                value={input.normalization}
              />
              <Field.Root className="field field-gap" name="maxGap">
                <Field.Label data-glossary-skip="">
                  구(句) 최대 간격
                </Field.Label>
                <Input
                  className="text-control"
                  id="max-gap-input"
                  min="0"
                  onValueChange={(value) => {
                    updateInput('maxGap', value);
                  }}
                  type="number"
                  value={input.maxGap}
                />
              </Field.Root>
            </div>

            <Field.Root className="field field-text" name="text">
              <div className="field-label-row">
                <Field.Label data-glossary-skip="">검색할 텍스트</Field.Label>
                <span>{input.text.length.toLocaleString('ko-KR')}자</span>
              </div>
              <Field.Control
                className="text-control"
                id="text-input"
                onValueChange={(value) => {
                  updateInput('text', value);
                }}
                render={<textarea aria-label="검색할 텍스트" rows={8} />}
                value={input.text}
              />
            </Field.Root>

            <Button
              className="run-button"
              data-glossary-skip=""
              id="run-button"
              type="submit"
              disabled={!isEngineReady}
            >
              검색 실행
            </Button>

            <div className="resource-loader" id="resource-loader">
              <div>
                <strong>고급 smart 리소스</strong>
                <span id="resource-status">
                  필요한 경우 R2에서 45.6 MiB를 받습니다.
                </span>
              </div>
              <Button
                data-glossary-skip=""
                id="resource-button"
                disabled={!isEngineReady}
              >
                Component asset 불러오기
              </Button>
            </div>
          </Form>

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

            <Collapsible.Root className="raw-details">
              <Collapsible.Trigger data-glossary-skip="">
                Raw match JSON
              </Collapsible.Trigger>
              <Collapsible.Panel keepMounted>
                <pre id="raw-output">[]</pre>
              </Collapsible.Panel>
            </Collapsible.Root>
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
