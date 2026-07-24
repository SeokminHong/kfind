import type { ComponentResourceStatus } from '../../playground';

import { Button } from '@base-ui/react/button';

import { DocumentLocale } from '../../app/i18n';
import { ComponentResourceState } from '../../playground';

import * as styles from './component-resource-card.css';

interface ComponentResourceCardProps {
  readonly disabled: boolean;
  readonly locale: DocumentLocale;
  readonly onLoad: () => void;
  readonly status: ComponentResourceStatus;
}

const componentResourceCopy = {
  [DocumentLocale.Korean]: {
    action: '판정 리소스 불러오기 · 35.4 MiB',
    checking: '브라우저 저장소 확인 중',
    eyebrow: 'SMART 구조 판정 · 35.4 MiB',
    heading: '형태 구성 요소 판정 리소스',
    loading: '판정 리소스 불러오는 중',
    ready: '사용 가능',
    role: 'smart 경계가 원문 token 내부의 같은 품사 형태 구성 요소인지, 또는 인접 token 구조가 성립하는지 검증할 때 쓰는 compact index입니다. 전체 문장을 분석하거나 검색어를 확장하는 full POS 사전은 아닙니다.',
  },
  [DocumentLocale.English]: {
    action: 'Load verification resource · 35.4 MiB',
    checking: 'Checking browser storage',
    eyebrow: 'SMART STRUCTURAL VERIFICATION · 35.4 MiB',
    heading: 'Morphological component verification resource',
    loading: 'Loading verification resource',
    ready: 'Available',
    role: 'This compact index lets a smart boundary verify that a span is a same-POS component inside a source token or that an adjacent-token structure is valid. It is not a full-POS dictionary that analyzes whole sentences or expands queries.',
  },
} as const;

const headingId = 'playground-component-resource-heading';

export function ComponentResourceCard({
  disabled,
  locale,
  onLoad,
  status,
}: ComponentResourceCardProps): React.JSX.Element {
  const copy = componentResourceCopy[locale];

  return (
    <section
      aria-labelledby={headingId}
      className={styles.card}
      data-state={status.state}
    >
      <div className={styles.explanation}>
        <p className={styles.eyebrow}>{copy.eyebrow}</p>
        <h2 className={styles.heading} id={headingId}>
          {copy.heading}
        </h2>
        <p className={styles.role}>{copy.role}</p>
      </div>

      <div className={styles.control}>
        <div
          aria-live="polite"
          className={styles.status}
          data-state={status.state}
          role="status"
        >
          <span className={styles.statusDot} aria-hidden="true" />
          <span>{status.message}</span>
        </div>
        {status.state === ComponentResourceState.Ready ? (
          <span className={styles.ready}>{copy.ready}</span>
        ) : (
          <Button
            className={styles.button}
            data-glossary-skip=""
            disabled={disabled}
            onClick={onLoad}
            type="button"
          >
            {buttonLabel(status.state, copy)}
          </Button>
        )}
      </div>
    </section>
  );
}

function buttonLabel(
  state: ComponentResourceState,
  copy: (typeof componentResourceCopy)[DocumentLocale],
): string {
  if (state === ComponentResourceState.Checking) {
    return copy.checking;
  }

  if (state === ComponentResourceState.Loading) {
    return copy.loading;
  }

  return copy.action;
}
