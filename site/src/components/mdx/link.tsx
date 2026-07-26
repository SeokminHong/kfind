import type { ComponentPropsWithoutRef } from 'react';

import { Link } from 'react-router';

import { localizedDocumentHref, useDocumentLocale } from '../../app/i18n';
import { RoutePath } from '../../app/navigation';
import { getGlossaryContent } from '../glossary';
import { GlossaryTooltip } from '../glossary-tooltip';

export type MdxLinkProps = ComponentPropsWithoutRef<'a'>;

export function MdxLink({
  children,
  href = '',
  ...props
}: MdxLinkProps): React.JSX.Element {
  const locale = useDocumentLocale();
  const glossaryPrefix = `${RoutePath.Glossary}#`;

  if (href.startsWith(glossaryPrefix) && typeof children === 'string') {
    const termId = href.slice(glossaryPrefix.length);
    const term = getGlossaryContent(locale).terms.find(
      (candidate) => candidate.id === termId,
    );

    if (term !== undefined) {
      return <GlossaryTooltip term={term}>{children}</GlossaryTooltip>;
    }
  }

  if (href.startsWith('/')) {
    return (
      <Link {...props} to={localizedDocumentHref(href, locale)}>
        {children}
      </Link>
    );
  }

  return (
    <a {...props} href={href}>
      {children}
    </a>
  );
}
