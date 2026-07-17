import { execFileSync } from 'node:child_process';
import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [react(), vanillaExtractPlugin()],
  define: {
    __KFIND_COMPONENT_RESOURCE_VERSION__: JSON.stringify(
      readComponentResourceVersion(),
    ),
  },
  build: {
    target: 'es2022',
  },
});

function readComponentResourceVersion(): string {
  const hasWorkingTreeChanges = readGitValue(['status', '--porcelain']) !== '';

  if (!hasWorkingTreeChanges) {
    try {
      return readGitValue([
        'describe',
        '--tags',
        '--exact-match',
        '--match',
        'v[0-9]*',
      ]);
    } catch {
      // This is a clean development build when HEAD has no version tag.
    }
  }

  return readGitValue([
    'log',
    '-1',
    '--format=%H',
    '--',
    ':(top)scripts/build-component-resource.sh',
  ]);
}

function readGitValue(arguments_: readonly string[]): string {
  return execFileSync('git', [...arguments_], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
  }).trim();
}
