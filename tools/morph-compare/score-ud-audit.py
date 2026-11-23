#!/usr/bin/env python3

from __future__ import annotations

import argparse
import hashlib
import json
from collections import defaultdict
from pathlib import Path

from python.quality import (
    contract_expected,
    contract_quality_metrics,
    contract_span_predictions,
    quality_metrics,
)
from python.query_matrix_contract import (
    ContractReview,
    apply_contract_reviews,
    load_contract_reviews,
)
from python.validation import load_cases, sha256


FIXTURES = {
    "eaafbae7757dbb62c9a0ca6b370dadf0f41b53c3e3d044f21b191c2aab8e3682": "raw-gsd",
    "5231c0e7c4d3c72de2982e0c38fc5420f01dcbecd764428e993b903e56dc8308": "corpus-positives-gsd",
    "482e55d417516e8db503dbd421725fcc7affc6c6afcf5b5d42cec3bbd4d35d54": "corpus-negatives",
}
GSD_SPLITS = {"raw-gsd", "corpus-positives-gsd"}
SourceKey = tuple[str, str, str]
ByteRange = tuple[int, int]


def source_ranges_for_reviews(
    evidence: list[dict[str, object]],
    selected: set[tuple[str, str]],
) -> dict[str, dict[str, object]]:
    source_reviews = {}
    for review in evidence:
        if "source_spans" not in review or (review["registry"], review["id"]) not in selected:
            continue
        if review["id"] in source_reviews:
            raise ValueError(f"duplicate source ranges: {review['id']}")
        source_reviews[review["id"]] = review
    return source_reviews


def other_gsd_positive_ranges(
    evidence: list[dict[str, object]],
    reviews: dict[tuple[str, str], ContractReview],
    current_split: str,
) -> dict[SourceKey, set[ByteRange]]:
    ranges = defaultdict(set)
    if current_split not in GSD_SPLITS:
        return ranges
    for entry in evidence:
        if not entry.get("source_spans"):
            continue
        review = reviews.get((entry["registry"], entry["id"]))
        if (
            review is None or review.query_mode != "explicit-pos"
            or review.split not in GSD_SPLITS or review.split == current_split
            or not (
                review.contract_status == "contract-positive"
                or (review.contract_status == "confirmed" and review.strict_expected)
            )
        ):
            continue
        source = entry["source_text"]
        if hashlib.sha256(source.encode()).hexdigest() != review.text_sha256:
            raise ValueError(f"source range review identity differs: {review.case_id}")
        ranges[source, review.query, review.pos].update(
            (part["byte_start"], part["byte_end"]) for part in entry["source_spans"]
        )
    return ranges


