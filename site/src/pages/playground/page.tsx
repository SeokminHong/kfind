import type { Match } from '../../kfind-wasm';
import type {
  PlaygroundController,
  PlaygroundInput,
  PlaygroundResult,
} from '../../playground';

import { Button } from '@base-ui/react/button';
import { Collapsible } from '@base-ui/react/collapsible';
import { Field } from '@base-ui/react/field';
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
  ComponentResourceState,
  formatProvenance,
  initialComponentResourceStatus,
  initializePlayground,
  initialPlaygroundInput,
  initialPlaygroundStatus,
  PlaygroundPresetName,
  playgroundPresetOptions,
  PlaygroundResultState,
  PlaygroundState,
} from '../../playground';

import { SearchEditor } from './search-editor';
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
  const controllerRef = useRef<PlaygroundController>(null);
  const [input, setInput] = useState(initialPlaygroundInput);
  const [status, setStatus] = useState(initialPlaygroundStatus);
  const [resourceStatus, setResourceStatus] = useState(
    initialComponentResourceStatus,
  );
  const [result, setResult] = useState<PlaygroundResult>();

  useEffect(() => {
    const controller = initializePlayground(initialPlaygroundInput, {
      onResourceStatusChange: setResourceStatus,
      onResult: setResult,
      onStatusChange: setStatus,
    });
    controllerRef.current = controller;

    return () => {
      controllerRef.current = null;
      controller.dispose();
    };
  }, []);

  useEffect(() => {
    controllerRef.current?.scheduleRun(input);
  }, [input]);

  function updateInput<Key extends keyof PlaygroundInput>(
    key: Key,
    value: PlaygroundInput[Key],
  ): void {
    setInput((current) => ({ ...current, [key]: value }));
  }

  const currentResult = result?.input === input ? result : undefined;
  const currentMatches =
    currentResult?.state === PlaygroundResultState.Success
      ? currentResult.matches
      : [];
  const isEngineReady = status.state === PlaygroundState.Ready;
  const isResourceButtonDisabled =
    !isEngineReady ||
    resourceStatus.state === ComponentResourceState.Loading ||
    resourceStatus.state === ComponentResourceState.Ready;

  return (
    <DocumentPage>
      <PageIntro
        eyebrow="PLAYGROUND · WEBASSEMBLY"
        title="브라우저에서 검색 계획 실행하기"
        summary="현재 source에서 빌드한 kfind-wasm을 사용합니다. 입력한 query와 text는 브라우저 안에서만 처리하며 외부 분석 API로 전송하지 않습니다."
      />

      <DocumentSection title="검색 실습">
        <div className="section-title-row">
          <p>
            Query, text나 옵션을 바꾸고 검색을 실행하면 embedded lexicon으로
            query plan을 다시 컴파일합니다. 일치한 span은 editor에서 바로
            확인하고 아래에서 각 branch의 provenance를 볼 수 있습니다.
          </p>
          <div
            className="wasm-state"
            data-state={status.state}
            role="status"
            aria-live="polite"
          >
            <span className="state-dot" />
            <span>{status.message}</span>
          </div>
        </div>

        <div className="playground-layout">
          <Form
            className="playground-controls"
            onFormSubmit={() => {
              controllerRef.current?.run(input);
            }}
          >
            <Field.Root className="field field-query" name="query">
              <Field.Label data-glossary-skip="">Query</Field.Label>
              <Input
                className="text-control"
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

            <div className="preset-picker">
              <SelectField<PlaygroundPresetName>
                id="preset-select"
                label="예시 전체 설정"
                name="preset"
                onValueChange={(value) => {
                  setInput(applyPlaygroundPreset(value));
                }}
                options={playgroundPresetOptions}
                placeholder="Query · text · 옵션 불러오기"
                value={null}
              />
              <p>예시를 고르면 아래 검색 옵션도 함께 바뀝니다.</p>
            </div>

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
                  min="0"
                  onValueChange={(value) => {
                    updateInput('maxGap', value);
                  }}
                  type="number"
                  value={input.maxGap}
                />
              </Field.Root>
            </div>

            <SearchEditor
              matches={currentMatches}
              onValueChange={(value) => {
                updateInput('text', value);
              }}
              value={input.text}
            />

            <Button
              className="run-button"
              data-glossary-skip=""
              type="submit"
              disabled={!isEngineReady}
            >
              검색 실행
            </Button>

            <div className="resource-loader">
              <div>
                <strong>고급 smart 리소스</strong>
                <span data-state={resourceStatus.state}>
                  {resourceStatus.message}
                </span>
              </div>
              <Button
                data-glossary-skip=""
                disabled={isResourceButtonDisabled}
                onClick={() => {
                  controllerRef.current?.loadComponentResource();
                }}
                type="button"
              >
                {resourceStatus.state === ComponentResourceState.Ready
                  ? 'Component asset 준비됨'
                  : 'Component asset 불러오기'}
              </Button>
            </div>
          </Form>

          <PlaygroundOutput input={input} result={currentResult} />
        </div>

        <p>
          기본 WASM에는 embedded lexicon만 포함되어 있습니다. <code>smart</code>{' '}
          검색이 명사·대명사·수사·관형사 또는 full-POS 일반 용언의 component
          근거를 요구하면 사용자가 고급 resource를 명시적으로 불러와야 합니다.
          이때 같은 origin의 Pages Function이 R2 객체를 streaming하고, engine은
          schema와 checksum 검증을 마친 뒤에만 resource를 적용해 검색을 다시
          실행합니다. Resource를 불러올 필요가 없는 query는 이 network 요청과
          초기화 비용을 발생시키지 않습니다.
        </p>
      </DocumentSection>
    </DocumentPage>
  );
}

