import { referenceDocuments } from '../../content/technical/reference';
import { TechnicalDocument } from '../../content/technical/types';

export { createLocationDocumentMeta as meta } from '../../app/metadata';

export default function ReferenceDocumentPage(): React.JSX.Element {
  return <TechnicalDocument documents={referenceDocuments} />;
}
