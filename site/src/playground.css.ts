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
  overflow: 'hidden',
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.medium,
  background: vars.color.surface,
});

globalStyle('.playground-workspace', {
  display: 'grid',
  gridTemplateColumns: 'minmax(0, 1.7fr) minmax(21rem, 0.85fr)',
  borderBlockEnd: `1px solid ${vars.color.border}`,
  background: vars.color.surfaceMuted,
});

globalStyle('.playground-main-inputs', {
  display: 'grid',
  minWidth: 0,
  alignContent: 'start',
  gap: vars.space.medium,
  padding: vars.space.large,
  background: vars.color.surfaceMuted,
});

globalStyle('.playground-main-inputs .field-query', {
  width: 'min(24rem, 100%)',
});

globalStyle('.desktop-settings', {
  minWidth: 0,
  padding: vars.space.large,
  borderInlineStart: `1px solid ${vars.color.border}`,
  background: vars.color.surface,
});

globalStyle('.playground-settings', {
  display: 'grid',
  alignContent: 'start',
  gap: vars.space.large,
});

globalStyle('.mobile-settings', {
  display: 'none',
});

globalStyle('.field', {
  display: 'grid',
  gap: vars.space.xsmall,
});

globalStyle('.field label', {
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

globalStyle('.preset-picker', {
  display: 'grid',
  gap: vars.space.small,
});

globalStyle('.control-heading', {
  display: 'flex',
  flexWrap: 'wrap',
  alignItems: 'baseline',
  gap: vars.space.small,
});

globalStyle('.control-heading strong', {
  color: vars.color.heading,
  fontSize: '0.74rem',
});

globalStyle('.control-heading span', {
  color: vars.color.subtle,
  fontSize: '0.66rem',
});

globalStyle('.preset-actions', {
  display: 'flex',
  flexWrap: 'wrap',
  gap: vars.space.xsmall,
});

globalStyle('.preset-actions button', {
  minHeight: '2rem',
  padding: `${vars.space.xsmall} ${vars.space.small}`,
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.pill,
  background: vars.color.surface,
  color: vars.color.link,
  cursor: 'pointer',
  fontSize: '0.68rem',
  lineHeight: 1.2,
});

globalStyle('.preset-actions button:hover', {
  borderColor: vars.color.link,
  background: vars.color.linkWash,
});

globalStyle('.option-panel', {
  display: 'grid',
  gap: vars.space.small,
  paddingBlockStart: vars.space.large,
  borderBlockStart: `1px solid ${vars.color.border}`,
});

globalStyle('.option-grid', {
  display: 'grid',
  gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
  gap: vars.space.medium,
});

globalStyle('.option-grid .field > p', {
  margin: 0,
  color: vars.color.muted,
  fontSize: '0.68rem',
  lineHeight: 1.45,
});

globalStyle('.resource-loader', {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: vars.space.medium,
  padding: `${vars.space.small} ${vars.space.medium}`,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
});

globalStyle('.resource-loader > div', {
  display: 'flex',
  minWidth: 0,
  alignItems: 'center',
  gap: vars.space.small,
});

globalStyle('.resource-loader > div > div', {
  display: 'grid',
  gap: '0.15rem',
});

globalStyle('.resource-dot', {
  width: '0.5rem',
  height: '0.5rem',
  flex: '0 0 auto',
  borderRadius: '50%',
  background: vars.color.warning,
});

globalStyle('.resource-loader[data-state="ready"]', {
  borderColor: vars.color.success,
  background: vars.color.successWash,
});

globalStyle('.resource-loader[data-state="ready"] .resource-dot', {
  background: vars.color.success,
});

globalStyle('.resource-loader[data-state="error"]', {
  borderColor: vars.color.danger,
  background: vars.color.dangerWash,
});

globalStyle('.resource-loader[data-state="error"] .resource-dot', {
  background: vars.color.danger,
});

globalStyle('.resource-loader strong', {
  color: vars.color.heading,
  fontSize: '0.72rem',
});

globalStyle('.resource-loader span', {
  color: vars.color.muted,
  fontSize: '0.66rem',
});

globalStyle('.resource-loader[data-state="needed"] span', {
  color: vars.color.warning,
});

globalStyle('.resource-loader button', {
  minHeight: '2rem',
  flex: '0 0 auto',
  paddingInline: vars.space.small,
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
  color: vars.color.link,
  fontSize: '0.7rem',
});

globalStyle('.resource-loader button:disabled', {
  color: vars.color.subtle,
});

globalStyle('.desktop-settings .resource-loader', {
  alignItems: 'stretch',
  flexDirection: 'column',
});

globalStyle('.desktop-settings .resource-loader button', {
  width: '100%',
});

globalStyle('.resource-ready-label', {
  flex: '0 0 auto',
  padding: `${vars.space.xsmall} ${vars.space.small}`,
  borderRadius: vars.radius.pill,
  background: vars.color.surface,
  color: `${vars.color.success} !important`,
  fontWeight: 650,
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

globalStyle('.playground-output #result-summary', {
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

globalStyle('.result-error', {
  margin: 0,
  color: vars.color.danger,
});

globalStyle('.result-tabs', {
  display: 'grid',
  gap: vars.space.medium,
  marginBlockStart: vars.space.medium,
});

globalStyle('.result-tab-list', {
  display: 'flex',
  gap: vars.space.xsmall,
  borderBlockEnd: `1px solid ${vars.color.border}`,
});

globalStyle('.result-tab-list button', {
  display: 'inline-flex',
  alignItems: 'center',
  gap: vars.space.xsmall,
  padding: `${vars.space.small} ${vars.space.medium}`,
  border: 0,
  borderBlockEnd: '2px solid transparent',
  background: 'transparent',
  color: vars.color.muted,
  cursor: 'pointer',
  fontSize: '0.72rem',
  fontWeight: 650,
});

globalStyle('.result-tab-list button[data-active]', {
  borderBlockEndColor: vars.color.link,
  color: vars.color.link,
});

globalStyle('.result-tab-list button span', {
  minWidth: '1.25rem',
  paddingInline: vars.space.xsmall,
  borderRadius: vars.radius.pill,
  background: vars.color.surfaceMuted,
  color: vars.color.subtle,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.6rem',
  textAlign: 'center',
});

globalStyle('.result-tab-panel:focus-visible', {
  outline: `2px solid ${vars.color.linkWash}`,
  outlineOffset: vars.space.xsmall,
});

globalStyle('.match-list', {
  display: 'grid',
  maxHeight: '20rem',
  margin: 0,
  padding: 0,
  overflowY: 'auto',
  listStyle: 'none',
});

globalStyle('.match-list li', {
  borderBlockEnd: `1px solid ${vars.color.border}`,
});

globalStyle('.match-item', {
  display: 'grid',
  width: '100%',
  gridTemplateColumns: '1.7rem minmax(6rem, 0.35fr) auto minmax(0, 1fr)',
  alignItems: 'center',
  gap: vars.space.small,
  padding: `${vars.space.small} 0`,
  border: 0,
  background: 'transparent',
  color: vars.color.text,
  cursor: 'pointer',
  textAlign: 'start',
});

globalStyle('.match-item:hover', {
  background: vars.color.surfaceMuted,
});

globalStyle('.match-index', {
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

globalStyle('.match-provenance', {
  margin: 0,
  overflow: 'hidden',
  color: vars.color.subtle,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.62rem',
  lineHeight: 1.5,
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

globalStyle('.match-description', {
  display: 'grid',
  minWidth: 0,
  gap: '0.1rem',
});

globalStyle('.match-analysis', {
  overflow: 'hidden',
  color: vars.color.muted,
  fontSize: '0.7rem',
  lineHeight: 1.45,
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

globalStyle('.match-empty', {
  padding: vars.space.medium,
  color: vars.color.muted,
  fontSize: '0.76rem',
});

globalStyle('.raw-json-panel pre', {
  maxHeight: '18rem',
  margin: 0,
  overflow: 'auto',
  fontSize: '0.68rem',
});

globalStyle('.options-modal-heading', {
  display: 'flex',
  alignItems: 'start',
  justifyContent: 'space-between',
  gap: vars.space.medium,
});

globalStyle('.mobile-settings-summary', {
  overflow: 'hidden',
  color: vars.color.muted,
  fontSize: '0.66rem',
  fontWeight: 400,
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
});

globalStyle('.mobile-settings-heading', {
  display: 'flex',
  minWidth: 0,
  flexWrap: 'wrap',
  alignItems: 'center',
  gap: vars.space.xsmall,
});

globalStyle('.mobile-resource-state', {
  padding: '0.15rem 0.35rem',
  borderRadius: vars.radius.pill,
  background: vars.color.warningWash,
  color: vars.color.warning,
  fontSize: '0.6rem',
  fontWeight: 650,
  whiteSpace: 'nowrap',
});

globalStyle('.mobile-resource-state[data-state="ready"]', {
  background: vars.color.successWash,
  color: vars.color.success,
});

globalStyle('.mobile-resource-state[data-state="error"]', {
  background: vars.color.dangerWash,
  color: vars.color.danger,
});

globalStyle('.playground-workspace', {
  '@media': {
    '(max-width: 64rem)': {
      gridTemplateColumns: 'minmax(0, 1fr)',
    },
  },
});

globalStyle('.playground-main-inputs', {
  '@media': {
    '(max-width: 34rem)': {
      padding: vars.space.medium,
    },
  },
});

globalStyle('.desktop-settings', {
  '@media': {
    '(max-width: 64rem)': {
      display: 'none',
    },
  },
});

globalStyle('.mobile-settings', {
  '@media': {
    '(max-width: 64rem)': {
      display: 'block',
      padding: vars.space.large,
      borderBlockEnd: `1px solid ${vars.color.border}`,
      background: vars.color.surfaceMuted,
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

globalStyle('.resource-loader', {
  '@media': {
    '(max-width: 34rem)': {
      alignItems: 'stretch',
      flexDirection: 'column',
    },
  },
});

globalStyle('.resource-loader button', {
  '@media': {
    '(max-width: 34rem)': {
      width: '100%',
    },
  },
});

globalStyle('.match-item', {
  '@media': {
    '(max-width: 46rem)': {
      gridTemplateColumns: '1.7rem minmax(0, 1fr) auto',
    },
  },
});

globalStyle('.match-description', {
  '@media': {
    '(max-width: 46rem)': {
      gridColumn: '2 / -1',
    },
  },
});
