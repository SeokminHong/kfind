import type { Config } from '@react-router/dev/config';

import { copyFile, rm } from 'node:fs/promises';
import { join } from 'node:path';

import { documentRoutePaths } from './src/app/navigation';

const config: Config = {
  appDirectory: 'src',
  ssr: false,
  prerender: [...documentRoutePaths, '/404'],
  async buildEnd({ reactRouterConfig }) {
    const clientDirectory = join(reactRouterConfig.buildDirectory, 'client');

    await copyFile(
      join(clientDirectory, '404', 'index.html'),
      join(clientDirectory, '404.html'),
    );
    await rm(join(clientDirectory, '__spa-fallback.html'), { force: true });
  },
};

export default config;
