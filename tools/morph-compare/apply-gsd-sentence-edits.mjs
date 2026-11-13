import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...options] = process.argv.slice(2);
const contract = options.includes('--review-contract');
const shared = options.includes('--align-shared-sentences');
const restored = options.includes('--restore-source-passages');
const proofread = options.includes('--proofread');
const independent = options.includes('--independent-proofreading');
const contextAnnotations = options.includes('--context-annotations');
const nominalModifiers = options.includes('--nominal-modifiers');
const derivedNominals = options.includes('--derived-nominals');
const scopeNominals = options.includes('--scope-nominals');
const jiNominals = options.includes('--ji-nominals');
const motSpacing = options.includes('--mot-spacing');
const ijenContext = options.includes('--ijen-context');
const quantifierAnnotations = options.includes('--quantifier-annotations');
const lexicalWhole = options.includes('--lexical-whole');
const contextLexemes = options.includes('--context-lexemes');
const receiveDerivation = options.includes('--receive-derivation');
const missegmentedLexemes = options.includes('--missegmented-lexemes');
const properBoundaries = options.includes('--proper-boundaries');
const nominalDerivedBoundaries = options.includes('--nominal-derived-boundaries');
const compoundBoundaries = options.includes('--compound-boundaries');
const predicateReanalysis = options.includes('--predicate-reanalysis');
if (!input || (restored && !shared) || (proofread && !restored)
    || (independent || contextAnnotations || nominalModifiers || derivedNominals || scopeNominals || jiNominals || motSpacing || ijenContext || quantifierAnnotations || lexicalWhole || contextLexemes || receiveDerivation || missegmentedLexemes || properBoundaries || nominalDerivedBoundaries || compoundBoundaries || predicateReanalysis ? options.length !== 1
      : options.length !== Number(contract) + Number(shared) + Number(restored) + Number(proofread))) {
  throw new Error('usage: node apply-gsd-sentence-edits.mjs GSD_POSITIVES.jsonl ([--review-contract] [--align-shared-sentences [--restore-source-passages [--proofread]]] | --independent-proofreading | --context-annotations | --nominal-modifiers | --derived-nominals | --scope-nominals | --ji-nominals | --mot-spacing | --ijen-context | --quantifier-annotations | --lexical-whole | --context-lexemes | --receive-derivation | --missegmented-lexemes | --proper-boundaries | --nominal-derived-boundaries | --compound-boundaries | --predicate-reanalysis)');
}
const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
const previousInput = 'd34735e79fad865a62685522859f2664e56d9f37330a1a11d17614aab82cda1e';
const currentFixtureInput = '8f3aa236f20a0913e78e04139539b636835a538ab5d552891856a73a5e77e5da';
const contextPosReviewInput = '120ad5000b84b3d61fef05214b6d703e0f573eb55330f0741cf455583806dce9';
if (predicateReanalysis ? checksum !== 'cdde94ed29316f0cf4e32b7b02aebda5d0b7dc19100122382a9b4ea2bc954fa7'
  : compoundBoundaries ? checksum !== '78846cf33374a9df14391b28003e0e5bb6f09953d7db8420499be051068d7434'
  : nominalDerivedBoundaries ? checksum !== '05de72d018f39d9a03e2787f95f36c2fcf052e0d82313e8bf7c1714135e6f39d'
  : properBoundaries ? checksum !== '6184892a41099170a8a1085ec1b619f96765a60cdf64cef20b00540864626b8e'
  : missegmentedLexemes ? checksum !== '2766c3fc291cab4c945c4d57fa3ed94c117a2df0d78d90612158249c77459985'
  : receiveDerivation ? checksum !== '03ef9ff20b40e0f6756a45dab2488e357ad814c08faf904d2954ebda2d24f006'
  : contextLexemes ? checksum !== '247faf42cb30ea97665ed7e7495b2b4402f0721825545490723c96a7a84ea7fd'
  : lexicalWhole ? checksum !== '13bf24075dd714060f0366e04ddcf17a385bc902cc704a88d1d6cda3457a2152'
  : quantifierAnnotations ? checksum !== 'cb67ca3aacdeda3cf5a8b756c60489684142591abfba8950f9fe5e7318df321d'
  : ijenContext ? checksum !== 'a1a77de82af73f1e801cae68f309c1ae245e5fcbdf3f97857d9cd51c27aa7165'
  : motSpacing ? checksum !== '1552fa35dfc7e06dd49e8e79208755f84fefb2e4a7e4673dba8b256194c3c1e6'
  : jiNominals ? checksum !== 'a990e0f0fa2eb5488ff817b3f177fa38c7561c06cae3a9df92437cad63edaeef'
  : scopeNominals ? checksum !== 'e09e933e865128f1f2bfd937a5d192e26d5250967800a82e4e786108c5554ff3'
  : derivedNominals ? checksum !== '957dad65dbc029053f56d531d8969638c4a52cb4270fa8b898c8380f722eef9d'
  : nominalModifiers ? checksum !== '1d8380f3c6c6f892fab50f510b0013ad8db55ea1105901ac24f15508510c4bb2'
  : contextAnnotations ? checksum !== '457c654f940283ce6010293d8497b9cd70faf39d254560b551fdb0eeb2f691ae'
  : independent ? checksum !== contextPosReviewInput
  : checksum !== previousInput && !(shared && checksum === currentFixtureInput)) {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}

