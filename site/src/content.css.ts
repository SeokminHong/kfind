import { globalStyle } from '@vanilla-extract/css';

import { vars } from './theme.css';

globalStyle('.section-lead', {
  marginBlockEnd: vars.space.large,
});

globalStyle('.note[data-tone="warning"]', {
  borderInlineStartColor: vars.color.warning,
  background: vars.color.warningWash,
});

globalStyle('.note div > :last-child', {
  marginBlockEnd: 0,
});

globalStyle('.example-pair', {
  display: 'grid',
  gridTemplateColumns: 'minmax(10rem, 0.65fr) minmax(0, 1.35fr)',
  marginBlock: vars.space.large,
  overflow: 'hidden',
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.medium,
});

globalStyle('.example-pair > div', {
  display: 'grid',
  alignContent: 'center',
  minHeight: '7.5rem',
  gap: vars.space.xsmall,
  padding: vars.space.large,
  background: vars.color.surface,
});

globalStyle('.example-pair > div:first-child', {
  borderInlineEnd: `1px solid ${vars.color.border}`,
  background: vars.color.codeBackground,
});

globalStyle('.example-pair span, .example-grid span', {
  color: vars.color.subtle,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.64rem',
  fontWeight: 700,
  letterSpacing: '0.06em',
});

globalStyle('.example-pair strong', {
  color: vars.color.heading,
  fontSize: '1.65rem',
});

globalStyle('.example-pair p', {
  margin: 0,
  color: vars.color.heading,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.86rem',
});

globalStyle('.route-card-grid', {
  display: 'grid',
  gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
  gap: vars.space.medium,
});

globalStyle('.route-card', {
  display: 'grid',
  gap: vars.space.xsmall,
  minHeight: '9rem',
  padding: vars.space.large,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.medium,
  color: vars.color.text,
  textDecoration: 'none',
});

globalStyle('.route-card:hover', {
  borderColor: vars.color.link,
  background: vars.color.linkWash,
});

globalStyle('.route-card > span', {
  color: vars.color.link,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.64rem',
  fontWeight: 700,
  letterSpacing: '0.06em',
});

globalStyle('.route-card strong', {
  color: vars.color.heading,
  fontSize: '1rem',
});

globalStyle('.route-card p', {
  margin: 0,
  color: vars.color.muted,
  fontSize: '0.8rem',
});

globalStyle('.tag-list, .defaults-strip, .metric-timeline', {
  display: 'flex',
  flexWrap: 'wrap',
  gap: vars.space.small,
});

globalStyle('.tag-list', {
  marginBlockStart: vars.space.large,
});

globalStyle('.tag-list code, .defaults-strip span, .metric-timeline span', {
  padding: `${vars.space.xsmall} ${vars.space.small}`,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.pill,
  background: vars.color.surfaceMuted,
  fontSize: '0.72rem',
});

globalStyle('.defaults-strip', {
  marginBlockStart: vars.space.xlarge,
});

globalStyle('.defaults-strip code', {
  color: vars.color.link,
});

globalStyle('.next-link', {
  marginBlockStart: vars.space.large,
  fontSize: '0.86rem',
});

globalStyle('.option-card-grid', {
  display: 'grid',
  gridTemplateColumns: 'repeat(3, minmax(0, 1fr))',
  gap: vars.space.medium,
  marginBlockStart: vars.space.large,
});

globalStyle('.option-card', {
  padding: vars.space.large,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.medium,
  background: vars.color.surface,
});

globalStyle('.option-card[data-featured="true"]', {
  borderColor: vars.color.link,
  boxShadow: `inset 0 3px 0 ${vars.color.link}`,
});

globalStyle('.option-card header', {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  gap: vars.space.small,
  marginBlockEnd: vars.space.medium,
});

globalStyle('.option-card header code', {
  color: vars.color.heading,
  fontSize: '0.92rem',
  fontWeight: 700,
});

globalStyle('.option-card header span', {
  padding: '0.15rem 0.4rem',
  borderRadius: vars.radius.pill,
  background: vars.color.linkWash,
  color: vars.color.link,
  fontSize: '0.62rem',
  fontWeight: 650,
});

