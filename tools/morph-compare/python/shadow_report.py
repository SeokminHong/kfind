from __future__ import annotations

from collections import defaultdict
from typing import Any


KFIND_PROFILES = ("kfind-embedded", "kfind-full-pos")
SHADOW_COUNTERS = (
    "raw_anchor_hits",
    "verified_program_hits",
    "structural_candidate_hits",
    "unique_structural_windows",
)


def shadow_verification_summary(
    by_case: dict[str, dict[str, object]],
    cases: list[dict[str, object]] | None = None,
) -> dict[str, object]:
    totals = {
        name: sum(int(counters[name]) for counters in by_case.values())
        for name in SHADOW_COUNTERS
    }
    lattice_statuses: dict[str, int] = defaultdict(int)
    lattice_decisions: dict[str, int] = defaultdict(int)
    lattice_cases_by_decision: dict[str, int] = defaultdict(int)
    case_metadata = {str(case["id"]): case for case in cases or []}
    lattice_outcomes_by_class: dict[str, dict[str, int]] = defaultdict(
        lambda: defaultdict(int)
    )
    for case_id, counters in by_case.items():
        case = case_metadata.get(case_id)
        for evidence in counters.get("diagnostic_lattice", []):
            status = str(evidence["status"])
            decision = evidence.get("decision")
            outcome = str(decision) if decision is not None else status
            lattice_statuses[status] += 1
            if decision is not None:
                lattice_decisions[str(decision)] += 1
            if case is not None:
                class_name = "positive" if bool(case["expected"]) else "negative"
                lattice_outcomes_by_class[class_name][outcome] += 1
        case_decisions = {
            str(evidence["decision"])
            for evidence in counters.get("diagnostic_lattice", [])
            if evidence.get("decision") is not None
        }
        for decision in case_decisions:
            lattice_cases_by_decision[decision] += 1

    path_classification = classify_lattice_paths(by_case, case_metadata)
    def sorted_outcomes(
        grouped: dict[str, dict[str, int]],
    ) -> dict[str, dict[str, int]]:
        return {
            group: dict(sorted(counts.items()))
            for group, counts in sorted(grouped.items())
        }

    return {
        "totals": totals,
        "cases_with_structural_candidates": sum(
            counters["structural_candidate_hits"] > 0
            for counters in by_case.values()
        ),
        "lattice_statuses": dict(sorted(lattice_statuses.items())),
        "lattice_decisions": dict(sorted(lattice_decisions.items())),
        "lattice_cases_by_decision": dict(
            sorted(lattice_cases_by_decision.items())
        ),
        "lattice_outcomes_by_class": sorted_outcomes(
            lattice_outcomes_by_class
        ),
        "lattice_path_classification": path_classification,
        "by_case": by_case,
    }


def classify_lattice_paths(
    by_case: dict[str, dict[str, object]],
    case_metadata: dict[str, dict[str, object]],
) -> dict[str, object]:
    by_case_classification: dict[str, dict[str, object]] = {}
    path_types: dict[str, dict[str, dict[str, int]]] = defaultdict(
        lambda: defaultdict(lambda: defaultdict(int))
    )
    p1_candidates: dict[str, dict[str, int]] = defaultdict(
        lambda: defaultdict(int)
    )
    for case_id, counters in by_case.items():
        case = case_metadata.get(case_id)
        class_name = (
            "positive" if case is not None and bool(case["expected"]) else "negative"
        )
        decisions: dict[str, object] = {}
        evidence = counters.get("diagnostic_lattice", [])
        if not isinstance(evidence, list):
            continue
        for decision in ("accept", "reject"):
            selected = _select_component_path(evidence, decision)
            if selected is None:
                continue
            classified = _classify_component_path(selected, decision)
            decisions[decision] = classified
            path_types[class_name][decision][str(classified["path_type"])] += 1
            candidate = classified.get("p1_rule_candidate")
            if candidate is not None:
                p1_candidates[class_name][str(candidate)] += 1
        if decisions:
            by_case_classification[case_id] = {
                "class": class_name,
                "decisions": decisions,
            }
    return {
        "path_types_by_class": _sorted_nested_counts(path_types),
        "p1_rule_candidates_by_class": {
            class_name: dict(sorted(counts.items()))
            for class_name, counts in sorted(p1_candidates.items())
        },
        "by_case": by_case_classification,
    }


