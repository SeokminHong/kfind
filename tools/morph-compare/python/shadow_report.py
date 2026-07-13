from __future__ import annotations

from collections import defaultdict


KFIND_PROFILES = ("kfind-embedded", "kfind-full-pos")
SHADOW_COUNTERS = (
    "raw_anchor_hits",
    "verified_branch_hits",
    "local_lattice_candidate_hits",
    "unique_analysis_windows",
    "nominal_component_candidate_hits",
    "unique_component_windows",
)


def shadow_verification_summary(
    by_case: dict[str, dict[str, object]],
    cases: list[dict[str, object]] | None = None,
) -> dict[str, object]:
    totals = {
        name: sum(int(counters[name]) for counters in by_case.values())
        for name in SHADOW_COUNTERS
    }
    statuses: dict[str, int] = defaultdict(int)
    decisions: dict[str, int] = defaultdict(int)
    component_statuses: dict[str, int] = defaultdict(int)
    component_decisions: dict[str, int] = defaultdict(int)
    component_cases_by_decision: dict[str, int] = defaultdict(int)
    case_metadata = {str(case["id"]): case for case in cases or []}
    outcomes_by_class: dict[str, dict[str, int]] = defaultdict(
        lambda: defaultdict(int)
    )
    outcomes_by_target_group: dict[str, dict[str, int]] = defaultdict(
        lambda: defaultdict(int)
    )
    component_outcomes_by_class: dict[str, dict[str, int]] = defaultdict(
        lambda: defaultdict(int)
    )
    for case_id, counters in by_case.items():
        case = case_metadata.get(case_id)
        for evidence in counters.get("lattice", []):
            status = str(evidence["status"])
            decision = evidence.get("decision")
            outcome = str(decision) if decision is not None else status
            statuses[status] += 1
            if decision is not None:
                decisions[str(decision)] += 1
            if case is not None:
                class_name = "positive" if bool(case["expected"]) else "negative"
                outcomes_by_class[class_name][outcome] += 1
                target_group = case.get("target_group")
                if target_group is not None:
                    outcomes_by_target_group[str(target_group)][outcome] += 1
        for evidence in counters.get("component", []):
            status = str(evidence["status"])
            decision = evidence.get("decision")
            outcome = str(decision) if decision is not None else status
            component_statuses[status] += 1
            if decision is not None:
                component_decisions[str(decision)] += 1
            if case is not None:
                class_name = "positive" if bool(case["expected"]) else "negative"
                component_outcomes_by_class[class_name][outcome] += 1
        case_decisions = {
            str(evidence["decision"])
            for evidence in counters.get("component", [])
            if evidence.get("decision") is not None
        }
        for decision in case_decisions:
            component_cases_by_decision[decision] += 1

    def sorted_outcomes(
        grouped: dict[str, dict[str, int]],
    ) -> dict[str, dict[str, int]]:
        return {
            group: dict(sorted(counts.items()))
            for group, counts in sorted(grouped.items())
        }

    return {
        "totals": totals,
        "cases_with_local_candidates": sum(
            counters["local_lattice_candidate_hits"] > 0
            for counters in by_case.values()
        ),
        "cases_with_component_candidates": sum(
            counters["nominal_component_candidate_hits"] > 0
            for counters in by_case.values()
        ),
        "lattice_statuses": dict(sorted(statuses.items())),
        "lattice_decisions": dict(sorted(decisions.items())),
        "lattice_outcomes_by_class": sorted_outcomes(outcomes_by_class),
        "lattice_outcomes_by_target_group": sorted_outcomes(
            outcomes_by_target_group
        ),
        "component_statuses": dict(sorted(component_statuses.items())),
        "component_decisions": dict(sorted(component_decisions.items())),
        "component_cases_by_decision": dict(
            sorted(component_cases_by_decision.items())
        ),
        "component_outcomes_by_class": sorted_outcomes(
            component_outcomes_by_class
        ),
        "by_case": by_case,
    }


def append_shadow_verification(
    lines: list[str], report: dict[str, object]
) -> None:
    lines.extend(
        [
            "",
            "## Shadow verification",
            "",
            "Counters are collected outside the timed evaluation and do not change matches.",
            "",
            "| profile | raw anchor hits | verified branch hits | lattice candidates | lattice windows | component candidates | component windows | cases with component candidates |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for profile in KFIND_PROFILES:
        summary = report["shadow_verification"][profile]
        totals = summary["totals"]
        lines.append(
            f"| {profile} | {totals['raw_anchor_hits']} | "
            f"{totals['verified_branch_hits']} | "
            f"{totals['local_lattice_candidate_hits']} | "
            f"{totals['unique_analysis_windows']} | "
            f"{totals['nominal_component_candidate_hits']} | "
            f"{totals['unique_component_windows']} | "
            f"{summary['cases_with_component_candidates']} |"
        )
        statuses = ", ".join(
            f"{name}={count}"
            for name, count in summary["lattice_statuses"].items()
        ) or "none"
        decisions = ", ".join(
            f"{name}={count}"
            for name, count in summary["lattice_decisions"].items()
        ) or "none"
        lines.append(f"- {profile}: statuses {statuses}; decisions {decisions}")
        component_statuses = ", ".join(
            f"{name}={count}"
            for name, count in summary["component_statuses"].items()
        ) or "none"
        component_decisions = ", ".join(
            f"{name}={count}"
            for name, count in summary["component_decisions"].items()
        ) or "none"
        lines.append(
            f"- {profile} component: statuses {component_statuses}; "
            f"decisions {component_decisions}"
        )


def append_component_shadow_table(
    lines: list[str], shadow_verification: dict[str, object]
) -> None:
    lines.extend(
        [
            "",
            "| profile | component candidate cases | accept cases | reject cases |",
            "| --- | ---: | ---: | ---: |",
        ]
    )
    for profile in KFIND_PROFILES:
        summary = shadow_verification[profile]
        decisions = summary["component_cases_by_decision"]
        lines.append(
            f"| {profile} | {summary['cases_with_component_candidates']} | "
            f"{decisions.get('accept', 0)} | {decisions.get('reject', 0)} |"
        )