globalStyle('.option-card > p', {
  color: vars.color.muted,
  fontSize: '0.78rem',
});

globalStyle('.option-card pre', {
  padding: vars.space.small,
  fontSize: '0.68rem',
});

globalStyle('.option-card .option-result', {
  marginBlockEnd: vars.space.xsmall,
  color: vars.color.muted,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.68rem',
});

globalStyle('.option-result strong', {
  color: vars.color.heading,
});

globalStyle('.example-grid', {
  display: 'grid',
  gridTemplateColumns: 'repeat(3, minmax(0, 1fr))',
  gap: vars.space.medium,
  marginBlock: vars.space.large,
});

globalStyle('.example-grid article', {
  display: 'grid',
  alignContent: 'start',
  gap: vars.space.small,
  padding: vars.space.medium,
  borderBlockStart: `3px solid ${vars.color.borderStrong}`,
  background: vars.color.surfaceMuted,
});

globalStyle('.example-grid code', {
  color: vars.color.heading,
  fontWeight: 650,
});

globalStyle('.example-grid p', {
  margin: 0,
  color: vars.color.muted,
  fontSize: '0.75rem',
});

globalStyle('.compact-grid, .metric-definition-grid, .principle-grid', {
  display: 'grid',
  gridTemplateColumns: 'repeat(3, minmax(0, 1fr))',
  gap: vars.space.small,
  marginBlockStart: vars.space.large,
});

globalStyle('.compact-grid > div', {
  display: 'grid',
  gap: vars.space.xsmall,
  padding: vars.space.medium,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
});

globalStyle('.compact-grid strong', {
  color: vars.color.heading,
  fontSize: '0.78rem',
});

globalStyle('.compact-grid code', {
  color: vars.color.muted,
  fontSize: '0.66rem',
  lineHeight: 1.6,
});

globalStyle('.layer-stack', {
  display: 'grid',
  marginBlock: vars.space.large,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.medium,
});

globalStyle('.layer-stack > div', {
  display: 'grid',
  gridTemplateColumns: '2rem minmax(0, 1fr) auto',
  alignItems: 'center',
  gap: vars.space.medium,
  padding: vars.space.medium,
  borderBlockEnd: `1px solid ${vars.color.border}`,
});

globalStyle('.layer-stack > div:last-child', {
  borderBlockEnd: 0,
});

globalStyle('.layer-stack > div > span', {
  display: 'grid',
  width: '1.7rem',
  height: '1.7rem',
  placeItems: 'center',
  borderRadius: '50%',
  background: vars.color.linkWash,
  color: vars.color.link,
  fontSize: '0.7rem',
  fontWeight: 700,
});

globalStyle('.layer-stack strong', {
  color: vars.color.heading,
  fontSize: '0.82rem',
});

globalStyle('.layer-stack p', {
  margin: 0,
  color: vars.color.muted,
  fontSize: '0.72rem',
});

globalStyle('.layer-stack code', {
  fontSize: '0.68rem',
});

globalStyle('.decision-table', {
  display: 'grid',
  gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
  marginBlock: vars.space.large,
  borderBlockStart: `1px solid ${vars.color.border}`,
  borderInlineStart: `1px solid ${vars.color.border}`,
});

globalStyle('.decision-table > div', {
  display: 'grid',
  gap: vars.space.xsmall,
  padding: vars.space.medium,
  borderBlockEnd: `1px solid ${vars.color.border}`,
  borderInlineEnd: `1px solid ${vars.color.border}`,
});

globalStyle('.decision-table strong', {
  color: vars.color.heading,
  fontSize: '0.75rem',
});

globalStyle('.decision-table span', {
  color: vars.color.muted,
  fontSize: '0.74rem',
});

globalStyle('.architecture-lanes', {
  display: 'grid',
  gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
  marginBlock: vars.space.large,
  overflow: 'hidden',
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.medium,
});

globalStyle('.architecture-lanes > div', {
  padding: vars.space.large,
  background: vars.color.surfaceMuted,
});

globalStyle('.architecture-lanes > div:first-child', {
  borderInlineEnd: `1px solid ${vars.color.border}`,
});

