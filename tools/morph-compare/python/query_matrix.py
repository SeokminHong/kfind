from __future__ import annotations

import argparse
import hashlib
import json
import random
from collections import Counter, defaultdict
from dataclasses import asdict, replace
from pathlib import Path

try:
    from .dataset import (
        BenchmarkCase,
        GoldCandidate,
        Sentence,
        apply_sentence_review,
        load_sentence_reviews,
        parse_conllu,
        positive_case,
        rank,
        resolve_source_set,
        sentence_review_path,
        sha256,
    )
    from .quality import contract_expected
    from .query_matrix_contract import (
        apply_contract_reviews,
        load_contract_reviews,
        sha256 as contract_review_sha256,
    )
except ImportError:
    from dataset import (
        BenchmarkCase,
        GoldCandidate,
        Sentence,
        apply_sentence_review,
        load_sentence_reviews,
        parse_conllu,
        positive_case,
        rank,
        resolve_source_set,
        sentence_review_path,
        sha256,
    )
    from quality import contract_expected
    from query_matrix_contract import (
        apply_contract_reviews,
        load_contract_reviews,
        sha256 as contract_review_sha256,
    )


MAX_PRESENT_QUERIES_PER_SENTENCE = 3
BOOTSTRAP_RESAMPLES = 10_000


def load_cases(path: Path) -> list[dict[str, object]]:
    with path.open(encoding="utf-8") as fixture_file:
        return [json.loads(line) for line in fixture_file if line.strip()]


def unique_sentence_candidates(
    sentence: Sentence,
) -> tuple[list[GoldCandidate], int]:
    by_query_pos: dict[tuple[str, str], list[GoldCandidate]] = defaultdict(list)
    for candidate in sentence.candidates:
        by_query_pos[(candidate.query, candidate.pos)].append(candidate)
    unique = [candidates[0] for candidates in by_query_pos.values() if len(candidates) == 1]
    repeated = sum(len(candidates) > 1 for candidates in by_query_pos.values())
    return unique, repeated


def candidate_key(candidate: GoldCandidate) -> tuple[str, str, int, int]:
    return (
        candidate.query,
        candidate.pos,
        candidate.byte_start,
        candidate.byte_end,
    )


def case_candidate_key(case: dict[str, object]) -> tuple[str, str, int, int]:
    return (
        str(case["query"]),
        str(case["pos"]),
        int(case["gold_byte_start"]),
        int(case["gold_byte_end"]),
    )


def select_sentence_positives(
    sentence: Sentence,
    canonical: list[dict[str, object]],
    seed: str,
) -> tuple[list[tuple[GoldCandidate, str | None]], int]:
    if len(canonical) > MAX_PRESENT_QUERIES_PER_SENTENCE:
        raise ValueError(
            f"{sentence.source}/{sentence.sent_id} has {len(canonical)} canonical "
            f"positives; matrix limit is {MAX_PRESENT_QUERIES_PER_SENTENCE}"
        )
    additional_candidates, repeated = unique_sentence_candidates(sentence)
    indexed = {
        candidate_key(candidate): candidate for candidate in sentence.candidates
    }
    selected: list[tuple[GoldCandidate, str | None]] = []
    selected_keys = set()
    for case in canonical:
        key = case_candidate_key(case)
        candidate = indexed.get(key)
        if candidate is None:
            raise ValueError(f"canonical positive {case['id']} is not uniquely aligned")
        selected.append((candidate, str(case["id"])))
        selected_keys.add(key)

    remaining = sorted(
        (
            candidate
            for candidate in additional_candidates
            if candidate_key(candidate) not in selected_keys
        ),
        key=lambda candidate: rank(
            seed,
            "query-matrix-positive",
            candidate.source,
            candidate.sent_id,
            candidate.token_id,
            candidate.morph_index,
            candidate.query,
        ),
    )
    while len(selected) < MAX_PRESENT_QUERIES_PER_SENTENCE and remaining:
        selected_pos = {candidate.pos for candidate, _ in selected}
        index = next(
            (
                candidate_index
                for candidate_index, candidate in enumerate(remaining)
                if candidate.pos not in selected_pos
            ),
            0,
        )
        selected.append((remaining.pop(index), None))
    return selected, repeated


