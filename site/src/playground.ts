import type { CompileOptions, KfindEngine, Match } from './kfind-wasm';

import {
  BoundaryPolicy,
  componentResourceVersion,
  ExpandMode,
  findMatches,
  loadComponentResource,
  loadKfind,
  NormalizationMode,
  PartOfSpeech,
  restoreComponentResource,
} from './kfind-wasm';

export enum PlaygroundState {
  Loading = 'loading',
  Ready = 'ready',
  Error = 'error',
}

export enum PlaygroundResultState {
  Success = 'success',
  EmptyQuery = 'empty-query',
  Error = 'error',
}

export enum ComponentResourceState {
  Checking = 'checking',
  Idle = 'idle',
  Needed = 'needed',
  Loading = 'loading',
  Ready = 'ready',
  Error = 'error',
}

export enum PlaygroundPresetName {
  Predicate = 'predicate',
  Phrase = 'phrase',
  Component = 'component',
  Literal = 'literal',
  LargeInput = 'large-input',
}

export interface PlaygroundInput {
  readonly boundary: BoundaryPolicy;
  readonly expand: ExpandMode;
  readonly maxGap: string;
  readonly pos: PartOfSpeech;
  readonly query: string;
  readonly text: string;
}

export interface PlaygroundStatus {
  readonly message: string;
  readonly state: PlaygroundState;
}

export interface ComponentResourceStatus {
  readonly message: string;
  readonly state: ComponentResourceState;
}

export interface PlaygroundResult {
  readonly elapsedMilliseconds: number | null;
  readonly input: PlaygroundInput;
  readonly matches: readonly Match[];
  readonly message: string;
  readonly state: PlaygroundResultState;
}

export interface PlaygroundController {
  readonly dispose: () => void;
  readonly loadComponentResource: () => void;
  readonly scheduleRun: (input: PlaygroundInput) => void;
}

interface PlaygroundCallbacks {
  readonly onResourceStatusChange: (status: ComponentResourceStatus) => void;
  readonly onResult: (result: PlaygroundResult) => void;
  readonly onStatusChange: (status: PlaygroundStatus) => void;
}

interface PresetDefinition {
  readonly boundary: BoundaryPolicy;
  readonly expand: ExpandMode;
  readonly maxGap: string;
  readonly pos: PartOfSpeech;
  readonly query: string;
  readonly text: string | (() => string);
}

const SEARCH_DEBOUNCE_MILLISECONDS = 250;
const LARGE_INPUT_BYTE_LENGTH = 1024 * 1024;
const LARGE_INPUT_HEADER =
  '기준표식은 대용량 입력의 전체 scan 시간을 확인하기 위해 한 번만 등장합니다.\n';
const LARGE_INPUT_FILLER =
  '2000-01-01T00:00:00Z level=info request=00000000 status=ok latency=12ms\n';

let cachedLargeInput: string | undefined;

const presets: Readonly<Record<PlaygroundPresetName, PresetDefinition>> = {
  [PlaygroundPresetName.Predicate]: {
    query: '걷다',
    text: '오늘은 공원을 걸었다.\n내일도 천천히 걷고 싶다.\n산책길을 걷는 사람을 만났다.',
    pos: PartOfSpeech.Verb,
    boundary: BoundaryPolicy.Smart,
    expand: ExpandMode.Inflection,
    maxGap: '24',
  },
  [PlaygroundPresetName.Phrase]: {
    query: 'n:사용자 v:검증하다',
    text: '에이전트가 결과를 만들면 사용자가 문맥을 다시 검증했습니다.\n사용자 권한만 확인했습니다.',
    pos: PartOfSpeech.Auto,
    boundary: BoundaryPolicy.Any,
    expand: ExpandMode.Inflection,
    maxGap: '24',
  },
  [PlaygroundPresetName.Component]: {
    query: 'n:요리',
    text: '중국요리를 만드는 법을 정리했다.\n요리 도구도 함께 준비했다.\n요리사라는 직업도 있다.',
    pos: PartOfSpeech.Auto,
    boundary: BoundaryPolicy.Smart,
    expand: ExpandMode.Inflection,
    maxGap: '24',
  },
  [PlaygroundPresetName.Literal]: {
    query: '걸어',
    text: '길을 걸어 갔다.\n그는 걷다가 멈췄다.\n걸어라는 문자열만 그대로 찾는다.',
    pos: PartOfSpeech.Literal,
    boundary: BoundaryPolicy.Smart,
    expand: ExpandMode.Literal,
    maxGap: '24',
  },
  [PlaygroundPresetName.LargeInput]: {
    query: '기준표식',
    text: createLargeInput,
    pos: PartOfSpeech.Literal,
    boundary: BoundaryPolicy.Any,
    expand: ExpandMode.Literal,
    maxGap: '24',
  },
};

