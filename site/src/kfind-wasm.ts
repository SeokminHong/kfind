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
  free: () => void;
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

export interface LoadedComponentResource {
  readonly byteLength: number;
  readonly stored: boolean;
}

export interface RestoredComponentResource {
  readonly byteLength: number;
  readonly migrated: boolean;
}

declare const __KFIND_COMPONENT_RESOURCE_VERSION__: string;

const COMPONENT_RESOURCE_CACHE = 'kfind-component-resource-v1';
const COMPONENT_RESOURCE_URL = '/api/component-resource';

export const componentResourceVersion = __KFIND_COMPONENT_RESOURCE_VERSION__;

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
  signal?: AbortSignal,
): Promise<LoadedComponentResource> {
  const request = componentResourceRequest();
  const response = await fetch(request, { signal });

  if (!response.ok) {
    throw new Error(
      `component resource download failed: HTTP ${response.status}`,
    );
  }

  const cacheResponse = response.clone();
  const bytes = new Uint8Array(await response.arrayBuffer());
  engine.loadComponentResource(bytes);

  return {
    byteLength: bytes.byteLength,
    stored: await storeComponentResource(request, cacheResponse),
  };
}

export async function restoreComponentResource(
  engine: KfindEngine,
  signal?: AbortSignal,
): Promise<RestoredComponentResource | null> {
  if (!('caches' in globalThis)) {
    return null;
  }

  signal?.throwIfAborted();
  const cache = await globalThis.caches.open(COMPONENT_RESOURCE_CACHE);
  const currentRequest = componentResourceRequest();
  const cachedRequests = await componentResourceCacheCandidates(
    cache,
    currentRequest,
  );

  return restoreComponentResourceCandidate({
    cache,
    cachedRequests,
    currentRequest,
    engine,
    index: 0,
    signal,
  });
}

interface RestoreComponentResourceCandidateOptions {
  readonly cache: Cache;
  readonly cachedRequests: readonly Request[];
  readonly currentRequest: Request;
  readonly engine: KfindEngine;
  readonly index: number;
  readonly signal?: AbortSignal;
}

async function restoreComponentResourceCandidate({
  cache,
  cachedRequests,
  currentRequest,
  engine,
  index,
  signal,
}: RestoreComponentResourceCandidateOptions): Promise<RestoredComponentResource | null> {
  const cachedRequest = cachedRequests[index];

  if (cachedRequest === undefined) {
    return null;
  }

  const response = await cache.match(cachedRequest);

  if (response === undefined) {
    return restoreComponentResourceCandidate({
      cache,
      cachedRequests,
      currentRequest,
      engine,
      index: index + 1,
      signal,
    });
  }

  const migrationResponse = response.clone();
  const bytes = new Uint8Array(await response.arrayBuffer());
  signal?.throwIfAborted();

  try {
    engine.loadComponentResource(bytes);
  } catch {
    await cache.delete(cachedRequest);
    return restoreComponentResourceCandidate({
      cache,
      cachedRequests,
      currentRequest,
      engine,
      index: index + 1,
      signal,
    });
  }

  const migrated = cachedRequest.url !== currentRequest.url;

  if (migrated) {
    await cache.put(currentRequest, migrationResponse);
  }

  await deleteStaleComponentResources(cache, currentRequest);

  return { byteLength: bytes.byteLength, migrated };
}

function componentResourceRequest(): Request {
  const url = new URL(COMPONENT_RESOURCE_URL, globalThis.location.origin);
  url.searchParams.set('resource', componentResourceVersion);
  return new Request(url, { method: 'GET' });
}

async function componentResourceCacheCandidates(
  cache: Cache,
  currentRequest: Request,
): Promise<readonly Request[]> {
  const cachedRequests = await cache.keys();
  const compatibleRequests = cachedRequests.filter((request) => {
    const url = new URL(request.url);

    return (
      request.method === 'GET' &&
      url.origin === globalThis.location.origin &&
      url.pathname === COMPONENT_RESOURCE_URL
    );
  });

  return [
    ...compatibleRequests.filter(
      (request) => request.url === currentRequest.url,
    ),
    ...compatibleRequests.filter(
      (request) => request.url !== currentRequest.url,
    ),
  ];
}

async function deleteStaleComponentResources(
  cache: Cache,
  currentRequest: Request,
): Promise<void> {
  try {
    const cachedRequests = await cache.keys();
    await Promise.all(
      cachedRequests
        .filter((cachedRequest) => cachedRequest.url !== currentRequest.url)
        .map(async (cachedRequest) => cache.delete(cachedRequest)),
    );
  } catch {
    // The verified current entry remains usable when stale-entry cleanup fails.
  }
}

async function storeComponentResource(
  request: Request,
  response: Response,
): Promise<boolean> {
  if (!('caches' in globalThis)) {
    return false;
  }

  try {
    const cache = await globalThis.caches.open(COMPONENT_RESOURCE_CACHE);
    await cache.put(request, response);
    await deleteStaleComponentResources(cache, request);

    return true;
  } catch {
    return false;
  }
}
