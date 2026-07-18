export const koreanTranslation = {
  'common.brand.document_suffix': '문서',
  'common.brand.home_aria': 'kfind 문서 처음으로',
  'common.footer.license': 'MIT License',
  'common.header.external_aria': '외부 문서',
  'common.loading.document': '문서를 불러오는 중…',
  'common.mobile_navigation.trigger': '문서 메뉴',
  'common.navigation.toc_aria': '문서 목차',
  'common.skip_to_content': '본문으로 건너뛰기',
  'metadata.analysis.description':
    'Query에서 표면형과 verifier를 만드는 형태 분석 원리를 설명합니다.',
  'metadata.analysis.title': '형태 분석',
  'metadata.architecture.description':
    'Query compile부터 anchor scan, verifier와 출력까지의 구조를 설명합니다.',
  'metadata.architecture.title': '아키텍처',
  'metadata.benchmarks.description':
    'kfind의 품질과 성능 측정 계약을 설명합니다.',
  'metadata.benchmarks.title': '벤치마크',
  'metadata.getting_started.description':
    'kfind 설치와 첫 형태 검색 방법을 설명합니다.',
  'metadata.getting_started.title': '시작하기',
  'metadata.glossary.description':
    'kfind 문서에서 사용하는 검색과 형태 분석 용어를 정의합니다.',
  'metadata.glossary.title': '단어장',
  'metadata.not_found.description': '요청한 kfind 문서 경로가 없습니다.',
  'metadata.not_found.title': '페이지를 찾을 수 없음',
  'metadata.optimization.description':
    '검색 계획과 실행 엔진의 성능 설계를 설명합니다.',
  'metadata.optimization.title': '설계와 최적화',
  'metadata.options.description':
    '확장, 경계, 품사, 정규화와 구 검색 옵션을 설명합니다.',
  'metadata.options.title': '쿼리와 옵션',
  'metadata.overview.description':
    'kfind의 목적, 검색 모델과 사용 경로를 설명합니다.',
  'metadata.overview.title': '개요',
  'metadata.playground.description':
    '브라우저에서 kfind WebAssembly 검색을 실행합니다.',
  'metadata.playground.title': 'Playground',
  'navigation.group.evidence': '근거',
  'navigation.group.internals': '내부 원리',
  'navigation.group.reference': '참조',
  'navigation.group.start': '시작',
  'navigation.item.analysis': '형태 분석',
  'navigation.item.architecture': '아키텍처',
  'navigation.item.benchmarks': '벤치마크',
  'navigation.item.getting_started': '시작하기',
  'navigation.item.glossary': '단어장',
  'navigation.item.optimization': '설계와 최적화',
  'navigation.item.options': '쿼리와 옵션',
  'navigation.item.overview': '개요',
  'navigation.item.playground': 'Playground',
} as const;

export type DocumentTranslationKey = keyof typeof koreanTranslation;
