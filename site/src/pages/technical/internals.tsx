import type { TechnicalDocuments } from '../../content/technical/types';

import { internalDocuments } from '../../content/technical/internals';
import { morphologyDocuments } from '../../content/technical/morphology';
import { TechnicalDocument } from '../../content/technical/types';

export { createLocationDocumentMeta as meta } from '../../app/metadata';

const documents: TechnicalDocuments = {
  ...internalDocuments,
  ...morphologyDocuments,
};

export default function InternalsDocumentPage(): React.JSX.Element {
  return <TechnicalDocument documents={documents} />;
}
