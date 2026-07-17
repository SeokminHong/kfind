import { globalStyle, style } from '@vanilla-extract/css';

import { vars } from '../../theme.css';

export const labelRow = style({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: vars.space.small,
});

export const tagTrigger = style({
  padding: 0,
  border: 0,
  background: 'transparent',
  color: vars.color.link,
  cursor: 'help',
  fontSize: '0.68rem',
  textDecorationLine: 'underline',
  textDecorationStyle: 'dotted',
  textUnderlineOffset: '0.2em',
});

export const positioner = style({
  zIndex: 60,
  width: 'max-content',
  maxWidth: 'calc(100vw - 2rem)',
});

export const tooltip = style({
  width: 'min(22rem, 100%)',
  padding: vars.space.medium,
  borderRadius: vars.radius.small,
  background: vars.color.heading,
  color: vars.color.surface,
  fontSize: '0.72rem',
  lineHeight: 1.5,
  opacity: 1,
  transition: 'opacity 120ms ease',
  selectors: {
    '&[data-starting-style], &[data-ending-style]': {
      opacity: 0,
    },
  },
});

export const tagList = style({
  display: 'grid',
  gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
  gap: `${vars.space.xsmall} ${vars.space.medium}`,
  margin: `${vars.space.small} 0 0`,
});

globalStyle(`${tagList} > div`, {
  display: 'grid',
  gridTemplateColumns: '3.2rem 1fr',
  gap: vars.space.xsmall,
});

globalStyle(`${tagList} dt, ${tagList} dd`, {
  margin: 0,
});
