import type { ReactNode } from 'react';

import type { GlossaryTerm } from './glossary';

import { GlossaryTooltip } from './glossary-tooltip';

interface GlossaryAlias {
  readonly alias: string;
  readonly normalized: string;
  readonly requiresAsciiBoundary: boolean;
  readonly term: GlossaryTerm;
}

interface GlossaryMatch {
  readonly alias: GlossaryAlias;
  readonly index: number;
}

const asciiWordCharacter = /\w/u;

function buildAliases(
  terms: readonly GlossaryTerm[],
): readonly GlossaryAlias[] {
  return terms
    .flatMap((term) =>
      term.aliases.map((alias) => ({
        alias,
        normalized: alias.toLowerCase(),
        requiresAsciiBoundary: /[A-Za-z]/u.test(alias),
        term,
      })),
    )
    .sort((left, right) => right.alias.length - left.alias.length);
}

function hasAsciiWordCharacter(value: string | undefined): boolean {
  return value !== undefined && asciiWordCharacter.test(value);
}

function findAliasIndex(
  text: string,
  normalizedText: string,
  alias: GlossaryAlias,
  startIndex: number,
): number {
  let index = normalizedText.indexOf(alias.normalized, startIndex);

  while (index >= 0) {
    const endIndex = index + alias.alias.length;
    const hasValidBoundary =
      !alias.requiresAsciiBoundary ||
      (!hasAsciiWordCharacter(text[index - 1]) &&
        !hasAsciiWordCharacter(text[endIndex]));

    if (hasValidBoundary) {
      return index;
    }

    index = normalizedText.indexOf(alias.normalized, index + 1);
  }

  return -1;
}

function findNextMatch(
  text: string,
  normalizedText: string,
  seenTerms: ReadonlySet<string>,
  aliases: readonly GlossaryAlias[],
  startIndex: number,
): GlossaryMatch | undefined {
  let nextMatch: GlossaryMatch | undefined;

  for (const alias of aliases) {
    if (seenTerms.has(alias.term.id)) {
      continue;
    }

    const index = findAliasIndex(text, normalizedText, alias, startIndex);

    if (index >= 0 && (nextMatch === undefined || index < nextMatch.index)) {
      nextMatch = { alias, index };
    }
  }

  return nextMatch;
}

export function annotateGlossaryText(
  text: string,
  seenTerms: Set<string>,
  terms: readonly GlossaryTerm[],
): ReactNode {
  const aliases = buildAliases(terms);
  const normalizedText = text.toLowerCase();
  const nodes: ReactNode[] = [];
  let startIndex = 0;
  let match = findNextMatch(
    text,
    normalizedText,
    seenTerms,
    aliases,
    startIndex,
  );

  while (match !== undefined) {
    const { alias, index } = match;
    const endIndex = index + alias.alias.length;

    if (index > startIndex) {
      nodes.push(text.slice(startIndex, index));
    }

    nodes.push(
      <GlossaryTooltip key={alias.term.id} term={alias.term}>
        {text.slice(index, endIndex)}
      </GlossaryTooltip>,
    );
    seenTerms.add(alias.term.id);
    startIndex = endIndex;
    match = findNextMatch(text, normalizedText, seenTerms, aliases, startIndex);
  }

  if (nodes.length === 0) {
    return text;
  }

  if (startIndex < text.length) {
    nodes.push(text.slice(startIndex));
  }

  return nodes;
}
