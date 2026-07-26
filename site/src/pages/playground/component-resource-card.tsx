import type { ComponentResourceStatus } from '../../playground';

import { Switch } from '@base-ui/react/switch';

import { DocumentLocale } from '../../app/i18n';

import * as styles from './component-resource-card.css';

interface ComponentResourceCardProps {
  readonly disabled: boolean;
  readonly locale: DocumentLocale;
  readonly onEnabledChange: (enabled: boolean) => void;
  readonly status: ComponentResourceStatus;
}

const componentResourceCopy = {
  [DocumentLocale.Korean]: {
    control: '형태 구성 요소 판정',
    eyebrow: 'SMART 구조 판정 · 35.4 MiB',
    heading: '형태 구성 요소 판정 리소스',
    off: '꺼짐',
    on: '켜짐',
    role: '이 compact index는 smart 경계 판정에서 원문 token 내부의 같은 품사 구성 요소와 인접 token 구조를 확인합니다. 문장 전체를 분석하거나 검색어를 확장하지 않습니다.',
  },
  [DocumentLocale.English]: {
    control: 'Morphological component verification',
    eyebrow: 'SMART STRUCTURAL VERIFICATION · 35.4 MiB',
    heading: 'Morphological component verification resource',
    off: 'Off',
    on: 'On',
    role: 'This compact index checks same-POS components inside source tokens and adjacent-token structures for smart boundary decisions. It does not analyze entire sentences or expand queries.',
  },
} as const;

const headingId = 'playground-component-resource-heading';
const switchLabelId = 'playground-component-resource-switch-label';

export function ComponentResourceCard({
  disabled,
  locale,
  onEnabledChange,
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
        <div className={styles.switchRow} data-glossary-skip="">
          <span className={styles.switchCopy}>
            <strong id={switchLabelId}>{copy.control}</strong>
            <span>{status.enabled ? copy.on : copy.off}</span>
          </span>
          <Switch.Root
            aria-labelledby={switchLabelId}
            checked={status.enabled}
            className={styles.switchRoot}
            disabled={disabled}
            onCheckedChange={onEnabledChange}
          >
            <Switch.Thumb className={styles.switchThumb} />
          </Switch.Root>
        </div>
        <div
          aria-live="polite"
          className={styles.status}
          data-state={status.state}
          role="status"
        >
          <span className={styles.statusDot} aria-hidden="true" />
          <span>{status.message}</span>
        </div>
      </div>
    </section>
  );
}
