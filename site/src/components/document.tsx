import type { ReactNode } from 'react';

import { Link } from 'react-router';

interface PageIntroProps {
  readonly eyebrow: string;
  readonly title: string;
  readonly summary: string;
  readonly children?: ReactNode;
}

interface DocumentSectionProps {
  readonly id?: string;
  readonly title: string;
  readonly lead?: string;
  readonly children: ReactNode;
}

interface CalloutProps {
  readonly title: string;
  readonly children: ReactNode;
  readonly tone?: 'info' | 'warning';
}

interface RouteCardProps {
  readonly eyebrow: string;
  readonly title: string;
  readonly description: string;
  readonly to: string;
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
  id,
  title,
  lead,
  children,
}: DocumentSectionProps): React.JSX.Element {
  return (
    <section className="doc-section" id={id}>
      <h2>{title}</h2>
      {lead === undefined ? null : <p className="section-lead">{lead}</p>}
      {children}
    </section>
  );
}

export function Callout({
  title,
  children,
  tone = 'info',
}: CalloutProps): React.JSX.Element {
  return (
    <aside className="note" data-tone={tone}>
      <strong>{title}</strong>
      <div>{children}</div>
    </aside>
  );
}

export function RouteCard({
  eyebrow,
  title,
  description,
  to,
}: RouteCardProps): React.JSX.Element {
  return (
    <Link className="route-card" to={to}>
      <span>{eyebrow}</span>
      <strong>{title}</strong>
      <p>{description}</p>
    </Link>
  );
}
