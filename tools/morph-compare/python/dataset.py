from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable


TAG_TO_POS = {
    "nng": "noun",
    "nnp": "noun",
    "nnb": "noun",
    "ncn": "noun",
    "ncpa": "noun",
    "ncps": "noun",
    "nbn": "noun",
    "nbu": "noun",
    "nq": "noun",
    "np": "pronoun",
    "npd": "pronoun",
    "npp": "pronoun",
    "nr": "numeral",
    "sn": "numeral",
    "nnc": "numeral",
    "nno": "numeral",
    "vv": "verb",
    "vx": "verb",
    "pvg": "verb",
    "px": "verb",
    "va": "adjective",
    "vcp": "adjective",
    "vcn": "adjective",
    "paa": "adjective",
    "pad": "adjective",
    "jp": "adjective",
    "mm": "determiner",
    "mmd": "determiner",
    "mma": "determiner",
    "mag": "adverb",
    "maj": "adverb",
}
PREDICATE_TAGS = {
    "vv",
    "vx",
    "pvg",
    "px",
    "va",
    "vcp",
    "vcn",
    "paa",
    "pad",
    "jp",
}


@dataclass(frozen=True)
class GoldCandidate:
    source: str
    sent_id: str
    text: str
    token_id: str
    morph_index: int
    query: str
    pos: str
    byte_start: int
    byte_end: int
    raw_lemma: str
    raw_tag: str


@dataclass(frozen=True)
class Sentence:
    source: str
    sent_id: str
    text: str
    candidates: tuple[GoldCandidate, ...]
    fully_aligned: bool


@dataclass(frozen=True)
class BenchmarkCase:
    id: str
    source: str
    sent_id: str
    query: str
    pos: str
    text: str
    expected: bool
    gold_byte_start: int | None
    gold_byte_end: int | None
    gold_token_id: str | None
    gold_raw_lemma: str | None
    gold_raw_tag: str | None
    paired_positive_id: str | None


def is_hangul_syllables(value: str) -> bool:
    return bool(value) and all("가" <= character <= "힣" for character in value)


def normalize_gold(lemma: str, tag: str) -> tuple[str, str] | None:
    normalized_tag = tag.lower()
    pos = TAG_TO_POS.get(normalized_tag)
    if pos is None or not is_hangul_syllables(lemma):
        return None
    query = f"{lemma}다" if normalized_tag in PREDICATE_TAGS else lemma
    return query, pos


def utf8_offsets(text: str) -> list[int]:
    offsets = [0]
    for character in text:
        offsets.append(offsets[-1] + len(character.encode("utf-8")))
    return offsets


def locate_token_spans(text: str, forms: Iterable[str]) -> list[tuple[int, int]]:
    offsets = utf8_offsets(text)
    cursor = 0
    spans = []
    for form in forms:
        while cursor < len(text) and text[cursor].isspace():
            cursor += 1
        start = text.find(form, cursor)
        if start < 0 or text[cursor:start].strip():
            raise ValueError(f"cannot align token {form!r} after character {cursor} in {text!r}")
        end = start + len(form)
        spans.append((offsets[start], offsets[end]))
        cursor = end
    return spans


def parse_conllu(source: str, path: Path) -> tuple[list[Sentence], dict[str, int]]:
    sentences = []
    stats: Counter[str] = Counter()
    sent_id = None
    text = None
    rows: list[list[str]] = []

    def finish_sentence() -> None:
        nonlocal sent_id, text, rows
        if not rows:
            return
        if sent_id is None or text is None:
            raise ValueError(f"sentence without sent_id or text in {path}")
        token_rows = [row for row in rows if row[0].isdigit()]
        spans = locate_token_spans(text, (row[1] for row in token_rows))
        candidates = []
        seen = set()
        fully_aligned = True
        for row, (byte_start, byte_end) in zip(token_rows, spans):
            stats["tokens"] += 1
            lemmas = row[2].split("+")
            tags = row[4].split("+")
            original_lemma = next(
                (
                    field.removeprefix("OrigLemma=")
                    for field in row[9].split("|")
                    if field.startswith("OrigLemma=")
                ),
                None,
            )
            if len(lemmas) != len(tags) and original_lemma is not None:
                lemmas = original_lemma.split("+")
                stats["orig_lemma_tokens"] += 1
            if len(lemmas) != len(tags):
                stats["unaligned_tokens"] += 1
                fully_aligned = False
                continue
            for morph_index, (lemma, tag) in enumerate(zip(lemmas, tags)):
                normalized = normalize_gold(lemma, tag)
                if normalized is None:
                    if tag.lower() not in TAG_TO_POS:
                        stats["unsupported_morphemes"] += 1
                    else:
                        stats["non_hangul_morphemes"] += 1
                    continue
                query, pos = normalized
                key = (query, pos, byte_start, byte_end)
                if key in seen:
                    stats["duplicate_morphemes"] += 1
                    continue
                seen.add(key)
                candidates.append(
                    GoldCandidate(
                        source=source,
                        sent_id=sent_id,
                        text=text,
                        token_id=row[0],
                        morph_index=morph_index,
                        query=query,
                        pos=pos,
                        byte_start=byte_start,
                        byte_end=byte_end,
                        raw_lemma=lemma,
                        raw_tag=tag,
                    )
                )
                stats["eligible_morphemes"] += 1
        sentences.append(
            Sentence(source, sent_id, text, tuple(candidates), fully_aligned)
        )
        stats["sentences"] += 1
        sent_id = None
        text = None
        rows = []

    with path.open(encoding="utf-8") as conllu_file:
        for raw_line in conllu_file:
            line = raw_line.rstrip("\n")
            if not line:
                finish_sentence()
            elif line.startswith("# sent_id = "):
                sent_id = line.removeprefix("# sent_id = ")
            elif line.startswith("# text = "):
                text = line.removeprefix("# text = ")
            elif not line.startswith("#"):
                fields = line.split("\t")
                if len(fields) != 10:
                    raise ValueError(f"invalid CoNLL-U row in {path}: {line!r}")
                rows.append(fields)
    finish_sentence()
    return sentences, dict(sorted(stats.items()))


