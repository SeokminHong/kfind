from __future__ import annotations

import argparse
import hashlib
import json
from collections import Counter, defaultdict
from dataclasses import asdict, dataclass, replace
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
MAX_CANONICAL_POSITIVES_PER_SENTENCE = 3
SENTENCE_REVIEW_REASON_CLASSES = {
    "foreign-text-typo",
    "fragment",
    "hangul-typo",
    "nonstandard-syntax",
    "orthographic-confusion",
    "repetition",
    "source-artifact",
    "spacing-merge",
    "spacing-split",
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


def morpheme_candidates(
    source: str,
    sent_id: str,
    text: str,
    row: list[str],
    byte_start: int,
    byte_end: int,
    stats: Counter[str],
) -> tuple[list[GoldCandidate], bool]:
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
        return [], False

    candidates = []
    for morph_index, (lemma, tag) in enumerate(zip(lemmas, tags)):
        normalized = normalize_gold(lemma, tag)
        if normalized is None:
            if tag.lower() not in TAG_TO_POS:
                stats["unsupported_morphemes"] += 1
            else:
                stats["non_hangul_morphemes"] += 1
            continue
        query, pos = normalized
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
    return candidates, True


SOURCE_ROW_ADAPTERS = {
    "morpheme": morpheme_candidates,
}


def parse_conllu(
    source: str, path: Path, adapter: str = "morpheme"
) -> tuple[list[Sentence], dict[str, int]]:
    row_adapter = SOURCE_ROW_ADAPTERS.get(adapter)
    if row_adapter is None:
        raise ValueError(f"unsupported source adapter: {adapter}")
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
            row_candidates, row_aligned = row_adapter(
                source,
                sent_id,
                text,
                row,
                byte_start,
                byte_end,
                stats,
            )
            if not row_aligned:
                fully_aligned = False
            for candidate in row_candidates:
                key = (
                    candidate.query,
                    candidate.pos,
                    candidate.byte_start,
                    candidate.byte_end,
                )
                if key in seen:
                    stats["duplicate_morphemes"] += 1
                    continue
                seen.add(key)
                candidates.append(candidate)
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
    sentences: list[Sentence],
    quotas: dict[str, int],
    seed: str,
    max_per_sentence: int | None = None,
) -> list[BenchmarkCase]:
    by_pos: dict[str, list[GoldCandidate]] = defaultdict(list)
    for sentence in sentences:
        for candidate in sentence.candidates:
            by_pos[candidate.pos].append(candidate)

    selected = []
    selected_per_sentence: Counter[tuple[str, str]] = Counter()
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
            sentence_key = (candidate.source, candidate.sent_id)
            if (
                max_per_sentence is not None
                and selected_per_sentence[sentence_key] >= max_per_sentence
            ):
                continue
            unique_queries.add(candidate.query)
            selected.append(positive_case(candidate))
            selected_per_sentence[sentence_key] += 1
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


