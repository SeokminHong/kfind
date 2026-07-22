import { agentDocuments } from '../../content/technical/agents';
import { TechnicalDocument } from '../../content/technical/types';

export { createLocationDocumentMeta as meta } from '../../app/metadata';

export default function AgentDocumentPage(): React.JSX.Element {
  return <TechnicalDocument documents={agentDocuments} />;
}
