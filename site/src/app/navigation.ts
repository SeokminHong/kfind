export enum RoutePath {
  Overview = '/',
  GettingStarted = '/guide/getting-started',
  Options = '/reference/options',
  Analysis = '/concepts/analysis',
  Architecture = '/concepts/architecture',
  Optimization = '/concepts/optimization',
  Benchmarks = '/benchmarks',
  Playground = '/playground',
}

export interface NavigationItem {
  readonly label: string;
  readonly path: RoutePath;
}

export interface NavigationGroup {
  readonly label: string;
  readonly items: readonly NavigationItem[];
}

export const navigationGroups: readonly NavigationGroup[] = [
  {
    label: '시작',
    items: [
      { label: '개요', path: RoutePath.Overview },
      { label: '시작하기', path: RoutePath.GettingStarted },
      { label: 'Playground', path: RoutePath.Playground },
    ],
  },
  {
    label: '참조',
    items: [{ label: '쿼리와 옵션', path: RoutePath.Options }],
  },
  {
    label: '내부 원리',
    items: [
      { label: '형태 분석', path: RoutePath.Analysis },
      { label: '아키텍처', path: RoutePath.Architecture },
      { label: '설계와 최적화', path: RoutePath.Optimization },
    ],
  },
  {
    label: '근거',
    items: [{ label: '벤치마크', path: RoutePath.Benchmarks }],
  },
];
