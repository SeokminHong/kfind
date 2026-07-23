import type {
  KeyboardEvent as ReactKeyboardEvent,
  MouseEvent as ReactMouseEvent,
  PointerEvent as ReactPointerEvent,
} from 'react';

import type { GlossaryTerm } from './glossary';

import { PreviewCard } from '@base-ui/react/preview-card';
import { useId, useRef, useState } from 'react';
import { Link } from 'react-router';

import { RoutePath } from '../app/navigation';

import * as styles from './glossary-tooltip.css';

interface GlossaryTooltipProps {
  readonly children: string;
  readonly term: GlossaryTerm;
}

const tooltipGap = 8;

export function GlossaryTooltip({
  children,
  term,
}: GlossaryTooltipProps): React.JSX.Element {
  const hasPendingDirectActivation = useRef(false);
  const isTooltipArmed = useRef(false);
  const [isOpen, setIsOpen] = useState(false);
  const tooltipId = useId();

  function beginKeyboardActivation(
    event: ReactKeyboardEvent<HTMLAnchorElement>,
  ): void {
    if (event.key === 'Enter') {
      hasPendingDirectActivation.current = true;
    }
  }

  function beginPointerActivation(
    event: ReactPointerEvent<HTMLAnchorElement>,
  ): void {
    hasPendingDirectActivation.current = event.pointerType === 'mouse';
  }

  function beginTouchActivation(): void {
    hasPendingDirectActivation.current = false;
  }

  function clearPendingActivation(): void {
    hasPendingDirectActivation.current = false;
  }

  function handleClick(event: ReactMouseEvent<HTMLAnchorElement>): void {
    const shouldNavigateDirectly = hasPendingDirectActivation.current;

    clearPendingActivation();

    if (shouldNavigateDirectly) {
      isTooltipArmed.current = false;
      return;
    }

    if (isTooltipArmed.current) {
      isTooltipArmed.current = false;
      return;
    }

    event.preventDefault();
    isTooltipArmed.current = true;
    setIsOpen(true);
  }

  return (
    <PreviewCard.Root
      open={isOpen}
      onOpenChange={(open) => {
        setIsOpen(open);

        if (!open) {
          isTooltipArmed.current = false;
        }
      }}
    >
      <PreviewCard.Trigger
        aria-describedby={tooltipId}
        className={styles.trigger}
        closeDelay={0}
        delay={0}
        onBlur={clearPendingActivation}
        onClick={handleClick}
        onKeyDown={beginKeyboardActivation}
        onKeyUp={clearPendingActivation}
        onPointerCancel={clearPendingActivation}
        onPointerDown={beginPointerActivation}
        onTouchStart={beginTouchActivation}
        render={<Link to={`${RoutePath.Glossary}#${term.id}`} />}
      >
        {children}
      </PreviewCard.Trigger>
      <PreviewCard.Portal>
        <PreviewCard.Positioner
          className={styles.positioner}
          side="top"
          sideOffset={tooltipGap}
        >
          <PreviewCard.Popup
            className={styles.tooltip}
            id={tooltipId}
            role="tooltip"
          >
            {term.notation === undefined ? null : (
              <span className={styles.notation}>{term.notation}</span>
            )}
            <span>{term.definition}</span>
          </PreviewCard.Popup>
        </PreviewCard.Positioner>
      </PreviewCard.Portal>
    </PreviewCard.Root>
  );
}