def compare_reviewed_ranges(
    cases: list[dict[str, object]],
    spans: dict[str, list[dict[str, int]]],
    adjusted: dict[str, bool],
    source_reviews: dict[str, dict[str, object]],
    other_positive_ranges: dict[SourceKey, set[ByteRange]] | None = None,
) -> tuple[dict[str, object], dict[str, list[tuple[int, int]]]]:
    annotated = defaultdict(set)
    for case in cases:
        if case["expected"] and contract_expected(case) is True:
            key = (case["text"], case["query"], case["pos"])
            review = source_reviews.get(case["id"])
            if review is None:
                annotated[key].add((case["gold_byte_start"], case["gold_byte_end"]))
            else:
                annotated[key].update(
                    (part["byte_start"], part["byte_end"]) for part in review["source_spans"]
                )

    verified_rows = unverified_positive_rows = reused_ranges = missing_count = extra_count = 0
    verified_excluded_rows = partially_excluded_positive_rows = 0
    differences = []
    unverified_ranges = {}
    pending_exclusions = {}
    for case in cases:
        case_id = case["id"]
        case_expected = contract_expected(case)
        review = source_reviews.get(case_id)
        if review is None:
            if case_expected is True:
                if case["expected"]:
                    gold = (case["gold_byte_start"], case["gold_byte_end"])
                    other = {
                        (part["byte_start"], part["byte_end"])
                        for part in spans[case_id]
                    } - {gold}
                    key = (case["text"], case["query"], case["pos"])
                    pending = other - annotated[key]
                    if pending and other_positive_ranges:
                        pending.difference_update(other_positive_ranges.get(key, ()))
                    reused_ranges += len(other) - len(pending)
                    if pending:
                        unverified_ranges[case_id] = sorted(pending)
                else:
                    unverified_positive_rows += 1
            elif case_expected is None:
                pending_exclusions[case_id] = sorted({
                    (part["byte_start"], part["byte_end"]) for part in spans[case_id]
                })
            continue
        excluded = review.get("source_excluded_spans")
        expected = {(part["byte_start"], part["byte_end"]) for part in review["source_spans"]}
        if (
            case["text"] != review["source_text"] or case_expected is False
            or (case_expected is True and not expected)
            or (case_expected is None and not excluded)
        ):
            raise ValueError(f"source range review identity differs: {case_id}")
        actual = {(part["byte_start"], part["byte_end"]) for part in spans[case_id]}
        if excluded:
            excluded_ranges = [(part["byte_start"], part["byte_end"]) for part in excluded]
            actual = {
                (start, end) for start, end in actual
                if not any(left <= start and end <= right for left, right in excluded_ranges)
            }
        missing = sorted(expected - actual)
        extra = sorted(actual - expected)
        if case_expected is True:
            adjusted[case_id] = not missing
            verified_rows += 1
            partially_excluded_positive_rows += bool(excluded)
        else:
            verified_excluded_rows += 1
        missing_count += len(missing)
        extra_count += len(extra)
        if missing or extra:
            difference = {"id": case_id, "missing": missing, "extra": extra}
            if case_expected is None:
                difference["excluded"] = True
            differences.append(difference)
    return {
        "verified_rows": verified_rows,
        "partially_excluded_positive_rows": partially_excluded_positive_rows,
        "unverified_positive_rows": unverified_positive_rows,
        "unverified_raw_positive_rows": len(unverified_ranges),
        "unverified_raw_positive_spans": sum(map(len, unverified_ranges.values())),
        "verified_excluded_rows": verified_excluded_rows,
        "unverified_excluded_rows": len(pending_exclusions),
        "unverified_excluded_spans": sum(map(len, pending_exclusions.values())),
        "reused_raw_positive_spans": reused_ranges,
        "missing": missing_count,
        "extra": extra_count,
        "differences": differences,
    }, {**unverified_ranges, **pending_exclusions}


def score_contract(
    cases: list[dict[str, object]],
    predictions: dict[str, bool],
    spans: dict[str, list[dict[str, int]]],
    source_reviews: dict[str, dict[str, object]],
    *,
    include_residual: bool,
    other_positive_ranges: dict[SourceKey, set[ByteRange]] | None = None,
) -> dict[str, object]:
    adjusted = contract_span_predictions(cases, predictions, spans)
    reviewed_ranges, unverified_ranges = compare_reviewed_ranges(
        cases, spans, adjusted, source_reviews, other_positive_ranges
    )
    range_errors = {entry["id"]: entry for entry in reviewed_ranges["differences"]}
    contract = contract_quality_metrics(cases, adjusted)
    fpc = contract["contract_fp"] + sum(bool(entry["extra"]) for entry in range_errors.values())
    fnc = contract["contract_fn"] + sum(
        bool(entry["missing"]) for entry in range_errors.values() if entry.get("excluded")
    )
    zero_errors = fpc == fnc == 0
    report = {
        "contract": contract,
        "reviewed_ranges": reviewed_ranges,
        "contract_errors": {
            "fpc": fpc,
            "fnc": fnc,
            "zero_errors_observed": zero_errors,
            "zero_target_verified": zero_errors
            and reviewed_ranges["unverified_positive_rows"] == 0
            and not unverified_ranges,
        },
    }
    if include_residual:
        residual = []
        for case in cases:
            expected = contract_expected(case)
            case_id = case["id"]
            difference = range_errors.get(case_id)
            false_positive = (expected is False and adjusted[case_id]) or (
                difference is not None and bool(difference["extra"])
            )
            false_negative = (expected is True and not adjusted[case_id]) or (
                difference is not None and bool(difference["missing"])
            )
            unverified = unverified_ranges.get(case_id)
            if false_positive or false_negative or unverified is not None:
                row = {
                    "id": case_id,
                    "expected": expected,
                    "spans": spans[case_id],
                    "false_positive": false_positive,
                    "false_negative": false_negative,
                }
                if difference is not None:
                    row["missing"] = difference["missing"]
                    row["extra"] = difference["extra"]
                if unverified is not None:
                    row["unverified"] = unverified
                residual.append(row)
        report["residual"] = residual
    return report


