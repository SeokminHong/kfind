import { readdir, readFile } from 'node:fs/promises';
import { join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

import { readDocumentRoutePaths } from './document-routes.mjs';

const siteDirectory = fileURLToPath(new URL('..', import.meta.url));
const clientDirectory = join(siteDirectory, 'build', 'client');
const siteOrigin = 'https://kfind.pages.dev';
const socialImageUrl = new URL('/social-card.png', siteOrigin).href;
const localeSettings = {
  en: {
    documentLanguage: 'en',
    openGraphAlternateLocale: 'ko_KR',
    openGraphLocale: 'en_US',
  },
  ko: {
    documentLanguage: 'ko',
    openGraphAlternateLocale: 'en_US',
    openGraphLocale: 'ko_KR',
  },
};
const homeSeo = {
  en: {
    description:
      'Search 걷다 and find 걷고, 걸어, and 걸었다. kfind is a fast Korean lemma and inflection search engine for files, source code, and documentation through its CLI, Rust, and WebAssembly interfaces.',
    heading: 'Search Korean lemmas and inflections with kfind',
    title: 'kfind | Korean Lemma & Inflection Search Engine',
  },
  ko: {
    description:
      'kfind는 ‘걷다’로 ‘걷고’, ‘걸어’, ‘걸었다’까지 찾는 한국어 활용형 검색 엔진입니다. CLI, Rust, WebAssembly로 파일·코드·문서를 빠르게 검색합니다.',
    heading: '한국어 활용형까지 찾는 검색 엔진, kfind',
    title: 'kfind | 한국어 활용형·표제어 검색 엔진',
  },
};

function fail(message) {
  throw new Error(`SEO 검사 실패: ${message}`);
}

function escapePattern(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, String.raw`\$&`);
}

function decodeHtml(value) {
  return value
    .replaceAll('&amp;', '&')
    .replaceAll('&quot;', '"')
    .replaceAll('&#x27;', "'")
    .replaceAll('&#39;', "'");
}

function attributeFromTag(html, tagName, key, value, attribute) {
  const tag = html.match(
    new RegExp(
      String.raw`<${tagName}\s+[^>]*${key}="${escapePattern(value)}"[^>]*>`,
      'u',
    ),
  )?.[0];
  const attributeValue = tag?.match(
    new RegExp(String.raw`${attribute}="(?<value>[^"]*)"`, 'u'),
  )?.groups?.value;

  return attributeValue === undefined ? undefined : decodeHtml(attributeValue);
}

function metaContent(html, key, value) {
  return attributeFromTag(html, 'meta', key, value, 'content');
}

function linkHref(html, relation) {
  return attributeFromTag(html, 'link', 'rel', relation, 'href');
}

function typedLinkHref(html, type) {
  return attributeFromTag(html, 'link', 'type', type, 'href');
}

function alternateHref(html, language) {
  return attributeFromTag(html, 'link', 'hrefLang', language, 'href');
}

function titleFromHtml(html) {
  const title = html.match(/<title>(?<title>[^<]+)<\/title>/u)?.groups?.title;

  return title === undefined ? undefined : decodeHtml(title);
}

function headingFromHtml(html) {
  const heading = html.match(/<h1(?:\s[^>]*)?>(?<heading>.*?)<\/h1>/su)?.groups
    ?.heading;

  return heading === undefined
    ? undefined
    : decodeHtml(heading.replaceAll(/<!--.*?-->|<[^>]+>/gsu, ''));
}

