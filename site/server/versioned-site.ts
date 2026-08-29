const archivePrefix = 'site/versions';
const manifestKey = `${archivePrefix}/manifest.json`;
const maximumManifestBytes = 64 * 1024;
const maximumIndexBytes = 1024 * 1024;
const maximumArchiveBytes = 256 * 1024 * 1024;
const maximumVersionCount = 100;
const maximumFileCount = 4096;
const immutableCacheControl = 'public, max-age=31536000, immutable';
const versionPattern =
  /^(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)(?:-rc\.[1-9][0-9]*)?$/u;

export interface DocumentVersion {
  readonly path: string;
  readonly prerelease: boolean;
  readonly version: string;
}

export interface DocumentVersionManifest {
  readonly latest: string;
  readonly schemaVersion: 1;
  readonly versions: readonly DocumentVersion[];
}

interface ArchiveFile {
  readonly cacheControl: string;
  readonly contentType: string;
  readonly length: number;
  readonly offset: number;
  readonly sha256: string;
}

interface ArchiveIndex {
  readonly archiveSha256: string;
  readonly archiveSize: number;
  readonly files: Readonly<Record<string, ArchiveFile>>;
  readonly schemaVersion: 1;
  readonly version: string;
}

interface StoredManifest {
  readonly etag: string;
  readonly manifest: DocumentVersionManifest;
}

export async function readVersionManifest(
  bucket: R2Bucket,
): Promise<StoredManifest | undefined> {
  const object = await bucket.get(manifestKey);
  if (object === null) {
    return undefined;
  }
  if (object.size > maximumManifestBytes) {
    throw new Error('document version manifest exceeds the size limit');
  }

  const value: unknown = await object.json();
  if (!isDocumentVersionManifest(value)) {
    throw new Error('document version manifest is invalid');
  }
  return { etag: object.httpEtag, manifest: value };
}

export async function serveVersionedSite(
  request: Request,
  bucket: R2Bucket,
): Promise<Response> {
  if (request.method !== 'GET' && request.method !== 'HEAD') {
    return new Response('Method Not Allowed', {
      headers: { Allow: 'GET, HEAD' },
      status: 405,
    });
  }

  const url = new URL(request.url);
  const route = parseVersionedRoute(url.pathname);
  if (route === undefined) {
    return notFound();
  }
  const storedManifest = await readVersionManifest(bucket);
  if (
    storedManifest?.manifest.versions.some(
      (entry) => entry.version === route.version,
    ) !== true
  ) {
    return notFound();
  }

  const index = await readArchiveIndex(bucket, route.version);
  const requestedPath = archivePath(route.relativePath, url.searchParams);
  let file = index.files[requestedPath];
  let status = 200;
  if (file === undefined && requestedPath.endsWith('.html')) {
    file = index.files[localizedHtmlPath('404.html', url.searchParams)];
    status = 404;
  }
  if (file === undefined) {
    return notFound();
  }

  const etag = `"${file.sha256}"`;
  const headers = responseHeaders(file, etag, requestedPath, url.searchParams);
  if (request.headers.get('If-None-Match') === etag) {
    headers.delete('Content-Length');
    return new Response(null, { headers, status: 304 });
  }
  if (request.method === 'HEAD') {
    return new Response(null, { headers, status });
  }
  if (file.length === 0) {
    return new Response(null, { headers, status });
  }

  const archive = await bucket.get(
    `${archivePrefix}/${route.version}/site.tar`,
    {
      range: { length: file.length, offset: file.offset },
    },
  );
  if (archive === null) {
    throw new Error(
      `versioned site archive range is unavailable: ${requestedPath}`,
    );
  }
  if (
    archive.size !== index.archiveSize ||
    archive.range === undefined ||
    !('length' in archive.range) ||
    archive.range.length !== file.length
  ) {
    throw new Error(
      `versioned site archive range is unavailable: ${requestedPath}`,
    );
  }
  return new Response(archive.body, { headers, status });
}

function parseVersionedRoute(
  pathname: string,
): { readonly relativePath: string; readonly version: string } | undefined {
  const match = /^\/versions\/(?<version>[^/]+)(?<relativePath>\/.*)?$/u.exec(
    pathname,
  );
  const version = match?.groups?.version;
  const relativePath = match?.groups?.relativePath ?? '/';
  if (
    version === undefined ||
    !versionPattern.test(version) ||
    relativePath.includes('%') ||
    relativePath.includes('\\') ||
    relativePath.split('/').includes('..')
  ) {
    return undefined;
  }
  return { relativePath, version };
}

