import { globalStyle } from '@vanilla-extract/css';

import { vars } from './theme.css';

globalStyle('.docs-header', {
  position: 'sticky',
  zIndex: 30,
  top: 0,
  height: '3.75rem',
  borderBottom: `1px solid ${vars.color.border}`,
  background: vars.color.surface,
});

globalStyle('.header-inner', {
  display: 'flex',
  width: `min(100%, ${vars.content.shell})`,
  height: '100%',
  alignItems: 'center',
  justifyContent: 'space-between',
  marginInline: 'auto',
  paddingInline: vars.space.large,
});

globalStyle('.brand', {
  display: 'inline-flex',
  alignItems: 'center',
  gap: vars.space.small,
  color: vars.color.heading,
  fontSize: '0.95rem',
  fontWeight: 700,
  textDecoration: 'none',
});

globalStyle('.brand-mark', {
  display: 'grid',
  width: '1.8rem',
  height: '1.8rem',
  placeItems: 'center',
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.small,
  background: vars.color.codeBackground,
  color: vars.color.link,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.72rem',
});

globalStyle('.brand-suffix', {
  paddingInlineStart: vars.space.small,
  borderInlineStart: `1px solid ${vars.color.border}`,
  color: vars.color.muted,
  fontWeight: 500,
});

globalStyle('.header-links', {
  display: 'flex',
  gap: vars.space.large,
  fontSize: '0.82rem',
});

globalStyle('.docs-shell', {
  display: 'grid',
  width: `min(100%, ${vars.content.shell})`,
  minHeight: 'calc(100svh - 3.75rem)',
  gridTemplateColumns: '15rem minmax(0, 1fr)',
  marginInline: 'auto',
});

globalStyle('.docs-sidebar', {
  position: 'sticky',
  top: '3.75rem',
  height: 'calc(100svh - 3.75rem)',
  padding: `${vars.space.xlarge} ${vars.space.large}`,
  overflowY: 'auto',
  borderInlineEnd: `1px solid ${vars.color.border}`,
  background: vars.color.sidebar,
});

globalStyle('.docs-sidebar nav', {
  display: 'grid',
  gap: vars.space.large,
});

globalStyle('.navigation-group', {
  display: 'grid',
  gap: '0.15rem',
});

globalStyle('.navigation-group p', {
  margin: `0 0 ${vars.space.xsmall}`,
  color: vars.color.subtle,
  fontSize: '0.7rem',
  fontWeight: 700,
  letterSpacing: '0.06em',
  textTransform: 'uppercase',
});

globalStyle('.navigation-group a', {
  padding: '0.38rem 0.5rem',
  borderRadius: vars.radius.small,
  color: vars.color.muted,
  fontSize: '0.82rem',
  textDecoration: 'none',
});

globalStyle('.navigation-group a:hover', {
  background: vars.color.linkWash,
  color: vars.color.link,
});

globalStyle('.navigation-group a[aria-current="page"]', {
  background: vars.color.linkWash,
  color: vars.color.link,
  fontWeight: 650,
});

globalStyle('.mobile-navigation', {
  display: 'none',
  borderBlockEnd: `1px solid ${vars.color.border}`,
  background: vars.color.sidebar,
});

globalStyle('.mobile-navigation summary', {
  padding: `${vars.space.small} ${vars.space.large}`,
  color: vars.color.heading,
  cursor: 'pointer',
  fontSize: '0.8rem',
  fontWeight: 650,
});

globalStyle('.mobile-navigation nav', {
  display: 'grid',
  alignItems: 'start',
  gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
  gap: vars.space.large,
  padding: `0 ${vars.space.large} ${vars.space.large}`,
});

globalStyle('.route-loading', {
  display: 'grid',
  minHeight: '100svh',
  placeItems: 'center',
  color: vars.color.muted,
  fontSize: '0.82rem',
});

globalStyle('.docs-content', {
  minWidth: 0,
  padding: `3.5rem clamp(${vars.space.large}, 5vw, 5rem) ${vars.space.xlarge}`,
});

globalStyle('.docs-content > article, .docs-footer', {
  width: `min(100%, ${vars.content.article})`,
});

globalStyle('.document-intro', {
  paddingBlockEnd: vars.space.large,
});

globalStyle('.document-kind', {
  marginBlockEnd: vars.space.small,
  color: vars.color.subtle,
  fontSize: '0.78rem',
});

globalStyle('.document-intro h1', {
  marginBlockEnd: vars.space.medium,
  color: vars.color.heading,
  fontSize: '2.35rem',
  letterSpacing: '-0.035em',
  lineHeight: 1.15,
});

globalStyle('.lead', {
  maxWidth: '48rem',
  marginBlockEnd: vars.space.large,
  color: vars.color.muted,
  fontSize: '1.02rem',
});

globalStyle('.document-links', {
  display: 'flex',
  flexWrap: 'wrap',
  gap: `${vars.space.small} ${vars.space.large}`,
  marginBlockEnd: vars.space.xlarge,
  fontSize: '0.85rem',
});

