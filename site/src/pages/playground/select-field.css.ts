import { globalStyle, style } from '@vanilla-extract/css';

import { vars } from '../../theme.css';

export const label = style({
  color: vars.color.heading,
  fontSize: '0.74rem',
  fontWeight: 650,
});

export const trigger = style({
  display: 'flex',
  width: '100%',
  height: '2.5rem',
  alignItems: 'center',
  paddingInline: vars.space.small,
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
  color: vars.color.text,
  selectors: {
    '&[data-popup-open]': {
      borderColor: vars.color.link,
      outline: `2px solid ${vars.color.linkWash}`,
    },
  },
});

export const positioner = style({
  zIndex: 60,
  minWidth: 'var(--anchor-width)',
  maxWidth: 'var(--available-width)',
});

export const icon = style({
  marginInlineStart: 'auto',
  color: vars.color.subtle,
  fontSize: '0.7rem',
});

export const popup = style({
  minWidth: 'var(--anchor-width)',
  maxHeight: 'min(18rem, var(--available-height))',
  overflowY: 'auto',
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
  color: vars.color.text,
  transformOrigin: 'var(--transform-origin)',
});

export const list = style({
  padding: vars.space.xsmall,
});

export const item = style({
  padding: `${vars.space.xsmall} ${vars.space.small}`,
  borderRadius: vars.radius.small,
  cursor: 'default',
  fontSize: '0.78rem',
  outline: 0,
  selectors: {
    '&[data-highlighted]': {
      background: vars.color.linkWash,
      color: vars.color.link,
    },
    '&[data-selected]': {
      fontWeight: 650,
    },
  },
});

export const itemText = style({
  display: 'grid',
  gap: '0.1rem',
});

export const description = style({
  margin: 0,
  color: vars.color.muted,
  fontSize: '0.68rem',
  lineHeight: 1.45,
});

globalStyle(`${itemText} small`, {
  color: vars.color.muted,
  fontSize: '0.66rem',
  fontWeight: 400,
  lineHeight: 1.4,
});
