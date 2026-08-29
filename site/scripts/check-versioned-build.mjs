import { readdir, readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const siteDirectory = fileURLToPath(new URL('..', import.meta.url));
const clientDirectory = join(siteDirectory, 'build', 'client');
const basePath = process.env.KFIND_SITE_BASE_PATH;

if (
  basePath === undefined ||
  !/^\/versions\/(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)(?:-rc\.[1-9][0-9]*)?$/u.test(
    basePath,
  )
) {
  throw new Error('KFIND_SITE_BASE_PATH must select a versioned site build');
}

const paths = await readdir(clientDirectory, { recursive: true });
const htmlPaths = paths.filter((path) => path.endsWith('.html'));
if (!htmlPaths.includes('index.html') || !htmlPaths.includes('404.html')) {
  throw new Error('versioned site build is missing required HTML files');
}

await Promise.all(
  htmlPaths.map(async (path) => {
    const html = await readFile(join(clientDirectory, path), 'utf8');
    if (!html.includes(`href="${basePath}/assets/`)) {
      throw new Error(
        `versioned HTML does not use the asset base path: ${path}`,
      );
    }
    if (html.includes('href="/assets/') || html.includes('src="/assets/')) {
      throw new Error(`versioned HTML references current site assets: ${path}`);
    }
    for (const reference of [
      'import "/assets/',
      'from "/assets/',
      'import("/assets/',
    ]) {
      if (html.includes(reference)) {
        throw new Error(
          `versioned HTML references current site assets: ${path}`,
        );
      }
    }
  }),
);

const javascriptPaths = paths.filter((path) => path.endsWith('.js'));
const manifestPath = javascriptPaths.find((path) =>
  path.startsWith('assets/manifest-'),
);
if (manifestPath === undefined) {
  throw new Error('versioned site build is missing the client manifest');
}
const manifest = await readFile(join(clientDirectory, manifestPath), 'utf8');
if (
  !manifest.includes(`"${basePath}/assets/`) ||
  manifest.includes('"/assets/')
) {
  throw new Error('versioned client manifest does not use the asset base path');
}
await Promise.all(
  javascriptPaths.map(async (path) => {
    const source = await readFile(join(clientDirectory, path), 'utf8');
    if (source.includes(`@kfind/kfind${basePath}/assets/`)) {
      throw new Error(`versioned build rewrote package documentation: ${path}`);
    }
  }),
);

process.stdout.write(
  `버전 문서 검사 완료: ${htmlPaths.length}개 HTML · ${basePath}\n`,
);
