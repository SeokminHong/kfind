from __future__ import annotations

import csv
import hashlib
from collections import Counter
from dataclasses import dataclass
from pathlib import Path

try:
    from .quality import contract_expected
except ImportError:
    from quality import contract_expected


REVIEW_FIELDS = (
    "query_mode",
    "split",
    "case_id",
    "query",
    "pos",
    "strict_expected",
    "text_sha256",
    "contract_status",
    "contract_reason",
    "note",
)
CONTRACT_STATUSES = {"contract-positive", "excluded"}


@dataclass(frozen=True)
class ContractReview:
    query_mode: str
    split: str
    case_id: str
    query: str
    pos: str
    strict_expected: bool
    text_sha256: str
    contract_status: str
    contract_reason: str


def load_contract_reviews(path: Path) -> list[ContractReview]:
    with path.open(encoding="utf-8", newline="") as review_file:
        reader = csv.DictReader(review_file, delimiter="\t")
        if tuple(reader.fieldnames or ()) != REVIEW_FIELDS:
            raise ValueError(
                "contract review fields differ from the required schema: "
                + ", ".join(REVIEW_FIELDS)
            )
        reviews = []
        for row_number, row in enumerate(reader, start=2):
            if row["strict_expected"] not in {"true", "false"}:
                raise ValueError(
                    f"contract review row {row_number} has invalid strict_expected"
                )
            if row["contract_status"] not in CONTRACT_STATUSES:
                raise ValueError(
                    f"contract review row {row_number} has invalid contract_status"
                )
            if not row["note"].strip():
                raise ValueError(f"contract review row {row_number} has no note")
            reviews.append(
                ContractReview(
                    query_mode=row["query_mode"],
                    split=row["split"],
                    case_id=row["case_id"],
                    query=row["query"],
                    pos=row["pos"],
                    strict_expected=row["strict_expected"] == "true",
                    text_sha256=row["text_sha256"],
                    contract_status=row["contract_status"],
                    contract_reason=row["contract_reason"],
                )
            )
    return reviews


def apply_contract_reviews(
    cases: list[dict[str, object]],
    reviews: list[ContractReview],
    *,
    query_mode: str,
    split: str,
) -> dict[str, object]:
    scoped = [
        review
        for review in reviews
        if review.query_mode == query_mode and review.split == split
    ]
    by_id = {str(case["id"]): case for case in cases}
    if len({review.case_id for review in scoped}) != len(scoped):
        raise ValueError(f"duplicate contract review for {query_mode}/{split}")
    for review in scoped:
        case = by_id.get(review.case_id)
        if case is None:
            raise ValueError(f"contract review case is missing: {review.case_id}")
        identity = (
            str(case["query"]),
            str(case["pos"]),
            bool(case["expected"]),
            hashlib.sha256(str(case["text"]).encode()).hexdigest(),
        )
        reviewed_identity = (
            review.query,
            review.pos,
            review.strict_expected,
            review.text_sha256,
        )
        if identity != reviewed_identity:
            raise ValueError(f"contract review identity differs: {review.case_id}")
        case["contract_expected"] = (
            True if review.contract_status == "contract-positive" else None
        )
        case["contract_reason"] = review.contract_reason
        contract_expected(case)
    return contract_case_summary(cases)


def contract_case_summary(cases: list[dict[str, object]]) -> dict[str, object]:
    reclassified = Counter()
    excluded = Counter()
    for case in cases:
        expected = contract_expected(case)
        if "contract_expected" not in case:
            continue
        target = excluded if expected is None else reclassified
        target[str(case["contract_reason"])] += 1
    return {
        "reviewed_cases": sum(reclassified.values()) + sum(excluded.values()),
        "reclassified_cases": sum(reclassified.values()),
        "reclassified_by_reason": dict(sorted(reclassified.items())),
        "excluded_cases": sum(excluded.values()),
        "excluded_by_reason": dict(sorted(excluded.items())),
    }


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()
