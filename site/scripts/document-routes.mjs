import { readFile } from 'node:fs/promises';

const routePathSourceUrl = new URL('../src/app/route-path.ts', import.meta.url);

export async function readDocumentRoutePaths() {
  const source = await readFile(routePathSourceUrl, 'utf8');
  const paths = [...source.matchAll(/^\s+\w+: '(?<path>[^']+)',$/gmu)].map(
    (match) => match.groups?.path,
  );

  if (
    paths.length === 0 ||
    paths.some((path) => path === undefined) ||
    new Set(paths).size !== paths.length
  ) {
    throw new Error('문서 경로가 없거나 중복되었습니다.');
  }

  return paths;
}