globalStyle('.note', {
  padding: vars.space.medium,
  borderInlineStart: `3px solid ${vars.color.link}`,
  background: vars.color.linkWash,
  fontSize: '0.86rem',
});

globalStyle('.note strong', {
  display: 'block',
  marginBlockEnd: vars.space.xsmall,
  color: vars.color.heading,
});

globalStyle('.note p', {
  margin: 0,
  color: vars.color.muted,
});

globalStyle('.doc-section', {
  paddingBlock: vars.space.section,
  borderBlockStart: `1px solid ${vars.color.border}`,
  scrollMarginTop: '4.5rem',
});

globalStyle('.doc-section h2', {
  marginBlockEnd: vars.space.medium,
  color: vars.color.heading,
  fontSize: '1.5rem',
  letterSpacing: '-0.02em',
  lineHeight: 1.3,
});

globalStyle('.doc-section h3', {
  margin: `${vars.space.xlarge} 0 ${vars.space.small}`,
  color: vars.color.heading,
  fontSize: '1rem',
});

globalStyle('.doc-section > p', {
  maxWidth: '52rem',
  color: vars.color.muted,
});

globalStyle('.table-scroll', {
  marginBlockStart: vars.space.large,
  overflowX: 'auto',
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
});

globalStyle('table', {
  width: '100%',
  borderCollapse: 'collapse',
  fontSize: '0.84rem',
  textAlign: 'start',
});

globalStyle('th, td', {
  padding: '0.7rem 0.85rem',
  borderBlockEnd: `1px solid ${vars.color.border}`,
  textAlign: 'start',
  verticalAlign: 'top',
});

globalStyle('thead th', {
  background: vars.color.surfaceMuted,
  color: vars.color.heading,
  fontWeight: 650,
});

globalStyle('tbody th', {
  color: vars.color.heading,
  fontWeight: 600,
});

globalStyle('tbody tr:last-child > *', {
  borderBlockEnd: 0,
});

globalStyle('.steps', {
  display: 'grid',
  gap: vars.space.large,
  margin: `${vars.space.large} 0 0`,
  paddingInlineStart: '1.5rem',
});

globalStyle('.steps li', {
  paddingInlineStart: vars.space.small,
});

globalStyle('.steps strong', {
  color: vars.color.heading,
  fontSize: '0.9rem',
});

globalStyle('.steps p', {
  margin: `${vars.space.xsmall} 0 0`,
  color: vars.color.muted,
  fontSize: '0.88rem',
});

globalStyle('.section-title-row', {
  display: 'flex',
  alignItems: 'start',
  justifyContent: 'space-between',
  gap: vars.space.large,
  marginBlockEnd: vars.space.large,
});

globalStyle('.section-title-row h2', {
  marginBlockEnd: vars.space.xsmall,
});

globalStyle('.section-title-row p', {
  margin: 0,
  color: vars.color.muted,
  fontSize: '0.86rem',
});

globalStyle('.benchmark-figure', {
  margin: `${vars.space.xlarge} 0 0`,
  padding: vars.space.medium,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
});

globalStyle('.benchmark-figure figcaption', {
  marginBlockStart: vars.space.small,
  color: vars.color.muted,
  fontSize: '0.76rem',
});

globalStyle('.reference-link', {
  marginBlockStart: vars.space.large,
  fontSize: '0.86rem',
});

globalStyle('.reference-list', {
  display: 'grid',
  gap: vars.space.xsmall,
  marginBlockStart: vars.space.large,
  paddingInlineStart: '1.25rem',
  fontSize: '0.84rem',
});

globalStyle('.docs-footer', {
  display: 'flex',
  justifyContent: 'space-between',
  paddingBlock: vars.space.xlarge,
  borderBlockStart: `1px solid ${vars.color.border}`,
  color: vars.color.subtle,
  fontSize: '0.76rem',
});

globalStyle('.docs-footer a', {
  color: vars.color.muted,
});

globalStyle('.docs-content', {
  '@media': {
    '(max-width: 52rem)': {
      padding: `${vars.space.xlarge} ${vars.space.large}`,
    },
  },
});

globalStyle('.docs-shell', {
  '@media': {
    '(max-width: 52rem)': {
      display: 'block',
    },
  },
});

globalStyle('.docs-sidebar', {
  '@media': {
    '(max-width: 52rem)': {
      display: 'none',
    },
  },
});

globalStyle('.mobile-navigation', {
  '@media': {
    '(max-width: 52rem)': {
      display: 'block',
    },
  },
});

globalStyle('.header-inner', {
  '@media': {
    '(max-width: 34rem)': {
      paddingInline: vars.space.medium,
    },
  },
});

globalStyle('.header-links', {
  '@media': {
    '(max-width: 34rem)': {
      gap: vars.space.medium,
    },
  },
});

globalStyle('.brand-suffix', {
  '@media': {
    '(max-width: 34rem)': {
      display: 'none',
    },
  },
});

globalStyle('.section-title-row', {
  '@media': {
    '(max-width: 42rem)': {
      display: 'grid',
    },
  },
});
