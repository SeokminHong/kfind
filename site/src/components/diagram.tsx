import { useId } from 'react';

export interface DiagramStep {
  readonly label: string;
  readonly title: string;
  readonly description: string;
}

interface FlowDiagramProps {
  readonly title: string;
  readonly caption: string;
  readonly steps: readonly DiagramStep[];
}

interface SplitDiagramProps {
  readonly title: string;
  readonly caption: string;
  readonly source: DiagramStep;
  readonly paths: readonly DiagramStep[];
}

function DiagramNode({
  label,
  title,
  description,
}: DiagramStep): React.JSX.Element {
  return (
    <li className="diagram-node">
      <span>{label}</span>
      <strong>{title}</strong>
      <p>{description}</p>
    </li>
  );
}

export function FlowDiagram({
  title,
  caption,
  steps,
}: FlowDiagramProps): React.JSX.Element {
  const titleId = useId();

  return (
    <figure className="diagram" aria-labelledby={titleId}>
      <div className="diagram-heading">
        <span>FLOW</span>
        <h3 id={titleId}>{title}</h3>
      </div>
      <ol className="flow-diagram">
        {steps.map((step) => (
          <DiagramNode key={step.label} {...step} />
        ))}
      </ol>
      <figcaption>{caption}</figcaption>
    </figure>
  );
}

export function SplitDiagram({
  title,
  caption,
  source,
  paths,
}: SplitDiagramProps): React.JSX.Element {
  const titleId = useId();

  return (
    <figure className="diagram" aria-labelledby={titleId}>
      <div className="diagram-heading">
        <span>DECISION</span>
        <h3 id={titleId}>{title}</h3>
      </div>
      <div className="split-diagram">
        <ol className="split-source">
          <DiagramNode {...source} />
        </ol>
        <ol className="split-paths">
          {paths.map((path) => (
            <DiagramNode key={path.label} {...path} />
          ))}
        </ol>
      </div>
      <figcaption>{caption}</figcaption>
    </figure>
  );
}
