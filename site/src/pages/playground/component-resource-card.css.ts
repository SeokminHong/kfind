import { style } from '@vanilla-extract/css';

import { vars } from '../../theme.css';

export const card = style({
  display: 'grid',
  gridTemplateColumns: 'minmax(0, 1fr) minmax(15rem, auto)',
  alignItems: 'center',
  gap: vars.space.medium,
  padding: vars.space.medium,
  borderBlockEnd: `1px solid ${vars.color.border}`,
  background: vars.color.linkWash,
  selectors: {
    '&[data-state="ready"]': {
      background: vars.color.successWash,
    },
    '&[data-state="error"]': {
      background: vars.color.dangerWash,
    },
  },
  '@media': {
    '(max-width: 64rem)': {
      gridTemplateColumns: 'minmax(0, 1fr)',
      gap: vars.space.medium,
    },
  },
});

export const explanation = style({
  minWidth: 0,
});

export const eyebrow = style({
  marginBlockEnd: vars.space.xsmall,
  color: vars.color.link,
  fontSize: '0.66rem',
  fontWeight: 700,
  letterSpacing: '0.05em',
});

export const heading = style({
  margin: 0,
  color: vars.color.heading,
  fontSize: '1rem',
  lineHeight: 1.35,
});

export const role = style({
  maxWidth: '68rem',
  margin: `${vars.space.small} 0 0`,
  color: vars.color.text,
  fontSize: '0.76rem',
  lineHeight: 1.6,
});

export const control = style({
  display: 'grid',
  minWidth: 0,
  justifyItems: 'stretch',
  gap: vars.space.small,
});

export const status = style({
  display: 'flex',
  minWidth: 0,
  alignItems: 'center',
  gap: vars.space.small,
  color: vars.color.muted,
  fontSize: '0.68rem',
  lineHeight: 1.45,
  selectors: {
    '&[data-state="needed"]': {
      color: vars.color.warning,
    },
    '&[data-state="error"]': {
      color: vars.color.danger,
    },
  },
});

export const statusDot = style({
  width: '0.5rem',
  height: '0.5rem',
  flex: '0 0 auto',
  borderRadius: '50%',
  background: vars.color.warning,
  selectors: {
    [`${status}[data-state="ready"] &`]: {
      background: vars.color.success,
    },
    [`${status}[data-state="error"] &`]: {
      background: vars.color.danger,
    },
  },
});

export const button = style({
  minHeight: '2.5rem',
  paddingInline: vars.space.medium,
  border: `1px solid ${vars.color.link}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
  color: vars.color.link,
  fontSize: '0.72rem',
  fontWeight: 650,
  selectors: {
    '&:hover:not(:disabled)': {
      background: vars.color.linkWash,
    },
    '&:disabled': {
      borderColor: vars.color.borderStrong,
      color: vars.color.subtle,
    },
  },
});

export const ready = style({
  width: 'fit-content',
  justifySelf: 'end',
  padding: `${vars.space.xsmall} ${vars.space.small}`,
  borderRadius: vars.radius.pill,
  background: vars.color.surface,
  color: vars.color.success,
  fontSize: '0.7rem',
  fontWeight: 650,
  '@media': {
    '(max-width: 64rem)': {
      justifySelf: 'start',
    },
  },
});