def matrix_positive(
    candidate: GoldCandidate,
    *,
    canonical_positive_id: str | None,
    group_id: str,
    query_mode: str,
    slot: int,
) -> dict[str, object]:
    prefix = "untagged:matrix" if query_mode == "untagged" else "matrix"
    base = replace(
        positive_case(candidate),
        id=f"{prefix}:pos:{candidate.source}:{candidate.sent_id}:{slot}",
    )
    return {
        **asdict(base),
        "matrix_group_id": group_id,
        "matrix_slot": f"present-{slot}",
        "canonical_positive_id": canonical_positive_id,
    }


def negative_query_pool(sentences: list[Sentence]) -> list[tuple[str, str]]:
    return sorted(
        {
            (candidate.query, candidate.pos)
            for sentence in sentences
            for candidate in sentence.candidates
        }
    )


def select_absent_query(
    *,
    group_id: str,
    positive: dict[str, object],
    pool: list[tuple[str, str]],
    sentence: Sentence,
    query_mode: str,
    seed: str,
    used_queries: set[str],
) -> tuple[str, str]:
    sentence_pairs = {
        (candidate.query, candidate.pos) for candidate in sentence.candidates
    }
    sentence_queries = {query for query, _ in sentence_pairs}
    eligible = []
    for query, pos in pool:
        if pos != positive["pos"] or query in used_queries:
            continue
        if query_mode == "untagged":
            if query in sentence_queries:
                continue
        elif (query, pos) in sentence_pairs:
            continue
        eligible.append((query, pos))
    if not eligible:
        raise ValueError(f"no absent query for {positive['id']}")
    return min(
        eligible,
        key=lambda item: rank(
            seed,
            "query-matrix-negative",
            query_mode,
            group_id,
            positive["id"],
            item[0],
            item[1],
        ),
    )


def matrix_negative(
    positive: dict[str, object],
    *,
    query: str,
    pos: str,
    query_mode: str,
    slot: int,
) -> dict[str, object]:
    prefix = "untagged:matrix" if query_mode == "untagged" else "matrix"
    base = BenchmarkCase(
        id=f"{prefix}:neg:{positive['source']}:{positive['sent_id']}:{slot}",
        source=str(positive["source"]),
        sent_id=str(positive["sent_id"]),
        query=query,
        pos=pos,
        text=str(positive["text"]),
        expected=False,
        gold_byte_start=None,
        gold_byte_end=None,
        gold_token_id=None,
        gold_raw_lemma=None,
        gold_raw_tag=None,
        paired_positive_id=str(positive["id"]),
    )
    return {
        **asdict(base),
        "matrix_group_id": positive["matrix_group_id"],
        "matrix_slot": f"absent-{slot}",
        "canonical_positive_id": None,
    }


def source_metadata(
    source: dict[str, object],
    split_name: str,
    parsing: dict[str, int],
    cases: list[dict[str, object]],
    sentence_review: dict[str, object] | None,
) -> dict[str, object]:
    split = source["splits"][split_name]
    source_cases = [case for case in cases if case["source"] == source["name"]]
    return {
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
        "positive_cases": sum(bool(case["expected"]) for case in source_cases),
        "negative_cases": sum(not case["expected"] for case in source_cases),
        "sentences": len({str(case["matrix_group_id"]) for case in source_cases}),
    }


