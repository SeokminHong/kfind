import { globalStyle, style } from '@vanilla-extract/css';

import { vars } from '../../theme.css';

export const field = style({
  display: 'grid',
  gap: vars.space.xsmall,
});

export const labelRow = style({
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: vars.space.small,
});

globalStyle(`${labelRow} label`, {
  color: vars.color.heading,
  fontSize: '0.74rem',
  fontWeight: 650,
});

globalStyle(`${labelRow} span`, {
  color: vars.color.subtle,
  fontSize: '0.68rem',
});

export const surface = style({
  position: 'relative',
  minHeight: '14rem',
  maxHeight: '30rem',
  overflow: 'auto',
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.small,
  background: vars.color.codeBackground,
  selectors: {
    '&:focus-within': {
      borderColor: vars.color.link,
      outline: `2px solid ${vars.color.linkWash}`,
      outlineOffset: 0,
    },
  },
});

const textLayer = style({
  width: '100%',
  minHeight: '14rem',
  boxSizing: 'border-box',
  padding: vars.space.medium,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.76rem',
  lineHeight: 1.65,
  overflowWrap: 'anywhere',
  tabSize: 2,
  whiteSpace: 'pre-wrap',
});

export const highlights = style([
  textLayer,
  {
    position: 'absolute',
    inset: 0,
    color: vars.color.codeText,
    pointerEvents: 'none',
  },
]);

globalStyle(`${highlights} mark`, {
  padding: '0.05em 0.12em',
  borderBlockEnd: `1px solid ${vars.color.markBorder}`,
  background: vars.color.mark,
  color: vars.color.heading,
  fontWeight: 650,
});

export const editor = style([
  textLayer,
  {
    position: 'relative',
    border: 0,
    outline: 0,
    background: 'transparent',
    caretColor: vars.color.text,
    color: 'transparent',
    WebkitTextFillColor: 'transparent',
  },
]);

globalStyle(`${editor}::selection`, {
  background: vars.color.linkWash,
});

export const description = style({
  margin: 0,
  color: vars.color.muted,
  fontSize: '0.68rem',
});