function glossaryLinkCount(source) {
  return [...source.matchAll(/\[[^\]]+\]\(\/reference\/glossary#[^)]+\)/gu)]
    .length;
}

function glossaryTooltipCount(html) {
  return [
    ...html.matchAll(
      /<a(?=[^>]*\baria-describedby=")(?=[^>]*\bhref="\/reference\/glossary(?:\?hl=en)?#[^"]+")[^>]*>/gu,
    ),
  ].length;
}

function structuredDataFromHtml(html) {
  const scripts = [
    ...html.matchAll(
      /<script type="application\/ld\+json">(?<json>.*?)<\/script>/gu,
    ),
  ];
  if (scripts.length !== 1) {
    return fail(`JSON-LD script 수가 ${scripts.length}개입니다.`);
  }

  const json = scripts[0]?.groups?.json;
  if (json === undefined) {
    return fail('JSON-LD를 읽지 못했습니다.');
  }

  try {
    return JSON.parse(json);
  } catch {
    return fail('JSON-LD가 유효한 JSON이 아닙니다.');
  }
}

function localizedUrl(path, locale) {
  const url = new URL(path, siteOrigin);

  if (locale === 'en') {
    url.searchParams.set('hl', 'en');
  }
  return url.href;
}

function routeHtmlFile(path, locale) {
  const directory =
    locale === 'en' ? join(clientDirectory, '_i18n', 'en') : clientDirectory;

  return path === '/'
    ? join(directory, 'index.html')
    : join(directory, `${path.slice(1)}.html`);
}

function routeDocumentSourceFile(path, locale) {
  const relativePath = path === '/' ? 'index.mdx' : `${path.slice(1)}.mdx`;

  return join(siteDirectory, 'src', 'documents', locale, relativePath);
}

function assertEqual(actual, expected, context) {
  if (actual !== expected) {
    fail(`${context}: "${expected}" 대신 "${actual ?? '없음'}"`);
  }
}

function assertIndexableDocument(
  path,
  html,
  source,
  locale,
  titles,
  descriptions,
) {
  const settings = localeSettings[locale];
  const canonicalUrl = localizedUrl(path, locale);
  const koreanUrl = localizedUrl(path, 'ko');
  const englishUrl = localizedUrl(path, 'en');
  const title = titleFromHtml(html);
  const description = metaContent(html, 'name', 'description');

  assertEqual(
    typedLinkHref(html, 'image/svg+xml'),
    '/favicon.svg',
    `${path} SVG favicon`,
  );
  assertEqual(
    typedLinkHref(html, 'image/x-icon'),
    '/favicon.ico',
    `${path} ICO favicon`,
  );
  assertEqual(
    linkHref(html, 'apple-touch-icon'),
    '/apple-touch-icon.png',
    `${path} touch icon`,
  );

  if (
    !html.startsWith(
      `<!DOCTYPE html><html lang="${settings.documentLanguage}" dir="ltr">`,
    )
  ) {
    fail(`${path} ${locale} document language가 올바르지 않습니다.`);
  }
  if ((html.match(/<h1(?:\s|>)/gu) ?? []).length !== 1) {
    fail(`${path}에 단일 h1이 없습니다.`);
  }
  if (/<a(?:\s|>)[^>]*>(?:(?!<\/a>).)*<a(?:\s|>)/su.test(html)) {
    fail(`${path} ${locale}에 중첩 link가 있습니다.`);
  }
  if (source !== undefined) {
    assertEqual(
      glossaryTooltipCount(html),
      glossaryLinkCount(source),
      `${path} ${locale} MDX 단어장 tooltip 수`,
    );
  }
  if (title === undefined || title.length === 0) {
    fail(`${path}의 title이 없습니다.`);
  }
  if (description === undefined || description.length === 0) {
    fail(`${path}의 description이 없습니다.`);
  }
  const localizedTitle = `${locale}:${title}`;
  const localizedDescription = `${locale}:${description}`;
  if (titles.has(localizedTitle)) {
    fail(`${path}의 title "${title}"이 중복됩니다.`);
  }
  if (descriptions.has(localizedDescription)) {
    fail(`${path}의 description "${description}"이 중복됩니다.`);
  }
  titles.add(localizedTitle);
  descriptions.add(localizedDescription);

  assertEqual(linkHref(html, 'canonical'), canonicalUrl, `${path} canonical`);
  assertEqual(
    alternateHref(html, 'ko'),
    koreanUrl,
    `${path} ${locale} hreflang ko`,
  );
  assertEqual(
    alternateHref(html, 'en'),
    englishUrl,
    `${path} ${locale} hreflang en`,
  );
  assertEqual(
    alternateHref(html, 'x-default'),
    koreanUrl,
    `${path} ${locale} hreflang x-default`,
  );
  assertEqual(
    metaContent(html, 'property', 'og:title'),
    title,
    `${path} og:title`,
  );
  assertEqual(
    metaContent(html, 'property', 'og:description'),
    description,
    `${path} og:description`,
  );
  assertEqual(
    metaContent(html, 'property', 'og:type'),
    'website',
    `${path} og:type`,
  );
  assertEqual(
    metaContent(html, 'property', 'og:url'),
    canonicalUrl,
    `${path} og:url`,
  );
  assertEqual(
    metaContent(html, 'property', 'og:site_name'),
    'kfind',
    `${path} og:site_name`,
  );
  assertEqual(
    metaContent(html, 'property', 'og:locale'),
    settings.openGraphLocale,
    `${path} og:locale`,
  );
  assertEqual(
    metaContent(html, 'property', 'og:locale:alternate'),
    settings.openGraphAlternateLocale,
    `${path} og:locale:alternate`,
  );
  assertEqual(
    metaContent(html, 'property', 'og:image'),
    socialImageUrl,
    `${path} og:image`,
  );
  assertEqual(
    metaContent(html, 'property', 'og:image:type'),
    'image/png',
    `${path} og:image:type`,
  );
  assertEqual(
    metaContent(html, 'property', 'og:image:width'),
    '1200',
    `${path} og:image:width`,
  );
  assertEqual(
    metaContent(html, 'property', 'og:image:height'),
    '630',
    `${path} og:image:height`,
  );
  if (metaContent(html, 'property', 'og:image:alt') === undefined) {
    fail(`${path}의 og:image:alt가 없습니다.`);
  }

  assertEqual(
    metaContent(html, 'name', 'twitter:card'),
    'summary_large_image',
    `${path} twitter:card`,
  );
  assertEqual(
    metaContent(html, 'name', 'twitter:title'),
    title,
    `${path} twitter:title`,
  );
  assertEqual(
    metaContent(html, 'name', 'twitter:description'),
    description,
    `${path} twitter:description`,
  );
  assertEqual(
    metaContent(html, 'name', 'twitter:image'),
    socialImageUrl,
    `${path} twitter:image`,
  );
  if (metaContent(html, 'name', 'twitter:image:alt') === undefined) {
    fail(`${path}의 twitter:image:alt가 없습니다.`);
  }
  if (metaContent(html, 'name', 'robots')?.includes('noindex') === true) {
    fail(`${path}에 noindex가 설정되었습니다.`);
  }

  const structuredData = structuredDataFromHtml(html);
  assertEqual(
    structuredData['@context'],
    'https://schema.org',
    `${path} JSON-LD context`,
  );

  if (path === '/') {
    assertEqual(structuredData['@type'], 'WebSite', '/ JSON-LD type');
    assertEqual(structuredData.name, 'kfind', '/ WebSite name');
    assertEqual(
      JSON.stringify(structuredData.alternateName),
      JSON.stringify(['kfind Korean Search', 'kfind 한국어 검색']),
      '/ WebSite alternateName',
    );
    assertEqual(structuredData.url, canonicalUrl, '/ WebSite URL');
    assertEqual(title, homeSeo[locale].title, `/ ${locale} title`);
    assertEqual(
      description,
      homeSeo[locale].description,
      `/ ${locale} description`,
    );
    assertEqual(
      headingFromHtml(html),
      homeSeo[locale].heading,
      `/ ${locale} h1`,
    );
    return;
  }

  assertEqual(
    structuredData['@type'],
    'BreadcrumbList',
    `${path} JSON-LD type`,
  );
  const items = structuredData.itemListElement;
  if (!Array.isArray(items) || items.length < 2 || items.length > 3) {
    fail(`${path}의 breadcrumb 계층이 유효하지 않습니다.`);
  }
  assertEqual(items[0]?.['@type'], 'ListItem', `${path} breadcrumb 1 type`);
  assertEqual(items[0]?.position, 1, `${path} breadcrumb 1 position`);
  assertEqual(items[0]?.name, 'kfind', `${path} breadcrumb 1 name`);
  assertEqual(
    items[0]?.item,
    localizedUrl('/', locale),
    `${path} breadcrumb 1 URL`,
  );
  const current = items.at(-1);
  assertEqual(
    current?.['@type'],
    'ListItem',
    `${path} breadcrumb current type`,
  );
  assertEqual(
    current?.position,
    items.length,
    `${path} breadcrumb current position`,
  );
  if (typeof current?.name !== 'string' || current.name.length === 0) {
    fail(`${path} breadcrumb current name이 없습니다.`);
  }
  assertEqual(current?.item, canonicalUrl, `${path} breadcrumb current URL`);
  if (items.length === 3) {
    assertEqual(items[1]?.['@type'], 'ListItem', `${path} breadcrumb GNB type`);
    assertEqual(items[1]?.position, 2, `${path} breadcrumb GNB position`);
    if (
      typeof items[1]?.name !== 'string' ||
      items[1].name.length === 0 ||
      items[1]?.item === localizedUrl('/', locale) ||
      items[1]?.item === canonicalUrl
    ) {
      fail(`${path}의 GNB breadcrumb가 유효하지 않습니다.`);
    }
  }
}

async function assertPngImage(filename, width, height, label) {
  const image = await readFile(join(clientDirectory, filename));
  const pngSignature = '89504e470d0a1a0a';

  if (image.subarray(0, 8).toString('hex') !== pngSignature) {
    fail(`${label}가 PNG 형식이 아닙니다.`);
  }
  assertEqual(image.readUInt32BE(16), width, `${label} 너비`);
  assertEqual(image.readUInt32BE(20), height, `${label} 높이`);
}

async function assertBrandAssets() {
  const faviconSvg = await readFile(
    join(clientDirectory, 'favicon.svg'),
    'utf8',
  );
  if (
    !faviconSvg.includes('viewBox="0 0 256 256"') ||
    !faviconSvg.includes('fill="#1d63c7"') ||
    !faviconSvg.includes('id="brand-k"') ||
    faviconSvg.includes('stroke-linecap=')
  ) {
    fail('favicon.svg가 kfind brand mark를 포함하지 않습니다.');
  }

  const socialCardSvg = await readFile(
    join(clientDirectory, 'social-card.svg'),
    'utf8',
  );
  if (
    !socialCardSvg.includes('fill="#1d63c7"') ||
    !socialCardSvg.includes('id="brand-k"') ||
    socialCardSvg.includes('stroke-linecap=')
  ) {
    fail('social-card.svg의 brand mark path가 유효하지 않습니다.');
  }

  const faviconIco = await readFile(join(clientDirectory, 'favicon.ico'));
  if (faviconIco.subarray(0, 6).toString('hex') !== '000001000100') {
    fail('favicon.ico가 단일 Windows icon 형식이 아닙니다.');
  }

  await Promise.all([
    assertPngImage('apple-touch-icon.png', 180, 180, 'touch icon'),
    assertPngImage('icon-256.png', 256, 256, 'package icon'),
    assertPngImage('social-card.png', 1200, 630, 'social card'),
  ]);
}

async function main() {
  const paths = await readDocumentRoutePaths();
  const titles = new Set();
  const descriptions = new Set();

  const documents = await Promise.all(
    Object.keys(localeSettings).flatMap((locale) =>
      paths.map(async (path) => {
        const source =
          path === '/playground'
            ? undefined
            : await readFile(routeDocumentSourceFile(path, locale), 'utf8');

        return {
          html: await readFile(routeHtmlFile(path, locale), 'utf8'),
          locale,
          path,
          source,
        };
      }),
    ),
  );
  for (const { html, locale, path, source } of documents) {
    assertIndexableDocument(path, html, source, locale, titles, descriptions);
  }

  const sitemap = await readFile(join(clientDirectory, 'sitemap.xml'), 'utf8');
  const sitemapUrls = [...sitemap.matchAll(/<loc>(?<url>[^<]+)<\/loc>/gu)].map(
    (match) => match.groups?.url,
  );
  const expectedUrls = paths.flatMap((path) => [
    localizedUrl(path, 'ko'),
    localizedUrl(path, 'en'),
  ]);
  assertEqual(
    JSON.stringify(sitemapUrls),
    JSON.stringify(expectedUrls),
    'sitemap URL 집합',
  );
  const sitemapEntries = [
    ...sitemap.matchAll(/<url>(?<entry>.*?)<\/url>/gsu),
  ].map((match) => match.groups?.entry);
  assertEqual(sitemapEntries.length, expectedUrls.length, 'sitemap entry 수');
  for (const [index, entry] of sitemapEntries.entries()) {
    if (entry === undefined) {
      fail(`sitemap entry ${index}를 읽지 못했습니다.`);
    }
    const path = paths[Math.floor(index / 2)];
    if (path === undefined) {
      fail(`sitemap entry ${index}의 route가 없습니다.`);
    }
    for (const [language, expected] of [
      ['ko', localizedUrl(path, 'ko')],
      ['en', localizedUrl(path, 'en')],
      ['x-default', localizedUrl(path, 'ko')],
    ]) {
      const actual = entry.match(
        new RegExp(
          String.raw`<xhtml:link\s+[^>]*hreflang="${language}"[^>]*href="(?<href>[^"]+)"`,
          'u',
        ),
      )?.groups?.href;
      assertEqual(actual, expected, `sitemap ${path} ${language} alternate`);
    }
  }

  const robots = await readFile(join(clientDirectory, 'robots.txt'), 'utf8');
  if (!robots.includes(`Sitemap: ${siteOrigin}/sitemap.xml`)) {
    fail('robots.txt가 canonical sitemap을 가리키지 않습니다.');
  }

  const notFound = await readFile(join(clientDirectory, '404.html'), 'utf8');
  if (metaContent(notFound, 'name', 'robots') !== 'noindex') {
    fail('404.html에 noindex가 없습니다.');
  }

  const clientPaths = await readdir(clientDirectory, { recursive: true });
  const htmlFiles = clientPaths
    .filter((path) => path.endsWith('.html'))
    .map((path) => relative(clientDirectory, join(clientDirectory, path)))
    .sort();
  const expectedHtmlFiles = [
    '404.html',
    ...paths.map((path) =>
      path === '/' ? 'index.html' : `${path.slice(1)}.html`,
    ),
    ...paths.map((path) =>
      path === '/'
        ? join('_i18n', 'en', 'index.html')
        : join('_i18n', 'en', `${path.slice(1)}.html`),
    ),
  ].sort();
  assertEqual(
    JSON.stringify(htmlFiles),
    JSON.stringify(expectedHtmlFiles),
    'prerender HTML 집합',
  );

  await assertBrandAssets();
  process.stdout.write(
    `SEO 검사 완료: ${paths.length}개 route × ${Object.keys(localeSettings).length}개 locale\n`,
  );
}

await main();