export const playgroundPresetOptions = [
  { label: '용언 활용 · smart', value: PlaygroundPresetName.Predicate },
  { label: '구(句) 검색 · any', value: PlaygroundPresetName.Phrase },
  {
    label: '형태 component · smart',
    value: PlaygroundPresetName.Component,
  },
  { label: 'Literal 검색', value: PlaygroundPresetName.Literal },
  { label: '대용량 1 MiB · literal', value: PlaygroundPresetName.LargeInput },
] as const;

export const initialPlaygroundInput = createPresetInput(
  PlaygroundPresetName.Predicate,
);

export const initialPlaygroundStatus: PlaygroundStatus = {
  state: PlaygroundState.Loading,
  message: 'WASM engine을 불러오는 중…',
};

export const initialComponentResourceStatus: ComponentResourceStatus = {
  state: ComponentResourceState.Checking,
  message: `저장된 resource 확인 중 · ${formatResourceVersion()}`,
};

const idleComponentResourceStatus: ComponentResourceStatus = {
  state: ComponentResourceState.Idle,
  message: `필요한 경우 R2에서 35.4 MiB를 받습니다 · ${formatResourceVersion()}`,
};

export function applyPlaygroundPreset(
  presetName: PlaygroundPresetName,
): PlaygroundInput {
  return createPresetInput(presetName);
}

export function initializePlayground(
  initialInput: PlaygroundInput,
  callbacks: PlaygroundCallbacks,
): PlaygroundController {
  const abortController = new AbortController();
  const { signal } = abortController;
  let engine: KfindEngine | undefined;
  let latestInput = initialInput;
  let pendingRun: ReturnType<typeof globalThis.setTimeout> | undefined;
  let resourceState = initialComponentResourceStatus.state;
  let resourceCheckComplete = false;

  const setResourceStatus = (status: ComponentResourceStatus): void => {
    resourceState = status.state;
    callbacks.onResourceStatusChange(status);
  };

  const execute = (): void => {
    if (engine === undefined || signal.aborted || !resourceCheckComplete) {
      return;
    }

    const result = executeSearch(engine, latestInput);

    if (
      result.state === PlaygroundResultState.Error &&
      result.message.toLowerCase().includes('component') &&
      resourceState === ComponentResourceState.Idle
    ) {
      setResourceStatus({
        state: ComponentResourceState.Needed,
        message: '이 query를 실행하려면 component asset이 필요합니다.',
      });
    } else if (resourceState === ComponentResourceState.Needed) {
      setResourceStatus(idleComponentResourceStatus);
    }

    callbacks.onResult(result);
  };

  const scheduleRun = (input: PlaygroundInput): void => {
    latestInput = input;
    globalThis.clearTimeout(pendingRun);

    if (engine === undefined) {
      return;
    }

    pendingRun = globalThis.setTimeout(execute, SEARCH_DEBOUNCE_MILLISECONDS);
  };

  callbacks.onStatusChange(initialPlaygroundStatus);
  callbacks.onResourceStatusChange(initialComponentResourceStatus);

  void loadKfind()
    .then(async (loaded) => {
      if (signal.aborted) {
        loaded.engine.free();
        return;
      }

      engine = loaded.engine;
      callbacks.onStatusChange({
        state: PlaygroundState.Ready,
        message: `WASM ready · embedded lexicon · ${loaded.loadMilliseconds.toFixed(0)} ms`,
      });

      try {
        const restoredByteLength = await restoreComponentResource(
          loaded.engine,
          signal,
        );

        if (isAborted(signal)) {
          return;
        }

        setResourceStatus(
          restoredByteLength === null
            ? idleComponentResourceStatus
            : {
                state: ComponentResourceState.Ready,
                message: `${formatMebibytes(restoredByteLength)} MiB 저장소에서 복원 완료 · ${formatResourceVersion()}`,
              },
        );
      } catch (error) {
        if (isAborted(signal)) {
          return;
        }

        setResourceStatus({
          state: ComponentResourceState.Error,
          message: `저장된 resource 검증 실패: ${readableError(error)}`,
        });
      }

      resourceCheckComplete = true;
      execute();
    })
    .catch((error: unknown) => {
      if (signal.aborted) {
        return;
      }

      const message = readableError(error);
      callbacks.onStatusChange({ state: PlaygroundState.Error, message });
      callbacks.onResult(createErrorResult(latestInput, message));
    });

  return {
    dispose() {
      abortController.abort();
      globalThis.clearTimeout(pendingRun);
      engine?.free();
    },
    loadComponentResource() {
      if (engine !== undefined) {
        void enableComponentResource(
          engine,
          setResourceStatus,
          execute,
          signal,
        );
      }
    },
    scheduleRun,
  };
}

export function mergeMatchSpans(
  matches: readonly Match[],
  textLength: number,
): ReadonlyArray<{ readonly start: number; readonly end: number }> {
  const sorted = matches
    .map((match) => ({
      start: Math.max(0, Math.min(textLength, match.start)),
      end: Math.max(0, Math.min(textLength, match.end)),
    }))
    .filter((span) => span.end > span.start)
    .sort((left, right) => {
      const startDifference = left.start - right.start;
      return startDifference === 0 ? left.end - right.end : startDifference;
    });
  const merged: Array<{ start: number; end: number }> = [];

  for (const span of sorted) {
    const previous = merged[merged.length - 1];

    if (previous !== undefined && span.start <= previous.end) {
      previous.end = Math.max(previous.end, span.end);
    } else {
      merged.push({ ...span });
    }
  }

  return merged;
}

