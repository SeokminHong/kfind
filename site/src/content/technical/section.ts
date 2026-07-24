import type { TechnicalSectionContent } from './types';

export function section(
  title: string,
  paragraphs: readonly string[],
  details: Pick<TechnicalSectionContent, 'code' | 'items' | 'links'> = {},
): TechnicalSectionContent {
  return { ...details, paragraphs, title };
}
