import type {
  KeyboardEvent as ReactKeyboardEvent,
  MouseEvent as ReactMouseEvent,
} from 'react';

import type { GlossaryTerm } from './glossary';

import { PreviewCard } from '@base-ui/react/preview-card';
import { useRef, useState } from 'react';
import { Link } from 'react-router';

import { RoutePath } from '../app/navigation';

import * as styles from './glossary-tooltip.css';

interface GlossaryTooltipProps {
  readonly children: string;
  readonly term: GlossaryTerm;
}

const tooltipGap = 8;
const hoverlessEnvironmentQuery = '(hover: none)';

export function GlossaryTooltip({
  children,
  term,
}: GlossaryTooltipProps): React.JSX.Element {
  const hasPendingKeyboardActivation = useRef(false);
  const isHoverlessTooltipArmed = useRef(false);
  const [isOpen, setIsOpen] = useState(false);
  const tooltipId = `glossary-tooltip-${term.id}`;

  function beginKeyboardActivation(
    event: ReactKeyboardEvent<HTMLAnchorElement>,
  ): void {
    if (event.key === 'Enter') {
      hasPendingKeyboardActivation.current = true;
    }
  }

  function clearKeyboardActivation(): void {
    hasPendingKeyboardActivation.current = false;
  }

  function handleClick(event: ReactMouseEvent<HTMLAnchorElement>): void {
    const isKeyboardActivation = hasPendingKeyboardActivation.current;

    clearKeyboardActivation();

    if (
      isKeyboardActivation ||
      !globalThis.matchMedia(hoverlessEnvironmentQuery).matches
    ) {
      return;
    }

    if (isHoverlessTooltipArmed.current) {
      isHoverlessTooltipArmed.current = false;
      return;
    }

    event.preventDefault();
    isHoverlessTooltipArmed.current = true;
    setIsOpen(true);
  }

  return (
    <PreviewCard.Root
      open={isOpen}
      onOpenChange={(open) => {
        setIsOpen(open);

        if (!open) {
          isHoverlessTooltipArmed.current = false;
        }
      }}
    >
      <PreviewCard.Trigger
        aria-describedby={tooltipId}
        className={styles.trigger}
        closeDelay={0}
        delay={0}
        onBlur={clearKeyboardActivation}
        onClick={handleClick}
        onKeyDown={beginKeyboardActivation}
        onKeyUp={clearKeyboardActivation}
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
            {term.definition}
          </PreviewCard.Popup>
        </PreviewCard.Positioner>
      </PreviewCard.Portal>
    </PreviewCard.Root>
  );
}