def build_query_matrix(
    *,
    manifest_path: Path,
    sources_dir: Path,
    canonical_cases_path: Path,
    output: Path,
    metadata_path: Path,
    contract_reviews_path: Path,
    split_name: str,
    query_mode: str,
) -> dict[str, object]:
    if query_mode not in {"explicit-pos", "untagged"}:
        raise ValueError(f"unsupported query mode: {query_mode}")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    sources, _, scoring_status = resolve_source_set(manifest, "canonical")
    review_path = sentence_review_path(manifest_path, manifest, "canonical")
    reviews = load_sentence_reviews(review_path) if review_path is not None else None
    canonical_cases = load_cases(canonical_cases_path)
    canonical_positives = [case for case in canonical_cases if case["expected"]]
    canonical_by_sentence: dict[tuple[str, str], list[dict[str, object]]] = defaultdict(list)
    for case in canonical_positives:
        canonical_by_sentence[(str(case["source"]), str(case["sent_id"]))].append(case)

    seed = str(manifest["seed"])
    all_cases = []
    metadata_sources = []
    repeated_pairs = 0
    per_sentence_counts: Counter[int] = Counter()
    for source in sources:
        split = source["splits"].get(split_name)
        if split is None:
            raise ValueError(f"source {source['name']} has no {split_name} split")
        source_path = sources_dir / split["data_file"]
        if sha256(source_path) != split["data_sha256"]:
            raise ValueError(f"source hash mismatch: {source['name']}")
        sentences, parsing = parse_conllu(str(source["name"]), source_path)
        sentence_review = None
        if reviews is not None and review_path is not None:
            sentences, sentence_review = apply_sentence_review(
                sentences=sentences,
                source_name=str(source["name"]),
                split_name=split_name,
                seed=seed,
                reviews=reviews,
                review_file=review_path.name,
            )
        sentence_index = {sentence.sent_id: sentence for sentence in sentences}
        pool = negative_query_pool(sentences)
        source_cases = []
        source_sentence_ids = sorted(
            sent_id
            for source_name, sent_id in canonical_by_sentence
            if source_name == source["name"]
        )
        for sent_id in source_sentence_ids:
            sentence = sentence_index.get(sent_id)
            if sentence is None:
                raise ValueError(f"canonical sentence is missing: {source['name']}/{sent_id}")
            group_id = f"{source['name']}:{sent_id}"
            selected, excluded_repeated = select_sentence_positives(
                sentence,
                canonical_by_sentence[(str(source["name"]), sent_id)],
                seed,
            )
            repeated_pairs += excluded_repeated
            per_sentence_counts[len(selected)] += 1
            positives = [
                matrix_positive(
                    candidate,
                    canonical_positive_id=canonical_id,
                    group_id=group_id,
                    query_mode=query_mode,
                    slot=slot,
                )
                for slot, (candidate, canonical_id) in enumerate(selected, start=1)
            ]
            used_negative_queries: set[str] = set()
            negatives = []
            for slot, positive in enumerate(positives, start=1):
                query, pos = select_absent_query(
                    group_id=group_id,
                    positive=positive,
                    pool=pool,
                    sentence=sentence,
                    query_mode=query_mode,
                    seed=seed,
                    used_queries=used_negative_queries,
                )
                used_negative_queries.add(query)
                negatives.append(
                    matrix_negative(
                        positive,
                        query=query,
                        pos=pos,
                        query_mode=query_mode,
                        slot=slot,
                    )
                )
            source_cases.extend(positives)
            source_cases.extend(negatives)
        all_cases.extend(source_cases)
        metadata_sources.append(
            source_metadata(
                source,
                split_name,
                parsing,
                source_cases,
                sentence_review,
            )
        )

    all_cases.sort(key=lambda case: rank(seed, "query-matrix-case-order", case["id"]))
    contract_review = apply_contract_reviews(
        all_cases,
        load_contract_reviews(contract_reviews_path),
        query_mode=query_mode,
        split=split_name,
    )
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8") as fixture_file:
        for case in all_cases:
            fixture_file.write(
                json.dumps(case, ensure_ascii=False, sort_keys=True) + "\n"
            )
    positive_counts = Counter(
        str(case["pos"]) for case in all_cases if case["expected"]
    )
    negative_counts = Counter(
        str(case["pos"]) for case in all_cases if not case["expected"]
    )
    metadata = {
        "schema_version": 1,
        "fixture_type": "query-matrix",
        "split": split_name,
        "query_mode": query_mode,
        "source_set": "canonical",
        "scoring_status": scoring_status,
        "ud_release": manifest["ud_release"],
        "seed": seed,
        "fixture_sha256": sha256(output),
        "derived_from_fixture_sha256": sha256(canonical_cases_path),
        "cases": len(all_cases),
        "positive_cases": sum(bool(case["expected"]) for case in all_cases),
        "negative_cases": sum(not case["expected"] for case in all_cases),
        "sentences": sum(per_sentence_counts.values()),
        "max_present_queries_per_sentence": MAX_PRESENT_QUERIES_PER_SENTENCE,
        "present_queries_per_sentence": {
            str(count): sentences
            for count, sentences in sorted(per_sentence_counts.items())
        },
        "positive_pos_counts": dict(sorted(positive_counts.items())),
        "negative_pos_counts": dict(sorted(negative_counts.items())),
        "canonical_positive_cases": len(canonical_positives),
        "canonical_positive_coverage": sum(
            case["canonical_positive_id"] is not None
            for case in all_cases
            if case["expected"]
        ),
        "repeated_query_pos_pairs": repeated_pairs,
        "contract_review": {
            "registry_sha256": contract_review_sha256(contract_reviews_path),
            **contract_review,
        },
        "sources": metadata_sources,
    }
    metadata_path.write_text(
        json.dumps(metadata, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    return metadata


def percentile(values: list[float], probability: float) -> float:
    ordered = sorted(values)
    position = (len(ordered) - 1) * probability
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    fraction = position - lower
    return ordered[lower] * (1 - fraction) + ordered[upper] * fraction


def query_matrix_metrics(
    cases: list[dict[str, object]],
    predictions: dict[str, bool],
    seed: str,
    *,
    contract_adjusted: bool = False,
) -> dict[str, object]:
    groups: dict[str, list[dict[str, object]]] = defaultdict(list)
    for case in cases:
        expected = (
            contract_expected(case)
            if contract_adjusted
            else bool(case["expected"])
        )
        if expected:
            groups[str(case["matrix_group_id"])].append(case)
    recovered_distribution: Counter[str] = Counter()
    cluster_counts = []
    all_recovered = 0
    for group_cases in groups.values():
        recovered = sum(bool(predictions[str(case["id"])]) for case in group_cases)
        total = len(group_cases)
        recovered_distribution[f"{recovered}/{total}"] += 1
        all_recovered += recovered == total
        cluster_counts.append((recovered, total - recovered))
    rng = random.Random(int.from_bytes(hashlib.sha256(seed.encode()).digest()[:8]))
    bootstrap_recalls = []
    for _ in range(BOOTSTRAP_RESAMPLES):
        tp = fn = 0
        for _ in cluster_counts:
            cluster_tp, cluster_fn = cluster_counts[rng.randrange(len(cluster_counts))]
            tp += cluster_tp
            fn += cluster_fn
        bootstrap_recalls.append(100 * tp / (tp + fn))
    return {
        "sentences": len(groups),
        "all_present_queries_recovered": all_recovered,
        "all_present_queries_recovered_percent": round(
            100 * all_recovered / len(groups), 2
        ),
        "recovered_query_distribution": dict(sorted(recovered_distribution.items())),
        "recall_sentence_cluster_bootstrap_95_percent": [
            round(percentile(bootstrap_recalls, 0.025), 2),
            round(percentile(bootstrap_recalls, 0.975), 2),
        ],
        "bootstrap_resamples": BOOTSTRAP_RESAMPLES,
    }


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--sources", type=Path, required=True)
    parser.add_argument("--canonical-cases", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--metadata", type=Path, required=True)
    parser.add_argument("--contract-reviews", type=Path, required=True)
    parser.add_argument("--split", choices=("dev", "test"), default="test")
    parser.add_argument(
        "--query-mode", choices=("explicit-pos", "untagged"), default="explicit-pos"
    )
    args = parser.parse_args()
    metadata = build_query_matrix(
        manifest_path=args.manifest,
        sources_dir=args.sources,
        canonical_cases_path=args.canonical_cases,
        output=args.output,
        metadata_path=args.metadata,
        contract_reviews_path=args.contract_reviews,
        split_name=args.split,
        query_mode=args.query_mode,
    )
    print(json.dumps(metadata, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
