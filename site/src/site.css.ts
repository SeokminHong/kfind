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
  gap: vars.space.large,
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
  alignItems: 'center',
  gap: vars.space.medium,
  fontSize: '0.82rem',
});

globalStyle('.header-links a', {
  color: vars.color.muted,
  fontWeight: 600,
  textDecoration: 'none',
});

globalStyle('.header-cta', {
  padding: `${vars.space.xsmall} ${vars.space.medium}`,
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
});

globalStyle('.header-actions', {
  display: 'flex',
  flexShrink: 0,
  alignItems: 'center',
  gap: vars.space.medium,
});

globalStyle('.primary-navigation', {
  display: 'flex',
  minWidth: 0,
  alignItems: 'center',
  justifyContent: 'center',
  gap: vars.space.xsmall,
});

globalStyle('.primary-navigation a', {
  padding: `${vars.space.small} ${vars.space.medium}`,
  borderRadius: vars.radius.small,
  color: vars.color.muted,
  fontSize: '0.78rem',
  fontWeight: 600,
  textDecoration: 'none',
  whiteSpace: 'nowrap',
});

globalStyle('.primary-navigation a:hover', {
  background: vars.color.surfaceMuted,
  color: vars.color.heading,
});

globalStyle('.primary-navigation a[aria-current="page"]', {
  background: vars.color.linkWash,
  color: vars.color.link,
});

globalStyle('.language-control', {
  display: 'inline-flex',
  padding: '0.15rem',
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.pill,
  background: vars.color.surfaceMuted,
});

globalStyle('.language-control button', {
  padding: `0.2rem ${vars.space.small}`,
  border: 0,
  borderRadius: vars.radius.pill,
  background: 'transparent',
  color: vars.color.muted,
  fontSize: '0.7rem',
});

