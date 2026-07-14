import type { MouseEvent as ReactMouseEvent } from 'react';

import type { GlossaryTerm } from './glossary';

import { PreviewCard } from '@base-ui/react/preview-card';
import { useState } from 'react';
import { Link } from 'react-router';

import { RoutePath } from '../app/navigation';

import * as styles from './glossary-tooltip.css';

interface GlossaryTooltipProps {
  readonly children: string;
  readonly term: GlossaryTerm;
}

const tooltipGap = 8;
const hoverlessPointerQuery = '(hover: none)';

export function GlossaryTooltip({
  children,
  term,
}: GlossaryTooltipProps): React.JSX.Element {
  const [isOpen, setIsOpen] = useState(false);
  const [isHoverlessTooltipOpen, setIsHoverlessTooltipOpen] = useState(false);
  const tooltipId = `glossary-tooltip-${term.id}`;

  function handleClick(event: ReactMouseEvent<HTMLAnchorElement>): void {
    const isHoverlessPointerActivation =
      event.detail > 0 && globalThis.matchMedia(hoverlessPointerQuery).matches;

    if (!isHoverlessPointerActivation || isHoverlessTooltipOpen) {
      return;
    }

    event.preventDefault();
    setIsHoverlessTooltipOpen(true);
    setIsOpen(true);
  }

  return (
    <PreviewCard.Root
      open={isOpen}
      onOpenChange={(open) => {
        setIsOpen(open);

        if (!open) {
          setIsHoverlessTooltipOpen(false);
        }
      }}
    >
      <PreviewCard.Trigger
        aria-describedby={tooltipId}
        className={styles.trigger}
        closeDelay={0}
        delay={0}
        onClick={handleClick}
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
