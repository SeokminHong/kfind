import { globalStyle, style } from '@vanilla-extract/css';

import { vars } from '../../theme.css';

export const eyebrow = style({
  marginBlockEnd: vars.space.small,
  color: vars.color.subtle,
  fontSize: '0.78rem',
});

export const lead = style({
  display: 'grid',
  gap: vars.space.small,
  maxWidth: '48rem',
  marginBlockEnd: vars.space.large,
  color: vars.color.muted,
  lineHeight: 1.78,
});

export const callout = style({
  display: 'grid',
  gap: vars.space.xsmall,
  maxWidth: '48rem',
  marginBlock: vars.space.xlarge,
  padding: `${vars.space.medium} ${vars.space.large}`,
  borderInlineStart: `${vars.space.xsmall} solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.small,
  background: vars.color.surfaceMuted,
});

export const calloutNote = style({
  borderInlineStartColor: vars.color.borderStrong,
});

export const calloutTip = style({
  borderInlineStartColor: vars.color.success,
  background: vars.color.successWash,
});

export const calloutWarning = style({
  borderInlineStartColor: vars.color.warning,
  background: vars.color.warningWash,
});

export const calloutDanger = style({
  borderInlineStartColor: vars.color.danger,
  background: vars.color.dangerWash,
});

export const calloutLabel = style({
  color: vars.color.heading,
  fontSize: '0.875rem',
  fontWeight: 650,
  lineHeight: 1.5,
});

export const calloutContent = style({
  minWidth: 0,
  color: vars.color.muted,
  lineHeight: 1.78,
});

globalStyle(`${calloutContent} > :first-child`, {
  marginBlockStart: 0,
});

globalStyle(`${calloutContent} > :last-child`, {
  marginBlockEnd: 0,
});

export const tableScroller = style({
  maxWidth: '100%',
  marginBlock: vars.space.large,
  overflowX: 'auto',
});

export const codeTitle = style({
  margin: `${vars.space.medium} 0 -${vars.space.medium}`,
  padding: `${vars.space.small} ${vars.space.medium}`,
  border: `1px solid ${vars.color.border}`,
  borderBlockEnd: 0,
  borderRadius: `${vars.radius.small} ${vars.radius.small} 0 0`,
  background: vars.color.surfaceMuted,
  color: vars.color.muted,
  fontFamily:
    '"SFMono-Regular", Consolas, "Liberation Mono", "Noto Sans Mono", monospace',
  fontSize: '0.72rem',
  fontWeight: 650,
});

export const steps = style({
  display: 'grid',
  gap: vars.space.medium,
  paddingInlineStart: '1.5rem',
});
