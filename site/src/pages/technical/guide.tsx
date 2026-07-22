import { guideDocuments } from '../../content/technical/guide';
import { TechnicalDocument } from '../../content/technical/types';

export { createLocationDocumentMeta as meta } from '../../app/metadata';

export default function GuideDocumentPage(): React.JSX.Element {
  return <TechnicalDocument documents={guideDocuments} />;
}
