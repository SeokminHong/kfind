import type { Match } from '../../kfind-wasm';
import type {
  PlaygroundController,
  PlaygroundInput,
  PlaygroundResult,
} from '../../playground';

import { Button } from '@base-ui/react/button';
import { Field } from '@base-ui/react/field';
import { Input } from '@base-ui/react/input';
import { Tabs } from '@base-ui/react/tabs';
import { useEffect, useRef, useState } from 'react';

import {
  DocumentPage,
  DocumentSection,
  PageIntro,
} from '../../components/document';
import { BoundaryPolicy, ExpandMode, PartOfSpeech } from '../../kfind-wasm';
import {
  applyPlaygroundPreset,
  ComponentResourceState,
  formatProvenance,
  initialComponentResourceStatus,
  initializePlayground,
  initialPlaygroundInput,
  initialPlaygroundStatus,
  playgroundPresetOptions,
  PlaygroundResultState,
  PlaygroundState,
} from '../../playground';

import { QueryField } from './query-field';
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
  {
    label: 'smart',
    value: BoundaryPolicy.Smart,
    description: '품사별 형태 검증 후 완성된 token 경계를 확인합니다.',
  },
  {
    label: 'token',
    value: BoundaryPolicy.Token,
    description: 'core 시작과 완성된 token 양쪽 경계를 엄격히 확인합니다.',
  },
  {
    label: 'any',
    value: BoundaryPolicy.Any,
    description: '좌우 경계 없이 부분 문자열 후보까지 보존합니다.',
  },
];

const expandOptions = [
  {
    label: 'inflection',
    value: ExpandMode.Inflection,
    description: '품사를 유지하며 조사·어미 결합과 불규칙 활용을 찾습니다.',
  },
  {
    label: 'derivation',
    value: ExpandMode.Derivation,
    description: '활용에 더해 새 품사를 만드는 생산적 파생형까지 찾습니다.',
  },
  {
    label: 'literal',
    value: ExpandMode.Literal,
    description: '형태 분석 없이 입력 문자열만 그대로 찾습니다.',
  },
];

enum PlaygroundOutputTab {
  Matches = 'matches',
  RawJson = 'raw-json',
}

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
    resourceStatus.state === ComponentResourceState.Checking ||
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
            Query, text나 옵션을 바꾸면 잠시 뒤 embedded lexicon으로 query
            plan을 다시 컴파일합니다. 일치한 span은 editor에서 바로 확인하고
            아래에서 각 branch의 provenance를 볼 수 있습니다.
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
          <div className="playground-controls">
            <QueryField
              onValueChange={(value) => {
                updateInput('query', value);
              }}
              value={input.query}
            />

            <div className="preset-picker">
              <span className="preset-label">예시 전체 설정 불러오기</span>
              <div className="preset-actions">
                {playgroundPresetOptions.map((preset) => (
                  <Button
                    data-glossary-skip=""
                    key={preset.value}
                    onClick={() => {
                      setInput(applyPlaygroundPreset(preset.value));
                    }}
                    type="button"
                  >
                    {preset.label}
                  </Button>
                ))}
              </div>
              <p>각 버튼은 Query, text와 모든 검색 옵션을 함께 바꿉니다.</p>
            </div>

            <div className="option-grid">
              <SelectField<PartOfSpeech>
                description="Atom 태그와 선택 품사 중 어느 쪽도 우선하지 않습니다. auto가 아니면 서로 같아야 하며, 다르면 오류입니다."
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
                description={selectedOptionDescription(
                  boundaryOptions,
                  input.boundary,
                )}
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
                description={selectedOptionDescription(
                  expandOptions,
                  input.expand,
                )}
                id="expand-select"
                label="확장"
                name="expand"
                onValueChange={(value) => {
                  updateInput('expand', value);
                }}
                options={expandOptions}
                value={input.expand}
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
                <Field.Description>
                  Phrase atom 사이에 허용할 최대 Unicode 문자 수입니다.
                </Field.Description>
              </Field.Root>
            </div>

            <SearchEditor
              matches={currentMatches}
              onValueChange={(value) => {
                updateInput('text', value);
              }}
              value={input.text}
            />

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
                {componentResourceButtonLabel(resourceStatus.state)}
              </Button>
            </div>
          </div>

          <PlaygroundOutput input={input} result={currentResult} />
        </div>

        <p>
          기본 WASM에는 embedded lexicon만 포함되어 있습니다. <code>smart</code>{' '}
          검색이 명사·대명사·수사·관형사 또는 full-POS 일반 용언의 component
          근거를 요구하면 사용자가 고급 resource를 명시적으로 불러와야 합니다.
          이때 같은 origin의 Pages Function이 R2 객체를 streaming하고, engine은
          schema와 checksum 검증을 마친 뒤 현재 build key로 브라우저 저장소에
          보관합니다. 같은 build로 다시 들어오면 저장된 resource를 자동으로
          복원합니다. Resource가 필요 없는 query는 최초 network 요청을 하지
          않습니다.
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
  const [activeTab, setActiveTab] = useState(PlaygroundOutputTab.Matches);
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

      <Tabs.Root
        className="result-tabs"
        onValueChange={(value) => {
          if (isPlaygroundOutputTab(value)) {
            setActiveTab(value);
          }
        }}
        value={activeTab}
      >
        <Tabs.List activateOnFocus className="result-tab-list">
          <Tabs.Tab value={PlaygroundOutputTab.Matches}>
            Matches
            <span>{result?.matches.length ?? 0}</span>
          </Tabs.Tab>
          <Tabs.Tab value={PlaygroundOutputTab.RawJson}>Raw JSON</Tabs.Tab>
        </Tabs.List>
        <Tabs.Panel
          className="result-tab-panel"
          value={PlaygroundOutputTab.Matches}
        >
          <MatchList input={input} result={result} />
        </Tabs.Panel>
        <Tabs.Panel
          className="result-tab-panel raw-json-panel"
          value={PlaygroundOutputTab.RawJson}
        >
          <pre>
            {activeTab === PlaygroundOutputTab.RawJson
              ? JSON.stringify(rawOutput, null, 2)
              : null}
          </pre>
        </Tabs.Panel>
      </Tabs.Root>
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

function selectedOptionDescription<Value extends string>(
  options: ReadonlyArray<{
    readonly description: string;
    readonly value: Value;
  }>,
  value: Value,
): string {
  return options.find((option) => option.value === value)?.description ?? '';
}

function componentResourceButtonLabel(state: ComponentResourceState): string {
  if (state === ComponentResourceState.Checking) {
    return '브라우저 저장소 확인 중';
  }

  if (state === ComponentResourceState.Loading) {
    return 'Component asset 불러오는 중';
  }

  return state === ComponentResourceState.Ready
    ? 'Component asset 준비됨'
    : 'Component asset 불러오기';
}

function isPlaygroundOutputTab(value: unknown): value is PlaygroundOutputTab {
  return (
    value === PlaygroundOutputTab.Matches ||
    value === PlaygroundOutputTab.RawJson
  );
}
