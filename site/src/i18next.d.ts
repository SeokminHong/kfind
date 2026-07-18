import 'i18next';

import type { koreanTranslation } from './app/translations.ko';

declare module 'i18next' {
  interface CustomTypeOptions {
    defaultNS: 'translation';
    keySeparator: false;
    returnNull: false;
    resources: {
      translation: typeof koreanTranslation;
    };
  }
}