function tsv(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8')
    .split(/\r?\n/).slice(1).filter(Boolean).map(line => line.split('\t'));
}

const editsBySentence = new Map();
const sentenceEdits = derivedNominals || scopeNominals || jiNominals || ijenContext || quantifierAnnotations || lexicalWhole || receiveDerivation || properBoundaries || nominalDerivedBoundaries || compoundBoundaries ? [] : tsv(predicateReanalysis ? './gsd-predicate-reanalysis-edits.tsv'
  : missegmentedLexemes ? './gsd-missegmented-lexemes-edits.tsv'
  : motSpacing ? './gsd-mot-spacing-edits.tsv'
  : nominalModifiers ? './gsd-nominal-modifier-edits.tsv'
  : contextAnnotations ? './gsd-context-annotation-edits.tsv'
  : independent ? './gsd-independent-proofreading-edits.tsv'
  : contextLexemes ? './gsd-context-lexemes-edits.tsv' : './gsd-sentence-edits.tsv');
if (shared) sentenceEdits.push(...tsv('./gsd-shared-sentence-edits.tsv'));
if (restored) sentenceEdits.push(...tsv('./gsd-source-restoration-edits.tsv'));
if (proofread) sentenceEdits.push(...tsv('./gsd-proofreading-edits.tsv'));
for (const [id, source, replacement, reason] of sentenceEdits) {
  if (!source || source === replacement || !reason) throw new Error(`invalid edit: ${id}`);
  const edits = editsBySentence.get(id) ?? [];
  edits.push({ source, replacement });
  editsBySentence.set(id, edits);
}

const annotations = new Map();
const annotationEdits = tsv(predicateReanalysis ? './gsd-predicate-reanalysis-annotations.tsv'
  : compoundBoundaries ? './gsd-compound-boundaries-annotations.tsv'
  : nominalDerivedBoundaries ? './gsd-nominal-derived-boundaries-annotations.tsv'
  : properBoundaries ? './gsd-proper-boundaries-annotations.tsv'
  : missegmentedLexemes ? './gsd-missegmented-lexemes-annotations.tsv'
  : receiveDerivation ? './gsd-receive-derivation-annotations.tsv'
  : contextLexemes ? './gsd-context-lexemes-annotations.tsv'
  : lexicalWhole ? './gsd-lexical-whole-annotations.tsv'
  : quantifierAnnotations ? './gsd-quantifier-annotations.tsv'
  : ijenContext ? './gsd-ijen-context-annotations.tsv'
  : motSpacing ? './gsd-mot-spacing-annotations.tsv'
  : jiNominals ? './gsd-ji-nominal-annotations.tsv'
  : scopeNominals ? './gsd-scope-nominal-annotations.tsv'
  : derivedNominals ? './gsd-derived-nominal-annotations.tsv'
  : nominalModifiers ? './gsd-nominal-modifier-annotations.tsv'
  : contextAnnotations ? './gsd-context-annotations.tsv'
  : independent ? './gsd-independent-proofreading-annotations.tsv' : './gsd-sentence-edit-annotations.tsv');
if (shared) annotationEdits.push(...tsv('./gsd-shared-sentence-annotations.tsv'));
if (restored) annotationEdits.push(...tsv('./gsd-source-restoration-annotations.tsv'));
if (proofread) annotationEdits.push(...tsv('./gsd-proofreading-annotations.tsv'));
for (const [id, query, pos, form, lemma, xpos, goldForm = form] of annotationEdits) {
  if (annotations.has(id) || (query !== '-' && (!query || !pos || !form || !lemma || !xpos
      || !goldForm || !form.startsWith(goldForm)))) {
    throw new Error(`invalid annotation: ${id}`);
  }
  // A joined word can replace two original annotations with one searchable lexeme.
  annotations.set(id, query === '-' ? null : { goldForm, fields: { query, pos, form, lemma, xpos } });
}

