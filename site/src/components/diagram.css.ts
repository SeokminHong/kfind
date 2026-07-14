import { globalStyle } from '@vanilla-extract/css';

import { vars } from '../theme.css';

globalStyle('.diagram', {
  margin: `${vars.space.xlarge} 0`,
  padding: vars.space.large,
  overflow: 'hidden',
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.medium,
  background: vars.color.surfaceMuted,
});

globalStyle('.diagram-heading', {
  display: 'flex',
  alignItems: 'baseline',
  gap: vars.space.small,
  marginBlockEnd: vars.space.large,
});

globalStyle('.diagram-heading > span', {
  color: vars.color.link,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.66rem',
  fontWeight: 700,
  letterSpacing: '0.08em',
});

globalStyle('.diagram-heading h3', {
  margin: 0,
  fontSize: '0.9rem',
});

globalStyle('.flow-diagram, .split-source, .split-paths', {
  margin: 0,
  padding: 0,
  listStyle: 'none',
});

globalStyle('.flow-diagram', {
  display: 'grid',
  gridTemplateColumns: 'repeat(auto-fit, minmax(9.5rem, 1fr))',
  gap: vars.space.medium,
});

globalStyle('.diagram-node', {
  position: 'relative',
  minWidth: 0,
  padding: vars.space.medium,
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
});

globalStyle('.flow-diagram > .diagram-node:not(:last-child)::after', {
  position: 'absolute',
  top: '50%',
  right: `calc(-1 * ${vars.space.medium})`,
  color: vars.color.subtle,
  content: '→',
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  transform: 'translateY(-50%)',
});

globalStyle('.diagram-node > span', {
  display: 'block',
  marginBlockEnd: vars.space.xsmall,
  color: vars.color.link,
  fontFamily: '"SFMono-Regular", Consolas, monospace',
  fontSize: '0.64rem',
  fontWeight: 700,
});

globalStyle('.diagram-node strong', {
  display: 'block',
  color: vars.color.heading,
  fontSize: '0.82rem',
});

globalStyle('.diagram-node p', {
  margin: `${vars.space.xsmall} 0 0`,
  color: vars.color.muted,
  fontSize: '0.74rem',
  lineHeight: 1.55,
});

globalStyle('.diagram figcaption', {
  marginBlockStart: vars.space.medium,
  color: vars.color.subtle,
  fontSize: '0.7rem',
});

globalStyle('.split-diagram', {
  display: 'grid',
  gridTemplateColumns: 'minmax(10rem, 0.7fr) minmax(0, 1.3fr)',
  alignItems: 'center',
  gap: vars.space.xlarge,
});

globalStyle('.split-source', {
  position: 'relative',
});

globalStyle('.split-source::after', {
  position: 'absolute',
  top: '50%',
  right: `calc(-1 * ${vars.space.xlarge})`,
  color: vars.color.subtle,
  content: '→',
  transform: 'translateY(-50%)',
});

globalStyle('.split-paths', {
  display: 'grid',
  gap: vars.space.small,
});

globalStyle('.flow-diagram > .diagram-node:not(:last-child)::after', {
  '@media': {
    '(max-width: 46rem)': {
      display: 'none',
    },
  },
});

globalStyle('.split-diagram', {
  '@media': {
    '(max-width: 42rem)': {
      gridTemplateColumns: '1fr',
      gap: vars.space.medium,
    },
  },
});

globalStyle('.split-source::after', {
  '@media': {
    '(max-width: 42rem)': {
      display: 'none',
    },
  },
});
