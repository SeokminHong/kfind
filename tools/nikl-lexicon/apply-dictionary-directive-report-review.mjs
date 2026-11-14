import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-directive-report-review.mjs CASES.jsonl');
}

const original = readFileSync(input);
const digest = createHash('sha256').update(original).digest('hex');
if (digest !== 'a2e53ceccd3f794199a310b252d866d1437372b235b0c15d9cacdc53ab139f8e') {
  throw new Error(`unexpected dictionary input SHA-256: ${digest}`);
}

let reviewed = false;
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  if (row.id !== '거든:12') return line;
  const span = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (reviewed || row.source !== 'krdict' || row.source_id !== '66501'
      || row.query !== '하다' || row.pos !== 'verb' || row.expected !== true
      || row.surface !== '말라셨거든' || !span.equals(Buffer.from(row.surface))) {
    throw new Error(`unexpected directive annotation: ${row.id}`);
  }
  reviewed = true;
  return JSON.stringify({ ...row, query: '말다' });
});
if (!reviewed) throw new Error('missing directive annotation');
process.stdout.write(output.join('\n') + '\n');
