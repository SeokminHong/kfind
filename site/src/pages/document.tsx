import { MdxDocument } from '../components/mdx';

export { createLocationDocumentMeta as meta } from '../app/metadata';

export default function DocumentRoute(): React.JSX.Element {
  return <MdxDocument />;
}
