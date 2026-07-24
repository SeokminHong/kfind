import type { Match } from '../../kfind-wasm';
import type {
  ComponentResourceStatus,
  PlaygroundController,
  PlaygroundInput,
  PlaygroundResult,
} from '../../playground';

import type { MatchRevealRequest } from './match-list';
import type { SearchEditorHandle } from './search-editor';

import { Button } from '@base-ui/react/button';
import { Field } from '@base-ui/react/field';
import { Input } from '@base-ui/react/input';
import { Tabs } from '@base-ui/react/tabs';
import { useEffect, useRef, useState } from 'react';

import { DocumentLocale, useDocumentLocale } from '../../app/i18n';
import { createDocumentMeta } from '../../app/metadata';
import { RoutePath } from '../../app/navigation';
import { DocumentPage, PageIntro } from '../../components/document';
import { Modal } from '../../components/modal';
import { BoundaryPolicy, ExpandMode, PartOfSpeech } from '../../kfind-wasm';
import {
  applyPlaygroundPreset,
  ComponentResourceState,
  createInitialComponentResourceStatus,
  createInitialPlaygroundStatus,
  initializePlayground,
  initialPlaygroundInput,
  playgroundPresetOptions,
  PlaygroundResultState,
  PlaygroundState,
} from '../../playground';

import { MatchList } from './match-list';
import { QueryField } from './query-field';
import { SearchEditor } from './search-editor';
import { SelectField } from './select-field';

export const meta = createDocumentMeta(RoutePath.Playground);

const playgroundCopy = {
  [DocumentLocale.Korean]: {
    close: '닫기',
    compact: {
      checking: '저장소 확인 중',
      error: '리소스 오류',
      loading: '불러오는 중',
      needed: '리소스 필요',
      ready: '리소스 사용 가능',
    },
    empty: '옵션을 바꾸거나 다른 검색 질의를 사용해 보세요.',
    errorSummary: '검색 질의 컴파일 또는 검색 실행에 실패했습니다.',
    eyebrow: '실행 · WEBASSEMBLY',
    gapDescription: '구 atom 사이에 허용할 최대 Unicode 문자 수입니다.',
    gapLabel: '구 최대 간격',
    intro:
      '현재 source에서 빌드한 kfind-wasm을 사용합니다. 검색 질의와 원문은 브라우저 안에서만 처리합니다.',
    loadingResult: '검색을 실행하고 있습니다.',
    matchLabel: (surface: string) => `${surface} 일치를 편집기에서 보기`,
    options: '검색 옵션',
    optionsDescription: '변경한 설정은 검색에 바로 반영됩니다.',
    output: '결과 · compile + scan',
    pending: '검색 결과를 갱신하고 있습니다.',
    presetDescription: '검색 질의·원문·검색 설정 전체 적용',
    presetError: '공개 corpus를 불러오지 못했습니다.',
    presetHeading: '예시',
    presetLoading: '한국어 위키백과 corpus를 불러오는 중…',
    presetManifest: '추출 정보',
    presetSource: '대용량 본문',
    presetSourceName: '한국어 위키백과 2023-11-01',
    resourceDescription:
      '기본 WASM은 embedded lexicon을 포함합니다. smart 판정에 세부 품사 구성 요소가 필요하면 사용자가 고급 리소스를 불러옵니다. 같은 origin의 Pages Function이 R2 객체를 streaming하고, 엔진은 schema와 checksum을 검증한 뒤 resource revision별로 브라우저 저장소에 보관합니다. 리소스가 필요 없는 검색 질의는 network 요청을 하지 않습니다.',
    resourceHeading: '고급 smart 리소스',
    resourceReady: '사용 가능',
    sectionDescription:
      '검색 질의, 원문이나 옵션을 바꾸면 embedded lexicon으로 검색 계획을 다시 컴파일합니다. 일치한 span과 각 branch의 provenance를 함께 확인할 수 있습니다.',
    settingsDescription: '변경 후 250ms 뒤 자동 적용',
    settingsHeading: '검색 설정',
    title: '플레이그라운드',
    workspace: '검색 작업',
  },
  [DocumentLocale.English]: {
    close: 'Close',
    compact: {
      checking: 'Checking storage',
      error: 'Resource error',
      loading: 'Loading',
      needed: 'Resource required',
      ready: 'Resource available',
    },
    empty: 'Change the options or try another query.',
    errorSummary: 'Query compilation or search execution failed.',
    eyebrow: 'LIVE · WEBASSEMBLY',
    gapDescription: 'Maximum Unicode characters allowed between phrase atoms.',
    gapLabel: 'Maximum phrase gap',
    intro:
      'This page runs kfind-wasm built from the current source. The query and source text remain inside the browser.',
    loadingResult: 'Running the search.',
    matchLabel: (surface: string) =>
      `Reveal the ${surface} match in the editor`,
    options: 'Search options',
    optionsDescription: 'Changes apply to the search immediately.',
    output: 'Result · compile + scan',
    pending: 'Refreshing search results.',
    presetDescription: 'Apply query, text, and search settings',
    presetError: 'Could not load the public corpus.',
    presetHeading: 'Examples',
    presetLoading: 'Loading the Korean Wikipedia corpus…',
    presetManifest: 'Extraction manifest',
    presetSource: 'Large input source',
    presetSourceName: 'Korean Wikipedia · 2023-11-01',
    resourceDescription:
      'The base WASM includes the embedded lexicon. When a smart decision requires fine-POS components, the user loads the advanced resource. A same-origin Pages Function streams the R2 object; the engine validates its schema and checksum and stores it by resource revision. Queries that do not need the resource make no network request.',
    resourceHeading: 'Advanced smart resource',
    resourceReady: 'Available',
    sectionDescription:
      'Changing the query, source text, or options recompiles the query plan with the embedded lexicon. Matching spans and branch provenance are shown together.',
    settingsDescription: 'Applies automatically after 250 ms',
    settingsHeading: 'Search settings',
    title: 'Playground',
    workspace: 'Search workspace',
  },
} as const;

