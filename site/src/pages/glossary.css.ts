import { style } from '@vanilla-extract/css';

import { vars } from '../theme.css';

export const list = style({
  display: 'grid',
  gridTemplateColumns: 'repeat(auto-fit, minmax(min(100%, 20rem), 1fr))',
  gap: vars.space.large,
  margin: 0,
});

export const entry = style({
  padding: vars.space.large,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
  scrollMarginTop: '5rem',
});

export const heading = style({
  display: 'flex',
  flexWrap: 'wrap',
  alignItems: 'baseline',
  gap: vars.space.small,
  margin: 0,
});

export const term = style({
  color: vars.color.heading,
  fontSize: '1rem',
  fontStyle: 'normal',
  fontWeight: 700,
});

export const notation = style({
  color: vars.color.subtle,
  fontSize: '0.76rem',
});

export const definition = style({
  margin: `${vars.space.small} 0 0`,
  color: vars.color.muted,
  fontSize: '0.86rem',
});
