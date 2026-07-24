import walkHangStressText from '../../data/fixtures/walk_hang_stress.txt?raw';

import { BoundaryPolicy, ExpandMode } from './kfind-wasm';

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
  readonly query: string;
  readonly text: string;
}

interface PresetDefinition {
  readonly boundary: BoundaryPolicy;
  readonly expand: ExpandMode;
  readonly maxGap: string;
  readonly query: string;
  readonly text: string | (() => Promise<string>);
}

const LARGE_INPUT_BYTE_LENGTH = 1024 * 1024;
const LARGE_INPUT_SHA256 =
  '2bf73e793f1c43383bb2794d485ca8e81ae99879816feaaf4a00eab51f250d81';
const LARGE_INPUT_URL = '/playground/korean-wikipedia-20231101-ko-1mib.txt';

let cachedLargeInput: Promise<string> | undefined;

const presets: Readonly<Record<PlaygroundPresetName, PresetDefinition>> = {
  [PlaygroundPresetName.Predicate]: {
    query: 'v:걷다',
    text: walkHangStressText.trimEnd(),
    boundary: BoundaryPolicy.Smart,
    expand: ExpandMode.Inflection,
    maxGap: '24',
  },
  [PlaygroundPresetName.Phrase]: {
    query: 'n:사용자 v:검증하다',
    text: '에이전트가 결과를 만들면 사용자가 문맥을 다시 검증했습니다.\n사용자 권한만 확인했습니다.',
    boundary: BoundaryPolicy.Any,
    expand: ExpandMode.Inflection,
    maxGap: '24',
  },
  [PlaygroundPresetName.Component]: {
    query: 'n:요리',
    text: '중국요리를 만드는 법을 정리했다.\n요리 도구도 함께 준비했다.\n요리사라는 직업도 있다.',
    boundary: BoundaryPolicy.Smart,
    expand: ExpandMode.Inflection,
    maxGap: '24',
  },
  [PlaygroundPresetName.Literal]: {
    query: 'lit:걸어',
    text: '길을 걸어 갔다.\n그는 걷다가 멈췄다.\n걸어라는 문자열만 그대로 찾는다.',
    boundary: BoundaryPolicy.Smart,
    expand: ExpandMode.Literal,
    maxGap: '24',
  },
  [PlaygroundPresetName.LargeInput]: {
    query: 'v:말하다',
    text: loadLargeInput,
    boundary: BoundaryPolicy.Smart,
    expand: ExpandMode.Inflection,
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
  {
    label: '한국어 위키백과 1 MiB · smart',
    value: PlaygroundPresetName.LargeInput,
  },
] as const;

const initialPreset = presets[PlaygroundPresetName.Predicate];

export const initialPlaygroundInput = createPresetInput(
  initialPreset,
  walkHangStressText.trimEnd(),
);

export async function applyPlaygroundPreset(
  presetName: PlaygroundPresetName,
): Promise<PlaygroundInput> {
  const preset = presets[presetName];
  const text =
    typeof preset.text === 'function' ? await preset.text() : preset.text;

  return createPresetInput(preset, text);
}

function createPresetInput(
  preset: PresetDefinition,
  text: string,
): PlaygroundInput {
  return {
    boundary: preset.boundary,
    expand: preset.expand,
    maxGap: preset.maxGap,
    query: preset.query,
    text,
  };
}

async function loadLargeInput(): Promise<string> {
  if (cachedLargeInput !== undefined) {
    return cachedLargeInput;
  }

  cachedLargeInput = fetch(LARGE_INPUT_URL)
    .then(async (response) => {
      if (!response.ok) {
        throw new Error(
          `Wikipedia corpus load failed: ${response.status} ${response.statusText}`,
        );
      }

      const bytes = new Uint8Array(await response.arrayBuffer());

      if (bytes.byteLength !== LARGE_INPUT_BYTE_LENGTH) {
        throw new Error(
          `Wikipedia corpus size mismatch: ${bytes.byteLength} bytes`,
        );
      }

      const digest = await crypto.subtle.digest('SHA-256', bytes);
      const sha256 = Array.from(new Uint8Array(digest), (byte) =>
        byte.toString(16).padStart(2, '0'),
      ).join('');

      if (sha256 !== LARGE_INPUT_SHA256) {
        throw new Error(`Wikipedia corpus checksum mismatch: ${sha256}`);
      }

      return new TextDecoder().decode(bytes);
    })
    .catch((error: unknown) => {
      cachedLargeInput = undefined;
      throw error;
    });

  return cachedLargeInput;
}
