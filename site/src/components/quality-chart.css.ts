import { style } from '@vanilla-extract/css';

import { vars } from '../theme.css';

export const figure = style({
  margin: `${vars.space.xlarge} 0 0`,
  padding: vars.space.medium,
  overflowX: 'auto',
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
});

export const chart = style({
  display: 'block',
  width: '100%',
  minWidth: '680px',
  height: 'auto',
});

export const grid = style({
  stroke: vars.color.border,
  strokeWidth: 1,
});

export const axis = style({
  fill: vars.color.subtle,
  fontSize: '11px',
});

export const label = style({
  fill: vars.color.heading,
  fontSize: '12px',
});

export const rawBar = style({ fill: vars.color.link });
export const adjustedBar = style({ fill: vars.color.success });
export const durationBar = style({ fill: vars.color.link });

export const value = style({
  fill: vars.color.heading,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '10px',
});

export const legend = style({
  fill: vars.color.muted,
  fontSize: '11px',
});

export const caption = style({
  marginBlockStart: vars.space.small,
  color: vars.color.muted,
  fontSize: '0.76rem',
});