def select_untagged_negative(
    positive: BenchmarkCase, sentences: list[Sentence], seed: str
) -> BenchmarkCase:
    ordered = sorted(
        sentences,
        key=lambda sentence: rank(
            seed, "untagged-negative", positive.id, sentence.source, sentence.sent_id
        ),
    )
    for sentence in ordered:
        if sentence.sent_id == positive.sent_id or not sentence.fully_aligned:
            continue
        if positive.query in {candidate.query for candidate in sentence.candidates}:
            continue
        return BenchmarkCase(
            id=f"untagged-neg:{positive.id}:{sentence.sent_id}",
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
    raise ValueError(f"no untagged negative sentence found for {positive.id}")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def manifest_sources_by_name(
    manifest: dict[str, object],
) -> dict[str, dict[str, object]]:
    if manifest.get("schema_version") != 4:
        raise ValueError("unsupported source manifest schema")
    sources = manifest.get("sources")
    if not isinstance(sources, list) or not sources:
        raise ValueError("source manifest requires sources")
    indexed = {str(source["name"]): source for source in sources}
    if len(indexed) != len(sources):
        raise ValueError("source manifest names are not unique")
    return indexed


def select_manifest_sources(
    manifest: dict[str, object], names: Iterable[object]
) -> list[dict[str, object]]:
    source_names = [str(name) for name in names]
    if len(set(source_names)) != len(source_names):
        raise ValueError("selected source names are not unique")
    indexed = manifest_sources_by_name(manifest)
    unknown = [name for name in source_names if name not in indexed]
    if unknown:
        raise ValueError(f"selected sources are unknown: {unknown}")
    return [indexed[name] for name in source_names]


def resolve_source_set(
    manifest: dict[str, object], source_set_name: str
) -> tuple[list[dict[str, object]], dict[str, int], str]:
    source_sets = manifest.get("source_sets")
    if not isinstance(source_sets, dict):
        raise ValueError("source manifest requires source_sets")
    source_set = source_sets.get(source_set_name)
    if not isinstance(source_set, dict):
        raise ValueError(f"unknown source set: {source_set_name}")
    source_names = source_set.get("sources")
    quotas = source_set.get("positive_quotas_per_source")
    scoring_status = source_set.get("scoring_status")
    if not isinstance(source_names, list) or not source_names:
        raise ValueError(f"source set {source_set_name} requires sources")
    if not isinstance(quotas, dict) or not quotas:
        raise ValueError(f"source set {source_set_name} requires positive quotas")
    if not all(
        isinstance(pos, str) and isinstance(quota, int) and quota > 0
        for pos, quota in quotas.items()
    ):
        raise ValueError(f"source set {source_set_name} has invalid positive quotas")
    if scoring_status not in {"scored", "annotation-required"}:
        raise ValueError(f"source set {source_set_name} has invalid scoring status")
    return (
        select_manifest_sources(manifest, source_names),
        quotas,
        str(scoring_status),
    )


def sentence_review_path(
    manifest_path: Path,
    manifest: dict[str, object],
    source_set_name: str,
) -> Path | None:
    source_sets = manifest.get("source_sets")
    if not isinstance(source_sets, dict):
        raise ValueError("source manifest requires source_sets")
    source_set = source_sets.get(source_set_name)
    if not isinstance(source_set, dict):
        raise ValueError(f"unknown source set: {source_set_name}")
    review_file = source_set.get("sentence_review_file")
    if review_file is None:
        return None
    if not isinstance(review_file, str) or not review_file:
        raise ValueError(f"source set {source_set_name} has invalid review file")
    return manifest_path.parent / review_file


def load_sentence_reviews(path: Path) -> dict[str, object]:
    reviews = json.loads(path.read_text(encoding="utf-8"))
    if reviews.get("schema_version") != 1:
        raise ValueError("unsupported sentence review schema")
    policy = reviews.get("review_policy")
    if not isinstance(policy, str) or not policy:
        raise ValueError("sentence review requires review_policy")
    splits = reviews.get("splits")
    if not isinstance(splits, dict) or not splits:
        raise ValueError("sentence review requires splits")
    return reviews


def review_pool_cases(
    sentences: list[Sentence], quotas: dict[str, int], seed: str
) -> list[BenchmarkCase]:
    positives = select_positives(sentences, quotas, seed)
    negatives = [select_negative(case, sentences, seed) for case in positives]
    return positives + negatives


def review_pool_rows(cases: Iterable[BenchmarkCase]) -> list[dict[str, str]]:
    by_id: dict[tuple[str, str], str] = {}
    for case in cases:
        key = (case.source, case.sent_id)
        previous = by_id.setdefault(key, case.text)
        if previous != case.text:
            raise ValueError(f"sentence text changed within review pool: {key}")
    return [
        {"source": source, "sent_id": sent_id, "text": text}
        for (source, sent_id), text in by_id.items()
    ]


def review_pool_sha256(rows: Iterable[dict[str, str]]) -> str:
    serialized = sorted(
        json.dumps(
            row,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
        )
        for row in rows
    )
    payload = "".join(f"{line}\n" for line in serialized)
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()


def apply_sentence_review(
    *,
    sentences: list[Sentence],
    source_name: str,
    split_name: str,
    seed: str,
    reviews: dict[str, object],
    review_file: str,
) -> tuple[list[Sentence], dict[str, object]]:
    split_reviews = reviews["splits"].get(split_name)
    if not isinstance(split_reviews, dict):
        raise ValueError(f"sentence review has no {split_name} split")
    entry = split_reviews.get(source_name)
    if not isinstance(entry, dict):
        raise ValueError(f"sentence review has no {split_name}/{source_name} entry")

    review_quotas = entry.get("positive_quotas_per_source")
    if not isinstance(review_quotas, dict) or not all(
        isinstance(pos, str) and isinstance(quota, int) and quota > 0
        for pos, quota in review_quotas.items()
    ):
        raise ValueError(
            f"sentence review has invalid pool quotas: {split_name}/{source_name}"
        )
    rows = review_pool_rows(review_pool_cases(sentences, review_quotas, seed))
    expected_count = entry.get("pool_sentences")
    expected_sha256 = entry.get("pool_sha256")
    actual_sha256 = review_pool_sha256(rows)
    if expected_count != len(rows):
        raise ValueError(
            f"sentence review pool count mismatch for {split_name}/{source_name}: "
            f"expected {expected_count}, got {len(rows)}"
        )
    if expected_sha256 != actual_sha256:
        raise ValueError(
            f"sentence review pool hash mismatch for {split_name}/{source_name}: "
            f"expected {expected_sha256}, got {actual_sha256}"
        )

    rejected = entry.get("rejected")
    if not isinstance(rejected, list):
        raise ValueError(f"sentence review rejected list is missing: {split_name}/{source_name}")
    rejected_ids = set()
    reason_counts: Counter[str] = Counter()
    for rejection in rejected:
        if not isinstance(rejection, dict):
            raise ValueError("sentence review rejection must be an object")
        sent_id = rejection.get("sent_id")
        reason_class = rejection.get("reason_class")
        annotation = rejection.get("annotation")
        if not isinstance(sent_id, str) or not sent_id:
            raise ValueError("sentence review rejection requires sent_id")
        if sent_id in rejected_ids:
            raise ValueError(f"sentence review rejection is duplicated: {sent_id}")
        if reason_class not in SENTENCE_REVIEW_REASON_CLASSES:
            raise ValueError(f"invalid sentence review reason class: {reason_class}")
        if not isinstance(annotation, str) or not annotation:
            raise ValueError(f"sentence review rejection requires annotation: {sent_id}")
        rejected_ids.add(sent_id)
        reason_counts[str(reason_class)] += 1

    pool_ids = {row["sent_id"] for row in rows}
    unknown = sorted(rejected_ids - pool_ids)
    if unknown:
        raise ValueError(
            f"sentence review rejected IDs are outside the pool for "
            f"{split_name}/{source_name}: {unknown}"
        )
    accepted_ids = pool_ids - rejected_ids
    accepted = [sentence for sentence in sentences if sentence.sent_id in accepted_ids]
    return accepted, {
        "review_file": review_file,
        "review_policy": reviews["review_policy"],
        "positive_quotas_per_source": review_quotas,
        "pool_sentences": len(rows),
        "pool_sha256": actual_sha256,
        "accepted_sentences": len(accepted_ids),
        "rejected_sentences": len(rejected_ids),
        "rejected_reason_counts": dict(sorted(reason_counts.items())),
    }


def build_dataset(
    manifest_path: Path,
    sources_dir: Path,
    output: Path,
    metadata_path: Path,
    split_name: str = "test",
    query_mode: str = "explicit-pos",
    source_set_name: str = "canonical",
) -> dict[str, object]:
    if query_mode not in {"explicit-pos", "untagged"}:
        raise ValueError(f"unsupported query mode: {query_mode}")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    sources, quotas, scoring_status = resolve_source_set(manifest, source_set_name)
    review_path = sentence_review_path(manifest_path, manifest, source_set_name)
    reviews = load_sentence_reviews(review_path) if review_path is not None else None
    seed = manifest["seed"]
    all_cases = []
    source_metadata = []
    for source in sources:
        split = source["splits"].get(split_name)
        if split is None:
            raise ValueError(f"source {source['name']} has no {split_name} split")
        source_path = sources_dir / split["data_file"]
        if sha256(source_path) != split["data_sha256"]:
            raise ValueError(f"source hash mismatch: {source['name']}")
        sentences, parsing = parse_conllu(source["name"], source_path)
        sentence_review = None
        if reviews is not None and review_path is not None:
            sentences, sentence_review = apply_sentence_review(
                sentences=sentences,
                source_name=str(source["name"]),
                split_name=split_name,
                seed=str(seed),
                reviews=reviews,
                review_file=review_path.name,
            )
        positives = select_positives(
            sentences,
            quotas,
            seed,
            max_per_sentence=MAX_CANONICAL_POSITIVES_PER_SENTENCE,
        )
        if query_mode == "untagged":
            positives = [
                replace(case, id=f"untagged:{case.id}") for case in positives
            ]
            negatives = [
                select_untagged_negative(case, sentences, seed) for case in positives
            ]
        else:
            negatives = [select_negative(case, sentences, seed) for case in positives]
        all_cases.extend(positives)
        all_cases.extend(negatives)
        source_metadata.append(
            {
                "name": source["name"],
                "description": f"{source['description']} {split_name} split",
                "split": split_name,
                "data_file": split["data_file"],
                "data_url": split["data_url"],
                "data_sha256": split["data_sha256"],
                "license": source["license"],
                "license_file": source["license_file"],
                "parsing": parsing,
                "sentence_review": sentence_review,
                "positive_cases": len(positives),
                "negative_cases": len(negatives),
            }
        )

    all_cases.sort(key=lambda case: rank(seed, "case-order", case.id))
    expected_count = 2 * len(sources) * sum(quotas.values())
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
        "split": split_name,
        "query_mode": query_mode,
        "source_set": source_set_name,
        "scoring_status": scoring_status,
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
    parser.add_argument("--split", choices=("dev", "test"), default="test")
    parser.add_argument(
        "--query-mode", choices=("explicit-pos", "untagged"), default="explicit-pos"
    )
    parser.add_argument("--source-set", default="canonical")
    args = parser.parse_args()
    metadata = build_dataset(
        args.manifest,
        args.sources,
        args.output,
        args.metadata,
        args.split,
        args.query_mode,
        args.source_set,
    )
    print(json.dumps(metadata, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