def validate_runner_output(
    result: dict[str, object], checksum: str, cases: list[dict[str, object]]
) -> None:
    if result.get("fixture_sha256") != checksum:
        raise ValueError("runner가 읽은 입력의 SHA-256이 등록된 fixture와 다릅니다.")
    if (result["profile"], result["boundary"], result.get("query_mode")) != (
        "full-pos", "smart", "explicit-pos"
    ):
        raise ValueError("boundary full-pos smart 명시 품사 runner 출력이 필요합니다.")
    if [row["id"] for row in result["results"]] != [case["id"] for case in cases]:
        raise ValueError("runner 출력의 행 수 또는 질의 순서가 입력과 다릅니다.")


def main() -> int:
    parser = argparse.ArgumentParser(
        description="공개 UD 고정 감사 입력의 Smart 결과를 채점합니다."
    )
    parser.add_argument("fixture", type=Path)
    parser.add_argument("runner_output", type=Path)
    parser.add_argument(
        "--residual", action="store_true", help="계약 오탐·누락의 ID와 반환 범위 포함"
    )
    args = parser.parse_args()
    checksum = sha256(args.fixture)
    split = FIXTURES.get(checksum)
    if split is None:
        parser.error(f"등록된 UD 입력의 SHA-256과 일치하지 않습니다: {checksum}")
    cases = load_cases(args.fixture)
    result = json.loads(args.runner_output.read_text(encoding="utf-8"))
    try:
        validate_runner_output(result, checksum, cases)
    except ValueError as error:
        parser.error(str(error))

    review_dir = Path(__file__).resolve().parent
    reviews = []
    source_registry = {}
    for registry in sorted(review_dir.glob("*-corpus-reviews.tsv")):
        entries = load_contract_reviews(registry)
        reviews.extend(entries)
        source_registry.update(
            ((f"tools/morph-compare/{registry.name}", review.case_id), review)
            for review in entries
            if review.query_mode == "explicit-pos"
        )
    apply_contract_reviews(cases, reviews, query_mode="explicit-pos", split=split)
    evidence = load_cases(review_dir / "contract-contrast-evidence.jsonl")
    source_reviews = source_ranges_for_reviews(
        evidence, {key for key, review in source_registry.items() if review.split == split}
    )
    other_positive_ranges = other_gsd_positive_ranges(evidence, source_registry, split)
    del evidence

    predictions = {}
    spans = {}
    for case, row in zip(cases, result["results"], strict=True):
        spans[case["id"]] = row["spans"]
        predictions[case["id"]] = (
            any(
                span["byte_start"] == case["gold_byte_start"]
                and span["byte_end"] == case["gold_byte_end"]
                for span in row["spans"]
            )
            if case["expected"]
            else bool(row["spans"])
        )
    report = {
        "fixture": {"split": split, "sha256": checksum, "rows": len(cases)},
        "resources": {
            key: result[key]
            for key in (
                "lexicon_artifact_sha256",
                "component_artifact_sha256",
                "enriched_artifact_sha256",
            )
        },
        "raw": quality_metrics(cases, predictions),
        **score_contract(
            cases,
            predictions,
            spans,
            source_reviews,
            include_residual=args.residual,
            other_positive_ranges=other_positive_ranges,
        ),
    }
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