def _select_component_path(
    evidence: list[object], decision: str
) -> dict[str, object] | None:
    cost_name = "include_cost" if decision == "accept" else "exclude_cost"
    candidates: list[tuple[int, tuple[object, ...], dict[str, object]]] = []
    for raw_evidence in evidence:
        if not isinstance(raw_evidence, dict) or raw_evidence.get("decision") != decision:
            continue
        decision_cost = raw_evidence.get(cost_name)
        paths = raw_evidence.get("paths")
        if not isinstance(decision_cost, int) or not isinstance(paths, list):
            continue
        includes_query = decision == "accept"
        matching_paths = [
            path
            for path in paths
            if isinstance(path, dict)
            and path.get("includes_query") is includes_query
            and isinstance(path.get("cost"), int)
        ]
        if not matching_paths:
            continue
        path = min(matching_paths, key=_component_path_sort_key)
        candidates.append((decision_cost, _component_path_sort_key(path), raw_evidence | {"selected_path": path}))
    return min(candidates, key=lambda item: (item[0], item[1]))[2] if candidates else None


def _component_path_sort_key(path: dict[str, object]) -> tuple[object, ...]:
    nodes = path.get("nodes")
    node_keys = tuple(_component_node_sort_key(node) for node in nodes or [])
    return (int(path["cost"]), node_keys)


def _component_node_sort_key(node: object) -> tuple[object, ...]:
    if not isinstance(node, dict):
        return (0, 0, "")
    span = node.get("original")
    if not isinstance(span, dict):
        span = {}
    return (
        int(span.get("byte_start", 0)),
        int(span.get("byte_end", 0)),
        str(node.get("pos") or ""),
    )


def _classify_component_path(
    evidence: dict[str, object], decision: str
) -> dict[str, object]:
    path = evidence["selected_path"]
    nodes = path["nodes"]
    target = evidence["target"]
    window = evidence["window"]
    raw_window = window["raw"]
    target_span = (int(target["byte_start"]), int(target["byte_end"]))
    node_records = [node for node in nodes if isinstance(node, dict)]
    query_nodes = [
        node for node in node_records if _node_span(node) == target_span
    ] if decision == "accept" else []
    companion_nodes = [node for node in node_records if node not in query_nodes]
    pos_sequence = [str(node.get("pos") or "unknown") for node in node_records]
    components = [
        component
        for node in companion_nodes
        for component in str(node.get("pos") or "").split("+")
        if component
    ]
    has_unknown = any(bool(node.get("unknown")) for node in node_records)
    if decision == "accept":
        path_type = _accept_path_type(components, companion_nodes)
        p1_candidate = (
            path_type
            if path_type in {"numeric-unit", "derivational-continuation"}
            else None
        )
    else:
        path_type = _reject_path_type(components, has_unknown)
        p1_candidate = None
    return {
        "path_type": path_type,
        "p1_rule_candidate": p1_candidate,
        "target_position": _target_position(target_span, raw_window),
        "cost": int(path["cost"]),
        "pos_sequence": pos_sequence,
        "has_unknown": has_unknown,
    }


def _node_span(node: dict[str, object]) -> tuple[int, int] | None:
    span = node.get("original")
    if not isinstance(span, dict):
        return None
    return (int(span["byte_start"]), int(span["byte_end"]))


def _target_position(
    target_span: tuple[int, int], raw_window: dict[str, Any]
) -> str:
    window_start = int(raw_window["byte_start"])
    window_end = int(raw_window["byte_end"])
    starts = target_span[0] == window_start
    ends = target_span[1] == window_end
    if starts and ends:
        return "exact"
    if starts:
        return "prefix"
    if ends:
        return "suffix"
    return "internal"


