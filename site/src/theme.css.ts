import { createGlobalTheme, globalStyle } from '@vanilla-extract/css';

export const vars = createGlobalTheme(':root', {
  color: {
    background: '#ffffff',
    sidebar: '#f7f8fa',
    surface: '#ffffff',
    surfaceMuted: '#f5f7f9',
    text: '#1f2937',
    heading: '#111827',
    muted: '#5f6b7a',
    subtle: '#8490a0',
    border: '#dfe3e8',
    borderStrong: '#c7cdd5',
    link: '#1f5fbf',
    linkHover: '#174a96',
    linkWash: '#eef5ff',
    codeBackground: '#f6f8fa',
    codeText: '#253044',
    success: '#18794e',
    successWash: '#edf9f2',
    warning: '#98620b',
    warningWash: '#fff8e6',
    danger: '#b42318',
    dangerWash: '#fff1f0',
    mark: '#fff1a8',
    markBorder: '#e5c94b',
  },
  space: {
    xsmall: '0.25rem',
    small: '0.5rem',
    medium: '0.75rem',
    large: '1rem',
    xlarge: '1.5rem',
    section: '2.5rem',
  },
  radius: {
    small: '0.375rem',
    medium: '0.625rem',
    pill: '999px',
  },
  content: {
    shell: '88rem',
    article: '68rem',
  },
  font: {
    sans: '"Pretendard Variable", Pretendard, -apple-system, BlinkMacSystemFont, system-ui, "Segoe UI", "Noto Sans KR", sans-serif',
  },
});

globalStyle('*', {
  boxSizing: 'border-box',
  '@media': {
    '(prefers-reduced-motion: reduce)': {
      scrollBehavior: 'auto',
      transitionDuration: '0.01ms',
    },
  },
});

globalStyle('html', {
  scrollBehavior: 'smooth',
  scrollPaddingTop: '5rem',
});

globalStyle('body', {
  margin: 0,
  minWidth: 320,
  background: vars.color.background,
  color: vars.color.text,
  fontFamily: vars.font.sans,
  fontSynthesis: 'none',
  lineHeight: 1.65,
  textRendering: 'optimizeLegibility',
});

globalStyle('a', {
  color: vars.color.link,
  textUnderlineOffset: '0.18em',
});

globalStyle('a:hover', {
  color: vars.color.linkHover,
});

globalStyle('button, input, select, textarea', {
  color: 'inherit',
  font: 'inherit',
});

globalStyle('button', {
  cursor: 'pointer',
});

globalStyle('button:disabled', {
  cursor: 'not-allowed',
});

globalStyle('code, pre', {
  fontFamily:
    '"SFMono-Regular", Consolas, "Liberation Mono", "Noto Sans Mono", monospace',
});

globalStyle('code', {
  borderRadius: '0.2rem',
  color: vars.color.codeText,
  fontSize: '0.9em',
});

globalStyle('pre', {
  margin: `${vars.space.medium} 0`,
  padding: vars.space.medium,
  overflowX: 'auto',
  border: `1px solid ${vars.color.border}`,
  borderRadius: vars.radius.small,
  background: vars.color.codeBackground,
  color: vars.color.codeText,
  fontSize: '0.82rem',
  lineHeight: 1.65,
});

globalStyle('img', {
  display: 'block',
  maxWidth: '100%',
});

globalStyle('h1, h2, h3, p', {
  marginBlockStart: 0,
});

globalStyle(':focus-visible', {
  outline: `3px solid ${vars.color.link}`,
  outlineOffset: '2px',
});

globalStyle('.skip-link', {
  position: 'fixed',
  zIndex: 100,
  top: vars.space.small,
  left: vars.space.small,
  padding: `${vars.space.xsmall} ${vars.space.small}`,
  border: `1px solid ${vars.color.borderStrong}`,
  borderRadius: vars.radius.small,
  background: vars.color.surface,
  color: vars.color.heading,
  transform: 'translateY(-200%)',
});

globalStyle('.skip-link:focus', {
  transform: 'translateY(0)',
});
