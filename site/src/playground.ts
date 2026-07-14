import type { CompileOptions, KfindEngine, Match } from './kfind-wasm';

import {
  BoundaryPolicy,
  ExpandMode,
  findMatches,
  loadComponentResource,
  loadKfind,
  NormalizationMode,
  PartOfSpeech,
} from './kfind-wasm';

enum PlaygroundState {
  Loading = 'loading',
  Ready = 'ready',
  Error = 'error',
}

enum PresetName {
  Predicate = 'predicate',
  Phrase = 'phrase',
  Component = 'component',
  Literal = 'literal',
}

interface PlaygroundElements {
  readonly form: HTMLFormElement;
  readonly status: HTMLElement;
  readonly query: HTMLInputElement;
  readonly text: HTMLTextAreaElement;
  readonly pos: HTMLSelectElement;
  readonly boundary: HTMLSelectElement;
  readonly expand: HTMLSelectElement;
  readonly normalization: HTMLSelectElement;
  readonly maxGap: HTMLInputElement;
  readonly runButton: HTMLButtonElement;
  readonly textCount: HTMLElement;
  readonly summary: HTMLElement;
  readonly executionTime: HTMLElement;
  readonly preview: HTMLElement;
  readonly matchList: HTMLOListElement;
  readonly rawOutput: HTMLElement;
  readonly resourceButton: HTMLButtonElement;
  readonly resourceStatus: HTMLElement;
}

interface Preset {
  readonly query: string;
  readonly text: string;
  readonly pos: PartOfSpeech;
  readonly boundary: BoundaryPolicy;
  readonly expand: ExpandMode;
}

type ElementConstructor<T extends Element> = new () => T;

const presets: Readonly<Record<PresetName, Preset>> = {
  [PresetName.Predicate]: {
    query: '걷다',
    text: '오늘은 공원을 걸었다.\n내일도 천천히 걷고 싶다.\n산책길을 걷는 사람을 만났다.',
    pos: PartOfSpeech.Verb,
    boundary: BoundaryPolicy.Smart,
    expand: ExpandMode.Inflection,
  },
  [PresetName.Phrase]: {
    query: 'n:사용자 v:검증하다',
    text: '에이전트가 결과를 만들면 사용자가 문맥을 다시 검증했습니다.\n사용자 권한만 확인했습니다.',
    pos: PartOfSpeech.Auto,
    boundary: BoundaryPolicy.Any,
    expand: ExpandMode.Inflection,
  },
  [PresetName.Component]: {
    query: 'n:요리',
    text: '중국요리를 만드는 법을 정리했다.\n요리 도구도 함께 준비했다.\n요리사라는 직업도 있다.',
    pos: PartOfSpeech.Auto,
    boundary: BoundaryPolicy.Smart,
    expand: ExpandMode.Inflection,
  },
  [PresetName.Literal]: {
    query: '걸어',
    text: '길을 걸어 갔다.\n그는 걷다가 멈췄다.\n걸어라는 문자열만 그대로 찾는다.',
    pos: PartOfSpeech.Literal,
    boundary: BoundaryPolicy.Smart,
    expand: ExpandMode.Literal,
  },
};

export function initializePlayground(root: ParentNode): () => void {
  const elements = collectElements(root);
  const controller = new AbortController();
  const { signal } = controller;
  let engine: KfindEngine | undefined;
  let pendingRun: ReturnType<typeof globalThis.setTimeout> | undefined;

  const run = (): void => {
    if (engine === undefined || signal.aborted) {
      return;
    }

    executeSearch(engine, elements);
  };

  const scheduleRun = (): void => {
    globalThis.clearTimeout(pendingRun);
    pendingRun = globalThis.setTimeout(run, 120);
  };

  elements.form.addEventListener(
    'submit',
    (event) => {
      event.preventDefault();
      globalThis.clearTimeout(pendingRun);
      run();
    },
    { signal },
  );

  elements.resourceButton.addEventListener(
    'click',
    () => {
      if (engine === undefined) {
        return;
      }

      void enableComponentResource(engine, elements, run, signal);
    },
    { signal },
  );

  for (const input of [
    elements.query,
    elements.text,
    elements.pos,
    elements.boundary,
    elements.expand,
    elements.normalization,
    elements.maxGap,
  ]) {
    input.addEventListener(
      'input',
      () => {
        updateTextCount(elements);
        scheduleRun();
      },
      { signal },
    );
  }

  for (const button of root.querySelectorAll<HTMLButtonElement>(
    '[data-preset]',
  )) {
    button.addEventListener(
      'click',
      () => {
        const presetName = parseEnum(button.dataset.preset ?? '', PresetName);

        if (presetName === undefined) {
          return;
        }

        applyPreset(elements, presets[presetName]);
        run();
      },
      { signal },
    );
  }

  updateTextCount(elements);
  setState(elements, PlaygroundState.Loading, 'WASM engine을 불러오는 중…');

  void loadKfind()
    .then((loaded) => {
      if (signal.aborted) {
        loaded.engine.free();
        return;
      }

      engine = loaded.engine;
      setControlsEnabled(elements);
      setState(
        elements,
        PlaygroundState.Ready,
        `WASM ready · embedded lexicon · ${loaded.loadMilliseconds.toFixed(0)} ms`,
      );
      run();
    })
    .catch((error: unknown) => {
      if (signal.aborted) {
        return;
      }

      setState(elements, PlaygroundState.Error, readableError(error));
      renderError(elements, error);
    });

  return () => {
    controller.abort();
    globalThis.clearTimeout(pendingRun);
    engine?.free();
  };
}

