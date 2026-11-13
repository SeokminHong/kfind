import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...options] = process.argv.slice(2);
const contract = options.includes('--review-contract');
const shared = options.includes('--align-shared-sentences');
const restored = options.includes('--restore-source-passages');
const proofread = options.includes('--proofread');
const independent = options.includes('--independent-proofreading');
if (!input || (restored && !shared) || (proofread && !restored)
    || (independent ? options.length !== 1
      : options.length !== Number(contract) + Number(shared) + Number(restored) + Number(proofread))) {
  throw new Error('usage: node apply-gsd-sentence-edits.mjs GSD_POSITIVES.jsonl ([--review-contract] [--align-shared-sentences [--restore-source-passages [--proofread]]] | --independent-proofreading)');
}
const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
const previousInput = 'd34735e79fad865a62685522859f2664e56d9f37330a1a11d17614aab82cda1e';
const currentFixtureInput = '8f3aa236f20a0913e78e04139539b636835a538ab5d552891856a73a5e77e5da';
const contextPosReviewInput = '120ad5000b84b3d61fef05214b6d703e0f573eb55330f0741cf455583806dce9';
if (independent ? checksum !== contextPosReviewInput
  : checksum !== previousInput && !(shared && checksum === currentFixtureInput)) {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}

function tsv(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8')
    .split(/\r?\n/).slice(1).filter(Boolean).map(line => line.split('\t'));
}

const editsBySentence = new Map();
const sentenceEdits = tsv(independent ? './gsd-independent-proofreading-edits.tsv' : './gsd-sentence-edits.tsv');
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
const annotationEdits = tsv(independent ? './gsd-independent-proofreading-annotations.tsv' : './gsd-sentence-edit-annotations.tsv');
if (shared) annotationEdits.push(...tsv('./gsd-shared-sentence-annotations.tsv'));
if (restored) annotationEdits.push(...tsv('./gsd-source-restoration-annotations.tsv'));
if (proofread) annotationEdits.push(...tsv('./gsd-proofreading-annotations.tsv'));
for (const [id, query, pos, form, lemma, xpos] of annotationEdits) {
  if (annotations.has(id) || (query !== '-' && (!query || !pos || !form || !lemma || !xpos))) {
    throw new Error(`invalid annotation: ${id}`);
  }
  // A joined word can replace two original annotations with one searchable lexeme.
  annotations.set(id, query === '-' ? null : { query, pos, form, lemma, xpos });
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
  if ((shared || independent) && edits) {
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
    const form = Buffer.from(annotation.form);
    const offset = Buffer.from(span.text).indexOf(form, span.start);
    if (offset < 0 || offset >= span.end) {
      throw new Error(`corrected form missing from aligned span: ${row.id} in ${candidate}`);
    }
    span.start = offset;
    span.end = span.start + form.length;
    Object.assign(row, annotation);
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
