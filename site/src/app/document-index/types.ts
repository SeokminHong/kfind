import type { RoutePath } from '../route-path';
import type { DocumentTranslationKey } from '../translations.ko';

export interface LocalizedText {
  readonly en: string;
  readonly ko: string;
}

export interface DocumentSectionIndex {
  readonly id: string;
  readonly label: LocalizedText;
}

export interface DocumentPageIndex {
  readonly description: LocalizedText;
  readonly label: LocalizedText;
  readonly path: RoutePath;
  readonly sections: readonly DocumentSectionIndex[];
}

export interface DocumentCategoryIndex {
  readonly label?: LocalizedText;
  readonly pages: readonly DocumentPageIndex[];
}

export interface DocumentGroupIndex {
  readonly categories: readonly DocumentCategoryIndex[];
  readonly labelKey: DocumentTranslationKey;
}

export type SectionDefinition = readonly [id: string, ko: string, en: string];

export function localized(ko: string, en: string): LocalizedText {
  return { en, ko };
}

export function page(
  path: RoutePath,
  ko: string,
  en: string,
  descriptionKo: string,
  descriptionEn: string,
  sections: readonly SectionDefinition[],
): DocumentPageIndex {
  return {
    description: localized(descriptionKo, descriptionEn),
    label: localized(ko, en),
    path,
    sections: sections.map(([id, sectionKo, sectionEn]) => ({
      id,
      label: localized(sectionKo, sectionEn),
    })),
  };
}
