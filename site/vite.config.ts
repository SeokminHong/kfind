import { readFileSync } from 'node:fs';
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
    __KFIND_COMPONENT_RESOURCE_REVISION__: JSON.stringify(
      readComponentResourceRevision(),
    ),
    __KFIND_PLAYGROUND_CORPUS_METADATA__: JSON.stringify(
      readPlaygroundCorpusMetadata(),
    ),
    __KFIND_PRERENDER_LOCALE__: JSON.stringify(prerenderLocale),
  },
  build: {
    target: 'es2022',
  },
});

function readComponentResourceRevision(): string {
  const checksum = readFileSync(
    new URL(
      '../data/generated/morphology-component-compact.sha256',
      import.meta.url,
    ),
    'utf8',
  ).trim();

  if (!/^[0-9a-f]{64}$/u.test(checksum)) {
    throw new Error('component resource checksum must be 64 lowercase hex');
  }

  return checksum;
}

function readPlaygroundCorpusMetadata(): {
  readonly byteLength: number;
  readonly sha256: string;
} {
  const manifest: unknown = JSON.parse(
    readFileSync(
      new URL(
        './public/playground/korean-wikipedia-20231101-ko-1mib.sources.json',
        import.meta.url,
      ),
      'utf8',
    ),
  );

  if (!isRecord(manifest) || !isRecord(manifest.output)) {
    throw new Error('playground corpus manifest is invalid');
  }

  const byteLength = manifest.output.utf8_bytes;
  const sha256 = manifest.output.sha256;

  if (
    typeof byteLength !== 'number' ||
    !Number.isSafeInteger(byteLength) ||
    byteLength <= 0 ||
    typeof sha256 !== 'string' ||
    !/^[0-9a-f]{64}$/u.test(sha256)
  ) {
    throw new Error('playground corpus manifest is invalid');
  }

  return { byteLength, sha256 };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}
