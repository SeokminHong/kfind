from __future__ import annotations

import csv
import json
from collections import Counter
from dataclasses import dataclass
from pathlib import Path


ALLOWED_DISPOSITIONS = frozenset(
    {
        "product-fix",
        "dictionary-required",
        "structurally-unresolvable",
        "cost-prohibitive",
        "gold-or-adapter",
        "out-of-contract",
    }
)
LEDGER_FIELDS = (
    "fixture_sha256",
    "backend",
    "case_id",
    "query",
    "pos",
    "gold_surface",
    "failure_cause",
    "disposition",
    "rationale",
    "dictionary_evidence",
)


@dataclass(frozen=True)
class ContractFalseNegative:
    fixture_sha256: str
    backend: str
    case_id: str
    query: str
    pos: str
    gold_surface: str
    failure_cause: str


@dataclass(frozen=True)
class DispositionSummary:
    raw_contract_false_negatives: int
    unclassified_contract_false_negatives: int
    disposition_counts: dict[str, int]


def load_report(path: Path) -> dict[str, object]:
    with path.open(encoding="utf-8") as report_file:
        report = json.load(report_file)
    if not isinstance(report, dict):
        raise ValueError("benchmark report root must be an object")
    return report


def load_ledger(path: Path) -> list[dict[str, str]]:
    with path.open(encoding="utf-8", newline="") as ledger_file:
        reader = csv.DictReader(ledger_file, delimiter="\t")
        if tuple(reader.fieldnames or ()) != LEDGER_FIELDS:
            raise ValueError(
                "disposition ledger fields differ from the required schema: "
                + ", ".join(LEDGER_FIELDS)
            )
        return [dict(row) for row in reader]


def contract_false_negatives(
    report: dict[str, object], backend: str
) -> dict[str, ContractFalseNegative]:
    explicit_pos = report["query_matrix"]["explicit_pos"]
    fixture_sha256 = explicit_pos["dataset"]["fixture_sha256"]
    failures = explicit_pos["failures"]
    false_negatives = {}
    for failure in failures:
        case = failure["case"]
        expected = case.get("contract_expected", case["expected"])
        if not expected or failure["predictions"].get(backend) is not False:
            continue
        cause = failure["profile_causes"].get(backend) or failure["primary_cause"]
        if not isinstance(cause, str) or not cause:
            raise ValueError(f"contract FN has no failure cause: {case['id']}")
        case_id = str(case["id"])
        if case_id in false_negatives:
            raise ValueError(f"duplicate contract FN case: {case_id}")
        false_negatives[case_id] = ContractFalseNegative(
            fixture_sha256=str(fixture_sha256),
            backend=backend,
            case_id=case_id,
            query=str(case["query"]),
            pos=str(case["pos"]),
            gold_surface=_gold_surface(case),
            failure_cause=cause,
        )
    return false_negatives


def validate_disposition_ledger(
    report: dict[str, object], ledger: list[dict[str, str]], backend: str
) -> DispositionSummary:
    false_negatives = contract_false_negatives(report, backend)
    ledger_by_id = {}
    for row_number, row in enumerate(ledger, start=2):
        case_id = row["case_id"]
        if not case_id:
            raise ValueError(f"disposition ledger row {row_number} has no case_id")
        if case_id in ledger_by_id:
            raise ValueError(f"duplicate disposition ledger case: {case_id}")
        ledger_by_id[case_id] = row

    missing = sorted(set(false_negatives) - set(ledger_by_id))
    stale = sorted(set(ledger_by_id) - set(false_negatives))
    if missing or stale:
        details = []
        if missing:
            details.append("missing=" + ",".join(missing))
        if stale:
            details.append("stale=" + ",".join(stale))
        raise ValueError(
            "disposition ledger does not match current contract FN set: "
            + "; ".join(details)
        )

    counts = Counter()
    for case_id, expected in false_negatives.items():
        row = ledger_by_id[case_id]
        actual_identity = tuple(row[field] for field in LEDGER_FIELDS[:7])
        expected_identity = (
            expected.fixture_sha256,
            expected.backend,
            expected.case_id,
            expected.query,
            expected.pos,
            expected.gold_surface,
            expected.failure_cause,
        )
        if actual_identity != expected_identity:
            raise ValueError(f"disposition ledger identity differs from report: {case_id}")
        disposition = row["disposition"]
        if disposition not in ALLOWED_DISPOSITIONS:
            raise ValueError(f"invalid disposition for {case_id}: {disposition}")
        if not row["rationale"].strip():
            raise ValueError(f"disposition has no rationale: {case_id}")
        if not row["dictionary_evidence"].strip():
            raise ValueError(f"disposition has no dictionary evidence: {case_id}")
        counts[disposition] += 1

    return DispositionSummary(
        raw_contract_false_negatives=len(false_negatives),
        unclassified_contract_false_negatives=0,
        disposition_counts=dict(sorted(counts.items())),
    )


def _gold_surface(case: dict[str, object]) -> str:
    text = str(case["text"]).encode("utf-8")
    start = int(case["gold_byte_start"])
    end = int(case["gold_byte_end"])
    if start < 0 or end <= start or end > len(text):
        raise ValueError(f"invalid gold byte span: {case['id']}")
    try:
        return text[start:end].decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValueError(f"gold byte span is not UTF-8 aligned: {case['id']}") from error
