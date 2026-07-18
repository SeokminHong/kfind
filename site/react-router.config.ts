import type { Config } from '@react-router/dev/config';

import { copyFile, rm } from 'node:fs/promises';
import { join } from 'node:path';

import { documentRoutePaths, RoutePath } from './src/app/navigation';

const config: Config = {
  appDirectory: 'src',
  ssr: false,
  prerender: [...documentRoutePaths, '/404'],
  async buildEnd({ reactRouterConfig }) {
    const clientDirectory = join(reactRouterConfig.buildDirectory, 'client');

    await Promise.all(
      documentRoutePaths
        .filter((path) => path !== RoutePath.Overview)
        .map(async (path) => flattenPrerenderedPath(clientDirectory, path)),
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
  await rm(join(clientDirectory, relativePath), { recursive: true });
}
