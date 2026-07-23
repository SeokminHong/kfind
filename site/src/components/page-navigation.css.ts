import { style } from '@vanilla-extract/css';

import { vars } from '../theme.css';

export const navigation = style({
  display: 'grid',
  gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
  gap: vars.space.large,
  paddingBlock: vars.space.section,
  borderBlockStart: `1px solid ${vars.color.border}`,
  '@media': {
    '(max-width: 42rem)': {
      gridTemplateColumns: 'minmax(0, 1fr)',
    },
  },
});

const link = style({
  display: 'grid',
  minWidth: 0,
  gap: vars.space.small,
  padding: vars.space.large,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.medium,
  background: vars.color.surface,
  color: vars.color.heading,
  textDecoration: 'none',
  selectors: {
    '&:hover': {
      borderColor: vars.color.borderStrong,
      background: vars.color.linkWash,
      color: vars.color.linkHover,
    },
  },
});

export const previousLink = style([
  link,
  {
    gridColumn: 1,
    justifyItems: 'start',
  },
]);

export const nextLink = style([
  link,
  {
    gridColumn: 2,
    justifyItems: 'end',
    textAlign: 'end',
    '@media': {
      '(max-width: 42rem)': {
        gridColumn: 1,
      },
    },
  },
]);

export const direction = style({
  color: vars.color.subtle,
  fontSize: '0.75rem',
  fontWeight: 650,
});

export const title = style({
  overflowWrap: 'anywhere',
  fontSize: '0.95rem',
  fontWeight: 650,
});
