import config from '@seokminhong/configs/eslint';
import react from '@seokminhong/configs/eslint/react';

export default config({
  envs: ['browser', 'node'],
  extensions: [react({ reactRouter: true })],
  ignores: [
    'node_modules',
    'build',
    '.react-router',
    '.wrangler',
    'public/benchmarks',
    'src/generated-wasm',
    'worker-configuration.d.ts',
  ],
});
