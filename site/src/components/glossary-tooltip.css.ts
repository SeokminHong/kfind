import { style } from '@vanilla-extract/css';

import { vars } from '../theme.css';

export const trigger = style({
  color: 'inherit',
  textDecorationColor: vars.color.link,
  textDecorationLine: 'underline',
  textDecorationStyle: 'dotted',
  textDecorationThickness: '1px',
  textUnderlineOffset: '0.2em',
  '@media': {
    '(hover: hover)': {
      selectors: {
        '&:hover': {
          color: vars.color.linkHover,
        },
      },
    },
  },
});

export const positioner = style({
  zIndex: 50,
  width: 'max-content',
  maxWidth: 'calc(100vw - 2rem)',
});

export const tooltip = style({
  display: 'flex',
  flexDirection: 'column',
  gap: vars.space.xsmall,
  width: 'max-content',
  maxWidth: 'min(20rem, 100%)',
  padding: `${vars.space.small} ${vars.space.medium}`,
  borderRadius: vars.radius.small,
  background: vars.color.heading,
  color: vars.color.surface,
  fontSize: '0.76rem',
  lineHeight: 1.5,
  opacity: 1,
  transition: 'opacity 120ms ease',
  selectors: {
    '&[data-starting-style], &[data-ending-style]': {
      opacity: 0,
    },
  },
});

export const notation = style({
  fontWeight: 700,
});