function PlaygroundOutput({
  input,
  result,
}: {
  readonly input: PlaygroundInput;
  readonly result: PlaygroundResult | undefined;
}): React.JSX.Element {
  const isPending = result === undefined;
  const summary = resultSummary(result);
  const executionTime =
    result?.elapsedMilliseconds === null ||
    result?.elapsedMilliseconds === undefined
      ? '— ms'
      : `${result.elapsedMilliseconds.toFixed(2)} ms`;
  const rawOutput =
    result?.state === PlaygroundResultState.Error
      ? { error: result.message }
      : (result?.matches ?? []);

  return (
    <div className="playground-output" aria-busy={isPending} aria-live="polite">
      <div className="output-head">
        <div>
          <p className="output-label">결과 · compile + scan</p>
          <p id="result-summary">{summary}</p>
        </div>
        <span className="execution-time">{executionTime}</span>
      </div>

      {result?.state === PlaygroundResultState.Error ? (
        <p className="result-error">{result.message}</p>
      ) : null}

      <div className="match-section">
        <p className="output-label">Matches &amp; provenance</p>
        <MatchList input={input} result={result} />
      </div>

      <Collapsible.Root className="raw-details">
        <Collapsible.Trigger data-glossary-skip="">
          Raw match JSON
        </Collapsible.Trigger>
        <Collapsible.Panel keepMounted>
          <pre>{JSON.stringify(rawOutput, null, 2)}</pre>
        </Collapsible.Panel>
      </Collapsible.Root>
    </div>
  );
}

function resultSummary(result: PlaygroundResult | undefined): string {
  if (result === undefined) {
    return '검색 결과를 갱신하고 있습니다.';
  }

  return result.state === PlaygroundResultState.Error
    ? 'Query compile 또는 검색 실행에 실패했습니다.'
    : result.message;
}

function MatchList({
  input,
  result,
}: {
  readonly input: PlaygroundInput;
  readonly result: PlaygroundResult | undefined;
}): React.JSX.Element {
  if (result?.state !== PlaygroundResultState.Success) {
    return (
      <ol className="match-list">
        <li className="match-empty">
          {result === undefined
            ? '검색을 실행하고 있습니다.'
            : '옵션을 바꾸거나 다른 query로 검색해 보세요.'}
        </li>
      </ol>
    );
  }

  if (result.matches.length === 0) {
    return (
      <ol className="match-list">
        <li className="match-empty">
          옵션을 바꾸거나 다른 query로 검색해 보세요.
        </li>
      </ol>
    );
  }

  return (
    <ol className="match-list">
      {result.matches.map((match, index) => (
        <MatchItem
          index={index}
          key={matchKey(match, index)}
          match={match}
          text={input.text}
        />
      ))}
    </ol>
  );
}

function MatchItem({
  index,
  match,
  text,
}: {
  readonly index: number;
  readonly match: Match;
  readonly text: string;
}): React.JSX.Element {
  return (
    <li>
      <div>
        <span>{String(index + 1).padStart(2, '0')}</span>
        <strong>{text.slice(match.start, match.end)}</strong>
        <code>
          [{match.start}, {match.end})
        </code>
      </div>
      <p>{formatProvenance(match)}</p>
    </li>
  );
}

function matchKey(match: Match, index: number): string {
  return `${match.start}-${match.end}-${index}`;
}