function partOfSpeechOptions(locale: DocumentLocale) {
  const labels =
    locale === DocumentLocale.Korean
      ? [
          '자동',
          '명사',
          '대명사',
          '수사',
          '동사',
          '형용사',
          '관형사',
          '부사',
          '조사',
          '감탄사',
          'Literal',
        ]
      : [
          'Auto',
          'Noun',
          'Pronoun',
          'Numeral',
          'Verb',
          'Adjective',
          'Determiner',
          'Adverb',
          'Particle',
          'Interjection',
          'Literal',
        ];
  const values = [
    PartOfSpeech.Auto,
    PartOfSpeech.Noun,
    PartOfSpeech.Pronoun,
    PartOfSpeech.Numeral,
    PartOfSpeech.Verb,
    PartOfSpeech.Adjective,
    PartOfSpeech.Determiner,
    PartOfSpeech.Adverb,
    PartOfSpeech.Particle,
    PartOfSpeech.Interjection,
    PartOfSpeech.Literal,
  ] as const;

  return values.map((value, index) => ({
    label: labels[index] ?? value,
    value,
  }));
}

function boundaryOptions(locale: DocumentLocale) {
  const descriptions =
    locale === DocumentLocale.Korean
      ? [
          '품사별 형태 검증 후 완성된 token 경계를 확인합니다.',
          'core 시작과 완성된 token 양쪽 경계를 확인합니다.',
          '좌우 경계 없이 부분 문자열 후보까지 보존합니다.',
        ]
      : [
          'Verifies POS-specific morphology and the completed token boundary.',
          'Requires boundaries at the core start and completed token end.',
          'Preserves substring candidates without left or right boundaries.',
        ];

  return [BoundaryPolicy.Smart, BoundaryPolicy.Token, BoundaryPolicy.Any].map(
    (value, index) => ({
      label: value,
      value,
      description: descriptions[index] ?? '',
    }),
  );
}

