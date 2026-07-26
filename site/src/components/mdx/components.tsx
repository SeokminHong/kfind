import type {
  ComponentPropsWithoutRef,
  HTMLAttributes,
  ReactElement,
  ReactNode,
} from 'react';

import { Children, isValidElement } from 'react';
import { useLocation } from 'react-router';

import { DocumentLocale, useDocumentLocale } from '../../app/i18n';
import {
  navigationPageForPath,
  routePathFromPathname,
} from '../../app/navigation';
import { DocumentPage } from '../document';

import * as styles from './mdx.css';

export { MdxLink } from './link';

type CalloutKind = 'danger' | 'note' | 'tip' | 'warning';

interface CalloutProps {
  readonly children: ReactNode;
  readonly kind?: CalloutKind;
  readonly title?: string;
}

interface DocumentWrapperProps {
  readonly children: ReactNode;
}

const calloutStyles: Readonly<Record<CalloutKind, string>> = {
  danger: styles.calloutDanger,
  note: styles.calloutNote,
  tip: styles.calloutTip,
  warning: styles.calloutWarning,
};

const calloutLabels: Readonly<
  Record<CalloutKind, Readonly<Record<DocumentLocale, string>>>
> = {
  danger: { en: 'Danger', ko: '주의' },
  note: { en: 'Note', ko: '참고' },
  tip: { en: 'Tip', ko: '도움말' },
  warning: { en: 'Warning', ko: '경고' },
};

export function Callout({
  children,
  kind = 'note',
  title,
}: CalloutProps): React.JSX.Element {
  const locale = useDocumentLocale();

  return (
    <aside
      className={`${styles.callout} ${calloutStyles[kind]}`}
      data-glossary-skip
    >
      <strong className={styles.calloutLabel}>
        {title ?? calloutLabels[kind][locale]}
      </strong>
      <div className={styles.calloutContent}>{children}</div>
    </aside>
  );
}

export function CodeTitle({
  children,
}: {
  readonly children: ReactNode;
}): React.JSX.Element {
  return <div className={styles.codeTitle}>{children}</div>;
}

export function Eyebrow({
  children,
}: {
  readonly children: ReactNode;
}): React.JSX.Element {
  return <p className={styles.eyebrow}>{children}</p>;
}

export function Lead({
  children,
}: {
  readonly children: ReactNode;
}): React.JSX.Element {
  return <div className={styles.lead}>{children}</div>;
}

export function Steps({
  children,
}: {
  readonly children: ReactNode;
}): React.JSX.Element {
  return <ol className={styles.steps}>{children}</ol>;
}

export function DocumentWrapper({
  children,
}: DocumentWrapperProps): React.JSX.Element {
  const { pathname } = useLocation();
  validateDocumentStructure(children, pathname);

  return <DocumentPage>{children}</DocumentPage>;
}

function validateDocumentStructure(
  children: ReactNode,
  pathname: string,
): void {
  const headings = Children.toArray(children).filter(
    (child): child is ReactElement<{ readonly id?: string }> =>
      isValidElement(child) && child.type === 'h2',
  );
  if (headings.length === 0) {
    return;
  }

  const page = navigationPageForPath(routePathFromPathname(pathname));
  const expectedIds = page?.sections.map((section) => section.id) ?? [];
  const actualIds = headings.map((heading) => heading.props.id);

  if (
    expectedIds.length !== actualIds.length ||
    expectedIds.some((id, index) => id !== actualIds[index])
  ) {
    throw new Error(
      `MDX heading order does not match navigation for ${pathname}`,
    );
  }
}

export function MdxTable({
  ...props
}: ComponentPropsWithoutRef<'table'>): React.JSX.Element {
  return (
    <div className={styles.tableScroller}>
      <table {...props} />
    </div>
  );
}

export function MdxPre({
  ...props
}: HTMLAttributes<HTMLPreElement>): React.JSX.Element {
  return <pre {...props} data-glossary-skip />;
}
