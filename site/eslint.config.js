import config from '@seokminhong/configs/eslint';

export default config({
  envs: ['browser', 'node'],
  ignores: [
    'node_modules',
    'dist',
    '.wrangler',
    'public/benchmarks',
    'src/generated-wasm',
    'worker-configuration.d.ts',
  ],
});
