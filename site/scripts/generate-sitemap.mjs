import { writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';

import { readDocumentRoutePaths } from './document-routes.mjs';

const siteDirectory = fileURLToPath(new URL('..', import.meta.url));
const paths = await readDocumentRoutePaths();
const baseUrl = 'https://kfind.pages.dev';

function localizedUrl(path, locale) {
  const url = new URL(path, baseUrl);

  if (locale === 'en') {
    url.searchParams.set('hl', 'en');
  }
  return url.href.replaceAll('&', '&amp;');
}

function sitemapEntry(path, locale) {
  const koreanUrl = localizedUrl(path, 'ko');
  const englishUrl = localizedUrl(path, 'en');

  return [
    '  <url>',
    `    <loc>${locale === 'en' ? englishUrl : koreanUrl}</loc>`,
    `    <xhtml:link rel="alternate" hreflang="ko" href="${koreanUrl}" />`,
    `    <xhtml:link rel="alternate" hreflang="en" href="${englishUrl}" />`,
    `    <xhtml:link rel="alternate" hreflang="x-default" href="${koreanUrl}" />`,
    '  </url>',
  ].join('\n');
}

const urls = paths.flatMap((path) => [
  sitemapEntry(path, 'ko'),
  sitemapEntry(path, 'en'),
]);
const sitemap = [
  '<?xml version="1.0" encoding="UTF-8"?>',
  '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">',
  ...urls,
  '</urlset>',
  '',
].join('\n');

await writeFile(`${siteDirectory}/public/sitemap.xml`, sitemap, 'utf8');
