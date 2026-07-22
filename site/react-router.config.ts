import type { Config } from '@react-router/dev/config';

import { access, copyFile, rm, rmdir } from 'node:fs/promises';
import { join } from 'node:path';

import { documentRoutePaths, RoutePath } from './src/app/navigation';

const config: Config = {
  appDirectory: 'src',
  ssr: false,
  prerender: [...documentRoutePaths, '/404'],
  async buildEnd({ reactRouterConfig }) {
    const clientDirectory = join(reactRouterConfig.buildDirectory, 'client');

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
    await rm(join(clientDirectory, '__spa-fallback.html'), { force: true });
  },
};

export default config;

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
