import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const [input, ...extra] = process.argv.slice(2);
if (!input || extra.length) {
  throw new Error('usage: node apply-dictionary-explicit-copula-review.mjs CASES.jsonl');
}
const original = readFileSync(input);
const sha256 = createHash('sha256').update(original).digest('hex');
if (sha256 !== '76aecbfe6609a17bb9445d1f87d39a07984bd2c24135121c662eeb945ba403b5') {
  throw new Error(`unexpected dictionary input SHA-256: ${sha256}`);
}

// Reviewed explicit copula ranges, grouped by the inflected copula surface.
const ranges = {
  인: ['ㄴ:14', 'ㄴ:26'],
  인지: ['ㄴ지:2'],
  일걸: ['ㄹ걸:12'],
  일까: ['ㄹ까:1'],
  일꼬: ['ㄹ꼬:4', 'ㄹ꼬:11'],
  일는지: ['ㄹ는지:5', 'ㄹ는지:12'],
  일라: ['ㄹ라:1', 'ㄹ라:11'],
  일런가: ['ㄹ런가:4'],
  일망정: ['ㄹ망정:3', 'ㄹ망정:10'],
  일밖에: ['ㄹ밖에:1'],
  일수록: ['ㄹ수록:3', 'ㄹ수록:4', 'ㄹ수록:5'],
  일지: ['ㄹ지:10'],
  일지니: ['ㄹ지니:3'],
  일지라도: ['ㄹ지라도:3', 'ㄹ지라도:6'],
  일지언정: ['ㄹ지언정:4', 'ㄹ지언정:6'],
  일진대: ['ㄹ진대:3'],
  임에도: ['ㅁ에도:1'],
  입네: ['ㅂ네:3', 'ㅂ네:5', 'ㅂ네:6'],
  입니다: ['ㅂ니다:2', 'ㅂ니다:7'],
  입디다: ['ㅂ디다:2'],
  입죠: ['ㅂ죠:2', 'ㅂ죠:3'],
  입지요: ['ㅂ지요:3'],
  이거나: ['거나:2'],
  이거들랑: ['거들랑:4', '거들랑:14'],
  이건: ['건:4'],
  이것다: ['것다:2'],
  이게: ['게:6', '게:7'],
  이고: ['고:15', '고:28', '고:46'],
  이겠고: ['고:31'],
  이고말고: ['고말고:1', '고말고:5'],
  이군: ['군:5'],
  이기로: ['기로:4'],
  이기로서: ['기로서:6'],
  이기에: ['기에:4'],
  이길래: ['길래:2', '길래:5'],
  이옵나이다: ['나이다:5'],
  이냐: ['냐:1', '냐:5', '냐:6'],
  이냐고: ['냐고:5', '냐고:7'],
  이네: ['네:7'],
  이니: ['니:7', '니:8', '니:10', '니:19', '니:23'],
  이니까는: ['니까는:1'],
  이니깐: ['니깐:1'],
  이다: ['다:13', '다:15'],
  이더군: ['더군:10'],
  이더니: ['더니:19'],
  이더라: ['더라:10', '더라:12'],
  이더라며: ['더라며:7'],
  이더라면서: ['더라면서:7'],
  이던가: ['던가:9'],
  이라느니: ['라느니:2', '라느니:4'],
  이라도: ['라도:1'],
  이라며: ['라며:1', '라며:2', '라며:3'],
  이라야만: ['라야만:1', '라야만:2', '라야만:3'],
  이랍니까: ['랍니까:1', '랍니까:4'],
  이래요: ['래요:1'],
  이려니: ['려니:7'],
  이려니와: ['려니와:2'],
  이렷다: ['렷다:10'],
  이리: ['리:23'],
  이리니: ['리니:2'],
  이리니라: ['리니라:1'],
  이리다: ['리다:2'],
  이리라: ['리라:10'],
  이며: ['며:1', '며:2', '며:8', '며:10'],
  이면: ['면:3', '면:11', '면:18'],
  이므로: ['므로:5'],
  이세요: ['세요:5', '세요:6'],
  이시어요: ['시어요:7'],
  이야: ['야:1', '야:2', '야:12'],
  이어도: ['어도:5'],
  이어서: ['어서:9'],
  이어야: ['어야:9'],
  이어야지: ['어야지:2', '어야지:13'],
  이어요: ['어요:4'],
  이오: ['오:5', '오:12'],
  이오이까: ['오이까:1'],
  이오이다: ['오이다:1'],
  이옵나이까: ['옵나이까:2'],
  이옵니다: ['옵니다:2'],
  이옵디다: ['옵디다:1'],
  이죠: ['죠:13'],
  이지요: ['지요:26'],
};
const pending = new Map();
for (const [surface, ids] of Object.entries(ranges)) {
  for (const id of ids) {
    if (pending.has(id)) throw new Error(`duplicate copula annotation: ${id}`);
    pending.set(id, surface);
  }
}
const output = original.toString('utf8').trimEnd().split('\n').map(line => {
  const row = JSON.parse(line);
  const surface = pending.get(row.id);
  if (!surface) return line;
  const start = row.gold_byte_end - Buffer.byteLength(surface);
  const originalSpan = Buffer.from(row.text).subarray(row.gold_byte_start, row.gold_byte_end);
  if (row.query !== '이다' || row.pos !== 'adjective' || !row.expected
      || start <= row.gold_byte_start || !row.surface.endsWith(surface)
      || !originalSpan.equals(Buffer.from(row.surface))) {
    throw new Error(`unexpected copula annotation: ${row.id}`);
  }
  pending.delete(row.id);
  return JSON.stringify({ ...row, gold_byte_start: start });
});
if (pending.size) throw new Error(`missing copula annotations: ${[...pending.keys()]}`);
process.stdout.write(output.join('\n') + '\n');
