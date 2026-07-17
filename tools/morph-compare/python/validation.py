from __future__ import annotations

import hashlib
import json
from collections import defaultdict
from pathlib import Path

try:
    from .quality import contract_expected
except ImportError:
    from quality import contract_expected


HARD_NEGATIVE_SLICES = {
    "homonymous-other-pos",
    "compound-substring",
    "attached-predecessor-predicate",
    "nominalizer-particle",
    "same-surface-different-lemma",
    "one-syllable-boundary",
    "numeric-unit",
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
    metadata = {
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
    for key in ("source_set", "scoring_status", "query_mode"):
        if key in development_metadata:
            metadata[key] = development_metadata[key]
    return metadata

def validate_fixture_identity(
    cases_path: Path, cases: list[dict[str, object]], metadata: dict[str, object]
) -> None:
    if sha256(cases_path) != metadata["fixture_sha256"]:
        raise ValueError("fixture SHA-256 does not match metadata")
    case_ids = {case["id"] for case in cases}
    if len(case_ids) != len(cases):
        raise ValueError("benchmark case IDs are not unique")
    for case in cases:
        contract_expected(case)


def validate_dataset(
    cases_path: Path, cases: list[dict[str, object]], metadata: dict[str, object]
) -> None:
    if metadata.get("source_set") != "canonical":
        raise ValueError("benchmark requires the canonical source set")
    if metadata.get("scoring_status") != "scored":
        raise ValueError("benchmark requires a scored source set")
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


def validate_robustness_candidate_dataset(
    cases_path: Path,
    cases: list[dict[str, object]],
    metadata: dict[str, object],
    query_mode: str,
) -> None:
    if metadata.get("source_set") != "robustness-candidate":
        raise ValueError("robustness performance requires its candidate source set")
    if metadata.get("scoring_status") != "annotation-required":
        raise ValueError("robustness performance requires annotation-required data")
    if metadata.get("query_mode") != query_mode:
        raise ValueError(f"robustness performance requires query_mode={query_mode}")
    validate_fixture_identity(cases_path, cases, metadata)
    if len(cases) != metadata.get("cases"):
        raise ValueError("robustness candidate case count differs from metadata")
    positives = sum(bool(case["expected"]) for case in cases)
    negatives = len(cases) - positives
    if positives == 0 or positives != negatives:
        raise ValueError("robustness performance requires balanced candidate cases")
    if metadata.get("positive_cases") != positives:
        raise ValueError("robustness candidate positive count differs from metadata")
    if metadata.get("negative_cases") != negatives:
        raise ValueError("robustness candidate negative count differs from metadata")


def validate_query_matrix_dataset(
    cases_path: Path,
    cases: list[dict[str, object]],
    metadata: dict[str, object],
    query_mode: str,
) -> None:
    if metadata.get("source_set") != "canonical":
        raise ValueError("query matrix requires the canonical source set")
    if metadata.get("scoring_status") != "scored":
        raise ValueError("query matrix requires a scored source set")
    if metadata.get("fixture_type") != "query-matrix":
        raise ValueError("query matrix fixture_type must be query-matrix")
    if metadata.get("query_mode") != query_mode:
        raise ValueError(f"query matrix requires query_mode={query_mode}")
    validate_fixture_identity(cases_path, cases, metadata)
    if len(cases) != metadata.get("cases"):
        raise ValueError("query matrix case count differs from metadata")
    positives = [case for case in cases if case["expected"]]
    negatives = [case for case in cases if not case["expected"]]
    if not positives or len(positives) != len(negatives):
        raise ValueError("query matrix requires balanced positive and negative cases")
    if metadata.get("positive_cases") != len(positives):
        raise ValueError("query matrix positive count differs from metadata")
    if metadata.get("negative_cases") != len(negatives):
        raise ValueError("query matrix negative count differs from metadata")

    groups: dict[str, list[dict[str, object]]] = defaultdict(list)
    for case in cases:
        group_id = case.get("matrix_group_id")
        if not isinstance(group_id, str) or not group_id:
            raise ValueError("query matrix case has no matrix_group_id")
        groups[group_id].append(case)
    if metadata.get("sentences") != len(groups):
        raise ValueError("query matrix sentence count differs from metadata")

    canonical_ids = []
    sentence_distribution: dict[str, int] = defaultdict(int)
    for group_id, group_cases in groups.items():
        source_sent_text = {
            (str(case["source"]), str(case["sent_id"]), str(case["text"]))
            for case in group_cases
        }
        if len(source_sent_text) != 1:
            raise ValueError(f"query matrix group {group_id} mixes sentences")
        present = sorted(
            (case for case in group_cases if case["expected"]),
            key=lambda case: str(case["matrix_slot"]),
        )
        absent = sorted(
            (case for case in group_cases if not case["expected"]),
            key=lambda case: str(case["matrix_slot"]),
        )
        if len(present) != len(absent) or not 1 <= len(present) <= 3:
            raise ValueError(f"query matrix group {group_id} has invalid balance")
        expected_present_slots = [
            f"present-{index}" for index in range(1, len(present) + 1)
        ]
        expected_absent_slots = [
            f"absent-{index}" for index in range(1, len(absent) + 1)
        ]
        if [case["matrix_slot"] for case in present] != expected_present_slots:
            raise ValueError(f"query matrix group {group_id} has invalid present slots")
        if [case["matrix_slot"] for case in absent] != expected_absent_slots:
            raise ValueError(f"query matrix group {group_id} has invalid absent slots")
        if {
            str(case["paired_positive_id"]) for case in absent
        } != {str(case["id"]) for case in present}:
            raise ValueError(f"query matrix group {group_id} has invalid positive pairs")
        if any(case["paired_positive_id"] is not None for case in present):
            raise ValueError("query matrix positive cannot reference a paired positive")
        if any(case["canonical_positive_id"] is not None for case in absent):
            raise ValueError("query matrix negative cannot be canonical")
        for case in present:
            canonical_id = case.get("canonical_positive_id")
            if canonical_id is not None:
                canonical_ids.append(str(canonical_id))
        present_by_id = {str(case["id"]): case for case in present}
        for case in absent:
            paired = present_by_id[str(case["paired_positive_id"])]
            if case["pos"] != paired["pos"]:
                raise ValueError("query matrix pair must preserve coarse POS")
        if query_mode == "untagged":
            present_queries = {str(case["query"]) for case in present}
            absent_queries = [str(case["query"]) for case in absent]
            if len(absent_queries) != len(set(absent_queries)) or present_queries & set(
                absent_queries
            ):
                raise ValueError(f"query matrix group {group_id} repeats a query")
        else:
            present_pairs = {
                (str(case["query"]), str(case["pos"])) for case in present
            }
            absent_pairs = [
                (str(case["query"]), str(case["pos"])) for case in absent
            ]
            if len(absent_pairs) != len(set(absent_pairs)) or present_pairs & set(
                absent_pairs
            ):
                raise ValueError(f"query matrix group {group_id} repeats a query")
        sentence_distribution[str(len(present))] += 1

    if len(canonical_ids) != len(set(canonical_ids)):
        raise ValueError("query matrix repeats a canonical positive")
    expected_canonical = metadata.get("canonical_positive_cases")
    if len(canonical_ids) != expected_canonical:
        raise ValueError("query matrix does not cover every canonical positive")
    if metadata.get("canonical_positive_coverage") != expected_canonical:
        raise ValueError("query matrix canonical coverage metadata is inconsistent")
    if dict(sorted(sentence_distribution.items())) != metadata.get(
        "present_queries_per_sentence"
    ):
        raise ValueError("query matrix sentence distribution differs from metadata")
    if not isinstance(metadata.get("derived_from_fixture_sha256"), str):
        raise ValueError("query matrix has no canonical fixture SHA-256")
    if query_mode == "untagged" and any(
        not str(case["id"]).startswith("untagged:matrix:") for case in cases
    ):
        raise ValueError("untagged query matrix case ID has an invalid prefix")


def validate_hard_negatives(
    cases_path: Path, cases: list[dict[str, object]]
) -> dict[str, object]:
    if not cases or any(case["expected"] for case in cases):
        raise ValueError("hard-negative fixture must contain only negative cases")
    for case in cases:
        contract_expected(case)
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
