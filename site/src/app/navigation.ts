import type { DocumentTranslationKey } from './translations.ko';

export enum RoutePath {
  Overview = '/',
  GettingStarted = '/guide/getting-started',
  Options = '/reference/options',
  Agents = '/agents',
  Glossary = '/reference/glossary',
  Analysis = '/concepts/analysis',
  Architecture = '/concepts/architecture',
  Optimization = '/concepts/optimization',
  Benchmarks = '/benchmarks',
  Playground = '/playground',
}

export const documentRoutePaths: readonly RoutePath[] =
  Object.values(RoutePath);

export interface SectionNavigationItem {
  readonly id: string;
  readonly labelKey: DocumentTranslationKey;
}

export interface NavigationItem {
  readonly labelKey: DocumentTranslationKey;
  readonly path: RoutePath;
  readonly sections: readonly SectionNavigationItem[];
}

export interface NavigationGroup {
  readonly labelKey: DocumentTranslationKey;
  readonly items: readonly NavigationItem[];
}

export interface PrimaryNavigationItem {
  readonly labelKey: DocumentTranslationKey;
  readonly path: RoutePath;
}

export const navigationGroups: readonly NavigationGroup[] = [
  {
    labelKey: 'navigation.primary.home',
    items: [
      {
        labelKey: 'navigation.item.overview',
        path: RoutePath.Overview,
        sections: [
          {
            id: 'product-purpose',
            labelKey: 'navigation.section.overview.product_purpose',
          },
          {
            id: 'search-directed-morphology',
            labelKey: 'navigation.section.overview.search_model',
          },
          {
            id: 'grammar-scope',
            labelKey: 'navigation.section.overview.grammar_scope',
          },
          {
            id: 'usage-profiles',
            labelKey: 'navigation.section.overview.usage_profiles',
          },
        ],
      },
    ],
  },
  {
    labelKey: 'navigation.primary.get_started',
    items: [
      {
        labelKey: 'navigation.item.getting_started',
        path: RoutePath.GettingStarted,
        sections: [
          {
            id: 'cli-installation',
            labelKey: 'navigation.section.getting_started.cli_installation',
          },
          {
            id: 'npm-installation',
            labelKey: 'navigation.section.getting_started.npm_installation',
          },
          {
            id: 'first-search',
            labelKey: 'navigation.section.getting_started.first_search',
          },
          {
            id: 'pos-and-phrase',
            labelKey: 'navigation.section.getting_started.pos_phrase',
          },
          {
            id: 'automation-output',
            labelKey: 'navigation.section.getting_started.automation_output',
          },
          {
            id: 'agent-skill',
            labelKey: 'navigation.section.getting_started.agent_skill',
          },
        ],
      },
    ],
  },
  {
    labelKey: 'navigation.primary.cli',
    items: [
      {
        labelKey: 'navigation.item.options',
        path: RoutePath.Options,
        sections: [
          {
            id: 'query-plan',
            labelKey: 'navigation.section.options.query_plan',
          },
          {
            id: 'expansion',
            labelKey: 'navigation.section.options.expansion',
          },
          {
            id: 'boundary',
            labelKey: 'navigation.section.options.boundary',
          },
          { id: 'pos', labelKey: 'navigation.section.options.pos' },
          {
            id: 'normalization-and-phrase',
            labelKey: 'navigation.section.options.normalization_phrase',
          },
          {
            id: 'input-and-output',
            labelKey: 'navigation.section.options.input_output',
          },
        ],
      },
    ],
  },
  {
    labelKey: 'navigation.primary.agents',
    items: [
      {
        labelKey: 'navigation.item.agents',
        path: RoutePath.Agents,
        sections: [
          {
            id: 'search-primitive',
            labelKey: 'navigation.section.agents.search_primitive',
          },
          {
            id: 'recommended-workflow',
            labelKey: 'navigation.section.agents.workflow',
          },
          {
            id: 'skill-installation',
            labelKey: 'navigation.section.agents.skill_installation',
          },
          {
            id: 'supported-agents',
            labelKey: 'navigation.section.agents.supported_agents',
          },
          {
            id: 'automation-patterns',
            labelKey: 'navigation.section.agents.automation',
          },
          {
            id: 'integration-contract',
            labelKey: 'navigation.section.agents.contract',
          },
        ],
      },
    ],
  },
  {
    labelKey: 'navigation.primary.internals',
    items: [
      {
        labelKey: 'navigation.item.analysis',
        path: RoutePath.Analysis,
        sections: [
          {
            id: 'analysis-direction',
            labelKey: 'navigation.section.analysis.direction',
          },
          {
            id: 'lexicon-layers',
            labelKey: 'navigation.section.analysis.lexicons',
          },
          {
            id: 'particles-and-allomorphs',
            labelKey: 'navigation.section.analysis.particles',
          },
          {
            id: 'endings',
            labelKey: 'navigation.section.analysis.endings',
          },
          {
            id: 'irregulars-and-contractions',
            labelKey: 'navigation.section.analysis.irregulars',
          },
          {
            id: 'derivation-and-compounds',
            labelKey: 'navigation.section.analysis.derivation',
          },
          {
            id: 'structural-verification',
            labelKey: 'navigation.section.analysis.structural',
          },
        ],
      },
      {
        labelKey: 'navigation.item.architecture',
        path: RoutePath.Architecture,
        sections: [
          {
            id: 'query-and-corpus-lanes',
            labelKey: 'navigation.section.architecture.lanes',
          },
          {
            id: 'candidate-programs',
            labelKey: 'navigation.section.architecture.programs',
          },
          {
            id: 'local-verification',
            labelKey: 'navigation.section.architecture.verification',
          },
          {
            id: 'phrase-spans',
            labelKey: 'navigation.section.architecture.phrase',
          },
          {
            id: 'parallel-output',
            labelKey: 'navigation.section.architecture.output',
          },
          {
            id: 'execution-surfaces',
            labelKey: 'navigation.section.architecture.surfaces',
          },
        ],
      },
      {
        labelKey: 'navigation.item.optimization',
        path: RoutePath.Optimization,
        sections: [
          {
            id: 'cost-separation',
            labelKey: 'navigation.section.optimization.cost',
          },
          {
            id: 'anchors-and-matchers',
            labelKey: 'navigation.section.optimization.anchors',
          },
          {
            id: 'plan-limits',
            labelKey: 'navigation.section.optimization.limits',
          },
          {
            id: 'resource-initialization',
            labelKey: 'navigation.section.optimization.resources',
          },
          {
            id: 'scan-path',
            labelKey: 'navigation.section.optimization.scan',
          },
          {
            id: 'metric-boundaries',
            labelKey: 'navigation.section.optimization.metrics',
          },
        ],
      },
    ],
  },
  {
    labelKey: 'navigation.primary.benchmarks',
    items: [
      {
        labelKey: 'navigation.item.benchmarks',
        path: RoutePath.Benchmarks,
        sections: [
          {
            id: 'evaluation-scope',
            labelKey: 'navigation.section.benchmarks.scope',
          },
          {
            id: 'quality-contract',
            labelKey: 'navigation.section.benchmarks.contract',
          },
          {
            id: 'canonical-quality',
            labelKey: 'navigation.section.benchmarks.canonical',
          },
          {
            id: 'query-matrix-quality',
            labelKey: 'navigation.section.benchmarks.query_matrix',
          },
          {
            id: 'robust-quality',
            labelKey: 'navigation.section.benchmarks.robust',
          },
          {
            id: 'performance-units',
            labelKey: 'navigation.section.benchmarks.performance',
          },
          {
            id: 'source-evidence',
            labelKey: 'navigation.section.benchmarks.sources',
          },
        ],
      },
    ],
  },
  {
    labelKey: 'navigation.primary.reference',
    items: [
      {
        labelKey: 'navigation.item.glossary',
        path: RoutePath.Glossary,
        sections: [
          {
            id: 'search',
            labelKey: 'navigation.section.glossary.search',
          },
          {
            id: 'grammar',
            labelKey: 'navigation.section.glossary.grammar',
          },
          {
            id: 'execution',
            labelKey: 'navigation.section.glossary.execution',
          },
          {
            id: 'resource',
            labelKey: 'navigation.section.glossary.resource',
          },
          {
            id: 'quality',
            labelKey: 'navigation.section.glossary.quality',
          },
        ],
      },
    ],
  },
];

function firstNavigationItem(group: NavigationGroup): NavigationItem {
  const item = group.items[0];
  if (item === undefined) {
    throw new Error(`navigation group ${group.labelKey} has no pages`);
  }

  return item;
}

export const primaryNavigationItems: readonly PrimaryNavigationItem[] =
  navigationGroups.map((group) => ({
    labelKey: group.labelKey,
    path: firstNavigationItem(group).path,
  }));

export function routePathFromPathname(pathname: string): RoutePath {
  const candidate = pathname as RoutePath;
  return Object.values(RoutePath).includes(candidate)
    ? candidate
    : RoutePath.Overview;
}

export function navigationGroupForPath(pathname: RoutePath): NavigationGroup {
  const group = navigationGroups.find((candidate) =>
    candidate.items.some((item) => item.path === pathname),
  );
  if (group !== undefined) {
    return group;
  }

  const fallback = navigationGroups[0];
  if (fallback === undefined) {
    throw new Error('documentation navigation has no groups');
  }
  return fallback;
}
