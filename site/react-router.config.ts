import type { Config } from '@react-router/dev/config';

import { access, copyFile, rm, rmdir } from 'node:fs/promises';
import { join } from 'node:path';

import { documentRoutePaths, RoutePath } from './src/app/navigation';

const prerenderLocale = process.env.KFIND_PRERENDER_LOCALE ?? 'ko';
if (prerenderLocale !== 'ko' && prerenderLocale !== 'en') {
  throw new Error('KFIND_PRERENDER_LOCALE must be ko or en');
}
const siteBasePath = readSiteBasePath();

const config: Config = {
  appDirectory: 'src',
  basename: siteBasePath,
  buildDirectory: `build/${prerenderLocale}`,
  ssr: false,
  prerender: [...documentRoutePaths, '/404'],
  async buildEnd({ reactRouterConfig }) {
    const buildClientDirectory = join(
      reactRouterConfig.buildDirectory,
      'client',
    );
    const clientDirectory =
      siteBasePath === '/'
        ? buildClientDirectory
        : join(buildClientDirectory, siteBasePath.slice(1));

    const nestedPaths = [...documentRoutePaths]
      .filter((path) => path !== RoutePath.Overview)
      .sort((left, right) => right.length - left.length);
    await Promise.all(
      nestedPaths.map(async (path) =>
        flattenPrerenderedPath(clientDirectory, path),
      ),
    );
    await Promise.all(
      documentRoutePaths.map(async (path) => {
        const documentFile =
          path === RoutePath.Overview
            ? join(clientDirectory, 'index.html')
            : join(clientDirectory, `${path.slice(1)}.html`);
        await access(documentFile);
      }),
    );

    await copyFile(
      join(clientDirectory, '404', 'index.html'),
      join(clientDirectory, '404.html'),
    );
    await rm(join(clientDirectory, '404'), { recursive: true });
    await rm(
      siteBasePath === '/'
        ? join(clientDirectory, '__spa-fallback.html')
        : join(buildClientDirectory, 'index.html'),
      { force: true },
    );
  },
};

export default config;

function readSiteBasePath(): string {
  const value = process.env.KFIND_SITE_BASE_PATH ?? '/';
  if (
    value !== '/' &&
    !/^\/versions\/(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)(?:-rc\.[1-9][0-9]*)?$/u.test(
      value,
    )
  ) {
    throw new Error('KFIND_SITE_BASE_PATH must be / or /versions/VERSION');
  }
  return value;
}

async function flattenPrerenderedPath(
  clientDirectory: string,
  path: RoutePath,
): Promise<void> {
  const relativePath = path.slice(1);

  await copyFile(
    join(clientDirectory, relativePath, 'index.html'),
    join(clientDirectory, `${relativePath}.html`),
  );
  await rm(join(clientDirectory, relativePath, 'index.html'));
  try {
    await rmdir(join(clientDirectory, relativePath));
  } catch (error: unknown) {
    if (
      !(error instanceof Error) ||
      !('code' in error) ||
      error.code !== 'ENOTEMPTY'
    ) {
      throw error;
    }
  }
}