function expandOptions(locale: DocumentLocale) {
  const descriptions =
    locale === DocumentLocale.Korean
      ? [
          '품사를 유지하며 조사·어미 결합과 불규칙 활용을 찾습니다.',
          '활용과 생산적 파생형을 함께 찾습니다.',
          '형태 분석 없이 입력 문자열만 찾습니다.',
        ]
      : [
          'Finds particles, endings, and irregular conjugation without changing POS.',
          'Finds inflections and productive derived forms.',
          'Finds only the input string without morphology.',
        ];

  return [ExpandMode.Inflection, ExpandMode.Derivation, ExpandMode.Literal].map(
    (value, index) => ({
      label: value,
      value,
      description: descriptions[index] ?? '',
    }),
  );
}

function presetOptions(locale: DocumentLocale) {
  const labels =
    locale === DocumentLocale.Korean
      ? [
          '용언 활용 · smart',
          '구 검색 · any',
          '형태 구성 요소 · smart',
          'Literal 검색',
          '한국어 위키백과 1 MiB · smart',
        ]
      : [
          'Predicate inflection · smart',
          'Phrase search · any',
          'Morphological component · smart',
          'Literal search',
          'Korean Wikipedia 1 MiB · smart',
        ];

  return playgroundPresetOptions.map((preset, index) => ({
    label: labels[index] ?? preset.label,
    value: preset.value,
  }));
}

enum PlaygroundOutputTab {
  Matches = 'matches',
  RawJson = 'raw-json',
}

