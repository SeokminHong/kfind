import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-copula-span-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const checksum = createHash('sha256').update(original).digest('hex');
if (checksum !== '1f916f738b5d9cda3388dd5d8f5502cf385bb3c2d5117a01e485b18af202ad05') {
  throw new Error(`unexpected dictionary input SHA-256: ${checksum}`);
}

const corrections = new Map();
const registry = readFileSync(new URL('./dictionary-copula-span-review.tsv', import.meta.url), 'utf8');
for (const line of registry.trimEnd().split('\n').slice(1)) {
  const [id, source, sourceId, originalSurface, expectedSurface, startText, endText] = line.split('\t');
  const start = Number(startText);
  const end = Number(endText);
  if (!id || !source || !sourceId || !expectedSurface || corrections.has(id)
      || !originalSurface?.endsWith(expectedSurface) || originalSurface === expectedSurface
      || !Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start < 0
      || end - start !== Buffer.byteLength(expectedSurface)) {
    throw new Error(`invalid copula span correction: ${line}`);
  }
  corrections.set(id, { source, sourceId, originalSurface, expectedSurface, start, end });
}

const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const correction = corrections.get(row.id);
  if (!correction) return line;
  const { source, sourceId, originalSurface, expectedSurface, start, end } = correction;
  const text = Buffer.from(row.text);
  if (row.expected !== true || row.query !== '이다' || row.pos !== 'adjective'
      || row.source !== source || row.source_id !== sourceId || row.surface !== originalSurface
      || row.gold_byte_start >= start || row.gold_byte_end !== end
      || !text.subarray(row.gold_byte_start, end).equals(Buffer.from(originalSurface))
      || !text.subarray(start, end).equals(Buffer.from(expectedSurface))) {
    throw new Error(`correction no longer matches dictionary example: ${row.id}`);
  }
  corrections.delete(row.id);
  return JSON.stringify({ ...row, gold_byte_start: start });
});
if (corrections.size) throw new Error(`unused corrections: ${[...corrections.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
