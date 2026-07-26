interface MarkdownNode {
  children?: MarkdownNode[];
  data?: {
    hProperties?: Record<string, unknown>;
  };
  depth?: number;
  type?: string;
  value?: string;
}

const headingIdPattern = /\s+\[#(?<headingId>[a-z][a-z0-9-]*)\]\s*$/u;

export function remarkHeadingIds(): (tree: MarkdownNode) => void {
  return transformHeadingIds;
}

function transformHeadingIds(tree: MarkdownNode): void {
  const headingIds = new Set<string>();

  visit(tree, (node) => {
    if (node.type !== 'heading' || node.depth !== 2) {
      return;
    }

    const lastChild =
      node.children === undefined
        ? undefined
        : node.children[node.children.length - 1];
    const match = lastChild?.value?.match(headingIdPattern);
    if (lastChild === undefined || match === null || match === undefined) {
      throw new Error('MDX h2 headings require a stable [#heading-id]');
    }

    const headingId = match.groups?.headingId;
    if (headingId === undefined || headingIds.has(headingId)) {
      throw new Error(`MDX heading id is duplicated: ${headingId}`);
    }

    headingIds.add(headingId);
    lastChild.value = lastChild.value?.replace(headingIdPattern, '');
    node.data = {
      ...node.data,
      hProperties: { ...node.data?.hProperties, id: headingId },
    };
  });
}

function visit(
  node: MarkdownNode,
  visitor: (node: MarkdownNode) => void,
): void {
  visitor(node);
  for (const child of node.children ?? []) {
    visit(child, visitor);
  }
}
