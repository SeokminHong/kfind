#!/usr/bin/env python3

from __future__ import annotations

import argparse
import json
from pathlib import Path

from python.quality import (
    contract_expected,
    contract_quality_metrics,
    contract_span_predictions,
    quality_metrics,
)
from python.query_matrix_contract import apply_contract_reviews, load_contract_reviews
from python.validation import load_cases, sha256


FIXTURES = {
    "eaafbae7757dbb62c9a0ca6b370dadf0f41b53c3e3d044f21b191c2aab8e3682": "raw-gsd",
    "cd3a8e532e67ba0748875311d10eba91c5fcb0d644e5c541bf3a4d22ac7c5a37": "corpus-positives-gsd",
    "482e55d417516e8db503dbd421725fcc7affc6c6afcf5b5d42cec3bbd4d35d54": "corpus-negatives",
}


def compare_reviewed_ranges(
    cases: list[dict[str, object]],
    spans: dict[str, list[dict[str, int]]],
    adjusted: dict[str, bool],
    registry: Path,
) -> tuple[dict[str, object], dict[str, list[tuple[int, int]]]]:
    source_reviews = {}
    for review in load_cases(registry):
        if "source_spans" not in review:
            continue
        if review["id"] in source_reviews:
            raise ValueError(f"duplicate source ranges: {review['id']}")
        source_reviews[review["id"]] = review

    verified_rows = unverified_positive_rows = missing_count = extra_count = 0
    differences = []
    unverified_ranges = {}
    for case in cases:
        case_id = case["id"]
        review = source_reviews.get(case_id)
        if review is None:
            if contract_expected(case) is True:
                if case["expected"]:
                    gold = (case["gold_byte_start"], case["gold_byte_end"])
                    other = sorted({
                        (part["byte_start"], part["byte_end"])
                        for part in spans[case_id]
                    } - {gold})
                    if other:
                        unverified_ranges[case_id] = other
                else:
                    unverified_positive_rows += 1
            continue
        if contract_expected(case) is not True or case["text"] != review["source_text"]:
            raise ValueError(f"source range review identity differs: {case_id}")
        expected = {(part["byte_start"], part["byte_end"]) for part in review["source_spans"]}
        actual = {(part["byte_start"], part["byte_end"]) for part in spans[case_id]}
        missing = sorted(expected - actual)
        extra = sorted(actual - expected)
        adjusted[case_id] = not missing
        verified_rows += 1
        missing_count += len(missing)
        extra_count += len(extra)
        if missing or extra:
            differences.append({"id": case_id, "missing": missing, "extra": extra})
    return {
        "verified_rows": verified_rows,
        "unverified_positive_rows": unverified_positive_rows,
        "unverified_raw_positive_rows": len(unverified_ranges),
        "unverified_raw_positive_spans": sum(map(len, unverified_ranges.values())),
        "missing": missing_count,
        "extra": extra_count,
        "differences": differences,
    }, unverified_ranges


def score_contract(
    cases: list[dict[str, object]],
    predictions: dict[str, bool],
    spans: dict[str, list[dict[str, int]]],
    registry: Path,
    *,
    include_residual: bool,
) -> dict[str, object]:
    adjusted = contract_span_predictions(cases, predictions, spans)
    reviewed_ranges, unverified_ranges = compare_reviewed_ranges(cases, spans, adjusted, registry)
    range_errors = {entry["id"]: entry for entry in reviewed_ranges["differences"]}
    contract = contract_quality_metrics(cases, adjusted)
    fpc = contract["contract_fp"] + sum(bool(entry["extra"]) for entry in range_errors.values())
    fnc = contract["contract_fn"]
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
            if expected is None:
                continue
            case_id = case["id"]
            difference = range_errors.get(case_id)
            false_positive = (not expected and adjusted[case_id]) or (
                difference is not None and bool(difference["extra"])
            )
            false_negative = expected and not adjusted[case_id]
            unverified = unverified_ranges.get(case_id)
            if false_positive or false_negative or unverified:
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
                if unverified:
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
    reviews = [
        review
        for registry in sorted(review_dir.glob("*-corpus-reviews.tsv"))
        for review in load_contract_reviews(registry)
    ]
    apply_contract_reviews(cases, reviews, query_mode="explicit-pos", split=split)

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
            review_dir / "contract-contrast-evidence.jsonl",
            include_residual=args.residual,
        ),
    }
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