function executeSearch(
  engine: KfindEngine,
  elements: PlaygroundElements,
): void {
  const query = elements.query.value.trim();

  if (query.length === 0) {
    clearResults(elements, '쿼리를 입력해 주세요.');
    return;
  }

  try {
    const options = readOptions(elements);
    const startedAt = performance.now();
    const matches = findMatches(engine, query, elements.text.value, options);
    const elapsed = performance.now() - startedAt;

    renderResults(elements, matches, elapsed);
  } catch (error) {
    renderError(elements, error);

    if (readableError(error).toLowerCase().includes('component')) {
      elements.resourceStatus.dataset.state = 'needed';
      elements.resourceStatus.textContent =
        '이 query를 실행하려면 component asset이 필요합니다.';
    }
  }
}

async function enableComponentResource(
  engine: KfindEngine,
  elements: PlaygroundElements,
  rerun: () => void,
  signal: AbortSignal,
): Promise<void> {
  if (engine.componentResourceLoaded) {
    rerun();
    return;
  }

  elements.resourceButton.disabled = true;
  elements.resourceStatus.dataset.state = 'loading';
  elements.resourceStatus.textContent = 'R2에서 component asset을 불러오는 중…';

  try {
    const byteLength = await loadComponentResource(engine, signal);
    if (signal.aborted) {
      return;
    }
    elements.resourceStatus.dataset.state = 'ready';
    elements.resourceStatus.textContent = `${formatMebibytes(byteLength)} MiB 불러오기·검증 완료`;
    elements.resourceButton.textContent = 'Component asset 준비됨';
    rerun();
  } catch (error) {
    if (signal.aborted) {
      return;
    }

    elements.resourceStatus.dataset.state = 'error';
    elements.resourceStatus.textContent = readableError(error);
    elements.resourceButton.disabled = false;
  }
}

function renderResults(
  elements: PlaygroundElements,
  matches: readonly Match[],
  elapsed: number,
): void {
  elements.executionTime.textContent = `${elapsed.toFixed(2)} ms`;
  elements.summary.textContent =
    matches.length === 0
      ? '일치하는 span이 없습니다.'
      : `일치하는 span ${matches.length}개를 찾았습니다.`;
  elements.rawOutput.textContent = JSON.stringify(matches, null, 2);
  renderPreview(elements.preview, elements.text.value, matches);
  renderMatchList(elements.matchList, elements.text.value, matches);
}

function renderPreview(
  container: HTMLElement,
  text: string,
  matches: readonly Match[],
): void {
  container.replaceChildren();
  const spans = mergeSpans(matches, text.length);
  let cursor = 0;

  for (const span of spans) {
    container.append(document.createTextNode(text.slice(cursor, span.start)));
    const mark = document.createElement('mark');
    mark.textContent = text.slice(span.start, span.end);
    container.append(mark);
    cursor = span.end;
  }

  container.append(document.createTextNode(text.slice(cursor)));
}

function renderMatchList(
  container: HTMLOListElement,
  text: string,
  matches: readonly Match[],
): void {
  container.replaceChildren();

  if (matches.length === 0) {
    const empty = document.createElement('li');
    empty.className = 'match-empty';
    empty.textContent = '옵션을 바꾸거나 다른 query로 검색해 보세요.';
    container.append(empty);
    return;
  }

  for (const [index, match] of matches.entries()) {
    const item = document.createElement('li');
    const head = document.createElement('div');
    const number = document.createElement('span');
    const surface = document.createElement('strong');
    const span = document.createElement('code');
    const provenance = document.createElement('p');

    number.textContent = String(index + 1).padStart(2, '0');
    surface.textContent = text.slice(match.start, match.end);
    span.textContent = `[${match.start}, ${match.end})`;
    provenance.textContent = formatProvenance(match);
    head.append(number, surface, span);
    item.append(head, provenance);
    container.append(item);
  }
}

