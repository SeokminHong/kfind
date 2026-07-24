import type { CompileOptions, KfindEngine, Match } from './kfind-wasm';
import type { PlaygroundInput } from './playground-presets';

import { DocumentLocale } from './app/i18n';
import {
  componentResourceVersion,
  findMatches,
  loadComponentResource,
  loadKfind,
  NormalizationMode,
  PartOfSpeech,
  restoreComponentResource,
} from './kfind-wasm';

export {
  applyPlaygroundPreset,
  initialPlaygroundInput,
  playgroundPresetOptions,
  PlaygroundPresetName,
} from './playground-presets';
export type { PlaygroundInput } from './playground-presets';

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

interface PlaygroundMessages {
  readonly directMatch: string;
  readonly emptyQuery: string;
  readonly initialResource: string;
  readonly initialStatus: string;
  readonly matchCount: (count: number) => string;
  readonly noMatches: string;
  readonly resourceIdle: string;
  readonly resourceLoading: string;
  readonly resourceNeeded: string;
  readonly resourceRestored: (byteLength: number, migrated: boolean) => string;
  readonly resourceStored: (byteLength: number, stored: boolean) => string;
  readonly resourceVerificationFailed: (error: string) => string;
}

const SEARCH_DEBOUNCE_MILLISECONDS = 250;

const playgroundMessages: Readonly<Record<DocumentLocale, PlaygroundMessages>> =
  {
    [DocumentLocale.Korean]: {
      directMatch: '직접 일치 검증 완료',
      emptyQuery: '검색 질의를 입력해 주세요.',
      initialResource: `저장된 리소스 확인 중 · ${formatResourceVersion()}`,
      initialStatus: 'WASM 엔진을 불러오는 중…',
      matchCount: (count) => `일치하는 span ${count}개를 찾았습니다.`,
      noMatches: '일치하는 span이 없습니다.',
      resourceIdle: `필요할 때 R2에서 35.4 MiB를 받습니다 · ${formatResourceVersion()}`,
      resourceLoading: 'R2에서 형태 구성 요소 판정 리소스를 불러오는 중…',
      resourceNeeded:
        '이 검색 질의의 smart 구조 판정에는 형태 구성 요소 판정 리소스가 필요합니다.',
      resourceRestored: (byteLength, migrated) =>
        `${formatMebibytes(byteLength)} MiB ${migrated ? '저장소 복원 및 이전 완료' : '저장소 복원 완료'} · ${formatResourceVersion()}`,
      resourceStored: (byteLength, stored) =>
        stored
          ? `${formatMebibytes(byteLength)} MiB 로드·검증·저장 완료 · ${formatResourceVersion()}`
          : `${formatMebibytes(byteLength)} MiB 로드·검증 완료 · 저장소 미지원`,
      resourceVerificationFailed: (error) =>
        `저장된 리소스 검증 실패 · ${error}`,
    },
    [DocumentLocale.English]: {
      directMatch: 'Direct match verified',
      emptyQuery: 'Enter a query.',
      initialResource: `Checking stored resource · ${formatResourceVersion()}`,
      initialStatus: 'Loading the WASM engine…',
      matchCount: (count) =>
        `Found ${count.toLocaleString('en')} matching spans.`,
      noMatches: 'No matching spans.',
      resourceIdle: `Downloads 35.4 MiB from R2 when required · ${formatResourceVersion()}`,
      resourceLoading:
        'Loading the morphological component verification resource from R2…',
      resourceNeeded:
        'This query needs the morphological component verification resource for its smart structural decision.',
      resourceRestored: (byteLength, migrated) =>
        `${formatMebibytes(byteLength)} MiB ${migrated ? 'restored and migrated' : 'restored'} · ${formatResourceVersion()}`,
      resourceStored: (byteLength, stored) =>
        stored
          ? `${formatMebibytes(byteLength)} MiB loaded, verified, and stored · ${formatResourceVersion()}`
          : `${formatMebibytes(byteLength)} MiB loaded and verified · storage unavailable`,
      resourceVerificationFailed: (error) =>
        `Stored resource validation failed · ${error}`,
    },
  };

