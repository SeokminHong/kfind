import { execFileSync } from 'node:child_process';
import mdx from '@mdx-js/rollup';
import { reactRouter } from '@react-router/dev/vite';
import rehypeShiki from '@shikijs/rehype';
import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin';
import remarkGfm from 'remark-gfm';
import { defineConfig } from 'vite';

import { remarkHeadingIds } from './remark/heading-ids';

const prerenderLocale = process.env.KFIND_PRERENDER_LOCALE ?? 'ko';
if (prerenderLocale !== 'ko' && prerenderLocale !== 'en') {
  throw new Error('KFIND_PRERENDER_LOCALE must be ko or en');
}

export default defineConfig({
  plugins: [
    mdx({
      rehypePlugins: [
        [
          rehypeShiki,
          {
            defaultLanguage: 'text',
            theme: 'github-light',
          },
        ],
      ],
      remarkPlugins: [remarkGfm, remarkHeadingIds],
    }),
    reactRouter(),
    vanillaExtractPlugin(),
  ],
  define: {
    __KFIND_COMPONENT_RESOURCE_VERSION__: JSON.stringify(
      readComponentResourceVersion(),
    ),
    __KFIND_PRERENDER_LOCALE__: JSON.stringify(prerenderLocale),
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

  if (readGitValue(['rev-parse', '--is-shallow-repository']) === 'true') {
    throw new Error(
      'component resource version requires full Git history; checkout with fetch-depth: 0',
    );
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