function mergeSpans(
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

function formatProvenance(match: Match): string {
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

function readOptions(elements: PlaygroundElements): CompileOptions {
  const parsedMaxGap = Number.parseInt(elements.maxGap.value, 10);

  return {
    pos: parseEnum(elements.pos.value, PartOfSpeech) ?? PartOfSpeech.Auto,
    boundary:
      parseEnum(elements.boundary.value, BoundaryPolicy) ??
      BoundaryPolicy.Smart,
    expand:
      parseEnum(elements.expand.value, ExpandMode) ?? ExpandMode.Inflection,
    normalization:
      parseEnum(elements.normalization.value, NormalizationMode) ??
      NormalizationMode.Nfc,
    maxGap: Number.isNaN(parsedMaxGap) ? 0 : Math.max(0, parsedMaxGap),
  };
}

function applyPreset(elements: PlaygroundElements, preset: Preset): void {
  elements.query.value = preset.query;
  elements.text.value = preset.text;
  elements.pos.value = preset.pos;
  elements.boundary.value = preset.boundary;
  elements.expand.value = preset.expand;
  updateTextCount(elements);
}

function setState(
  elements: PlaygroundElements,
  state: PlaygroundState,
  message: string,
): void {
  elements.status.dataset.state = state;
  const messageElement = elements.status.querySelector('span:last-child');

  if (messageElement !== null) {
    messageElement.textContent = message;
  }
}

function setControlsEnabled(elements: PlaygroundElements): void {
  elements.runButton.disabled = false;
  elements.resourceButton.disabled = false;
}

function clearResults(elements: PlaygroundElements, message: string): void {
  elements.summary.textContent = message;
  elements.executionTime.textContent = '— ms';
  elements.preview.replaceChildren();
  elements.matchList.replaceChildren();
  elements.rawOutput.textContent = '[]';
}

function renderError(elements: PlaygroundElements, error: unknown): void {
  const message = readableError(error);
  elements.summary.textContent = 'Query compile 또는 검색 실행에 실패했습니다.';
  elements.executionTime.textContent = 'error';
  elements.preview.replaceChildren();
  const errorMessage = document.createElement('p');
  errorMessage.className = 'result-error';
  errorMessage.textContent = message;
  elements.preview.append(errorMessage);
  elements.matchList.replaceChildren();
  elements.rawOutput.textContent = JSON.stringify({ error: message }, null, 2);
}

function updateTextCount(elements: PlaygroundElements): void {
  elements.textCount.textContent = `${elements.text.value.length.toLocaleString('ko-KR')}자`;
}

function readableError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function formatMebibytes(byteLength: number): string {
  return (byteLength / (1024 * 1024)).toFixed(1);
}

function parseEnum<T extends string>(
  value: string,
  enumType: Readonly<Record<string, T>>,
): T | undefined {
  return Object.values(enumType).find((candidate) => candidate === value);
}

function collectElements(root: ParentNode): PlaygroundElements {
  return {
    form: requiredElement(root, '#playground-form', HTMLFormElement),
    status: requiredElement(root, '#wasm-status', HTMLElement),
    query: requiredElement(root, '#query-input', HTMLInputElement),
    text: requiredElement(root, '#text-input', HTMLTextAreaElement),
    pos: requiredElement(root, '#pos-select', HTMLSelectElement),
    boundary: requiredElement(root, '#boundary-select', HTMLSelectElement),
    expand: requiredElement(root, '#expand-select', HTMLSelectElement),
    normalization: requiredElement(
      root,
      '#normalization-select',
      HTMLSelectElement,
    ),
    maxGap: requiredElement(root, '#max-gap-input', HTMLInputElement),
    runButton: requiredElement(root, '#run-button', HTMLButtonElement),
    textCount: requiredElement(root, '#text-count', HTMLElement),
    summary: requiredElement(root, '#result-summary', HTMLElement),
    executionTime: requiredElement(root, '#execution-time', HTMLElement),
    preview: requiredElement(root, '#result-preview', HTMLElement),
    matchList: requiredElement(root, '#match-list', HTMLOListElement),
    rawOutput: requiredElement(root, '#raw-output', HTMLElement),
    resourceButton: requiredElement(
      root,
      '#resource-button',
      HTMLButtonElement,
    ),
    resourceStatus: requiredElement(root, '#resource-status', HTMLElement),
  };
}

function requiredElement<T extends Element>(
  root: ParentNode,
  selector: string,
  constructor: ElementConstructor<T>,
): T {
  const element = root.querySelector(selector);

  if (!(element instanceof constructor)) {
    throw new TypeError(`required playground element is missing: ${selector}`);
  }

  return element;
}
