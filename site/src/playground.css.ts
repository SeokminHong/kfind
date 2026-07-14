import { globalStyle } from '@vanilla-extract/css';

import { vars } from './theme.css';

globalStyle('.wasm-state', {
  display: 'inline-flex',
  flexShrink: 0,
  alignItems: 'center',
  width: 'fit-content',
  gap: vars.space.xsmall,
  padding: `${vars.space.xsmall} ${vars.space.small}`,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.pill,
  background: vars.color.surfaceMuted,
  color: vars.color.muted,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.68rem',
});

globalStyle('.state-dot', {
  width: '0.45rem',
  height: '0.45rem',
  borderRadius: '50%',
  background: vars.color.warning,
});

globalStyle('.wasm-state[data-state="ready"] .state-dot', {
  background: vars.color.success,
});

globalStyle('.wasm-state[data-state="error"]', {
  borderColor: vars.color.danger,
  background: vars.color.dangerWash,
  color: vars.color.danger,
});

globalStyle('.wasm-state[data-state="error"] .state-dot', {
  background: vars.color.danger,
});

globalStyle('.playground-layout', {
  display: 'grid',
  gridTemplateColumns: 'minmax(17rem, 0.8fr) minmax(0, 1.2fr)',
  overflow: 'hidden',
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.medium,
  background: vars.color.surface,
});

globalStyle('.playground-controls', {
  display: 'grid',
  alignContent: 'start',
  gap: vars.space.large,
  padding: vars.space.large,
  borderInlineEnd: `1px solid ${vars.color.border}`,
  background: vars.color.surfaceMuted,
});

globalStyle('.field', {
  display: 'grid',
  gap: vars.space.xsmall,
});

globalStyle('.field label, .preset-fieldset legend', {
  color: vars.color.heading,
  fontSize: '0.74rem',
  fontWeight: 650,
});

globalStyle('.text-control', {
  width: '100%',
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
  color: vars.color.text,
});

globalStyle('input.text-control', {
  height: '2.5rem',
  paddingInline: vars.space.small,
});

globalStyle('textarea.text-control', {
  minHeight: '10rem',
  resize: 'vertical',
  padding: vars.space.small,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.76rem',
  lineHeight: 1.6,
});

globalStyle('.text-control:focus', {
  borderColor: vars.color.link,
  outline: `2px solid ${vars.color.linkWash}`,
  outlineOffset: 0,
});

globalStyle('.field-query .text-control', {
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.88rem',
});

globalStyle('.field-query p', {
  margin: 0,
  color: vars.color.muted,
  fontSize: '0.68rem',
});

globalStyle('.preset-fieldset', {
  margin: 0,
  padding: 0,
  border: 0,
});

globalStyle('.preset-fieldset legend', {
  marginBlockEnd: vars.space.xsmall,
});

globalStyle('.preset-list', {
  display: 'flex',
  flexWrap: 'wrap',
  gap: vars.space.xsmall,
});

globalStyle('.preset-list button', {
  padding: '0.32rem 0.55rem',
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
  color: vars.color.muted,
  fontSize: '0.7rem',
});

globalStyle('.preset-list button:hover', {
  borderColor: vars.color.link,
  color: vars.color.link,
});

globalStyle('.option-grid', {
  display: 'grid',
  gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
  gap: vars.space.medium,
});

globalStyle('.field-gap', {
  gridColumn: '1 / -1',
});

globalStyle('.field-label-row', {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
});

globalStyle('.field-label-row span', {
  color: vars.color.subtle,
  fontSize: '0.68rem',
});

globalStyle('.run-button', {
  minHeight: '2.55rem',
  border: `1px solid ${vars.color.link}`,
  borderRadius: vars.radius.small,
  background: vars.color.link,
  color: vars.color.surface,
  fontSize: '0.78rem',
  fontWeight: 650,
});

globalStyle('.run-button:hover:not(:disabled)', {
  background: vars.color.linkHover,
});

globalStyle('.run-button:disabled', {
  borderColor: vars.color.borderStrong,
  background: vars.color.border,
  color: vars.color.subtle,
});

globalStyle('.resource-loader', {
  display: 'grid',
  gap: vars.space.small,
  padding: vars.space.small,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
  background: vars.color.warningWash,
});

globalStyle('.resource-loader > div', {
  display: 'grid',
  gap: '0.15rem',
});

globalStyle('.resource-loader strong', {
  color: vars.color.heading,
  fontSize: '0.72rem',
});

globalStyle('.resource-loader span', {
  color: vars.color.muted,
  fontSize: '0.66rem',
});

globalStyle('.resource-loader span[data-state="ready"]', {
  color: vars.color.success,
});

globalStyle('.resource-loader span[data-state="error"]', {
  color: vars.color.danger,
});