export function createInitialPlaygroundStatus(
  locale: DocumentLocale,
): PlaygroundStatus {
  return {
    state: PlaygroundState.Loading,
    message: playgroundMessages[locale].initialStatus,
  };
}

export function createInitialComponentResourceStatus(
  locale: DocumentLocale,
): ComponentResourceStatus {
  return {
    state: ComponentResourceState.Checking,
    message: playgroundMessages[locale].initialResource,
  };
}

export function initializePlayground(
  initialInput: PlaygroundInput,
  callbacks: PlaygroundCallbacks,
  locale: DocumentLocale = DocumentLocale.Korean,
): PlaygroundController {
  const messages = playgroundMessages[locale];
  const initialStatus = createInitialPlaygroundStatus(locale);
  const initialResourceStatus = createInitialComponentResourceStatus(locale);
  const idleResourceStatus: ComponentResourceStatus = {
    state: ComponentResourceState.Idle,
    message: messages.resourceIdle,
  };
  const abortController = new AbortController();
  const { signal } = abortController;
  let engine: KfindEngine | undefined;
  let latestInput = initialInput;
  let pendingRun: ReturnType<typeof globalThis.setTimeout> | undefined;
  let resourceState = initialResourceStatus.state;
  let resourceCheckComplete = false;

  const setResourceStatus = (status: ComponentResourceStatus): void => {
    resourceState = status.state;
    callbacks.onResourceStatusChange(status);
  };

  const execute = (): void => {
    if (engine === undefined || signal.aborted || !resourceCheckComplete) {
      return;
    }

    const result = executeSearch(engine, latestInput, messages);

    if (
      result.state === PlaygroundResultState.Error &&
      result.message.toLowerCase().includes('component') &&
      resourceState === ComponentResourceState.Idle
    ) {
      setResourceStatus({
        state: ComponentResourceState.Needed,
        message: messages.resourceNeeded,
      });
    } else if (resourceState === ComponentResourceState.Needed) {
      setResourceStatus(idleResourceStatus);
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

  callbacks.onStatusChange(initialStatus);
  callbacks.onResourceStatusChange(initialResourceStatus);

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
        const restoredResource = await restoreComponentResource(
          loaded.engine,
          signal,
        );

        if (isAborted(signal)) {
          return;
        }

        setResourceStatus(
          restoredResource === null
            ? idleResourceStatus
            : {
                state: ComponentResourceState.Ready,
                message: messages.resourceRestored(
                  restoredResource.byteLength,
                  restoredResource.migrated,
                ),
              },
        );
      } catch (error) {
        if (isAborted(signal)) {
          return;
        }

        setResourceStatus({
          state: ComponentResourceState.Error,
          message: messages.resourceVerificationFailed(readableError(error)),
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
          messages,
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

export function formatProvenance(
  match: Match,
  locale: DocumentLocale = DocumentLocale.Korean,
): string {
  const paths = new Set<string>();

  for (const atom of match.atoms) {
    for (const origin of atom.origins) {
      paths.add(
        origin.rulePath.length === 0 ? 'direct' : origin.rulePath.join(' → '),
      );
    }
  }

  return paths.size === 0
    ? playgroundMessages[locale].directMatch
    : [...paths].join(' · ');
}

const morphologyRuleNotations: Readonly<Record<string, string>> = {
  'contraction.ha-past': '하+였→했',
  'contraction.ha-yeo': '하여→해',
  'contraction.i-eo': '이+어→여',
  'contraction.o-a': '오+아→와',
  'contraction.oe-eo': '되+어→돼',
  'contraction.u-eo': '우+어→워',
  'contraction.yeo-eo': '여+어→여',
  'ending.adverbial-ge': '-게',
  'ending.aoeo': '-아/어',
  'ending.conditional': '-(으)면',
  'ending.connective-go': '-고',
  'ending.connective-ji': '-지',
  'ending.connective-jiman': '-지만',
  'ending.final-da': '-다',
  'ending.future-adnominal': '-(으)ㄹ',
  'ending.honorific': '-(으)시-',
  'ending.nominalizer': '-(으)ㅁ',
  'ending.nominalizer-gi': '-기',
  'ending.past': '-았/었-',
  'ending.past-adnominal': '-(으)ㄴ',
  'ending.polite-yo': '-요',
  'ending.present-adnominal': '-는',
  'lexical.b-to-wa': 'ㅂ→와',
  'lexical.b-to-wo': 'ㅂ→워',
  'lexical.copula': '이다',
  'lexical.d-to-l': 'ㄷ→ㄹ',
  'lexical.drop-h': 'ㅎ 탈락',
  'lexical.drop-s': 'ㅅ 탈락',
  'lexical.ha': '하→해',
  'lexical.reo': '러 불규칙',
  'lexical.reu-double-l': '르→ㄹㄹ',
  'lexical.suppletive': '보충형',
  'lexical.u-to-eo': '우→워',
};

export function formatMorphologyAnalysis(
  match: Match,
  locale: DocumentLocale = DocumentLocale.Korean,
): string {
  const analyses = match.atoms.flatMap((atom) =>
    atom.origins.map((origin) => {
      const notations = [
        ...new Set(
          origin.rulePath
            .map((ruleId) => morphologyRuleNotation(ruleId, locale))
            .filter((notation) => notation !== undefined),
        ),
      ];

      return origin.lemma === undefined || notations.length === 0
        ? undefined
        : [origin.lemma, ...notations].join(' + ');
    }),
  );

  const formatted = [
    ...new Set(analyses.filter((analysis) => analysis !== undefined)),
  ].join(' / ');
  return formatted.length === 0
    ? playgroundMessages[locale].directMatch
    : formatted;
}

function morphologyRuleNotation(
  ruleId: string,
  locale: DocumentLocale,
): string | undefined {
  if (ruleId === 'lexical.regular' || ruleId === 'lexical.surface-only') {
    return undefined;
  }

  const notation = morphologyRuleNotations[ruleId];
  if (notation !== undefined) {
    return notation;
  }

  const category = ruleId.split('.', 1)[0];
  const categoryLabels =
    locale === DocumentLocale.Korean
      ? {
          contraction: '축약',
          derivation: '파생',
          ending: '어미',
          lexical: '어간 변이',
          particle: '조사',
        }
      : {
          contraction: 'contraction',
          derivation: 'derivation',
          ending: 'ending',
          lexical: 'stem alternation',
          particle: 'particle',
        };

  return categoryLabels[category as keyof typeof categoryLabels];
}

async function enableComponentResource(
  engine: KfindEngine,
  setResourceStatus: (status: ComponentResourceStatus) => void,
  rerun: () => void,
  signal: AbortSignal,
  messages: PlaygroundMessages,
): Promise<void> {
  if (engine.componentResourceLoaded) {
    rerun();
    return;
  }

  setResourceStatus({
    state: ComponentResourceState.Loading,
    message: messages.resourceLoading,
  });

  try {
    const loaded = await loadComponentResource(engine, signal);
    if (signal.aborted) {
      return;
    }

    setResourceStatus({
      state: ComponentResourceState.Ready,
      message: messages.resourceStored(loaded.byteLength, loaded.stored),
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
  messages: PlaygroundMessages,
): PlaygroundResult {
  const query = input.query.trim();

  if (query.length === 0) {
    return {
      state: PlaygroundResultState.EmptyQuery,
      input,
      matches: [],
      elapsedMilliseconds: null,
      message: messages.emptyQuery,
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
          ? messages.noMatches
          : messages.matchCount(matches.length),
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

function readOptions(input: PlaygroundInput): CompileOptions {
  const parsedMaxGap = Number.parseInt(input.maxGap, 10);

  return {
    pos: PartOfSpeech.Auto,
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
