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

export const editor = style({
  overflow: 'hidden',
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

globalStyle(`${editor} .cm-editor`, {
  minHeight: '14rem',
  maxHeight: '30rem',
  background: vars.color.codeBackground,
  color: vars.color.codeText,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.76rem',
  lineHeight: 1.65,
});

globalStyle(`${editor} .cm-editor.cm-focused`, {
  outline: 0,
});

globalStyle(`${editor} .cm-scroller`, {
  minHeight: '14rem',
  maxHeight: '30rem',
  overflow: 'auto',
  fontFamily: 'inherit',
  lineHeight: 'inherit',
});

globalStyle(`${editor} .cm-content`, {
  minHeight: '14rem',
  padding: vars.space.medium,
  caretColor: vars.color.text,
  tabSize: 2,
});

globalStyle(`${editor} .cm-line`, {
  padding: 0,
});

globalStyle(`${editor} .cm-cursor`, {
  borderInlineStartColor: vars.color.text,
});

globalStyle(`${editor} .cm-selectionBackground`, {
  background: `${vars.color.linkWash} !important`,
});

globalStyle(`${editor} .cm-kfind-match`, {
  background: vars.color.mark,
  boxShadow: `inset 0 -1px ${vars.color.markBorder}`,
  color: vars.color.heading,
});

export const description = style({
  margin: 0,
  color: vars.color.muted,
  fontSize: '0.68rem',
});
