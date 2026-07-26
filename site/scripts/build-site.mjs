import { spawn } from 'node:child_process';
import { copyFile, cp, mkdir, rm } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { readDocumentRoutePaths } from './document-routes.mjs';

const siteDirectory = fileURLToPath(new URL('..', import.meta.url));
const buildDirectory = join(siteDirectory, 'build');
const clientDirectory = join(buildDirectory, 'client');
const englishAssetDirectory = join(clientDirectory, '_i18n', 'en');

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
  const koreanClientDirectory = join(buildDirectory, 'ko', 'client');
  const englishClientDirectory = join(buildDirectory, 'en', 'client');
  const paths = await readDocumentRoutePaths();

  await cp(koreanClientDirectory, clientDirectory, { recursive: true });
  await cp(
    join(englishClientDirectory, 'assets'),
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
}

await rm(buildDirectory, { force: true, recursive: true });
await runReactRouterBuild('ko');
await runReactRouterBuild('en');
await composeBuild();
await rm(join(buildDirectory, 'ko'), { recursive: true });
await rm(join(buildDirectory, 'en'), { recursive: true });
