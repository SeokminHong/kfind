import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-mothada-and-hasio-review.mjs CASES.jsonl');
}
const original = readFileSync(input);
const digest = createHash('sha256').update(original).digest('hex');
if (digest !== '488f356c42a8b917b39f23a08c4381b39b969a1165b33ecbd5b24a216d37e77e') {
  throw new Error(`unexpected dictionary input SHA-256: ${digest}`);
}

const reviewed = new Set();
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id !== '거든:6' && row.id !== '오:7') return line;
  const span = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (reviewed.has(row.id) || row.source !== 'krdict'
      || row.query !== '하다' || row.pos !== 'verb' || row.expected !== true
      || !span.equals(Buffer.from(row.surface))) {
    throw new Error(`unexpected predicate annotation: ${row.id}`);
  }
  reviewed.add(row.id);
  if (row.id === '거든:6' && row.source_id === '66501' && row.surface === '못했거든') {
    return JSON.stringify({ ...row, query: '못하다' });
  }
  if (row.id === '오:7' && row.source_id === '86112' && row.surface === '뭐하시오') {
    const start = row.gold_byte_start;
    const text = Buffer.from(row.text);
    return JSON.stringify({
      ...row,
      text: text.subarray(0, start).toString() + '뭐 하시오' + text.subarray(row.gold_byte_end).toString(),
      surface: '하시오',
      gold_byte_start: start + Buffer.byteLength('뭐 '),
      gold_byte_end: row.gold_byte_end + 1,
    });
  }
  throw new Error(`unexpected dictionary annotation: ${row.id}`);
});
if (reviewed.size !== 2) throw new Error('missing dictionary annotations');
process.stdout.write(output.join('\n') + '\n');
