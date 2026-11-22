import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const [gsd, pud, output, ...extra] = process.argv.slice(2);
if (!gsd || !pud || !output || extra.length) {
  throw new Error('usage: node generate-ud-audit-fixtures.mjs GSD.conllu PUD.conllu OUTPUT_DIR');
}

const sources = [
  ['gsd', gsd, '3d1df99bda4800235e14bcfd915baf706eafa1a3935a75ffd32420a51e57f5aa'],
  ['pud', pud, '55f9c8d465400e070664c5319091d31ce35a16ec220b0bebc5e3c9b12f530948'],
];
const posByTag = {
  VV: 'verb', VX: 'verb', VA: 'adjective', NNG: 'noun', NNP: 'noun', NNB: 'noun',
  NP: 'pronoun', NR: 'numeral', MAG: 'adverb', MM: 'determiner',
};
const shortQueries = {
  verb: `가다 갚다 감다 갈다 겪다 건다 걷다 굽다 굴다 그다 긋다 끄다 끓다 끊다
    나다 낳다 날다 남다 넣다 놀다 놓다 누다 눕다 닫다 달다 닮다 담다 대다 덜다 도다
    돌다 돋다 되다 두다 들다 듣다 뜨다 뛰다 떨다 띄다 떼다 따다 따르다 마다 말다
    맞다 매다 맺다 먹다 모다 몰다 묻다 무다 물다 밀다 받다 박다 밟다 베다 벗다
    보다 불다 붙다 빌다 빚다 빼다 사다 살다 서다 섞다 속다 쉬다 쓰다 쏘다 싣다
    심다 씻다 앉다 알다 얻다 열다 오다 옮다 울다 웃다 읽다 잃다 입다 잇다 잊다
    잡다 잠다 적다 접다 젖다 제다 조다 주다 줍다 줄다 지다 짓다 짜다 찌다 찍다
    쫓다 찾다 차다 치다 타다 털다 틀다 파다 팔다 펴다 풀다 피다 하다 향하다 헤다`,
  adjective: `가깝다 같다 검다 곱다 굳다 굵다 길다 깊다 낮다 낫다 넓다 높다 늦다
    달다 닮다 덥다 떫다 맑다 맵다 멀다 묽다 밝다 붉다 비다 세다 쉽다 싫다 싸다
    쓰다 없다 얇다 옅다 있다 작다 적다 젊다 좋다 짙다 짜다 짧다 춥다 크다 희다`,
  noun: `가 각 감 값 개 거 건 것 결 계 고 곡 곳 공 과 국 군 금 길 김 깃 꿈 나 날
    남 내 너 넋 널 노 논 눈 늪 단 달 답 당 대 더 덫 도 독 돌 동 두 들 등 때 땀 뜻
    말 맛 매 멋 몸 못 무 문 물 발 밤 밥 방 배 백 벌 별 병 봄 불 빛 빚 산 상 새 색
    생 서 섬 성 세 속 손 수 숲 술 숨 시 식 신 실 심 아 안 알 앞 약 양 어 얼 업 여
    열 옆 예 옷 왕 용 우 운 원 월 위 유 은 일 입 잎 자 장 재 적 전 절 점 정 제 조
    종 주 죽 줄 중 지 집 차 책 천 철 초 촉 총 추 춤 층 칠 칼 코 턱 털 틈 파 판 팔
    편 폭 풀 품 피 필 한 해 행 형 호 힘`,
  pronoun: '나 너 그 이 저 우리 저희 자기 자신 어디 누구 뭐 무엇 아무 당신 여기 거기 저기 이것 그것 저것',
  determiner: '한 두 세 네 이 그 저 새 옛 온 몇 여러 모든 각 전 첫 무슨 어느 어떤',
  adverb: '못 안 잘 더 덜 또 다 꼭 늘 막 참 바로 아주 이미 겨우 몹시 매우',
};

const firstSyllable = word => word.charCodeAt(0) - 0xac00;
const onset = word => Math.floor(firstSyllable(word) / 588);
const withoutFinal = word => Math.floor(firstSyllable(word) / 28);
const rootsByOnset = new Map();
for (const [pos, queries] of Object.entries(shortQueries)) {
  const predicate = pos === 'verb' || pos === 'adjective';
  for (const query of queries.split(/\s+/u)) {
    const root = predicate ? query.slice(0, -1) : query;
    const key = onset(root);
    const roots = rootsByOnset.get(key) ?? [];
    roots.push({ query, pos, root, predicate });
    rootsByOnset.set(key, roots);
  }
}