def _accept_path_type(
    components: list[str], companion_nodes: list[dict[str, object]]
) -> str:
    if not companion_nodes:
        return "exact-token"
    if any(component in {"SN", "NR"} for component in components):
        return "numeric-unit"
    if any(component in {"XSV", "XSA"} for component in components):
        return "derivational-continuation"
    if any(component in {"VCP", "VCN"} for component in components):
        return "copular-continuation"
    if any(_is_nominal(component) for component in components):
        return "nominal-compound"
    if components and all(component.startswith("J") for component in components):
        return "particle-continuation"
    return "mixed"


def _reject_path_type(components: list[str], has_unknown: bool) -> str:
    if has_unknown:
        return "unknown"
    if any(
        component.startswith(("V", "E"))
        or component in {"XSV", "XSA", "VCP", "VCN"}
        for component in components
    ):
        return "predicate"
    if components and all(
        _is_nominal(component) or component.startswith("J")
        for component in components
    ):
        return "nominal"
    return "mixed"


def _is_nominal(component: str) -> bool:
    return component.startswith("N") or component in {"SN", "XPN", "XSN"}


def _sorted_nested_counts(
    counts: dict[str, dict[str, dict[str, int]]],
) -> dict[str, dict[str, dict[str, int]]]:
    return {
        class_name: {
            decision: dict(sorted(path_counts.items()))
            for decision, path_counts in sorted(decisions.items())
        }
        for class_name, decisions in sorted(counts.items())
    }


def append_shadow_verification(
    lines: list[str], report: dict[str, object]
) -> None:
    lines.extend(
        [
            "",
            "## Structural verification",
            "",
            "Product structural counters and the separate full-lattice diagnostic are collected outside timed evaluation.",
            "",
            "| profile | raw anchor hits | verified program hits | structural candidates | structural windows | cases with structural candidates |",
            "| --- | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for profile in KFIND_PROFILES:
        summary = report["shadow_verification"][profile]
        totals = summary["totals"]
        lines.append(
            f"| {profile} | {totals['raw_anchor_hits']} | "
            f"{totals['verified_program_hits']} | "
            f"{totals['structural_candidate_hits']} | "
            f"{totals['unique_structural_windows']} | "
            f"{summary['cases_with_structural_candidates']} |"
        )
        lattice_statuses = ", ".join(
            f"{name}={count}"
            for name, count in summary["lattice_statuses"].items()
        ) or "none"
        lattice_decisions = ", ".join(
            f"{name}={count}"
            for name, count in summary["lattice_decisions"].items()
        ) or "none"
        lines.append(
            f"- {profile} diagnostic lattice: statuses {lattice_statuses}; "
            f"decisions {lattice_decisions}"
        )


def append_structural_shadow_table(
    lines: list[str], shadow_verification: dict[str, object]
) -> None:
    lines.extend(
        [
            "",
            "| profile | structural candidate cases | lattice accept cases | lattice reject cases |",
            "| --- | ---: | ---: | ---: |",
        ]
    )
    for profile in KFIND_PROFILES:
        summary = shadow_verification[profile]
        decisions = summary["lattice_cases_by_decision"]
        lines.append(
            f"| {profile} | {summary['cases_with_structural_candidates']} | "
            f"{decisions.get('accept', 0)} | {decisions.get('reject', 0)} |"
        )
    lines.extend(
        [
            "",
            "| profile | class | decision | path type | cases |",
            "| --- | --- | --- | --- | ---: |",
        ]
    )
    for profile in KFIND_PROFILES:
        grouped = shadow_verification[profile].get(
            "lattice_path_classification", {}
        ).get("path_types_by_class", {})
        for class_name, decisions in grouped.items():
            for decision, path_types in decisions.items():
                for path_type, count in path_types.items():
                    lines.append(
                        f"| {profile} | {class_name} | {decision} | "
                        f"{path_type} | {count} |"
                    )