def rank(seed: str, *parts: object) -> bytes:
    value = "\0".join([seed, *(str(part) for part in parts)])
    return hashlib.sha256(value.encode("utf-8")).digest()


def positive_case(candidate: GoldCandidate) -> BenchmarkCase:
    case_id = ":".join(
        [
            "pos",
            candidate.source,
            candidate.sent_id,
            candidate.token_id,
            str(candidate.morph_index),
        ]
    )
    return BenchmarkCase(
        id=case_id,
        source=candidate.source,
        sent_id=candidate.sent_id,
        query=candidate.query,
        pos=candidate.pos,
        text=candidate.text,
        expected=True,
        gold_byte_start=candidate.byte_start,
        gold_byte_end=candidate.byte_end,
        gold_token_id=candidate.token_id,
        gold_raw_lemma=candidate.raw_lemma,
        gold_raw_tag=candidate.raw_tag,
        paired_positive_id=None,
    )


def select_positives(
    sentences: list[Sentence], quotas: dict[str, int], seed: str
) -> list[BenchmarkCase]:
    by_pos: dict[str, list[GoldCandidate]] = defaultdict(list)
    for sentence in sentences:
        for candidate in sentence.candidates:
            by_pos[candidate.pos].append(candidate)

    selected = []
    for pos, quota in quotas.items():
        ordered = sorted(
            by_pos[pos],
            key=lambda item: rank(
                seed,
                "positive",
                item.source,
                item.sent_id,
                item.token_id,
                item.morph_index,
                item.query,
            ),
        )
        unique_queries = set()
        for candidate in ordered:
            if candidate.query in unique_queries:
                continue
            unique_queries.add(candidate.query)
            selected.append(positive_case(candidate))
            if len(unique_queries) == quota:
                break
        if len(unique_queries) != quota:
            raise ValueError(
                f"{sentences[0].source} has {len(unique_queries)} unique {pos} queries; "
                f"quota requires {quota}"
            )
    return selected


def select_negative(
    positive: BenchmarkCase, sentences: list[Sentence], seed: str
) -> BenchmarkCase:
    ordered = sorted(
        sentences,
        key=lambda sentence: rank(
            seed, "negative", positive.id, sentence.source, sentence.sent_id
        ),
    )
    for sentence in ordered:
        if sentence.sent_id == positive.sent_id or not sentence.fully_aligned:
            continue
        gold = {(candidate.query, candidate.pos) for candidate in sentence.candidates}
        if (positive.query, positive.pos) in gold:
            continue
        return BenchmarkCase(
            id=f"neg:{positive.id}:{sentence.sent_id}",
            source=sentence.source,
            sent_id=sentence.sent_id,
            query=positive.query,
            pos=positive.pos,
            text=sentence.text,
            expected=False,
            gold_byte_start=None,
            gold_byte_end=None,
            gold_token_id=None,
            gold_raw_lemma=None,
            gold_raw_tag=None,
            paired_positive_id=positive.id,
        )
    raise ValueError(f"no negative sentence found for {positive.id}")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_dataset(
    manifest_path: Path, sources_dir: Path, output: Path, metadata_path: Path
) -> dict[str, object]:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("schema_version") != 1:
        raise ValueError("unsupported source manifest schema")
    quotas = manifest["positive_quotas_per_source"]
    seed = manifest["seed"]
    all_cases = []
    source_metadata = []
    for source in manifest["sources"]:
        source_path = sources_dir / source["data_file"]
        if sha256(source_path) != source["data_sha256"]:
            raise ValueError(f"source hash mismatch: {source['name']}")
        sentences, parsing = parse_conllu(source["name"], source_path)
        positives = select_positives(sentences, quotas, seed)
        negatives = [select_negative(case, sentences, seed) for case in positives]
        all_cases.extend(positives)
        all_cases.extend(negatives)
        source_metadata.append(
            {
                "name": source["name"],
                "description": source["description"],
                "data_file": source["data_file"],
                "data_url": source["data_url"],
                "data_sha256": source["data_sha256"],
                "license": source["license"],
                "license_file": source["license_file"],
                "parsing": parsing,
                "positive_cases": len(positives),
                "negative_cases": len(negatives),
            }
        )

    all_cases.sort(key=lambda case: rank(seed, "case-order", case.id))
    expected_count = 2 * len(manifest["sources"]) * sum(quotas.values())
    if len(all_cases) != expected_count:
        raise ValueError(f"expected {expected_count} cases, generated {len(all_cases)}")
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8") as fixture_file:
        for case in all_cases:
            fixture_file.write(
                json.dumps(asdict(case), ensure_ascii=False, sort_keys=True) + "\n"
            )
    metadata = {
        "schema_version": 1,
        "ud_release": manifest["ud_release"],
        "seed": seed,
        "fixture_sha256": sha256(output),
        "cases": len(all_cases),
        "positive_cases": sum(case.expected for case in all_cases),
        "negative_cases": sum(not case.expected for case in all_cases),
        "positive_quotas_per_source": quotas,
        "sources": source_metadata,
    }
    metadata_path.write_text(
        json.dumps(metadata, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    return metadata


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--sources", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--metadata", type=Path, required=True)
    args = parser.parse_args()
    metadata = build_dataset(args.manifest, args.sources, args.output, args.metadata)
    print(json.dumps(metadata, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