function alignedTokens(block, text) {
  const tokens = [];
  let cursor = 0;
  for (const line of block.split('\n')) {
    if (!/^\d+\t/u.test(line)) continue;
    const [id, form, lemma, upos, xpos] = line.split('\t');
    const start = text.indexOf(form, cursor);
    if (start < 0) continue;
    cursor = start + form.length;
    tokens.push({ id, form, lemma, upos, xpos, start, end: cursor });
  }
  return tokens;
}

function positiveCases(sid, text, tokens) {
  return tokens.flatMap(({ id, form, lemma, xpos, start, end }) => {
    const lemmas = lemma.split('+');
    const tags = xpos.split('+');
    if (lemmas.length !== tags.length || !/^[가-힣]+$/u.test(form)) return [];
    const derived = tags.findIndex(tag => tag === 'XSV' || tag === 'XSA');
    const prefix = lemmas.slice(0, derived + 1);
    let query = ['VV', 'VX', 'VA'].includes(tags[0]) ? lemmas[0] + '다' : lemmas[0];
    let pos = posByTag[tags[0]];
    if (derived > 0 && prefix.every(part => /^[가-힣]+$/u.test(part))) {
      query = prefix.join('') + '다';
      pos = tags[derived] === 'XSV' ? 'verb' : 'adjective';
    }
    if (!pos || !/^[가-힣]+$/u.test(query)) return [];
    return [{ id: `gsd:${sid}:${id}`, query, pos, text, expected: true,
      gold_byte_start: Buffer.byteLength(text.slice(0, start)),
      gold_byte_end: Buffer.byteLength(text.slice(0, end)), form, lemma, xpos }];
  });
}

function negativeCases(source, sid, text, tokens) {
  const wordTag = /^(NN|NOUN|PROPN|ADJ|ADV|PRON|DET|NUM|PART|ADP|VERB)/u;
  const used = new Set();
  const cases = [];
  for (const { form, lemma, upos, xpos } of tokens) {
    if (!/^[가-힣]{1,8}$/u.test(form) || (!wordTag.test(xpos) && !wordTag.test(upos))) continue;
    for (const { query, pos, root, predicate } of rootsByOnset.get(onset(form)) ?? []) {
      const key = query + pos;
      if (used.has(key)) continue;
      const nearStem = form[0] === root[0]
        || (predicate && root.length === 1 && withoutFinal(form) === withoutFinal(root));
      if (!nearStem) continue;
      if (source === 'gsd' && tokens.some(token => token.lemma.split('+').includes(root)
          && token.xpos.split('+').some(tag => posByTag[tag] === pos))) continue;
      if (predicate && tokens.some(token => token.form.startsWith(root)
          && ['VERB', 'ADJ'].includes(token.upos))) continue;
      used.add(key);
      cases.push({ id: `${source}:${sid}:${pos}:${query}`, query, pos, text,
        expected: false, form, lemma, xpos });
    }
  }
  return cases;
}

const positives = [];
const negatives = [];
for (const [source, path, expectedHash] of sources) {
  const bytes = readFileSync(path);
  const hash = createHash('sha256').update(bytes).digest('hex');
  if (hash !== expectedHash) throw new Error(`unexpected ${source} source SHA-256: ${hash}`);
  for (const block of bytes.toString('utf8').trim().split(/\n\n+/u)) {
    const sid = block.match(/^# sent_id = (.+)$/mu)?.[1];
    const text = block.match(/^# text = (.+)$/mu)?.[1];
    if (!sid || !text) throw new Error(`missing sentence ID or text: ${source}`);
    const tokens = alignedTokens(block, text);
    if (source === 'gsd') positives.push(...positiveCases(sid, text, tokens));
    negatives.push(...negativeCases(source, sid, text, tokens));
  }
}

mkdirSync(output, { recursive: true });
for (const [name, rows] of Object.entries({ positives, negatives })) {
  writeFileSync(join(output, `${name}.jsonl`), rows.map(row => JSON.stringify(row)).join('\n') + '\n');
}
