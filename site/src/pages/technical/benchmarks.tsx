import { benchmarkDocuments } from '../../content/technical/benchmarks';
import { TechnicalDocument } from '../../content/technical/types';

export { createLocationDocumentMeta as meta } from '../../app/metadata';

export default function BenchmarkDocumentPage(): React.JSX.Element {
  return <TechnicalDocument documents={benchmarkDocuments} />;
}
