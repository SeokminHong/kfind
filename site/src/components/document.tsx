import type { ReactNode } from 'react';

interface PageIntroProps {
  readonly eyebrow: string;
  readonly title: string;
  readonly summary: string;
  readonly children?: ReactNode;
}

interface DocumentSectionProps {
  readonly title: string;
  readonly children: ReactNode;
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
