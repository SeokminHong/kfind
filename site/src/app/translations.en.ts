import type { DocumentTranslationKey } from './translations.ko';

export const englishTranslation: Readonly<
  Record<DocumentTranslationKey, string>
> = {
  'common.brand.document_suffix': 'Docs',
  'common.brand.home_aria': 'kfind documentation home',
  'common.footer.license': 'MIT License',
  'common.header.external_aria': 'External documentation',
  'common.language.aria': 'Document language',
  'common.language.english': 'English',
  'common.language.korean': '한국어',
  'common.loading.document': 'Loading documentation…',
  'common.mobile_navigation.trigger': 'Documentation menu',
  'common.navigation.toc_aria': 'Documentation table of contents',
  'common.skip_to_content': 'Skip to content',
  'metadata.analysis.description':
    'The morphology model that compiles query lemmas into searchable plans.',
  'metadata.analysis.title': 'Morphology',
  'metadata.architecture.description':
    'The compile, anchor scan, local verification, and output architecture.',
  'metadata.architecture.title': 'Architecture',
  'metadata.benchmarks.description':
    'Quality and performance contracts with raw and contract-adjusted metrics.',
  'metadata.benchmarks.title': 'Benchmarks',
  'metadata.getting_started.description':
    'Installation and the first lemma-aware search.',
  'metadata.getting_started.title': 'Getting started',
  'metadata.glossary.description':
    'Definitions for search, Korean grammar, and benchmark metrics.',
  'metadata.glossary.title': 'Glossary',
  'metadata.not_found.description':
    'The requested kfind documentation path does not exist.',
  'metadata.not_found.title': 'Page not found',
  'metadata.optimization.description':
    'Bounded compilation, byte scanning, resource loading, and streaming design.',
  'metadata.optimization.title': 'Optimization',
  'metadata.options.description':
    'Query grammar, expansion, boundary, POS, normalization, and phrase options.',
  'metadata.options.title': 'Query and options',
  'metadata.overview.description':
    'The purpose, scope, and execution model of kfind.',
  'metadata.overview.title': 'Overview',
  'metadata.playground.description':
    'Run the kfind WebAssembly search engine in the browser.',
  'metadata.playground.title': 'Playground',
  'navigation.group.evidence': 'Evidence',
  'navigation.group.internals': 'Internals',
  'navigation.group.reference': 'Reference',
  'navigation.group.start': 'Start',
  'navigation.item.analysis': 'Morphology',
  'navigation.item.architecture': 'Architecture',
  'navigation.item.benchmarks': 'Benchmarks',
  'navigation.item.getting_started': 'Getting started',
  'navigation.item.glossary': 'Glossary',
  'navigation.item.optimization': 'Optimization',
  'navigation.item.options': 'Query and options',
  'navigation.item.overview': 'Overview',
  'navigation.item.playground': 'Playground',
};
