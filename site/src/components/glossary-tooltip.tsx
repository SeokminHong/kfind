import type { CSSProperties, MouseEvent as ReactMouseEvent } from 'react';

import type { GlossaryTerm } from './glossary';

import { useEffect, useRef, useState } from 'react';
import { Link } from 'react-router';

import { RoutePath } from '../app/navigation';

import * as styles from './glossary-tooltip.css';

interface TooltipPosition {
  readonly left: number;
  readonly side: 'above' | 'below';
  readonly top: number;
}

interface GlossaryTooltipProps {
  readonly children: string;
  readonly term: GlossaryTerm;
}

const viewportMargin = 16;
const tooltipGap = 8;
const hoverlessPointerQuery = '(hover: none)';

export function GlossaryTooltip({
  children,
  term,
}: GlossaryTooltipProps): React.JSX.Element {
  const containerRef = useRef<HTMLSpanElement>(null);
  const triggerRef = useRef<HTMLAnchorElement>(null);
  const tooltipRef = useRef<HTMLSpanElement>(null);
  const [isHoverlessTooltipOpen, setIsHoverlessTooltipOpen] = useState(false);
  const [position, setPosition] = useState<TooltipPosition>();

  useEffect(() => {
    if (!isHoverlessTooltipOpen) {
      return;
    }

    function closeTooltipOutside(event: PointerEvent): void {
      const target = event.target;

      if (
        target instanceof Node &&
        containerRef.current?.contains(target) === true
      ) {
        return;
      }

      setIsHoverlessTooltipOpen(false);
    }

    document.addEventListener('pointerdown', closeTooltipOutside);

    return () => {
      document.removeEventListener('pointerdown', closeTooltipOutside);
    };
  }, [isHoverlessTooltipOpen]);

  function positionTooltip(): void {
    const trigger = triggerRef.current;
    const tooltip = tooltipRef.current;

    if (trigger === null || tooltip === null) {
      return;
    }

    const triggerRect = trigger.getBoundingClientRect();
    const tooltipWidth = tooltip.offsetWidth;
    const tooltipHeight = tooltip.offsetHeight;
    const spaceAbove = triggerRect.top - viewportMargin;
    const spaceBelow = window.innerHeight - triggerRect.bottom - viewportMargin;
    const side =
      spaceAbove >= tooltipHeight + tooltipGap || spaceAbove >= spaceBelow
        ? 'above'
        : 'below';
    const centeredLeft = triggerRect.left + triggerRect.width / 2;
    const minimumLeft = viewportMargin + tooltipWidth / 2;
    const maximumLeft = window.innerWidth - viewportMargin - tooltipWidth / 2;

    setPosition({
      left: Math.min(Math.max(centeredLeft, minimumLeft), maximumLeft),
      side,
      top:
        side === 'above'
          ? triggerRect.top - tooltipGap
          : triggerRect.bottom + tooltipGap,
    });
  }

  const tooltipStyle: CSSProperties | undefined =
    position === undefined
      ? undefined
      : { left: position.left, top: position.top };

  function handleClick(event: ReactMouseEvent<HTMLAnchorElement>): void {
    const isHoverlessPointerActivation =
      event.detail > 0 && globalThis.matchMedia(hoverlessPointerQuery).matches;

    if (!isHoverlessPointerActivation || isHoverlessTooltipOpen) {
      return;
    }

    event.preventDefault();
    positionTooltip();
    setIsHoverlessTooltipOpen(true);
  }

  return (
    <span
      className={styles.container}
      data-tooltip-open={isHoverlessTooltipOpen ? '' : undefined}
      data-tooltip-positioned={position === undefined ? undefined : ''}
      ref={containerRef}
    >
      <Link
        className={styles.trigger}
        ref={triggerRef}
        to={`${RoutePath.Glossary}#${term.id}`}
        aria-describedby={`glossary-tooltip-${term.id}`}
        onBlur={() => {
          setIsHoverlessTooltipOpen(false);
        }}
        onClick={handleClick}
        onFocus={positionTooltip}
        onMouseEnter={positionTooltip}
      >
        {children}
      </Link>
      <span
        className={styles.tooltip}
        data-side={position?.side}
        id={`glossary-tooltip-${term.id}`}
        ref={tooltipRef}
        role="tooltip"
        style={tooltipStyle}
      >
        {term.definition}
      </span>
    </span>
  );
}