globalStyle('.architecture-lanes > div > span', {
  color: vars.color.link,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.66rem',
  fontWeight: 700,
  letterSpacing: '0.06em',
});

globalStyle('.architecture-lanes ol', {
  display: 'grid',
  gap: vars.space.small,
  marginBlockEnd: 0,
  paddingInlineStart: '1.25rem',
  color: vars.color.muted,
  fontSize: '0.76rem',
});

globalStyle('.architecture-lanes > strong', {
  gridColumn: '1 / -1',
  padding: vars.space.medium,
  background: vars.color.link,
  color: vars.color.surface,
  fontSize: '0.78rem',
  textAlign: 'center',
});

globalStyle('.contract-list', {
  display: 'grid',
  gap: vars.space.xsmall,
  color: vars.color.muted,
  fontSize: '0.82rem',
});

globalStyle('.principle-grid article, .metric-definition-grid article', {
  padding: vars.space.medium,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
});

globalStyle('.principle-grid span', {
  color: vars.color.link,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.66rem',
  fontWeight: 700,
});

globalStyle('.principle-grid strong, .metric-definition-grid strong', {
  display: 'block',
  marginBlock: vars.space.xsmall,
  color: vars.color.heading,
  fontSize: '0.82rem',
});

globalStyle('.principle-grid p, .metric-definition-grid p', {
  margin: 0,
  color: vars.color.muted,
  fontSize: '0.72rem',
});

globalStyle('.limit-grid, .stat-strip', {
  display: 'grid',
  gridTemplateColumns: 'repeat(6, minmax(0, 1fr))',
  gap: vars.space.xsmall,
  marginBlock: vars.space.large,
});

globalStyle('.limit-grid > div, .stat-strip > div', {
  display: 'grid',
  gap: '0.15rem',
  padding: vars.space.small,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
  background: vars.color.surfaceMuted,
  textAlign: 'center',
});

globalStyle('.limit-grid span, .stat-strip span', {
  color: vars.color.subtle,
  fontSize: '0.58rem',
  fontWeight: 700,
});

globalStyle('.limit-grid strong, .stat-strip strong', {
  color: vars.color.heading,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.92rem',
});

globalStyle('.limit-grid small, .stat-strip small', {
  color: vars.color.muted,
  fontSize: '0.6rem',
});

globalStyle('.metric-timeline', {
  marginBlock: vars.space.large,
});

globalStyle('.metric-timeline span', {
  position: 'relative',
  borderRadius: vars.radius.small,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
});

globalStyle('.stat-strip', {
  gridTemplateColumns: 'repeat(4, minmax(0, 1fr))',
});

globalStyle(
  '.route-card-grid, .option-card-grid, .example-grid, .compact-grid, .metric-definition-grid, .principle-grid',
  {
    '@media': {
      '(max-width: 46rem)': {
        gridTemplateColumns: '1fr',
      },
    },
  },
);

globalStyle('.example-pair, .architecture-lanes', {
  '@media': {
    '(max-width: 42rem)': {
      gridTemplateColumns: '1fr',
    },
  },
});

globalStyle(
  '.example-pair > div:first-child, .architecture-lanes > div:first-child',
  {
    '@media': {
      '(max-width: 42rem)': {
        borderInlineEnd: 0,
        borderBlockEnd: `1px solid ${vars.color.border}`,
      },
    },
  },
);

globalStyle('.layer-stack > div', {
  '@media': {
    '(max-width: 42rem)': {
      gridTemplateColumns: '2rem minmax(0, 1fr)',
    },
  },
});

globalStyle('.layer-stack code', {
  '@media': {
    '(max-width: 42rem)': {
      gridColumn: '2',
    },
  },
});

globalStyle('.decision-table', {
  '@media': {
    '(max-width: 38rem)': {
      gridTemplateColumns: '1fr',
    },
  },
});

globalStyle('.limit-grid, .stat-strip', {
  '@media': {
    '(max-width: 54rem)': {
      gridTemplateColumns: 'repeat(3, minmax(0, 1fr))',
    },
    '(max-width: 34rem)': {
      gridTemplateColumns: 'repeat(2, minmax(0, 1fr))',
    },
  },
});