export function formatProvenance(match: Match): string {
  const paths = new Set<string>();

  for (const atom of match.atoms) {
    for (const origin of atom.origins) {
      paths.add(
        origin.rulePath.length === 0 ? 'direct' : origin.rulePath.join(' → '),
      );
    }
  }

  return paths.size === 0 ? 'direct match 검증 완료' : [...paths].join(' · ');
}

async function enableComponentResource(
  engine: KfindEngine,
  setResourceStatus: (status: ComponentResourceStatus) => void,
  rerun: () => void,
  signal: AbortSignal,
): Promise<void> {
  if (engine.componentResourceLoaded) {
    rerun();
    return;
  }

  setResourceStatus({
    state: ComponentResourceState.Loading,
    message: 'R2에서 component asset을 불러오는 중…',
  });

  try {
    const loaded = await loadComponentResource(engine, signal);
    if (signal.aborted) {
      return;
    }

    setResourceStatus({
      state: ComponentResourceState.Ready,
      message: loaded.stored
        ? `${formatMebibytes(loaded.byteLength)} MiB 불러오기·검증·저장 완료 · ${formatResourceVersion()}`
        : `${formatMebibytes(loaded.byteLength)} MiB 불러오기·검증 완료 · 저장소 미지원`,
    });
    rerun();
  } catch (error) {
    if (signal.aborted) {
      return;
    }

    setResourceStatus({
      state: ComponentResourceState.Error,
      message: readableError(error),
    });
  }
}

function executeSearch(
  engine: KfindEngine,
  input: PlaygroundInput,
): PlaygroundResult {
  const query = input.query.trim();

  if (query.length === 0) {
    return {
      state: PlaygroundResultState.EmptyQuery,
      input,
      matches: [],
      elapsedMilliseconds: null,
      message: '쿼리를 입력해 주세요.',
    };
  }

  try {
    const options = readOptions(input);
    const startedAt = performance.now();
    const matches = findMatches(engine, query, input.text, options);
    const elapsedMilliseconds = performance.now() - startedAt;

    return {
      state: PlaygroundResultState.Success,
      input,
      matches,
      elapsedMilliseconds,
      message:
        matches.length === 0
          ? '일치하는 span이 없습니다.'
          : `일치하는 span ${matches.length}개를 찾았습니다.`,
    };
  } catch (error) {
    const message = readableError(error);

    return createErrorResult(input, message);
  }
}

function createErrorResult(
  input: PlaygroundInput,
  message: string,
): PlaygroundResult {
  return {
    state: PlaygroundResultState.Error,
    input,
    matches: [],
    elapsedMilliseconds: null,
    message,
  };
}

function createPresetInput(presetName: PlaygroundPresetName): PlaygroundInput {
  const preset = presets[presetName];

  return {
    boundary: preset.boundary,
    expand: preset.expand,
    maxGap: preset.maxGap,
    pos: preset.pos,
    query: preset.query,
    text: typeof preset.text === 'function' ? preset.text() : preset.text,
  };
}

function createLargeInput(): string {
  if (cachedLargeInput !== undefined) {
    return cachedLargeInput;
  }

  const encoder = new TextEncoder();
  const headerByteLength = encoder.encode(LARGE_INPUT_HEADER).byteLength;
  const fillerByteLength = encoder.encode(LARGE_INPUT_FILLER).byteLength;
  const fillerCount = Math.floor(
    (LARGE_INPUT_BYTE_LENGTH - headerByteLength) / fillerByteLength,
  );
  const paddingLength =
    LARGE_INPUT_BYTE_LENGTH - headerByteLength - fillerByteLength * fillerCount;

  cachedLargeInput =
    LARGE_INPUT_HEADER +
    LARGE_INPUT_FILLER.repeat(fillerCount) +
    ' '.repeat(paddingLength);
  return cachedLargeInput;
}

function readOptions(input: PlaygroundInput): CompileOptions {
  const parsedMaxGap = Number.parseInt(input.maxGap, 10);

  return {
    pos: input.pos,
    boundary: input.boundary,
    expand: input.expand,
    normalization: NormalizationMode.Canonical,
    maxGap: Number.isNaN(parsedMaxGap) ? 0 : Math.max(0, parsedMaxGap),
  };
}

function readableError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function formatMebibytes(byteLength: number): string {
  return (byteLength / (1024 * 1024)).toFixed(1);
}

function formatResourceVersion(): string {
  return componentResourceVersion.startsWith('v')
    ? componentResourceVersion
    : componentResourceVersion.slice(0, 12);
}

function isAborted(signal: AbortSignal): boolean {
  return signal.aborted;
}