globalStyle('.resource-loader button', {
  minHeight: '2.3rem',
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
  color: vars.color.link,
  fontSize: '0.7rem',
});

globalStyle('.resource-loader button:disabled', {
  color: vars.color.subtle,
});

globalStyle('.playground-output', {
  minWidth: 0,
  padding: vars.space.large,
});

globalStyle('.output-head', {
  display: 'flex',
  alignItems: 'start',
  justifyContent: 'space-between',
  gap: vars.space.medium,
  marginBlockEnd: vars.space.medium,
});

globalStyle('.output-label', {
  marginBlockEnd: '0.15rem',
  color: vars.color.subtle,
  fontSize: '0.68rem',
  fontWeight: 700,
  letterSpacing: '0.05em',
  textTransform: 'uppercase',
});

globalStyle('#result-summary', {
  margin: 0,
  color: vars.color.heading,
  fontSize: '0.82rem',
  fontWeight: 600,
});

globalStyle('.execution-time', {
  padding: '0.2rem 0.45rem',
  borderRadius: vars.radius.small,
  background: vars.color.successWash,
  color: vars.color.success,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.65rem',
});

globalStyle('.result-preview', {
  minHeight: '11rem',
  padding: vars.space.medium,
  overflow: 'auto',
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
  background: vars.color.codeBackground,
  color: vars.color.codeText,
  fontSize: '0.82rem',
  lineHeight: 1.9,
  whiteSpace: 'pre-wrap',
  wordBreak: 'break-word',
});

globalStyle('.result-preview mark', {
  padding: '0.05em 0.12em',
  borderBlockEnd: `1px solid ${vars.color.markBorder}`,
  background: vars.color.mark,
  color: vars.color.heading,
  fontWeight: 650,
});

globalStyle('.result-error', {
  margin: 0,
  color: vars.color.danger,
});

globalStyle('.match-section', {
  marginBlockStart: vars.space.large,
});

globalStyle('.match-list', {
  display: 'grid',
  maxHeight: '17rem',
  gap: vars.space.small,
  margin: 0,
  padding: 0,
  overflowY: 'auto',
  listStyle: 'none',
});

globalStyle('.match-list li', {
  padding: vars.space.small,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
});

globalStyle('.match-list li > div', {
  display: 'grid',
  gridTemplateColumns: '1.7rem 1fr auto',
  alignItems: 'center',
  gap: vars.space.small,
});

globalStyle('.match-list li > div span:first-child', {
  color: vars.color.subtle,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.62rem',
});

globalStyle('.match-list strong', {
  color: vars.color.heading,
  fontSize: '0.8rem',
});

globalStyle('.match-list code', {
  color: vars.color.muted,
  fontSize: '0.64rem',
});

globalStyle('.match-list p', {
  margin: `${vars.space.xsmall} 0 0 2.45rem`,
  color: vars.color.muted,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.62rem',
  lineHeight: 1.5,
});

globalStyle('.match-empty', {
  color: vars.color.muted,
  fontSize: '0.76rem',
});

globalStyle('.raw-details', {
  marginBlockStart: vars.space.large,
  borderBlockStart: `1px solid ${vars.color.border}`,
});

globalStyle('.raw-details > button', {
  width: '100%',
  paddingBlock: vars.space.small,
  border: 0,
  background: 'transparent',
  color: vars.color.muted,
  cursor: 'pointer',
  fontSize: '0.72rem',
  textAlign: 'start',
});

globalStyle('.raw-details pre', {
  maxHeight: '18rem',
  marginBlockStart: 0,
  fontSize: '0.68rem',
});

globalStyle('.playground-note', {
  marginBlockStart: vars.space.medium,
});

globalStyle('.playground-layout', {
  '@media': {
    '(max-width: 68rem)': {
      gridTemplateColumns: '1fr',
    },
  },
});

globalStyle('.playground-controls', {
  '@media': {
    '(max-width: 68rem)': {
      borderInlineEnd: 0,
      borderBlockEnd: `1px solid ${vars.color.border}`,
    },
    '(max-width: 34rem)': {
      padding: vars.space.medium,
    },
  },
});

globalStyle('.playground-output', {
  '@media': {
    '(max-width: 34rem)': {
      padding: vars.space.medium,
    },
  },
});

globalStyle('.option-grid', {
  '@media': {
    '(max-width: 34rem)': {
      gridTemplateColumns: '1fr',
    },
  },
});

globalStyle('.field-gap', {
  '@media': {
    '(max-width: 34rem)': {
      gridColumn: 'auto',
    },
  },
});

globalStyle('.match-list li > div', {
  '@media': {
    '(max-width: 34rem)': {
      gridTemplateColumns: '1.7rem 1fr',
    },
  },
});

globalStyle('.match-list li > div code', {
  '@media': {
    '(max-width: 34rem)': {
      gridColumn: '2',
    },
  },
});
