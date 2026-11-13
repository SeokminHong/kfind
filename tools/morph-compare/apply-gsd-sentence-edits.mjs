import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input] = process.argv.slice(2);
if (!input || process.argv.length !== 3) {
  throw new Error('usage: node apply-gsd-sentence-edits.mjs GSD_POSITIVES.jsonl');
}
const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== 'd34735e79fad865a62685522859f2664e56d9f37330a1a11d17614aab82cda1e') {
  throw new Error(`unexpected GSD input SHA-256: ${checksum}`);
}

function tsv(path) {
  return readFileSync(new URL(path, import.meta.url), 'utf8')
    .split(/\r?\n/).slice(1).filter(Boolean).map(line => line.split('\t'));
}

const editsBySentence = new Map();
for (const [id, source, replacement, reason] of tsv('./gsd-sentence-edits.tsv')) {
  if (!source || source === replacement || !reason) throw new Error(`invalid edit: ${id}`);
  const edits = editsBySentence.get(id) ?? [];
  edits.push({ source, replacement });
  editsBySentence.set(id, edits);
}

const annotations = new Map();
for (const [id, query, pos, form, lemma, xpos] of tsv('./gsd-sentence-edit-annotations.tsv')) {
  if (annotations.has(id) || (query !== '-' && (!query || !pos || !form || !lemma || !xpos))) {
    throw new Error(`invalid annotation: ${id}`);
  }
  // A joined word can replace two original annotations with one searchable lexeme.
  annotations.set(id, query === '-' ? null : { query, pos, form, lemma, xpos });
}

function applyEdit(text, start, end, { source, replacement }) {
  const index = text.indexOf(source);
  if (index < 0 && text.includes(replacement)) return { text, start, end };
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
const output = original.toString('utf8').trimEnd().split('\n').flatMap(line => {
  const row = JSON.parse(line);
  const sentenceId = row.id.split(':')[1];
  const edits = editsBySentence.get(sentenceId);
  if (!edits) return line;
  usedSentences.add(sentenceId);
  let span = { text: row.text, start: row.gold_byte_start, end: row.gold_byte_end };
  const originalSpan = Buffer.from(span.text).subarray(span.start, span.end);
  for (const edit of edits) span = applyEdit(span.text, span.start, span.end, edit);

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
  return JSON.stringify(row);
});
if (annotations.size || usedSentences.size !== editsBySentence.size) {
  throw new Error(`unused edits or annotations: ${[...annotations.keys()]}`);
}
process.stdout.write(output.join('\n') + '\n');
