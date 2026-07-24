import type { Match } from '../../kfind-wasm';
import type { PlaygroundInput, PlaygroundResult } from '../../playground';

import { useVirtualizer } from '@tanstack/react-virtual';
import { useEffect, useRef } from 'react';

import { DocumentLocale } from '../../app/i18n';
import {
  formatMorphologyAnalysis,
  formatProvenance,
  PlaygroundResultState,
} from '../../playground';

const estimatedMatchRowHeight = 64;
const matchRowOverscan = 6;

export interface MatchRevealRequest {
  readonly index: number;
  readonly sequence: number;
}

interface MatchListProps {
  readonly active: boolean;
  readonly emptyLabel: string;
  readonly input: PlaygroundInput;
  readonly loadingLabel: string;
  readonly locale: DocumentLocale;
  readonly matchLabel: (surface: string) => string;
  readonly onMatchActivate: (match: Match) => void;
  readonly revealRequest: MatchRevealRequest | undefined;
  readonly result: PlaygroundResult | undefined;
}

export function MatchList({
  active,
  emptyLabel,
  input,
  loadingLabel,
  locale,
  matchLabel,
  onMatchActivate,
  revealRequest,
  result,
}: MatchListProps): React.JSX.Element {
  const scrollElementRef = useRef<HTMLDivElement>(null);
  const handledRevealSequenceRef = useRef<number | undefined>(undefined);
  const scheduledRevealSequenceRef = useRef<number | undefined>(undefined);
  const matches =
    result?.state === PlaygroundResultState.Success ? result.matches : [];
  const virtualizer = useVirtualizer({
    count: matches.length,
    estimateSize: () => estimatedMatchRowHeight,
    getItemKey: (index) => {
      const match = matches[index];

      return match === undefined ? index : matchKey(match, index);
    },
    getScrollElement: () => scrollElementRef.current,
    overscan: matchRowOverscan,
  });
  const virtualItems = virtualizer.getVirtualItems();

  useEffect(() => {
    if (
      !active ||
      revealRequest === undefined ||
      result?.state !== PlaygroundResultState.Success ||
      revealRequest.index >= result.matches.length ||
      scheduledRevealSequenceRef.current === revealRequest.sequence
    ) {
      return;
    }

    scheduledRevealSequenceRef.current = revealRequest.sequence;
    handledRevealSequenceRef.current = undefined;
    virtualizer.scrollToIndex(revealRequest.index, { align: 'center' });
  }, [active, result, revealRequest, virtualizer]);

  useEffect(() => {
    if (
      !active ||
      revealRequest === undefined ||
      handledRevealSequenceRef.current === revealRequest.sequence
    ) {
      return;
    }

    const button = scrollElementRef.current?.querySelector<HTMLButtonElement>(
      `[data-index="${revealRequest.index}"] button`,
    );

    if (button === null || button === undefined) {
      return;
    }

    button.focus({ preventScroll: true });
    handledRevealSequenceRef.current = revealRequest.sequence;
  }, [active, revealRequest, virtualItems]);

  if (result?.state !== PlaygroundResultState.Success) {
    return (
      <ol className="match-list match-list-static">
        <li className="match-empty">
          {result === undefined ? loadingLabel : emptyLabel}
        </li>
      </ol>
    );
  }

  if (matches.length === 0) {
    return (
      <ol className="match-list match-list-static">
        <li className="match-empty">{emptyLabel}</li>
      </ol>
    );
  }

  return (
    <div className="match-list" ref={scrollElementRef}>
      <ol
        className="match-list-items"
        style={{ height: `${virtualizer.getTotalSize()}px` }}
      >
        {virtualItems.map((virtualItem) => {
          const match = matches[virtualItem.index];

          return match === undefined ? null : (
            <MatchItem
              index={virtualItem.index}
              key={virtualItem.key}
              locale={locale}
              match={match}
              matchLabel={matchLabel}
              onActivate={() => {
                onMatchActivate(match);
              }}
              rowRef={virtualizer.measureElement}
              start={virtualItem.start}
              text={input.text}
              total={matches.length}
            />
          );
        })}
      </ol>
    </div>
  );
}

function MatchItem({
  index,
  locale,
  match,
  matchLabel,
  onActivate,
  rowRef,
  start,
  text,
  total,
}: {
  readonly index: number;
  readonly locale: DocumentLocale;
  readonly match: Match;
  readonly matchLabel: (surface: string) => string;
  readonly onActivate: () => void;
  readonly rowRef: (element: Element | null) => void;
  readonly start: number;
  readonly text: string;
  readonly total: number;
}): React.JSX.Element {
  const surface = text.slice(match.start, match.end);

  return (
    <li
      aria-posinset={index + 1}
      aria-setsize={total}
      data-index={index}
      ref={rowRef}
      style={{ transform: `translateY(${start}px)` }}
    >
      <button
        aria-label={matchLabel(surface)}
        className="match-item"
        data-glossary-skip=""
        onClick={onActivate}
        type="button"
      >
        <span className="match-index">
          {String(index + 1).padStart(2, '0')}
        </span>
        <strong>{surface}</strong>
        <code>
          [{match.start}, {match.end})
        </code>
        <span className="match-description">
          <span className="match-analysis">
            {formatMorphologyAnalysis(match, locale)}
          </span>
          <span className="match-provenance">
            {formatProvenance(match, locale)}
          </span>
        </span>
      </button>
    </li>
  );
}

function matchKey(match: Match, index: number): string {
  return `${match.start}-${match.end}-${index}`;
}
