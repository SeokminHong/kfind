from __future__ import annotations

import hashlib
import json
from collections import defaultdict
from pathlib import Path


HARD_NEGATIVE_SLICES = {
    "homonymous-other-pos",
    "compound-substring",
    "attached-predecessor-predicate",
    "same-surface-different-lemma",
    "one-syllable-boundary",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_cases(path: Path) -> list[dict[str, object]]:
    with path.open(encoding="utf-8") as fixture_file:
        return [json.loads(line) for line in fixture_file if line.strip()]


def validate_dataset(
    cases_path: Path, cases: list[dict[str, object]], metadata: dict[str, object]
) -> None:
    if len(cases) != 1_000 or metadata["cases"] != 1_000:
        raise ValueError("benchmark requires exactly 1,000 cases")
    if sha256(cases_path) != metadata["fixture_sha256"]:
        raise ValueError("fixture SHA-256 does not match metadata")
    expected_ids = {case["id"] for case in cases}
    if len(expected_ids) != len(cases):
        raise ValueError("benchmark case IDs are not unique")
    positives = sum(bool(case["expected"]) for case in cases)
    if positives != 500:
        raise ValueError(f"benchmark requires 500 positive cases, got {positives}")
    counts: dict[tuple[str, str, bool], int] = defaultdict(int)
    for case in cases:
        counts[(str(case["source"]), str(case["pos"]), bool(case["expected"]))] += 1
    quotas = metadata["positive_quotas_per_source"]
    for source in metadata["sources"]:
        for pos, quota in quotas.items():
            for expected in (True, False):
                actual = counts[(source["name"], pos, expected)]
                if actual != quota:
                    raise ValueError(
                        f"quota mismatch for {source['name']}/{pos}/{expected}: "
                        f"expected {quota}, got {actual}"
                    )


def validate_hard_negatives(
    cases_path: Path, cases: list[dict[str, object]]
) -> dict[str, object]:
    if not cases or any(case["expected"] for case in cases):
        raise ValueError("hard-negative fixture must contain only negative cases")
    case_ids = {case["id"] for case in cases}
    if len(case_ids) != len(cases):
        raise ValueError("hard-negative case IDs are not unique")
    slices = {str(case.get("slice")) for case in cases}
    if slices != HARD_NEGATIVE_SLICES:
        raise ValueError(
            f"hard-negative slices differ: expected {sorted(HARD_NEGATIVE_SLICES)}, "
            f"got {sorted(slices)}"
        )
    return {
        "schema_version": 1,
        "split": "hard-negative",
        "fixture_sha256": sha256(cases_path),
        "cases": len(cases),
        "positive_cases": 0,
        "negative_cases": len(cases),
        "seed": "version-controlled",
        "ud_release": "n/a",
        "sources": [],
    }
