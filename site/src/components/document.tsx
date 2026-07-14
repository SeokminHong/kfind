import type { ReactElement, ReactNode, Ref } from 'react';

import { Children, cloneElement, isValidElement } from 'react';
import { Link } from 'react-router';

import { annotateGlossaryText } from './glossary-annotation';

interface PageIntroProps {
  readonly eyebrow: string;
  readonly title: ReactNode;
  readonly summary: ReactNode;
  readonly children?: ReactNode;
}

interface DocumentSectionProps {
  readonly title: ReactNode;
  readonly children: ReactNode;
}

interface DocumentPageProps {
  readonly articleRef?: Ref<HTMLElement>;
  readonly children: ReactNode;
}

interface ElementWithChildren {
  readonly children?: ReactNode;
}

const skippedElements = new Set([
  'a',
  'button',
  'code',
  'input',
  'label',
  'option',
  'pre',
  'script',
  'select',
  'style',
  'svg',
  'textarea',
]);

function annotateChildren(
  children: ReactNode,
  seenTerms: Set<string>,
): ReactNode {
  return Children.map(children, (child): ReactNode =>
    annotateDocumentNode(child, seenTerms),
  );
}

function annotateDocumentNode(
  node: ReactNode,
  seenTerms: Set<string>,
): ReactNode {
  if (typeof node === 'string') {
    return annotateGlossaryText(node, seenTerms);
  }

  if (!isValidElement(node)) {
    return node;
  }

  if (node.type === Link) {
    return node;
  }

  if (node.type === PageIntro) {
    const element = node as ReactElement<PageIntroProps>;

    return cloneElement(
      element,
      {
        title: annotateDocumentNode(element.props.title, seenTerms),
        summary: annotateDocumentNode(element.props.summary, seenTerms),
      },
      annotateChildren(element.props.children, seenTerms),
    );
  }

  if (node.type === DocumentSection) {
    const element = node as ReactElement<DocumentSectionProps>;

    return cloneElement(
      element,
      { title: annotateDocumentNode(element.props.title, seenTerms) },
      annotateChildren(element.props.children, seenTerms),
    );
  }

  if (typeof node.type === 'string' && skippedElements.has(node.type)) {
    return node;
  }

  const element = node as ReactElement<ElementWithChildren>;

  if (element.props.children === undefined) {
    return element;
  }

  return cloneElement(
    element,
    undefined,
    annotateChildren(element.props.children, seenTerms),
  );
}

export function DocumentPage({
  articleRef,
  children,
}: DocumentPageProps): React.JSX.Element {
  const seenTerms = new Set<string>();

  return (
    <article ref={articleRef}>{annotateChildren(children, seenTerms)}</article>
  );
}

export function PageIntro({
  eyebrow,
  title,
  summary,
  children,
}: PageIntroProps): React.JSX.Element {
  return (
    <header className="document-intro">
      <p className="document-kind">{eyebrow}</p>
      <h1>{title}</h1>
      <p className="lead">{summary}</p>
      {children}
    </header>
  );
}

export function DocumentSection({
  title,
  children,
}: DocumentSectionProps): React.JSX.Element {
  return (
    <section className="doc-section">
      <h2>{title}</h2>
      {children}
    </section>
  );
}
