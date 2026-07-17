import { globalStyle, style } from '@vanilla-extract/css';

import { vars } from '../theme.css';

export const backdrop = style({
  position: 'fixed',
  zIndex: 70,
  inset: 0,
  background: `color-mix(in srgb, ${vars.color.heading} 48%, transparent)`,
});

export const viewport = style({
  position: 'fixed',
  zIndex: 71,
  inset: 0,
  display: 'grid',
  alignItems: 'end',
  padding: vars.space.small,
  overflowY: 'auto',
});

export const content = style({
  width: '100%',
  maxWidth: '34rem',
  maxHeight: 'calc(100dvh - 1rem)',
  marginInline: 'auto',
  overflowY: 'auto',
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.medium,
  background: vars.color.surface,
  color: vars.color.text,
});

export const section = style({
  padding: vars.space.large,
});

globalStyle(`${section} + ${section}`, {
  borderBlockStart: `1px solid ${vars.color.border}`,
});

export const trigger = style({
  display: 'flex',
  width: '100%',
  minHeight: '2.75rem',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: vars.space.small,
  paddingInline: vars.space.medium,
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
  color: vars.color.heading,
  fontSize: '0.74rem',
  fontWeight: 650,
  textAlign: 'start',
});

export const title = style({
  margin: 0,
  color: vars.color.heading,
  fontSize: '1rem',
  fontWeight: 700,
});

export const description = style({
  margin: `${vars.space.xsmall} 0 0`,
  color: vars.color.muted,
  fontSize: '0.72rem',
});

export const close = style({
  minHeight: '2rem',
  paddingInline: vars.space.small,
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
  color: vars.color.link,
  fontSize: '0.7rem',
});