globalStyle('.language-control button[aria-pressed="true"]', {
  background: vars.color.surface,
  color: vars.color.heading,
  fontWeight: 650,
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

globalStyle('.document-navigation', {
  display: 'grid',
  alignContent: 'start',
  gap: vars.space.small,
});

globalStyle('.document-navigation-title', {
  margin: `0 0 ${vars.space.small}`,
  color: vars.color.subtle,
  fontSize: '0.7rem',
  fontWeight: 700,
  letterSpacing: '0.06em',
  textTransform: 'uppercase',
});

globalStyle('.document-navigation-category', {
  display: 'grid',
  gap: vars.space.xsmall,
  paddingBlockEnd: vars.space.medium,
});

globalStyle('.document-navigation-category-title', {
  margin: `${vars.space.small} ${vars.space.small} ${vars.space.xsmall}`,
  color: vars.color.subtle,
  fontSize: '0.65rem',
  fontWeight: 700,
  letterSpacing: '0.05em',
  textTransform: 'uppercase',
});

globalStyle('.document-navigation-page', {
  display: 'grid',
  gap: vars.space.xsmall,
});

globalStyle('.document-navigation-page-link', {
  padding: '0.38rem 0.5rem',
  borderRadius: vars.radius.small,
  color: vars.color.muted,
  fontSize: '0.82rem',
  textDecoration: 'none',
});

globalStyle('.document-navigation-page-link:hover', {
  background: vars.color.linkWash,
  color: vars.color.link,
});

globalStyle('.document-navigation-page-link[aria-current="page"]', {
  background: vars.color.linkWash,
  color: vars.color.link,
  fontWeight: 650,
});

globalStyle('.document-section-links', {
  display: 'grid',
  gap: '0.1rem',
  margin: 0,
  padding: `${vars.space.xsmall} 0 ${vars.space.small} ${vars.space.medium}`,
  borderInlineStart: `1px solid ${vars.color.border}`,
  listStyle: 'none',
});

globalStyle('.document-section-links a', {
  display: 'block',
  padding: `${vars.space.xsmall} ${vars.space.small}`,
  borderRadius: vars.radius.small,
  color: vars.color.subtle,
  fontSize: '0.76rem',
  lineHeight: 1.4,
  textDecoration: 'none',
});

globalStyle('.document-section-links a:hover', {
  color: vars.color.link,
});

globalStyle('.document-section-links a[aria-current="location"]', {
  background: vars.color.linkWash,
  color: vars.color.link,
  fontWeight: 650,
});

globalStyle('.mobile-navigation', {
  display: 'none',
  borderBlockEnd: `1px solid ${vars.color.border}`,
  background: vars.color.sidebar,
});

globalStyle('.mobile-navigation > button', {
  display: 'flex',
  width: '100%',
  padding: `${vars.space.small} ${vars.space.large}`,
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: vars.space.small,
  border: 0,
  background: 'transparent',
  color: vars.color.heading,
  cursor: 'pointer',
  fontSize: '0.8rem',
  fontWeight: 650,
  textAlign: 'start',
});

globalStyle('.mobile-navigation-chevron', {
  width: '1rem',
  height: '1rem',
  flex: '0 0 auto',
  fill: 'none',
  stroke: 'currentColor',
  strokeLinecap: 'round',
  strokeLinejoin: 'round',
  strokeWidth: 1.5,
  transition: 'transform 160ms ease',
});

globalStyle(
  '.mobile-navigation > button[aria-expanded="true"] .mobile-navigation-chevron',
  {
    transform: 'rotate(180deg)',
  },
);

globalStyle('.mobile-navigation-panel', {
  display: 'grid',
  gap: vars.space.xlarge,
  padding: `0 ${vars.space.large} ${vars.space.large}`,
});

globalStyle('.mobile-navigation .primary-navigation', {
  display: 'grid',
  gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
  alignItems: 'stretch',
});

globalStyle('.mobile-navigation .primary-navigation a', {
  whiteSpace: 'normal',
});

globalStyle('.mobile-navigation .document-navigation', {
  paddingBlockStart: vars.space.large,
  borderBlockStart: `1px solid ${vars.color.border}`,
});

globalStyle('.mobile-utilities', {
  display: 'flex',
  gap: vars.space.large,
  paddingBlockStart: vars.space.large,
  borderBlockStart: `1px solid ${vars.color.border}`,
  fontSize: '0.8rem',
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

globalStyle('.header-links', {
  '@media': {
    '(max-width: 42rem)': {
      display: 'none',
    },
  },
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

globalStyle('.document-overview', {
  display: 'grid',
  gap: vars.space.small,
  maxWidth: '48rem',
  marginBlockEnd: vars.space.large,
});

globalStyle('.document-overview p', {
  margin: 0,
  color: vars.color.muted,
  lineHeight: 1.78,
});

globalStyle('.document-links', {
  display: 'flex',
  flexWrap: 'wrap',
  gap: `${vars.space.small} ${vars.space.large}`,
  marginBlockEnd: vars.space.xlarge,
  fontSize: '0.85rem',
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
  maxWidth: '48rem',
  color: vars.color.muted,
  lineHeight: 1.78,
});

globalStyle('.table-scroll', {
  marginBlockStart: vars.space.large,
  overflowX: 'auto',
  WebkitOverflowScrolling: 'touch',
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
});

globalStyle('.table-scroll table', {
  width: 'max-content',
  minWidth: '100%',
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
  whiteSpace: 'nowrap',
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
  overflowX: 'auto',
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
});

globalStyle('.benchmark-figure img', {
  display: 'block',
  width: '100%',
  '@media': {
    '(max-width: 42rem)': {
      minWidth: '48rem',
    },
  },
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

globalStyle('.source-identifiers code', {
  overflowWrap: 'anywhere',
  wordBreak: 'break-all',
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
  flexWrap: 'wrap',
  gap: vars.space.large,
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
    '(max-width: 70rem)': {
      padding: `${vars.space.xlarge} ${vars.space.large}`,
    },
  },
});

globalStyle('.docs-shell', {
  '@media': {
    '(max-width: 70rem)': {
      display: 'block',
    },
  },
});

globalStyle('.docs-sidebar', {
  '@media': {
    '(max-width: 70rem)': {
      display: 'none',
    },
  },
});

globalStyle('.mobile-navigation', {
  '@media': {
    '(max-width: 70rem)': {
      display: 'block',
    },
  },
});

globalStyle('.header-inner > .primary-navigation', {
  '@media': {
    '(max-width: 70rem)': {
      display: 'none',
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
