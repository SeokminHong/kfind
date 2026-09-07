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
            [
              'product-purpose',
              '첫 검색과 문서 안내',
              'First search and documentation',
            ],
            ['usage-profiles', '사용 환경', 'Usage environments'],
            ['grammar-scope', '검색 범위와 한계', 'Search scope and limits'],
            [
              'search-directed-morphology',
              '검색 엔진 구조',
              'Search engine structure',
            ],
          ],
        ),
      ],
    },
  ],
};
