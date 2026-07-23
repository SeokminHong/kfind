import type { DocumentGroupIndex } from './types';

import { RoutePath } from '../route-path';

import { localized, page } from './types';

export const guideGroup: DocumentGroupIndex = {
  labelKey: 'navigation.primary.get_started',
  categories: [
    {
      label: localized('시작', 'START'),
      pages: [
        page(
          RoutePath.GettingStarted,
          '시작하기',
          'Get started',
          '설치, 첫 검색과 자동화 출력을 한 흐름으로 설명합니다.',
          'Install kfind, run a first search, and produce automation output.',
          [
            ['cli-installation', 'native CLI 설치', 'Native CLI installation'],
            ['npm-installation', 'npm 설치', 'npm installation'],
            ['first-search', '첫 검색', 'First search'],
            ['pos-and-phrase', '품사와 구 검색', 'POS and phrase search'],
            ['automation-output', '자동화 출력', 'Automation output'],
            ['agent-skill', '에이전트 통합', 'Agent integration'],
          ],
        ),
        page(
          RoutePath.Installation,
          '설치',
          'Installation',
          'Homebrew, Cargo와 npm 배포물의 실행 환경과 포함 resource를 구분합니다.',
          'Compare the runtime and resource profiles of Homebrew, Cargo, and npm distributions.',
          [
            ['distribution-profiles', '배포 profile', 'Distribution profiles'],
            ['native-installation', 'native 설치', 'Native installation'],
            ['npm-installation', 'npm 설치', 'npm installation'],
            ['installation-check', '설치 확인', 'Installation check'],
          ],
        ),
        page(
          RoutePath.Workflows,
          '검색 절차',
          'Workflows',
          '탐색 검색, 정밀 검색과 에이전트 자동화를 재현 가능한 절차로 정리합니다.',
          'Use reproducible workflows for exploration, precise search, and agent automation.',
          [
            ['exploration', '탐색 검색', 'Exploration'],
            ['precision', '정밀 검색', 'Precision search'],
            ['automation', '자동화', 'Automation'],
          ],
        ),
        page(
          RoutePath.Goals,
          '목표와 비목표',
          'Goals and non-goals',
          '표제어 검색기의 제품 계약과 형태소 분석기가 아닌 범위를 정의합니다.',
          'Define the lemma-search contract and the boundaries that keep kfind from becoming a general analyzer.',
          [
            ['product-goals', '제품 목표', 'Product goals'],
            ['non-goals', '비목표', 'Non-goals'],
            ['selection-guide', '선택 기준', 'Selection guide'],
          ],
        ),
      ],
    },
  ],
};
