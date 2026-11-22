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
) -> dict[str, object]:
    source_reviews = {}
    for review in load_cases(registry):
        if "source_spans" not in review:
            continue
        if review["id"] in source_reviews:
            raise ValueError(f"duplicate source ranges: {review['id']}")
        source_reviews[review["id"]] = review

    verified_rows = missing_count = extra_count = 0
    differences = []
    for case in cases:
        case_id = case["id"]
        review = source_reviews.get(case_id)
        if review is None:
            continue
        if case.get("contract_expected") is not True or case["text"] != review["source_text"]:
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
        "missing": missing_count,
        "extra": extra_count,
        "differences": differences,
    }


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
    if (result["profile"], result["boundary"]) != ("full-pos", "smart"):
        parser.error("boundary full-pos smart runner 출력이 필요합니다.")
    if [row["id"] for row in result["results"]] != [case["id"] for case in cases]:
        parser.error("runner 출력의 행 수 또는 질의 순서가 입력과 다릅니다.")

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
    adjusted = contract_span_predictions(cases, predictions, spans)
    reviewed_ranges = compare_reviewed_ranges(
        cases, spans, adjusted, review_dir / "contract-contrast-evidence.jsonl"
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
        "contract": contract_quality_metrics(cases, adjusted),
        "reviewed_ranges": reviewed_ranges,
    }
    if args.residual:
        report["residual"] = [
            {"id": case["id"], "expected": expected, "spans": spans[case["id"]]}
            for case in cases
            if (expected := contract_expected(case)) is not None
            and adjusted[case["id"]] != expected
        ]
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
