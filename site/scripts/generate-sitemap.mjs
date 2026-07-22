import { readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

const siteDirectory = fileURLToPath(new URL('..', import.meta.url));
const routePathSource = await readFile(
  new URL('../src/app/route-path.ts', import.meta.url),
  'utf8',
);
const paths = [
  ...routePathSource.matchAll(/^\s+\w+: '(?<path>[^']+)',$/gmu),
].map((match) => match.groups?.path);

if (
  paths.length === 0 ||
  paths.some((path) => path === undefined) ||
  new Set(paths).size !== paths.length
) {
  throw new Error('문서 경로가 없거나 중복되었습니다.');
}

const baseUrl = 'https://kfind.pages.dev';
const urls = paths.map((path) => {
  if (path === undefined) {
    throw new Error('문서 경로를 읽지 못했습니다.');
  }
  return `  <url><loc>${baseUrl}${path}</loc></url>`;
});
const sitemap = [
  '<?xml version="1.0" encoding="UTF-8"?>',
  '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">',
  ...urls,
  '</urlset>',
  '',
].join('\n');

await writeFile(`${siteDirectory}/public/sitemap.xml`, sitemap, 'utf8');
