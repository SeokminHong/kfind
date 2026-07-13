from __future__ import annotations

import hashlib
import json
from collections import defaultdict
from pathlib import Path


HARD_NEGATIVE_SLICES = {
    "homonymous-other-pos",
    "compound-substring",
    "attached-predecessor-predicate",
    "nominalizer-particle",
    "same-surface-different-lemma",
    "one-syllable-boundary",
}


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_cases(path: Path) -> list[dict[str, object]]:
    with path.open(encoding="utf-8") as fixture_file:
        return [json.loads(line) for line in fixture_file if line.strip()]


def select_smoke_cases(
    cases: list[dict[str, object]],
    group_keys: tuple[str, ...] = ("source", "pos", "expected"),
) -> list[dict[str, object]]:
    selected_ids = set()
    selected_groups = set()
    for case in cases:
        group = tuple(case[key] for key in group_keys)
        if group not in selected_groups:
            selected_groups.add(group)
            selected_ids.add(case["id"])
    return [case for case in cases if case["id"] in selected_ids]


def write_cases(path: Path, cases: list[dict[str, object]]) -> None:
    with path.open("w", encoding="utf-8") as fixture_file:
        for case in cases:
            fixture_file.write(
                json.dumps(case, ensure_ascii=False, sort_keys=True) + "\n"
            )


def smoke_metadata(
    cases_path: Path,
    cases: list[dict[str, object]],
    development_metadata: dict[str, object],
    split: str = "dev-smoke",
) -> dict[str, object]:
    return {
        "schema_version": 1,
        "split": split,
        "fixture_sha256": sha256(cases_path),
        "cases": len(cases),
        "positive_cases": sum(bool(case["expected"]) for case in cases),
        "negative_cases": sum(not case["expected"] for case in cases),
        "seed": development_metadata["seed"],
        "ud_release": development_metadata["ud_release"],
        "sources": development_metadata["sources"],
    }

def validate_fixture_identity(
    cases_path: Path, cases: list[dict[str, object]], metadata: dict[str, object]
) -> None:
    if sha256(cases_path) != metadata["fixture_sha256"]:
        raise ValueError("fixture SHA-256 does not match metadata")
    case_ids = {case["id"] for case in cases}
    if len(case_ids) != len(cases):
        raise ValueError("benchmark case IDs are not unique")


def validate_dataset(
    cases_path: Path, cases: list[dict[str, object]], metadata: dict[str, object]
) -> None:
    if len(cases) != 1_000 or metadata["cases"] != 1_000:
        raise ValueError("benchmark requires exactly 1,000 cases")
    validate_fixture_identity(cases_path, cases, metadata)
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


def validate_untagged_dataset(
    cases_path: Path, cases: list[dict[str, object]], metadata: dict[str, object]
) -> None:
    validate_dataset(cases_path, cases, metadata)
    if metadata.get("query_mode") != "untagged":
        raise ValueError("untagged benchmark requires query_mode=untagged")
    positive_ids = {str(case["id"]) for case in cases if case["expected"]}
    for case in cases:
        if case["expected"]:
            if not str(case["id"]).startswith("untagged:pos:"):
                raise ValueError("untagged positive case ID has an invalid prefix")
            continue
        if case.get("paired_positive_id") not in positive_ids:
            raise ValueError("untagged negative does not reference a positive case")


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
