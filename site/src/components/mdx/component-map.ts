import {
  BenchmarkSourceEvidence,
  MorphologyEvidence,
  SearchBaselineEvidence,
} from './benchmark-evidence';
import {
  Callout,
  CodeTitle,
  DocumentWrapper,
  Eyebrow,
  Lead,
  MdxLink,
  MdxPre,
  MdxTable,
  Steps,
} from './components';
import { GlossaryIndex } from './glossary-index';

export const mdxComponents = {
  BenchmarkSourceEvidence,
  Callout,
  CodeTitle,
  Eyebrow,
  GlossaryIndex,
  Lead,
  MorphologyEvidence,
  SearchBaselineEvidence,
  Steps,
  a: MdxLink,
  pre: MdxPre,
  table: MdxTable,
  wrapper: DocumentWrapper,
};
