import { style } from '@vanilla-extract/css';

import { vars } from '../../theme.css';

export const trigger = style({
  display: 'inline-flex',
  maxWidth: '8.5rem',
  height: '1.75rem',
  alignItems: 'center',
  gap: vars.space.xsmall,
  paddingInline: vars.space.small,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
  color: vars.color.muted,
  fontSize: '0.7rem',
  fontWeight: 650,
  selectors: {
    '&[data-popup-open]': {
      borderColor: vars.color.link,
    },
  },
});

export const value = style({
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

export const icon = style({
  flexShrink: 0,
  color: vars.color.subtle,
  fontSize: '0.62rem',
});

export const positioner = style({
  zIndex: 80,
  minWidth: 'var(--anchor-width)',
  maxWidth: 'var(--available-width)',
});

export const popup = style({
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
  fontSize: '0.72rem',
  outline: 0,
  whiteSpace: 'nowrap',
  selectors: {
    '&[data-highlighted]': {
      background: vars.color.linkWash,
      color: vars.color.link,
    },
    '&[data-selected]': {
      fontWeight: 700,
    },
  },
});