export default function PlaygroundPage(): React.JSX.Element {
  const locale = useDocumentLocale();
  const copy = playgroundCopy[locale];
  const controllerRef = useRef<PlaygroundController>(null);
  const searchEditorRef = useRef<SearchEditorHandle>(null);
  const matchRevealSequenceRef = useRef(0);
  const presetRequestSequenceRef = useRef(0);
  const [activeOutputTab, setActiveOutputTab] = useState(
    PlaygroundOutputTab.Matches,
  );
  const [input, setInput] = useState(initialPlaygroundInput);
  const [isOptionsModalOpen, setIsOptionsModalOpen] = useState(false);
  const [isPresetLoading, setIsPresetLoading] = useState(false);
  const [matchRevealRequest, setMatchRevealRequest] =
    useState<MatchRevealRequest>();
  const [status, setStatus] = useState(() =>
    createInitialPlaygroundStatus(locale),
  );
  const [resourceStatus, setResourceStatus] = useState(() =>
    createInitialComponentResourceStatus(locale),
  );
  const [presetError, setPresetError] = useState<string>();
  const [result, setResult] = useState<PlaygroundResult>();

  useEffect(() => {
    const controller = initializePlayground(
      initialPlaygroundInput,
      {
        onResourceStatusChange: setResourceStatus,
        onResult: setResult,
        onStatusChange: setStatus,
      },
      locale,
    );
    controllerRef.current = controller;

    return () => {
      controllerRef.current = null;
      controller.dispose();
    };
  }, [locale]);

  useEffect(() => {
    controllerRef.current?.scheduleRun(input);
  }, [input]);

  function updateInput<Key extends keyof PlaygroundInput>(
    key: Key,
    value: PlaygroundInput[Key],
  ): void {
    setInput((current) => ({ ...current, [key]: value }));
  }

  async function applyPreset(
    preset: (typeof playgroundPresetOptions)[number]['value'],
  ): Promise<void> {
    presetRequestSequenceRef.current += 1;
    const requestSequence = presetRequestSequenceRef.current;
    setIsPresetLoading(true);
    setPresetError(undefined);

    try {
      const nextInput = await applyPlaygroundPreset(preset);

      if (presetRequestSequenceRef.current === requestSequence) {
        setInput(nextInput);
      }
    } catch (error) {
      if (presetRequestSequenceRef.current === requestSequence) {
        setPresetError(
          `${copy.presetError} ${error instanceof Error ? error.message : String(error)}`,
        );
      }
    } finally {
      if (presetRequestSequenceRef.current === requestSequence) {
        setIsPresetLoading(false);
      }
    }
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
  const compactResourceStatus = componentResourceCompactStatus(
    resourceStatus.state,
    locale,
  );

  return (
    <DocumentPage>
      <PageIntro eyebrow={copy.eyebrow} title={copy.title}>
        <p>{copy.intro}</p>
      </PageIntro>

      <section aria-label={copy.workspace} className="doc-section">
        <div className="section-title-row">
          <p>{copy.sectionDescription}</p>
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
          <div className="playground-workspace">
            <div className="playground-main-inputs">
              <QueryField
                locale={locale}
                onValueChange={(value) => {
                  updateInput('query', value);
                }}
                value={input.query}
              />

              <SearchEditor
                locale={locale}
                ref={searchEditorRef}
                matches={currentMatches}
                onMatchActivate={(_match, index) => {
                  matchRevealSequenceRef.current += 1;
                  setActiveOutputTab(PlaygroundOutputTab.Matches);
                  setMatchRevealRequest({
                    index,
                    sequence: matchRevealSequenceRef.current,
                  });
                }}
                onValueChange={(value) => {
                  updateInput('text', value);
                }}
                value={input.text}
              />
            </div>

            <aside className="desktop-settings" aria-label={copy.options}>
              <PlaygroundSettings
                idPrefix="desktop"
                input={input}
                isPresetLoading={isPresetLoading}
                isResourceButtonDisabled={isResourceButtonDisabled}
                locale={locale}
                onInputChange={updateInput}
                onLoadResource={() => {
                  controllerRef.current?.loadComponentResource();
                }}
                onPresetApply={applyPreset}
                presetError={presetError}
                resourceStatus={resourceStatus}
              />
            </aside>
          </div>

          <div className="mobile-settings">
            <Modal
              onOpenChange={setIsOptionsModalOpen}
              open={isOptionsModalOpen}
            >
              <Modal.Trigger data-glossary-skip="">
                <span className="mobile-settings-heading">
                  <span>{copy.options}</span>
                  {compactResourceStatus === undefined ? null : (
                    <small
                      className="mobile-resource-state"
                      data-state={resourceStatus.state}
                    >
                      {compactResourceStatus}
                    </small>
                  )}
                </span>
                <small className="mobile-settings-summary">
                  {formatSettingsSummary(input, locale)}
                </small>
              </Modal.Trigger>
              <Modal.Content>
                <Modal.Section>
                  <div className="options-modal-heading">
                    <div>
                      <Modal.Title>{copy.options}</Modal.Title>
                      <Modal.Description>
                        {copy.optionsDescription}
                      </Modal.Description>
                    </div>
                    <Modal.Close aria-label={copy.close} data-glossary-skip="">
                      <svg aria-hidden="true" viewBox="0 0 16 16">
                        <path d="m3.5 3.5 9 9m0-9-9 9" />
                      </svg>
                    </Modal.Close>
                  </div>
                </Modal.Section>
                <Modal.Section>
                  <PlaygroundSettings
                    idPrefix="mobile"
                    input={input}
                    isPresetLoading={isPresetLoading}
                    isResourceButtonDisabled={isResourceButtonDisabled}
                    locale={locale}
                    onInputChange={updateInput}
                    onLoadResource={() => {
                      controllerRef.current?.loadComponentResource();
                    }}
                    onPresetApply={applyPreset}
                    presetError={presetError}
                    resourceStatus={resourceStatus}
                  />
                </Modal.Section>
              </Modal.Content>
            </Modal>
          </div>

          <PlaygroundOutput
            activeTab={activeOutputTab}
            input={input}
            locale={locale}
            onActiveTabChange={setActiveOutputTab}
            onMatchActivate={(match) => {
              searchEditorRef.current?.revealMatch(match);
            }}
            revealRequest={matchRevealRequest}
            result={currentResult}
          />
        </div>

        <p>{copy.resourceDescription}</p>
      </section>
    </DocumentPage>
  );
}

interface PlaygroundSettingsProps {
  readonly idPrefix: string;
  readonly input: PlaygroundInput;
  readonly isPresetLoading: boolean;
  readonly isResourceButtonDisabled: boolean;
  readonly locale: DocumentLocale;
  readonly onInputChange: <Key extends keyof PlaygroundInput>(
    key: Key,
    value: PlaygroundInput[Key],
  ) => void;
  readonly onLoadResource: () => void;
  readonly onPresetApply: (
    preset: (typeof playgroundPresetOptions)[number]['value'],
  ) => Promise<void>;
  readonly presetError: string | undefined;
  readonly resourceStatus: ComponentResourceStatus;
}

function PlaygroundSettings({
  idPrefix,
  input,
  isPresetLoading,
  isResourceButtonDisabled,
  locale,
  onInputChange,
  onLoadResource,
  onPresetApply,
  presetError,
  resourceStatus,
}: PlaygroundSettingsProps): React.JSX.Element {
  const copy = playgroundCopy[locale];
  const localizedPartOfSpeechOptions = partOfSpeechOptions(locale);
  const localizedBoundaryOptions = boundaryOptions(locale);
  const localizedExpandOptions = expandOptions(locale);

  return (
    <div className="playground-settings">
      <div className="preset-picker">
        <div className="control-heading">
          <strong>{copy.presetHeading}</strong>
          <span>{copy.presetDescription}</span>
        </div>
        <div className="preset-actions">
          {presetOptions(locale).map((preset) => (
            <Button
              data-glossary-skip=""
              disabled={isPresetLoading}
              key={preset.value}
              onClick={() => {
                void onPresetApply(preset.value);
              }}
              type="button"
            >
              {preset.label}
            </Button>
          ))}
        </div>
        <p className="preset-source">
          <span>
            {isPresetLoading ? copy.presetLoading : copy.presetSource}
          </span>
          {' · '}
          <a href="https://huggingface.co/datasets/wikimedia/wikipedia">
            {copy.presetSourceName}
          </a>
          {' · '}
          <a href="https://creativecommons.org/licenses/by-sa/3.0/">
            CC BY-SA 3.0
          </a>
          {' · '}
          <a href="/playground/korean-wikipedia-20231101-ko-1mib.sources.json">
            {copy.presetManifest}
          </a>
        </p>
        {presetError === undefined ? null : (
          <p className="preset-error" role="alert">
            {presetError}
          </p>
        )}
      </div>

      <div className="option-panel">
        <div className="control-heading">
          <strong>{copy.settingsHeading}</strong>
          <span>{copy.settingsDescription}</span>
        </div>
        <div className="option-grid">
          <SelectField<PartOfSpeech>
            description={
              locale === DocumentLocale.Korean
                ? 'Atom 태그와 전역 POS가 다르면 compile 오류입니다.'
                : 'A conflicting atom tag and global POS is a compile error.'
            }
            id={`${idPrefix}-pos-select`}
            label={locale === DocumentLocale.Korean ? '품사' : 'Part of speech'}
            name={`${idPrefix}-pos`}
            onValueChange={(value) => {
              onInputChange('pos', value);
            }}
            options={localizedPartOfSpeechOptions}
            value={input.pos}
          />
          <SelectField<BoundaryPolicy>
            description={selectedOptionDescription(
              localizedBoundaryOptions,
              input.boundary,
            )}
            id={`${idPrefix}-boundary-select`}
            label={locale === DocumentLocale.Korean ? '경계' : 'Boundary'}
            name={`${idPrefix}-boundary`}
            onValueChange={(value) => {
              onInputChange('boundary', value);
            }}
            options={localizedBoundaryOptions}
            value={input.boundary}
          />
          <SelectField<ExpandMode>
            description={selectedOptionDescription(
              localizedExpandOptions,
              input.expand,
            )}
            id={`${idPrefix}-expand-select`}
            label={locale === DocumentLocale.Korean ? '확장' : 'Expansion'}
            name={`${idPrefix}-expand`}
            onValueChange={(value) => {
              onInputChange('expand', value);
            }}
            options={localizedExpandOptions}
            value={input.expand}
          />
          <Field.Root className="field" name={`${idPrefix}-max-gap`}>
            <Field.Label data-glossary-skip="">{copy.gapLabel}</Field.Label>
            <Input
              className="text-control"
              min="0"
              onValueChange={(value) => {
                onInputChange('maxGap', value);
              }}
              type="number"
              value={input.maxGap}
            />
            <Field.Description>{copy.gapDescription}</Field.Description>
          </Field.Root>
        </div>
      </div>

      <div
        className="resource-loader"
        data-state={resourceStatus.state}
        role="status"
        aria-live="polite"
      >
        <div>
          <span className="resource-dot" aria-hidden="true" />
          <div>
            <strong>{copy.resourceHeading}</strong>
            <span>{resourceStatus.message}</span>
          </div>
        </div>
        {resourceStatus.state === ComponentResourceState.Ready ? (
          <span className="resource-ready-label">{copy.resourceReady}</span>
        ) : (
          <Button
            data-glossary-skip=""
            disabled={isResourceButtonDisabled}
            onClick={onLoadResource}
            type="button"
          >
            {componentResourceButtonLabel(resourceStatus.state, locale)}
          </Button>
        )}
      </div>
    </div>
  );
}

interface PlaygroundOutputProps {
  readonly activeTab: PlaygroundOutputTab;
  readonly input: PlaygroundInput;
  readonly locale: DocumentLocale;
  readonly onActiveTabChange: (tab: PlaygroundOutputTab) => void;
  readonly onMatchActivate: (match: Match) => void;
  readonly revealRequest: MatchRevealRequest | undefined;
  readonly result: PlaygroundResult | undefined;
}

function PlaygroundOutput({
  activeTab,
  input,
  locale,
  onActiveTabChange,
  onMatchActivate,
  revealRequest,
  result,
}: PlaygroundOutputProps): React.JSX.Element {
  const copy = playgroundCopy[locale];
  const isPending = result === undefined;
  const summary = resultSummary(result, locale);
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
          <p className="output-label">{copy.output}</p>
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
            onActiveTabChange(value);
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
          <MatchList
            active={activeTab === PlaygroundOutputTab.Matches}
            emptyLabel={copy.empty}
            input={input}
            loadingLabel={copy.loadingResult}
            locale={locale}
            matchLabel={copy.matchLabel}
            onMatchActivate={onMatchActivate}
            revealRequest={revealRequest}
            result={result}
          />
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

function resultSummary(
  result: PlaygroundResult | undefined,
  locale: DocumentLocale,
): string {
  const copy = playgroundCopy[locale];

  if (result === undefined) {
    return copy.pending;
  }

  return result.state === PlaygroundResultState.Error
    ? copy.errorSummary
    : result.message;
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

function formatSettingsSummary(
  input: PlaygroundInput,
  locale: DocumentLocale,
): string {
  const partOfSpeech =
    partOfSpeechOptions(locale).find((option) => option.value === input.pos)
      ?.label ?? input.pos;

  return `${partOfSpeech} · ${input.boundary} · ${input.expand}`;
}

function componentResourceButtonLabel(
  state: ComponentResourceState,
  locale: DocumentLocale,
): string {
  if (state === ComponentResourceState.Checking) {
    return locale === DocumentLocale.Korean
      ? '브라우저 저장소 확인 중'
      : 'Checking browser storage';
  }

  if (state === ComponentResourceState.Loading) {
    return locale === DocumentLocale.Korean
      ? '구성 요소 asset 로드 중'
      : 'Loading component asset';
  }

  if (state === ComponentResourceState.Ready) {
    return locale === DocumentLocale.Korean
      ? '구성 요소 asset 준비됨'
      : 'Component asset ready';
  }

  return locale === DocumentLocale.Korean
    ? '구성 요소 asset 불러오기'
    : 'Load component asset';
}

function componentResourceCompactStatus(
  state: ComponentResourceState,
  locale: DocumentLocale,
): string | undefined {
  const compact = playgroundCopy[locale].compact;

  if (state === ComponentResourceState.Checking) {
    return compact.checking;
  }

  if (state === ComponentResourceState.Needed) {
    return compact.needed;
  }

  if (state === ComponentResourceState.Loading) {
    return compact.loading;
  }

  if (state === ComponentResourceState.Ready) {
    return compact.ready;
  }

  return state === ComponentResourceState.Error ? compact.error : undefined;
}

function isPlaygroundOutputTab(value: unknown): value is PlaygroundOutputTab {
  return (
    value === PlaygroundOutputTab.Matches ||
    value === PlaygroundOutputTab.RawJson
  );
}
