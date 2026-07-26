import { readFile } from 'node:fs/promises';

import { readDocumentRoutePaths } from './document-routes.mjs';

const locales = ['ko', 'en'];
const acronymLabelPattern = /^(?:[A-Z]{2,}|[A-Z]\d)(?:ᶜ)?$/u;
const documentRoutePaths = await readDocumentRoutePaths();
const routes = documentRoutePaths.filter((route) => route !== '/playground');
const routeResults = await Promise.all(
  routes.map((route) => checkRoute(route)),
);
const failures = routeResults.flatMap((result) => result.errors);
failures.push(...duplicateDocumentFailures(routeResults));

async function checkRoute(route) {
  const routeErrors = [];
  const sources = new Map();

  await Promise.all(
    locales.map(async (locale) => {
      const relativePath =
        route === '/' ? 'index.mdx' : `${route.slice(1)}.mdx`;
      const sourceUrl = new URL(
        `../src/documents/${locale}/${relativePath}`,
        import.meta.url,
      );
      let source;
      try {
        source = await readFile(sourceUrl, 'utf8');
      } catch {
        routeErrors.push(`${locale}:${route} MDX source가 없습니다.`);
        return;
      }

      sources.set(locale, source);
      const structuralSource = withoutFencedCode(source);
      const h1Count = [...structuralSource.matchAll(/^# (?!#)/gmu)].length;
      if (h1Count !== 1) {
        routeErrors.push(`${locale}:${route} h1이 ${h1Count}개입니다.`);
      }
      if (!/^<Eyebrow>.+<\/Eyebrow>$/mu.test(source)) {
        routeErrors.push(`${locale}:${route} eyebrow가 없습니다.`);
      }

      const glossaryLinkKeys = [
        ...structuralSource.matchAll(
          /\[(?<label>[^\]]+)\]\(\/reference\/glossary#(?<termId>[^)]+)\)/gu,
        ),
      ].map((match) => {
        const label = match.groups?.label;
        const termId = match.groups?.termId;

        return label !== undefined &&
          termId !== undefined &&
          acronymLabelPattern.test(label)
          ? `${termId}:${label}`
          : termId;
      });
      if (
        glossaryLinkKeys.some((key) => key === undefined) ||
        new Set(glossaryLinkKeys).size !== glossaryLinkKeys.length
      ) {
        routeErrors.push(
          `${locale}:${route} 단어장 link가 첫 등장 규칙을 중복합니다.`,
        );
      }

      if (locale === 'ko') {
        const structuralLabels = structuralSource
          .split('\n')
          .filter((line) => /^(?:<Eyebrow>|#{1,2}\s)/u.test(line))
          .join('\n');
        if (structuralLabels.includes('참조')) {
          routeErrors.push(
            `${locale}:${route} 구조 label에 직역어 참조가 있습니다.`,
          );
        }
      }
    }),
  );

  const korean = sources.get('ko');
  const english = sources.get('en');
  if (korean !== undefined && english !== undefined) {
    const koreanIds = headingIds(korean);
    const englishIds = headingIds(english);
    if (koreanIds.join('\n') !== englishIds.join('\n')) {
      routeErrors.push(`${route}의 한국어·영어 heading ID 순서가 다릅니다.`);
    }
  }

  return { errors: routeErrors, route, sources };
}

function duplicateDocumentFailures(results) {
  const duplicateFailures = [];

  for (const locale of locales) {
    const routeBySource = new Map();

    for (const { route, sources } of results) {
      const source = sources.get(locale);
      if (source === undefined) {
        continue;
      }

      const existingRoute = routeBySource.get(source);
      if (existingRoute !== undefined) {
        duplicateFailures.push(
          `${locale}:${route} MDX source가 ${existingRoute}와 중복됩니다.`,
        );
        continue;
      }

      routeBySource.set(source, route);
    }
  }

  return duplicateFailures;
}

const koreanCatalog = await readFile(
  new URL('../src/app/translations.ko.ts', import.meta.url),
  'utf8',
);
if (
  !koreanCatalog.includes("'navigation.primary.reference': '명세'") ||
  koreanCatalog.includes("'navigation.primary.reference': '참조'")
) {
  failures.push('한국어 명세 navigation label이 올바르지 않습니다.');
}

if (failures.length > 0) {
  throw new Error(`문서 검증 실패:\n- ${failures.join('\n- ')}`);
}

process.stdout.write(
  `문서 검증 완료: ${routes.length}개 route × ${locales.length}개 locale`,
);

function headingIds(source) {
  const structuralSource = withoutFencedCode(source);
  const ids = [
    ...structuralSource.matchAll(/^## .+ \[#(?<id>[a-z][a-z0-9-]*)\]\s*$/gmu),
  ].map((match) => match.groups?.id);

  if (ids.some((id) => id === undefined) || new Set(ids).size !== ids.length) {
    throw new Error('MDX heading ID가 없거나 중복되었습니다.');
  }
  return ids;
}

function withoutFencedCode(source) {
  let fenced = false;
  return source
    .split('\n')
    .map((line) => {
      if (line.startsWith('```')) {
        fenced = !fenced;
        return '';
      }
      return fenced ? '' : line;
    })
    .join('\n');
}
