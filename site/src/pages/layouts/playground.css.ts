import { globalStyle } from '@vanilla-extract/css';

import { vars } from '../../theme.css';

globalStyle('.header-cta[aria-current="page"]', {
  borderColor: vars.color.link,
  background: vars.color.linkWash,
  color: vars.color.link,
});

globalStyle('.playground-shell', {
  width: `min(100%, ${vars.content.shell})`,
  minHeight: 'calc(100svh - 3.75rem)',
  marginInline: 'auto',
});

globalStyle('.playground-content', {
  display: 'grid',
  minWidth: 0,
  gridTemplateColumns: 'minmax(0, 1fr)',
  alignContent: 'start',
  padding: `3.5rem clamp(${vars.space.large}, 4vw, 3.5rem) ${vars.space.xlarge}`,
  '@media': {
    '(max-width: 70rem)': {
      padding: `${vars.space.xlarge} ${vars.space.large}`,
    },
  },
});

globalStyle(
  '.playground-content > article, .playground-content > .docs-footer',
  {
    width: '100%',
    minWidth: 0,
  },
);
