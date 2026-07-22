import type { DocumentGroupIndex } from './types';

import { RoutePath } from '../route-path';

import { page } from './types';

export const homeGroup: DocumentGroupIndex = {
  labelKey: 'navigation.primary.home',
  categories: [
    {
      pages: [
        page(
          RoutePath.Overview,
          '개요',
          'Overview',
          '한국어 표제어 검색의 제품 범위, 문법 범위와 실행 profile을 설명합니다.',
          'Understand the product, grammar, and execution scope of Korean lemma search.',
          [
            ['product-purpose', '제품 목적', 'Product purpose'],
            [
              'search-directed-morphology',
              '검색 지향 형태 처리',
              'Search-directed morphology',
            ],
            ['grammar-scope', '문법 범위', 'Grammar scope'],
            ['usage-profiles', '사용 profile', 'Usage profiles'],
          ],
        ),
      ],
    },
  ],
};
