import { style } from '@vanilla-extract/css';

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
  gridTemplateColumns: 'auto minmax(0, 1fr)',
  gap: vars.space.medium,
  marginBlock: vars.space.large,
  padding: vars.space.large,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.medium,
  background: vars.color.surfaceMuted,
});

export const calloutNote = style({
  borderColor: vars.color.borderStrong,
});

export const calloutTip = style({
  borderColor: vars.color.success,
  background: vars.color.successWash,
});

export const calloutWarning = style({
  borderColor: vars.color.warning,
  background: vars.color.warningWash,
});

export const calloutDanger = style({
  borderColor: vars.color.danger,
  background: vars.color.dangerWash,
});

export const calloutLabel = style({
  color: vars.color.heading,
  fontSize: '0.75rem',
  fontWeight: 700,
  lineHeight: 1.65,
});

export const calloutContent = style({
  minWidth: 0,
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
