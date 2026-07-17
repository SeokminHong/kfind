from __future__ import annotations

import argparse
import json
from collections import Counter, defaultdict
from dataclasses import asdict, replace
from pathlib import Path

from dataset import (
    BenchmarkCase,
    GoldCandidate,
    MAX_CANONICAL_POSITIVES_PER_SENTENCE,
    Sentence,
    manifest_sources_by_name,
    parse_conllu,
    positive_case,
    rank,
    select_negative,
    select_untagged_negative,
    sha256,
)
from robustness_review import (
    SentenceNoiseReview,
    apply_candidate_review,
    case_review_sha256,
    load_negative_rejections,
    load_reviewed_noisy_sentences,
    parse_source_signals,
    source_signal_rows,
    validate_case_review,
)


def select_robustness_positives(
    sentences: list[Sentence],
    sentence_reviews: dict[str, SentenceNoiseReview],
    quotas: dict[str, int],
    target_quotas: dict[str, int],
    seed: str,
) -> list[BenchmarkCase]:
    if set(target_quotas) != set(quotas):
        raise ValueError("robustness target quotas must cover every POS quota")
    if any(target_quotas[pos] > quotas[pos] for pos in quotas):
        raise ValueError("robustness target quota exceeds positive quota")

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
        target = [
            candidate
            for candidate in ordered
            if candidate.token_id
            in sentence_reviews[candidate.sent_id].marked_token_ids
        ]
        unique_queries = set()

        def add(candidates: list[GoldCandidate], limit: int) -> None:
            for candidate in candidates:
                if len(unique_queries) >= limit:
                    break
                if candidate.query in unique_queries:
                    continue
                sentence_key = (candidate.source, candidate.sent_id)
                if (
                    selected_per_sentence[sentence_key]
                    >= MAX_CANONICAL_POSITIVES_PER_SENTENCE
                ):
                    continue
                unique_queries.add(candidate.query)
                selected.append(positive_case(candidate))
                selected_per_sentence[sentence_key] += 1

        add(target, target_quotas[pos])
        if len(unique_queries) != target_quotas[pos]:
            raise ValueError(
                f"robustness has {len(unique_queries)} target-span {pos} queries; "
                f"quota requires {target_quotas[pos]}"
            )
        add(ordered, quota)
        if len(unique_queries) != quota:
            raise ValueError(
                f"robustness has {len(unique_queries)} unique {pos} queries; "
                f"quota requires {quota}"
            )
    return selected


def select_reviewed_negative(
    positive: BenchmarkCase,
    sentences: list[Sentence],
    seed: str,
    query_mode: str,
    rejected_ids: set[str],
    seen_rejections: set[str],
) -> BenchmarkCase:
    remaining = sentences
    while True:
        if query_mode == "untagged":
            negative = select_untagged_negative(positive, remaining, seed)
        else:
            negative = select_negative(positive, remaining, seed)
        if negative.id not in rejected_ids:
            return negative
        seen_rejections.add(negative.id)
        remaining = [
            sentence
            for sentence in remaining
            if sentence.sent_id != negative.sent_id
        ]


