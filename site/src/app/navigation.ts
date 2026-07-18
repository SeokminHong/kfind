import type { DocumentTranslationKey } from './translations.ko';

export enum RoutePath {
  Overview = '/',
  GettingStarted = '/guide/getting-started',
  Options = '/reference/options',
  Glossary = '/reference/glossary',
  Analysis = '/concepts/analysis',
  Architecture = '/concepts/architecture',
  Optimization = '/concepts/optimization',
  Benchmarks = '/benchmarks',
  Playground = '/playground',
}

export const documentRoutePaths: readonly RoutePath[] =
  Object.values(RoutePath);

export interface NavigationItem {
  readonly labelKey: DocumentTranslationKey;
  readonly path: RoutePath;
}

export interface NavigationGroup {
  readonly labelKey: DocumentTranslationKey;
  readonly items: readonly NavigationItem[];
}

export const navigationGroups: readonly NavigationGroup[] = [
  {
    labelKey: 'navigation.group.start',
    items: [
      { labelKey: 'navigation.item.overview', path: RoutePath.Overview },
      {
        labelKey: 'navigation.item.getting_started',
        path: RoutePath.GettingStarted,
      },
      { labelKey: 'navigation.item.playground', path: RoutePath.Playground },
    ],
  },
  {
    labelKey: 'navigation.group.reference',
    items: [
      { labelKey: 'navigation.item.options', path: RoutePath.Options },
      { labelKey: 'navigation.item.glossary', path: RoutePath.Glossary },
    ],
  },
  {
    labelKey: 'navigation.group.internals',
    items: [
      { labelKey: 'navigation.item.analysis', path: RoutePath.Analysis },
      {
        labelKey: 'navigation.item.architecture',
        path: RoutePath.Architecture,
      },
      {
        labelKey: 'navigation.item.optimization',
        path: RoutePath.Optimization,
      },
    ],
  },
  {
    labelKey: 'navigation.group.evidence',
    items: [
      { labelKey: 'navigation.item.benchmarks', path: RoutePath.Benchmarks },
    ],
  },
];
