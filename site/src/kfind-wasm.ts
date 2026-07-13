export enum ExpandMode {
  Literal = 'literal',
  Inflection = 'inflection',
  Derivation = 'derivation',
}

export enum BoundaryPolicy {
  Smart = 'smart',
  Token = 'token',
  Any = 'any',
}

export enum PartOfSpeech {
  Auto = 'auto',
  Noun = 'noun',
  Pronoun = 'pronoun',
  Numeral = 'numeral',
  Verb = 'verb',
  Adjective = 'adjective',
  Determiner = 'determiner',
  Adverb = 'adverb',
  Particle = 'particle',
  Interjection = 'interjection',
  Literal = 'literal',
}

export enum NormalizationMode {
  Nfc = 'nfc',
  Canonical = 'canonical',
  None = 'none',
}

export interface CompileOptions {
  readonly expand: ExpandMode;
  readonly boundary: BoundaryPolicy;
  readonly pos: PartOfSpeech;
  readonly normalization: NormalizationMode;
  readonly maxGap: number;
}

export interface MatchOrigin {
  readonly analysisIndex: number;
  readonly rulePath: readonly string[];
}

export interface MatchAtom {
  readonly core: { readonly start: number; readonly end: number };
  readonly token: { readonly start: number; readonly end: number };
  readonly origins: readonly MatchOrigin[];
}

export interface Match {
  readonly start: number;
  readonly end: number;
  readonly atoms: readonly MatchAtom[];
}

interface KfindMatcher {
  findAll: (text: string) => readonly Match[];
  free: () => void;
}

export interface KfindEngine {
  compile: (query: string, options: CompileOptions) => KfindMatcher;
  loadComponentResource: (componentResource: Uint8Array) => void;
  readonly componentResourceLoaded: boolean;
}

interface KfindModule {
  default: () => Promise<unknown>;
  Kfind: new () => KfindEngine;
}

export interface LoadedKfind {
  readonly engine: KfindEngine;
  readonly loadMilliseconds: number;
}

export async function loadKfind(): Promise<LoadedKfind> {
  const startedAt = performance.now();
  const module = (await import('./generated-wasm/kfind.js')) as KfindModule;

  await module.default();

  return {
    engine: new module.Kfind(),
    loadMilliseconds: performance.now() - startedAt,
  };
}

export function findMatches(
  engine: KfindEngine,
  query: string,
  text: string,
  options: CompileOptions,
): readonly Match[] {
  const matcher = engine.compile(query, options);

  try {
    return matcher.findAll(text);
  } finally {
    matcher.free();
  }
}

export async function loadComponentResource(
  engine: KfindEngine,
): Promise<number> {
  const response = await fetch('/api/component-resource');

  if (!response.ok) {
    throw new Error(
      `component resource download failed: HTTP ${response.status}`,
    );
  }

  const bytes = new Uint8Array(await response.arrayBuffer());
  engine.loadComponentResource(bytes);
  return bytes.byteLength;
}