def build_robustness_dataset(
    *,
    manifest_path: Path,
    sources_dir: Path,
    output: Path,
    metadata_path: Path,
    query_mode: str,
    allow_draft_case_review: bool = False,
) -> dict[str, object]:
    if query_mode not in {"explicit-pos", "untagged"}:
        raise ValueError(f"unsupported query mode: {query_mode}")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    source_set = manifest.get("source_sets", {}).get("robustness")
    if not isinstance(source_set, dict):
        raise ValueError("source manifest requires robustness source set")
    if source_set.get("scoring_status") != "scored":
        raise ValueError("robustness source set must be scored")
    source_names = source_set.get("sources")
    quotas = source_set.get("positive_quotas_per_source")
    target_quotas = source_set.get("target_positive_quotas_per_source")
    review_file = source_set.get("sentence_review_file")
    if not isinstance(source_names, list) or len(source_names) != 1:
        raise ValueError("robustness source set requires exactly one source")
    if not isinstance(quotas, dict) or not quotas:
        raise ValueError("robustness source set requires positive quotas")
    if not isinstance(target_quotas, dict) or not target_quotas:
        raise ValueError("robustness source set requires target positive quotas")
    if not isinstance(review_file, str) or not review_file:
        raise ValueError("robustness source set requires sentence_review_file")

    source = manifest_sources_by_name(manifest)[str(source_names[0])]
    split = source["splits"]["test"]
    source_path = sources_dir / str(split["data_file"])
    if sha256(source_path) != split["data_sha256"]:
        raise ValueError(f"source hash mismatch: {source['name']}")
    sentences, parsing = parse_conllu(str(source["name"]), source_path)
    review_path = manifest_path.parent / review_file
    reviewed_sentences, sentence_reviews, sentence_review_metadata = (
        load_reviewed_noisy_sentences(
            source_name=str(source["name"]),
            sentences=sentences,
            source_path=source_path,
            review_path=review_path,
        )
    )
    reviewed_sentences, candidate_review_metadata = apply_candidate_review(
        reviewed_sentences, review_path
    )

    normalized_quotas = {str(pos): int(quota) for pos, quota in quotas.items()}
    normalized_target_quotas = {
        str(pos): int(quota) for pos, quota in target_quotas.items()
    }
    positives = select_robustness_positives(
        reviewed_sentences,
        sentence_reviews,
        normalized_quotas,
        normalized_target_quotas,
        str(manifest["seed"]),
    )
    rejected_negative_ids = load_negative_rejections(review_path, query_mode)
    seen_negative_rejections: set[str] = set()
    if query_mode == "untagged":
        positives = [replace(case, id=f"untagged:{case.id}") for case in positives]
    negatives = [
        select_reviewed_negative(
            case,
            reviewed_sentences,
            str(manifest["seed"]),
            query_mode,
            rejected_negative_ids,
            seen_negative_rejections,
        )
        for case in positives
    ]
    if seen_negative_rejections != rejected_negative_ids:
        missing = sorted(rejected_negative_ids - seen_negative_rejections)
        raise ValueError(f"robustness negative rejections were not selected: {missing}")

    enriched_cases = []
    for case in (*positives, *negatives):
        sentence_review = sentence_reviews[case.sent_id]
        noise_scope = (
            "target-span"
            if case.expected and case.gold_token_id in sentence_review.marked_token_ids
            else "context-only"
        )
        enriched_cases.append(
            {
                **asdict(case),
                "noise_origin": "natural",
                "noise_class": sentence_review.primary_noise_class,
                "noise_classes": list(sentence_review.noise_classes),
                "noise_scope": noise_scope,
                "sentence_annotation": sentence_review.annotation,
            }
        )
    enriched_cases.sort(
        key=lambda case: rank(manifest["seed"], "robustness-case-order", case["id"])
    )
    case_review = validate_case_review(
        review_path,
        query_mode=query_mode,
        cases=enriched_cases,
        allow_draft=allow_draft_case_review,
    )

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        "".join(
            f"{json.dumps(case, ensure_ascii=False, sort_keys=True)}\n"
            for case in enriched_cases
        ),
        encoding="utf-8",
    )
    metadata = {
        "schema_version": 2,
        "fixture_type": "robustness",
        "split": "test",
        "query_mode": query_mode,
        "source_set": "robustness",
        "scoring_status": "scored",
        "ud_release": manifest["ud_release"],
        "seed": manifest["seed"],
        "fixture_sha256": sha256(output),
        "cases": len(enriched_cases),
        "positive_cases": len(positives),
        "negative_cases": len(negatives),
        "positive_quotas_per_source": quotas,
        "target_positive_quotas_per_source": target_quotas,
        "case_review": case_review,
        "candidate_review": candidate_review_metadata,
        "negative_review": {
            "rejected_candidates": len(rejected_negative_ids),
        },
        "sentence_review": sentence_review_metadata,
        "noise_class_counts": dict(
            sorted(Counter(case["noise_class"] for case in enriched_cases).items())
        ),
        "noise_scope_counts": dict(
            sorted(Counter(case["noise_scope"] for case in enriched_cases).items())
        ),
        "sources": [
            {
                "name": source["name"],
                "description": f"{source['description']} test split",
                "data_file": split["data_file"],
                "data_url": split["data_url"],
                "data_sha256": split["data_sha256"],
                "license": source["license"],
                "license_file": source["license_file"],
                "parsing": parsing,
            }
        ],
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
    parser.add_argument(
        "--query-mode", choices=("explicit-pos", "untagged"), required=True
    )
    parser.add_argument("--allow-draft-case-review", action="store_true")
    args = parser.parse_args()
    metadata = build_robustness_dataset(
        manifest_path=args.manifest,
        sources_dir=args.sources,
        output=args.output,
        metadata_path=args.metadata,
        query_mode=args.query_mode,
        allow_draft_case_review=args.allow_draft_case_review,
    )
    print(json.dumps(metadata, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
