import { scaleBand, scaleLinear } from 'd3-scale';
import { useId } from 'react';

import * as styles from './quality-chart.css';

export interface QualityChartRow {
  readonly adjusted: number;
  readonly label: string;
  readonly raw: number;
}

export interface DurationChartRow {
  readonly label: string;
  readonly milliseconds: number;
}

interface QualityChartProps {
  readonly adjustedLabel: string;
  readonly caption: string;
  readonly description: string;
  readonly metricLabel: string;
  readonly rawLabel: string;
  readonly rows: readonly QualityChartRow[];
  readonly title: string;
}

const width = 760;
const margin = { top: 46, right: 56, bottom: 38, left: 164 } as const;

export function QualityChart({
  adjustedLabel,
  caption,
  description,
  metricLabel,
  rawLabel,
  rows,
  title,
}: QualityChartProps): React.JSX.Element {
  const titleId = useId();
  const descriptionId = useId();
  const rowHeight = 52;
  const height = margin.top + margin.bottom + rows.length * rowHeight;
  const x = scaleLinear()
    .domain([0, 100])
    .range([margin.left, width - margin.right]);
  const y = scaleBand()
    .domain(rows.map((row) => row.label))
    .range([margin.top, height - margin.bottom])
    .paddingInner(0.28);
  const ticks = x.ticks(5);
  const barHeight = Math.max(8, y.bandwidth() / 2 - 2);

  return (
    <figure className={styles.figure}>
      <svg
        aria-labelledby={`${titleId} ${descriptionId}`}
        className={styles.chart}
        role="img"
        viewBox={`0 0 ${width} ${height}`}
      >
        <title id={titleId}>{title}</title>
        <desc id={descriptionId}>{description}</desc>

        <g aria-hidden="true">
          <rect
            className={styles.rawBar}
            height="10"
            width="18"
            x={margin.left}
            y="14"
          />
          <text className={styles.legend} x={margin.left + 24} y="23">
            {rawLabel}
          </text>
          <rect
            className={styles.adjustedBar}
            height="10"
            width="18"
            x={margin.left + 100}
            y="14"
          />
          <text className={styles.legend} x={margin.left + 124} y="23">
            {adjustedLabel}
          </text>
        </g>

        {ticks.map((tick) => (
          <g aria-hidden="true" key={tick}>
            <line
              className={styles.grid}
              x1={x(tick)}
              x2={x(tick)}
              y1={margin.top}
              y2={height - margin.bottom}
            />
            <text
              className={styles.axis}
              textAnchor="middle"
              x={x(tick)}
              y={height - 14}
            >
              {tick}%
            </text>
          </g>
        ))}

        {rows.map((row) => {
          const top = y(row.label) ?? 0;
          const labelY = top + y.bandwidth() / 2 + 4;
          const adjustedY = top + barHeight + 4;

          return (
            <g key={row.label}>
              <text
                className={styles.label}
                textAnchor="end"
                x={margin.left - 10}
                y={labelY}
              >
                {row.label}
              </text>
              <rect
                aria-label={`${row.label} ${rawLabel} ${row.raw}%`}
                className={styles.rawBar}
                height={barHeight}
                width={x(row.raw) - x(0)}
                x={x(0)}
                y={top}
              />
              <rect
                aria-label={`${row.label} ${adjustedLabel} ${row.adjusted}%`}
                className={styles.adjustedBar}
                height={barHeight}
                width={x(row.adjusted) - x(0)}
                x={x(0)}
                y={adjustedY}
              />
              <text
                className={styles.value}
                x={x(row.raw) + 5}
                y={top + barHeight - 2}
              >
                {row.raw.toFixed(2)}
              </text>
              <text
                className={styles.value}
                x={x(row.adjusted) + 5}
                y={adjustedY + barHeight - 2}
              >
                {row.adjusted.toFixed(2)}
              </text>
            </g>
          );
        })}

        <text
          className={styles.axis}
          textAnchor="end"
          x={width - margin.right}
          y={height - 2}
        >
          {metricLabel}
        </text>
      </svg>
      <figcaption className={styles.caption}>{caption}</figcaption>
    </figure>
  );
}

interface DurationChartProps {
  readonly caption: string;
  readonly description: string;
  readonly rows: readonly DurationChartRow[];
  readonly title: string;
}

const durationMargin = {
  top: 24,
  right: 100,
  bottom: 38,
  left: 184,
} as const;

export function DurationChart({
  caption,
  description,
  rows,
  title,
}: DurationChartProps): React.JSX.Element {
  const titleId = useId();
  const descriptionId = useId();
  const rowHeight = 42;
  const height =
    durationMargin.top + durationMargin.bottom + rows.length * rowHeight;
  const maximum = Math.max(...rows.map((row) => row.milliseconds));
  const x = scaleLinear()
    .domain([0, maximum])
    .nice()
    .range([durationMargin.left, width - durationMargin.right]);
  const y = scaleBand()
    .domain(rows.map((row) => row.label))
    .range([durationMargin.top, height - durationMargin.bottom])
    .paddingInner(0.34);
  const ticks = x.ticks(5);

  return (
    <figure className={styles.figure}>
      <svg
        aria-labelledby={`${titleId} ${descriptionId}`}
        className={styles.chart}
        role="img"
        viewBox={`0 0 ${width} ${height}`}
      >
        <title id={titleId}>{title}</title>
        <desc id={descriptionId}>{description}</desc>

        {ticks.map((tick) => (
          <g aria-hidden="true" key={tick}>
            <line
              className={styles.grid}
              x1={x(tick)}
              x2={x(tick)}
              y1={durationMargin.top}
              y2={height - durationMargin.bottom}
            />
            <text
              className={styles.axis}
              textAnchor="middle"
              x={x(tick)}
              y={height - 14}
            >
              {tick.toLocaleString()} ms
            </text>
          </g>
        ))}

        {rows.map((row) => {
          const top = y(row.label) ?? 0;
          const barHeight = y.bandwidth();

          return (
            <g key={row.label}>
              <text
                className={styles.label}
                textAnchor="end"
                x={durationMargin.left - 10}
                y={top + barHeight / 2 + 4}
              >
                {row.label}
              </text>
              <rect
                aria-label={`${row.label} ${row.milliseconds.toFixed(2)} ms`}
                className={styles.durationBar}
                height={barHeight}
                width={x(row.milliseconds) - x(0)}
                x={x(0)}
                y={top}
              />
              <text
                className={styles.value}
                x={x(row.milliseconds) + 5}
                y={top + barHeight / 2 + 4}
              >
                {row.milliseconds.toFixed(2)}
              </text>
            </g>
          );
        })}
      </svg>
      <figcaption className={styles.caption}>{caption}</figcaption>
    </figure>
  );
}
