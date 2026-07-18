from __future__ import annotations

from collections import Counter, defaultdict
from pathlib import Path

try:
    from .query_matrix_contract import contract_case_summary
    from .validation import sha256
except ImportError:
    from query_matrix_contract import contract_case_summary
    from validation import sha256


def select_query_matrix_smoke_cases(
    cases: list[dict[str, object]],
) -> list[dict[str, object]]:
    groups: dict[str, list[dict[str, object]]] = defaultdict(list)
    for case in cases:
        groups[str(case["matrix_group_id"])].append(case)
    selected_groups = set()
    covered = set()
    for case in cases:
        if not case["expected"]:
            continue
        key = (str(case["source"]), str(case["pos"]))
        if key in covered:
            continue
        group_id = str(case["matrix_group_id"])
        selected_groups.add(group_id)
        covered.update(
            (str(group_case["source"]), str(group_case["pos"]))
            for group_case in groups[group_id]
            if group_case["expected"]
        )
    return [
        case for case in cases if str(case["matrix_group_id"]) in selected_groups
    ]


def query_matrix_smoke_metadata(
    cases_path: Path,
    cases: list[dict[str, object]],
    parent: dict[str, object],
) -> dict[str, object]:
    positive_cases = [case for case in cases if case["expected"]]
    negative_cases = [case for case in cases if not case["expected"]]
    groups = {str(case["matrix_group_id"]) for case in cases}
    distribution: Counter[str] = Counter()
    for group_id in groups:
        distribution[
            str(
                sum(
                    bool(case["expected"])
                    for case in cases
                    if case["matrix_group_id"] == group_id
                )
            )
        ] += 1
    sources = []
    for source in parent["sources"]:
        source_name = str(source["name"])
        source_cases = [case for case in cases if case["source"] == source_name]
        source_groups = {
            str(case["matrix_group_id"]) for case in source_cases
        }
        sources.append(
            {
                **source,
                "positive_cases": sum(
                    bool(case["expected"]) for case in source_cases
                ),
                "negative_cases": sum(
                    not case["expected"] for case in source_cases
                ),
                "sentences": len(source_groups),
            }
        )
    return {
        **parent,
        "split": f"{parent['split']}-smoke",
        "fixture_sha256": sha256(cases_path),
        "cases": len(cases),
        "positive_cases": len(positive_cases),
        "negative_cases": len(negative_cases),
        "sentences": len(groups),
        "present_queries_per_sentence": dict(sorted(distribution.items())),
        "canonical_positive_cases": sum(
            case["canonical_positive_id"] is not None for case in positive_cases
        ),
        "canonical_positive_coverage": sum(
            case["canonical_positive_id"] is not None for case in positive_cases
        ),
        "positive_pos_counts": dict(
            sorted(Counter(str(case["pos"]) for case in positive_cases).items())
        ),
        "negative_pos_counts": dict(
            sorted(Counter(str(case["pos"]) for case in negative_cases).items())
        ),
        "contract_review": {
            **parent["contract_review"],
            **contract_case_summary(cases),
        },
        "sources": sources,
    }