function archivePath(
  relativePath: string,
  searchParameters: URLSearchParams,
): string {
  const path = relativePath.replace(/^\/+|\/+$/gu, '');
  if (path === '') {
    return localizedHtmlPath('index.html', searchParameters);
  }
  if (path.startsWith('_i18n/')) {
    return '__not_found__';
  }
  if (path === 'api/component-resource') {
    return path;
  }

  const lastSegment = path.split('/').at(-1) ?? '';
  const filePath = lastSegment.includes('.') ? path : `${path}.html`;
  return localizedHtmlPath(filePath, searchParameters);
}

function localizedHtmlPath(
  path: string,
  searchParameters: URLSearchParams,
): string {
  return path.endsWith('.html') && searchParameters.get('hl') === 'en'
    ? `_i18n/en/${path}`
    : path;
}

async function readArchiveIndex(
  bucket: R2Bucket,
  version: string,
): Promise<ArchiveIndex> {
  const object = await bucket.get(`${archivePrefix}/${version}/index.json`);
  if (object === null) {
    throw new Error(`versioned site index is missing: ${version}`);
  }
  if (object.size > maximumIndexBytes) {
    throw new Error(`versioned site index exceeds the size limit: ${version}`);
  }
  const value: unknown = await object.json();
  if (!isArchiveIndex(value, version)) {
    throw new Error(`versioned site index is invalid: ${version}`);
  }
  return value;
}

function responseHeaders(
  file: ArchiveFile,
  etag: string,
  path: string,
  searchParameters: URLSearchParams,
): Headers {
  const headers = new Headers({
    'Cache-Control': file.cacheControl,
    'Content-Length': file.length.toString(),
    'Content-Type': file.contentType,
    ETag: etag,
    'X-Content-Type-Options': 'nosniff',
    'X-Robots-Tag': 'noindex',
  });
  if (path.endsWith('.html')) {
    headers.set(
      'Content-Language',
      searchParameters.get('hl') === 'en' ? 'en' : 'ko',
    );
  }
  return headers;
}

function isDocumentVersionManifest(
  value: unknown,
): value is DocumentVersionManifest {
  if (
    !isRecord(value) ||
    value.schemaVersion !== 1 ||
    typeof value.latest !== 'string' ||
    !versionPattern.test(value.latest) ||
    !Array.isArray(value.versions) ||
    value.versions.length === 0 ||
    value.versions.length > maximumVersionCount
  ) {
    return false;
  }
  const versions = new Set<string>();
  for (const entry of value.versions) {
    if (
      !isRecord(entry) ||
      typeof entry.version !== 'string' ||
      !versionPattern.test(entry.version) ||
      typeof entry.prerelease !== 'boolean' ||
      entry.prerelease !== entry.version.includes('-rc.') ||
      entry.path !== `/versions/${entry.version}` ||
      versions.has(entry.version)
    ) {
      return false;
    }
    versions.add(entry.version);
  }
  return versions.has(value.latest);
}

function isArchiveIndex(
  value: unknown,
  version: string,
): value is ArchiveIndex {
  if (
    !isRecord(value) ||
    value.schemaVersion !== 1 ||
    value.version !== version ||
    !isSha256(value.archiveSha256) ||
    !isSafeInteger(value.archiveSize) ||
    value.archiveSize <= 0 ||
    value.archiveSize > maximumArchiveBytes ||
    !isRecord(value.files)
  ) {
    return false;
  }
  const files = Object.entries(value.files);
  const archiveSize = value.archiveSize;
  if (files.length === 0 || files.length > maximumFileCount) {
    return false;
  }
  return files.every(
    ([path, file]) =>
      isSafeArchivePath(path) &&
      isRecord(file) &&
      file.cacheControl === immutableCacheControl &&
      typeof file.contentType === 'string' &&
      file.contentType.length <= 128 &&
      isSafeInteger(file.length) &&
      file.length >= 0 &&
      isSafeInteger(file.offset) &&
      file.offset >= 0 &&
      file.offset + file.length <= archiveSize &&
      isSha256(file.sha256),
  );
}

function isSafeArchivePath(path: string): boolean {
  return (
    path.length > 0 &&
    path.length <= 512 &&
    !path.startsWith('/') &&
    !path.includes('\\') &&
    !path.split('/').includes('..')
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function isSafeInteger(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value);
}

function isSha256(value: unknown): value is string {
  return typeof value === 'string' && /^[0-9a-f]{64}$/u.test(value);
}

function notFound(): Response {
  return new Response('Not Found', {
    headers: {
      'Content-Type': 'text/plain; charset=utf-8',
      'X-Robots-Tag': 'noindex',
    },
    status: 404,
  });
}
