import { cliDocuments } from '../../content/technical/cli';
import { TechnicalDocument } from '../../content/technical/types';

export { createLocationDocumentMeta as meta } from '../../app/metadata';

export default function CliDocumentPage(): React.JSX.Element {
  return <TechnicalDocument documents={cliDocuments} />;
}
