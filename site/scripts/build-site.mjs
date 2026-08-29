import { spawn } from 'node:child_process';
import {
  copyFile,
  cp,
  mkdir,
  readdir,
  readFile,
  rm,
  writeFile,
} from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { readDocumentRoutePaths } from './document-routes.mjs';

const siteDirectory = fileURLToPath(new URL('..', import.meta.url));
const buildDirectory = join(siteDirectory, 'build');
const clientDirectory = join(buildDirectory, 'client');
const englishAssetDirectory = join(clientDirectory, '_i18n', 'en');
const siteBasePath = readSiteBasePath();
const siteBaseRelativePath = siteBasePath === '/' ? '' : siteBasePath.slice(1);

async function runReactRouterBuild(locale) {
  await new Promise((resolve, reject) => {
    const child = spawn('react-router', ['build'], {
      cwd: siteDirectory,
      env: {
        ...process.env,
        KFIND_PRERENDER_LOCALE: locale,
      },
      stdio: 'inherit',
    });

    child.on('error', reject);
    child.on('exit', (code, signal) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(
        new Error(
          `react-router build (${locale}) failed with ${
            signal === null ? `exit code ${code}` : `signal ${signal}`
          }`,
        ),
      );
    });
  });
}

function routeHtmlFile(directory, path) {
  return path === '/'
    ? join(directory, 'index.html')
    : join(directory, `${path.slice(1)}.html`);
}

async function composeBuild() {
  const koreanBuildClientDirectory = join(buildDirectory, 'ko', 'client');
  const englishBuildClientDirectory = join(buildDirectory, 'en', 'client');
  const koreanClientDirectory = join(
    koreanBuildClientDirectory,
    siteBaseRelativePath,
  );
  const englishClientDirectory = join(
    englishBuildClientDirectory,
    siteBaseRelativePath,
  );
  const paths = await readDocumentRoutePaths();

  await cp(koreanBuildClientDirectory, clientDirectory, { recursive: true });
  if (siteBasePath !== '/') {
    await cp(koreanClientDirectory, clientDirectory, { recursive: true });
    await rm(join(clientDirectory, 'versions'), { recursive: true });
  }
  await cp(
    join(englishBuildClientDirectory, 'assets'),
    join(clientDirectory, 'assets'),
    { recursive: true },
  );

  await Promise.all(
    paths.map(async (path) => {
      const destination = routeHtmlFile(englishAssetDirectory, path);

      await mkdir(dirname(destination), { recursive: true });
      await copyFile(routeHtmlFile(englishClientDirectory, path), destination);
    }),
  );
  await copyFile(
    join(englishClientDirectory, '404.html'),
    join(englishAssetDirectory, '404.html'),
  );

  if (siteBasePath !== '/') {
    await rewriteVersionedAssetUrls();
  }
}

async function rewriteVersionedAssetUrls() {
  const paths = await readdir(clientDirectory, { recursive: true });
  const textPaths = paths.filter((path) => path.endsWith('.html'));
  await Promise.all(
    textPaths.map(async (path) => {
      const file = join(clientDirectory, path);
      const source = await readFile(file, 'utf8');
      const assetPath = `${siteBasePath}/assets/`;
      const rewritten = source
        .replaceAll('href="/assets/', `href="${assetPath}`)
        .replaceAll('src="/assets/', `src="${assetPath}`)
        .replaceAll('import "/assets/', `import "${assetPath}`)
        .replaceAll('from "/assets/', `from "${assetPath}`)
        .replaceAll('import("/assets/', `import("${assetPath}`);
      if (rewritten !== source) {
        await writeFile(file, rewritten);
      }
    }),
  );

  const manifestPath = paths.find(
    (path) => path.startsWith('assets/manifest-') && path.endsWith('.js'),
  );
  if (manifestPath === undefined) {
    throw new Error('React Router client manifest was not generated');
  }
  const manifestFile = join(clientDirectory, manifestPath);
  const manifest = await readFile(manifestFile, 'utf8');
  await writeFile(
    manifestFile,
    manifest.replaceAll('"/assets/', `"${siteBasePath}/assets/`),
  );
}

function readSiteBasePath() {
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

await rm(buildDirectory, { force: true, recursive: true });
await runReactRouterBuild('ko');
await runReactRouterBuild('en');
await composeBuild();
await rm(join(buildDirectory, 'ko'), { recursive: true });
await rm(join(buildDirectory, 'en'), { recursive: true });
