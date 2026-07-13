import { createGlobalTheme, globalStyle } from '@vanilla-extract/css';

export const vars = createGlobalTheme(':root', {
  color: {
    background: '#f3f5ef',
    ink: '#10131a',
    muted: '#59606d',
    accent: '#37c967',
  },
  space: {
    small: '0.75rem',
    medium: '1.5rem',
    large: '3rem',
  },
  radius: {
    medium: '1rem',
  },
});

globalStyle('*', {
  boxSizing: 'border-box',
});

globalStyle('html', {
  colorScheme: 'light',
  scrollBehavior: 'smooth',
});

globalStyle('body', {
  margin: 0,
  minWidth: 320,
  background: vars.color.background,
  color: vars.color.ink,
  fontFamily:
    '-apple-system, BlinkMacSystemFont, "Segoe UI", "Noto Sans KR", sans-serif',
});

globalStyle('.shell', {
  width: 'min(100% - 2rem, 72rem)',
  marginInline: 'auto',
  paddingBlock: '18vh',
});

globalStyle('.eyebrow', {
  color: vars.color.accent,
  fontWeight: 750,
  letterSpacing: '0.08em',
  textTransform: 'uppercase',
});

globalStyle('h1', {
  maxWidth: '14ch',
  marginBlock: vars.space.small,
  fontSize: 'clamp(3rem, 8vw, 6rem)',
  letterSpacing: '-0.06em',
  lineHeight: 0.96,
});

globalStyle('p', {
  maxWidth: '42rem',
  color: vars.color.muted,
  fontSize: '1.05rem',
  lineHeight: 1.7,
});