const excludedSentences = new Set();
if (contract && !restored) {
  for (const [id, reason, contrast] of tsv('./gsd-nonstandard-sentences.tsv')) {
    if (!reason || !contrast || excludedSentences.has(id)) {
      throw new Error(`invalid sentence exclusion: ${id}`);
    }
    excludedSentences.add(id);
  }
}

function applyEdit(text, start, end, { source, replacement }) {
  const index = text.indexOf(source);
  if (text.includes(replacement) && (index < 0 || replacement.includes(source))) {
    return { text, start, end };
  }
  if (index < 0 || text.indexOf(source, index + source.length) >= 0) {
    throw new Error(`edit source must occur once: ${source} in ${text}`);
  }
  // Keep common characters outside the changed range so adjacent annotations retain their span.
  let left = 0;
  let right = 0;
  while (left < source.length && left < replacement.length && source[left] === replacement[left]) {
    left++;
  }
  while (right < source.length - left && right < replacement.length - left
      && source.at(-1 - right) === replacement.at(-1 - right)) {
    right++;
  }
  const editStart = Buffer.byteLength(text.slice(0, index + left));
  const editEnd = Buffer.byteLength(text.slice(0, index + source.length - right));
  const delta = Buffer.byteLength(replacement) - Buffer.byteLength(source);
  return {
    text: text.slice(0, index) + replacement + text.slice(index + source.length),
    start: start >= editEnd ? start + delta : Math.min(start, editStart),
    end: end <= editStart ? end : Math.max(end, editEnd) + delta,
  };
}

const usedSentences = new Set();
const alignedSentences = new Map();
const output = original.toString('utf8').trimEnd().split('\n').flatMap(line => {
  const row = JSON.parse(line);
  const sentenceId = row.id.split(':')[1];
  const edits = editsBySentence.get(sentenceId);
  const excluded = excludedSentences.has(sentenceId);
  if (!edits && !excluded && !annotations.has(row.id)) return line;
  if (edits) usedSentences.add(sentenceId);
  let span = { text: row.text, start: row.gold_byte_start, end: row.gold_byte_end };
  const originalSpan = Buffer.from(span.text).subarray(span.start, span.end);
  for (const edit of edits ?? []) span = applyEdit(span.text, span.start, span.end, edit);
  if ((shared || independent || contextAnnotations || nominalModifiers || motSpacing) && edits) {
    const aligned = alignedSentences.get(sentenceId);
    if (aligned !== undefined && aligned !== span.text) {
      throw new Error(`inconsistent shared sentence: ${sentenceId}`);
    }
    alignedSentences.set(sentenceId, span.text);
  }

  const candidate = Buffer.from(span.text).subarray(span.start, span.end);
  const annotation = annotations.get(row.id);
  if (annotations.has(row.id)) {
    annotations.delete(row.id);
    if (!annotation) return [];
    const form = Buffer.from(annotation.fields.form);
    const offset = Buffer.from(span.text).indexOf(form, span.start);
    if (offset < 0 || offset >= span.end) {
      throw new Error(`corrected form missing from aligned span: ${row.id} in ${candidate}`);
    }
    span.start = offset;
    span.end = span.start + Buffer.byteLength(annotation.goldForm);
    Object.assign(row, annotation.fields);
  } else if (!candidate.equals(originalSpan)) {
    throw new Error(`annotation required: ${row.id}: ${originalSpan} → ${candidate}`);
  }
  row.text = span.text;
  row.gold_byte_start = span.start;
  row.gold_byte_end = span.end;
  const gold = Buffer.from(span.text).subarray(span.start, span.end);
  if (gold.length !== span.end - span.start || !gold.length
      || !gold.equals(Buffer.from(gold.toString('utf8')))) {
    throw new Error(`invalid corrected UTF-8 span: ${row.id}`);
  }
  if (excluded) {
    row.contract_expected = null;
    row.contract_reason = 'nonstandard-input';
  }
  return JSON.stringify(row);
});
if (annotations.size || usedSentences.size !== editsBySentence.size) {
  throw new Error(`unused edits or annotations: ${[...annotations.keys()]}`);
}
process.stdout.write(output.join('\n') + '\n');
