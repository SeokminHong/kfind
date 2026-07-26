declare module '*.mdx' {
  import type { ComponentType } from 'react';

  const MdxContent: ComponentType<{
    readonly components?: Record<string, ComponentType>;
  }>;

  export default MdxContent;
}
