import { style } from '@vanilla-extract/css';

import { vars } from '../theme.css';

export const container = style({
  position: 'relative',
  display: 'inline',
});

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

export const tooltip = style({
  position: 'fixed',
  zIndex: 50,
  width: 'max-content',
  maxWidth: 'min(20rem, calc(100vw - 2rem))',
  padding: `${vars.space.small} ${vars.space.medium}`,
  borderRadius: vars.radius.small,
  background: vars.color.heading,
  color: vars.color.surface,
  fontSize: '0.76rem',
  lineHeight: 1.5,
  opacity: 0,
  pointerEvents: 'none',
  transform: 'translateX(-50%)',
  transition: 'opacity 120ms ease',
  visibility: 'hidden',
  selectors: {
    '&[data-side="above"]': {
      transform: 'translate(-50%, -100%)',
    },
    [`.${container}[data-tooltip-positioned]:focus-within &`]: {
      opacity: 1,
      visibility: 'visible',
    },
    [`.${container}[data-tooltip-open][data-tooltip-positioned] &`]: {
      opacity: 1,
      visibility: 'visible',
    },
  },
  '@media': {
    '(hover: hover)': {
      selectors: {
        [`.${container}[data-tooltip-positioned]:hover &`]: {
          opacity: 1,
          visibility: 'visible',
        },
      },
    },
  },
});
