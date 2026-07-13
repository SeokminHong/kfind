from __future__ import annotations

import os
import platform
from collections import defaultdict


KFIND_PROFILES = ("kfind-embedded", "kfind-full-pos")
BACKENDS = (*KFIND_PROFILES, "kiwi", "lindera")
SHADOW_COUNTERS = (
    "raw_anchor_hits",
    "verified_branch_hits",
    "local_lattice_candidate_hits",
    "unique_analysis_windows",
    "nominal_component_candidate_hits",
    "unique_component_windows",
)


def quality_metrics(
    cases: list[dict[str, object]], predictions: dict[str, bool]
) -> dict[str, object]:
    tp = fp = tn = fn = 0
    for case in cases:
        expected = bool(case["expected"])
        predicted = predictions[case["id"]]
        if expected and predicted:
            tp += 1
        elif expected:
            fn += 1
        elif predicted:
            fp += 1
        else:
            tn += 1
    precision = tp / (tp + fp) if tp + fp else 0.0
    recall = tp / (tp + fn) if tp + fn else 0.0
    negative_precision = tn / (tn + fp) if tn + fp else 0.0
    f1 = 2 * precision * recall / (precision + recall) if precision + recall else 0.0
    return {
        "cases": len(cases),
        "tp": tp,
        "fp": fp,
        "tn": tn,
        "fn": fn,
        "accuracy_percent": round(100 * (tp + tn) / len(cases), 2),
        "precision_percent": round(100 * precision, 2),
        "hard_negative_precision_percent": round(100 * negative_precision, 2),
        "recall_percent": round(100 * recall, 2),
        "f1_percent": round(100 * f1, 2),
    }


def grouped_quality(
    cases: list[dict[str, object]], predictions: dict[str, bool], key: str
) -> dict[str, dict[str, object]]:
    groups: dict[str, list[dict[str, object]]] = defaultdict(list)
    for case in cases:
        groups[str(case[key])].append(case)
    return {
        name: quality_metrics(group_cases, predictions)
        for name, group_cases in sorted(groups.items())
    }


def kfind_profile_comparison(
    cases: list[dict[str, object]],
    predictions: dict[str, dict[str, bool]],
    matches: dict[str, dict[str, list[dict[str, object]]]],
) -> dict[str, list[dict[str, object]]]:
    recovered = []
    still_failing = []
    regressed = []
    for case in cases:
        if not case["expected"]:
            continue
        record = {
            "case": case,
            "matching_spans": {
                profile: matches[profile][case["id"]] for profile in KFIND_PROFILES
            },
        }
        embedded_prediction = predictions["kfind-embedded"][case["id"]]
        full_pos_prediction = predictions["kfind-full-pos"][case["id"]]
        if not embedded_prediction and full_pos_prediction:
            recovered.append(record)
        elif not embedded_prediction:
            still_failing.append(record)
        elif not full_pos_prediction:
            regressed.append(record)
    return {
        "recovered_with_full_pos": recovered,
        "still_failing_with_full_pos": still_failing,
        "regressed_with_full_pos": regressed,
    }


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


def build_report(
    cases: list[dict[str, object]],
    metadata: dict[str, object],
    versions: dict[str, dict[str, object]],
    predictions: dict[str, dict[str, bool]],
    matches: dict[str, dict[str, list[dict[str, object]]]],
    performance_metrics: dict[str, dict[str, object]],
    kfind_diagnostics: dict[str, dict[str, dict[str, object] | None]],
    shadow_verification: dict[str, dict[str, dict[str, object]]],
    include_performance: bool = True,
) -> dict[str, object]:
    quality = {}
    has_slices = all("slice" in case for case in cases)
    for backend in BACKENDS:
        backend_quality = {
            "overall": quality_metrics(cases, predictions[backend]),
            "by_source": grouped_quality(cases, predictions[backend], "source"),
            "by_pos": grouped_quality(cases, predictions[backend], "pos"),
        }
        if has_slices:
            backend_quality["by_slice"] = grouped_quality(
                cases, predictions[backend], "slice"
            )
        if all("target_raw_tag" in case for case in cases):
            backend_quality["by_target_raw_tag"] = grouped_quality(
                cases, predictions[backend], "target_raw_tag"
            )
        if all("target_group" in case for case in cases):
            backend_quality["by_target_group"] = grouped_quality(
                cases, predictions[backend], "target_group"
            )
        quality[backend] = backend_quality
    failures = []
    for case in cases:
        backend_predictions = {
            backend: predictions[backend][case["id"]] for backend in BACKENDS
        }
        if all(value == case["expected"] for value in backend_predictions.values()):
            continue
        profile_causes = {
            profile: classify_primary_cause(
                case,
                backend_predictions,
                profile,
                matches[profile][case["id"]],
                kfind_diagnostics[profile][case["id"]],
            )
            for profile in KFIND_PROFILES
        }
        profile_cause_evidence = {
            profile: kfind_diagnostics[profile][case["id"]]
            for profile in KFIND_PROFILES
        }
        failures.append(
            {
                "case": case,
                "predictions": backend_predictions,
                "primary_cause": profile_causes["kfind-embedded"],
                "cause_evidence": profile_cause_evidence["kfind-embedded"],
                "profile_causes": profile_causes,
                "profile_cause_evidence": profile_cause_evidence,
                "matching_spans": {
                    backend: matches[backend][case["id"]] for backend in BACKENDS
                },
            }
        )
    return {
        "schema_version": 6,
        "task": "sentence lemma/POS presence with positive gold-span overlap",
        "dataset": metadata,
        "versions": versions,
        "environment": environment_metadata(),
        "quality": quality,
        "performance": performance_metrics if include_performance else None,
        "kfind_profile_comparison": kfind_profile_comparison(
            cases, predictions, matches
        ),
        "shadow_verification": {
            profile: shadow_verification_summary(
                shadow_verification[profile], cases
            )
            for profile in KFIND_PROFILES
        },
        "failures": failures,
        "adapter_errors": [],
    }


def classify_primary_cause(
    case: dict[str, object],
    predictions: dict[str, bool],
    profile: str,
    profile_spans: list[dict[str, object]],
    diagnostic: dict[str, object] | None,
) -> str | None:
    if profile not in KFIND_PROFILES:
        raise ValueError(f"unknown kfind profile {profile}")
    if not case["expected"] or predictions[profile]:
        return None
    if not predictions["kiwi"] and not predictions["lindera"]:
        return "gold-or-adapter"
    if diagnostic is None:
        raise ValueError(f"missing kfind diagnostic for positive case {case['id']}")
    if not diagnostic["auto_has_expected_pos_analysis"]:
        return "lexicon-missing"
    if profile_spans:
        return "span-mismatch"
    if diagnostic["any_boundary_gold_overlap"]:
        return "boundary-rejected"
    if diagnostic["gold_anchor_overlap"]:
        return "continuation-rejected"
    return "surface-missing"


def environment_metadata() -> dict[str, object]:
    memory_kib = None
    try:
        with open("/proc/meminfo", encoding="utf-8") as meminfo:
            total = next(
                (line for line in meminfo if line.startswith("MemTotal:")), None
            )
        if total is not None:
            memory_kib = int(total.split()[1])
    except FileNotFoundError:
        pass
    return {
        "platform": platform.platform(),
        "machine": platform.machine(),
        "logical_cpu_count": os.cpu_count(),
        "memory_kib": memory_kib,
        "python": platform.python_version(),
    }


def render_markdown(report: dict[str, object]) -> str:
    dataset = report["dataset"]
    lines = [
        "# kfind profiles / Kiwi / Lindera held-out morphology benchmark",
        "",
        f"- fixture: `{dataset['fixture_sha256']}`",
        f"- cases: {dataset['cases']} ({dataset['positive_cases']} positive, "
        f"{dataset['negative_cases']} negative)",
        f"- seed: `{dataset['seed']}`",
        f"- UD release: {dataset['ud_release']}",
        f"- environment: `{report['environment']['platform']}` / "
        f"{report['environment']['logical_cpu_count']} logical CPUs",
        "",
        "## Sources",
        "",
        "| source | license | SHA-256 |",
        "| --- | --- | --- |",
    ]
    for source in dataset["sources"]:
        lines.append(
            f"| [{source['name']}]({source['data_url']}) | {source['license']} | "
            f"`{source['data_sha256']}` |"
        )
    lines.extend(
        [
            "",
            "## Versions and profiles",
            "",
            "| result | backend | version | profile | lexicon SHA-256 | morphology SHA-256 |",
            "| --- | --- | --- | --- | --- | --- |",
        ]
    )
    for result_name in BACKENDS:
        version = report["versions"][result_name]
        artifact = version["lexicon_artifact_sha256"] or "n/a"
        morphology = version.get("morphology_artifact_sha256") or "n/a"
        lines.append(
            f"| {result_name} | {version['backend']} | {version['version']} | "
            f"{version['profile'] or 'n/a'} | `{artifact}` | `{morphology}` |"
        )
    append_quality_sections(lines, report)
    append_shadow_verification(lines, report)
    append_profile_comparison(lines, report)
    append_failures(lines, report)
    append_development_summary(lines, report.get("development"))
    append_local_context_summary(lines, report.get("local_context"))
    append_hard_negative_summary(lines, report.get("hard_negatives"))
    lines.extend(
        [
            "",
            "Performance measures each backend's end-to-end search path after one initialization; "
            "it is not a tokenizer-only throughput comparison.",
        ]
    )
    return "\n".join(lines) + "\n"


def append_quality_sections(lines: list[str], report: dict[str, object]) -> None:
    lines.extend(
        [
            "",
            "## Overall quality",
            "",
            "| backend | accuracy | precision | recall | F1 | TP | FP | TN | FN |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for backend in BACKENDS:
        metrics = report["quality"][backend]["overall"]
        lines.append(
            f"| {backend} | {metrics['accuracy_percent']}% | {metrics['precision_percent']}% | "
            f"{metrics['recall_percent']}% | {metrics['f1_percent']}% | {metrics['tp']} | "
            f"{metrics['fp']} | {metrics['tn']} | {metrics['fn']} |"
        )
    append_performance(lines, report)
    append_grouped_quality(lines, report, "source", "by_source")
    append_grouped_quality(lines, report, "POS", "by_pos")


def append_performance(lines: list[str], report: dict[str, object]) -> None:
    lines.extend(
        [
            "",
            "## End-to-end performance",
            "",
            "| backend | runs | init median | cases/s median [min, max] | p95 median [min, max] | peak RSS median [min, max] |",
            "| --- | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for backend in BACKENDS:
        metrics = report["performance"][backend]
        rss = metrics["peak_rss_kib"]
        rss_text = f"{rss / 1024:.1f} MiB" if rss is not None else "n/a"
        rss_min = metrics["run_min"]["peak_rss_kib"]
        rss_max = metrics["run_max"]["peak_rss_kib"]
        rss_range = (
            f"[{rss_min / 1024:.1f}, {rss_max / 1024:.1f}] MiB"
            if rss_min is not None and rss_max is not None
            else "n/a"
        )
        lines.append(
            f"| {backend} | {metrics['runs']} | {metrics['initialization_seconds']:.4f}s | "
            f"{metrics['cases_per_second']} "
            f"[{metrics['run_min']['cases_per_second']}, {metrics['run_max']['cases_per_second']}] | "
            f"{metrics['latency_p95_ms']}ms "
            f"[{metrics['run_min']['latency_p95_ms']}, {metrics['run_max']['latency_p95_ms']}] | "
            f"{rss_text} {rss_range} |"
        )


def append_grouped_quality(
    lines: list[str], report: dict[str, object], label: str, key: str
) -> None:
    lines.extend(
        [
            "",
            f"## Quality by {label}",
            "",
            f"| {label} | backend | accuracy | precision | recall | F1 |",
            "| --- | --- | ---: | ---: | ---: | ---: |",
        ]
    )
    groups = sorted(report["quality"]["kfind-embedded"][key])
    for group in groups:
        for backend in BACKENDS:
            metrics = report["quality"][backend][key][group]
            lines.append(
                f"| {group} | {backend} | {metrics['accuracy_percent']}% | "
                f"{metrics['precision_percent']}% | {metrics['recall_percent']}% | "
                f"{metrics['f1_percent']}% |"
            )


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


def append_profile_comparison(lines: list[str], report: dict[str, object]) -> None:
    comparison = report["kfind_profile_comparison"]
    lines.extend(
        [
            "",
            "## kfind full-POS recovery",
            "",
            f"- recovered: {len(comparison['recovered_with_full_pos'])}",
            f"- still failing: {len(comparison['still_failing_with_full_pos'])}",
            f"- regressed: {len(comparison['regressed_with_full_pos'])}",
        ]
    )


def append_failures(lines: list[str], report: dict[str, object]) -> None:
    lines.extend(
        [
            "",
            f"## Failures ({len(report['failures'])} cases)",
            "",
            "| case | source | query/POS | embedded cause | full-POS cause | expected | kfind embedded | kfind full-POS | Kiwi | Lindera |",
            "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |",
        ]
    )
    for failure in report["failures"][:30]:
        case = failure["case"]
        predicted = failure["predictions"]
        lines.append(
            f"| {case['id']} | {case['source']} | {case['query']}/{case['pos']} | "
            f"{failure['profile_causes']['kfind-embedded'] or 'n/a'} | "
            f"{failure['profile_causes']['kfind-full-pos'] or 'n/a'} | "
            f"{case['expected']} | "
            f"{predicted['kfind-embedded']} | "
            f"{predicted['kfind-full-pos']} | {predicted['kiwi']} | "
            f"{predicted['lindera']} |"
        )
    if len(report["failures"]) > 30:
        lines.extend(["", "The JSON report contains every failure and matching span."])


def append_development_summary(
    lines: list[str], development: dict[str, object] | None
) -> None:
    if development is None:
        return
    dataset = development["dataset"]
    lines.extend(
        [
            "",
            "## Development split",
            "",
            f"- fixture: `{dataset['fixture_sha256']}`",
            f"- cases: {dataset['cases']}",
            "",
            "| backend | precision | recall | F1 | TP | FP | FN |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for backend in BACKENDS:
        metrics = development["quality"][backend]["overall"]
        lines.append(
            f"| {backend} | {metrics['precision_percent']}% | "
            f"{metrics['recall_percent']}% | {metrics['f1_percent']}% | "
            f"{metrics['tp']} | {metrics['fp']} | {metrics['fn']} |"
        )
    append_component_shadow_table(lines, development["shadow_verification"])


def append_hard_negative_summary(
    lines: list[str], hard_negatives: dict[str, object] | None
) -> None:
    if hard_negatives is None:
        return
    lines.extend(
        [
            "",
            "## Hard negatives",
            "",
            f"- fixture: `{hard_negatives['dataset']['fixture_sha256']}`",
            f"- cases: {hard_negatives['dataset']['cases']}",
            "",
            "| slice | backend | hard-negative precision | FP | TN |",
            "| --- | --- | ---: | ---: | ---: |",
        ]
    )
    slices = sorted(hard_negatives["quality"]["kfind-embedded"]["by_slice"])
    for slice_name in slices:
        for backend in BACKENDS:
            metrics = hard_negatives["quality"][backend]["by_slice"][slice_name]
            lines.append(
                f"| {slice_name} | {backend} | "
                f"{metrics['hard_negative_precision_percent']}% | "
                f"{metrics['fp']} | {metrics['tn']} |"
            )
    append_component_shadow_table(lines, hard_negatives["shadow_verification"])


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


def append_local_context_summary(
    lines: list[str], local_context: dict[str, object] | None
) -> None:
    if local_context is None:
        return
    dataset = local_context["dataset"]
    lines.extend(
        [
            "",
            "## Copula local-context slice",
            "",
            f"- fixture: `{dataset['fixture_sha256']}`",
            f"- cases: {dataset['cases']} ({dataset['positive_cases']} positive, "
            f"{dataset['negative_cases']} negative)",
            "- excluded from performance measurements",
            "",
            "| backend | precision | recall | F1 | TP | FP | TN | FN |",
            "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    for backend in BACKENDS:
        metrics = local_context["quality"][backend]["overall"]
        lines.append(
            f"| {backend} | {metrics['precision_percent']}% | "
            f"{metrics['recall_percent']}% | {metrics['f1_percent']}% | "
            f"{metrics['tp']} | {metrics['fp']} | {metrics['tn']} | "
            f"{metrics['fn']} |"
        )

    lines.extend(
        [
            "",
            "| source/raw tag | backend | precision | recall | TP | FP | TN | FN |",
            "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |",
        ]
    )
    target_groups = sorted(
        local_context["quality"]["kfind-embedded"]["by_target_group"]
    )
    for target_group in target_groups:
        for backend in BACKENDS:
            metrics = local_context["quality"][backend]["by_target_group"][
                target_group
            ]
            lines.append(
                f"| {target_group} | {backend} | "
                f"{metrics['precision_percent']}% | "
                f"{metrics['recall_percent']}% | {metrics['tp']} | "
                f"{metrics['fp']} | {metrics['tn']} | {metrics['fn']} |"
            )

    lines.extend(
        [
            "",
            "| profile | local candidates | analysis windows | cases with candidates |",
            "| --- | ---: | ---: | ---: |",
        ]
    )
    for profile in KFIND_PROFILES:
        shadow = local_context["shadow_verification"][profile]
        totals = shadow["totals"]
        lines.append(
            f"| {profile} | {totals['local_lattice_candidate_hits']} | "
            f"{totals['unique_analysis_windows']} | "
            f"{shadow['cases_with_local_candidates']} |"
        )

    lines.extend(
        [
            "",
            "| profile | class | accept | reject | ambiguous | other |",
            "| --- | --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for profile in KFIND_PROFILES:
        outcomes = local_context["shadow_verification"][profile].get(
            "lattice_outcomes_by_class", {}
        )
        for class_name in ("positive", "negative"):
            counts = outcomes.get(class_name, {})
            other = sum(
                count
                for outcome, count in counts.items()
                if outcome not in {"accept", "reject", "ambiguous"}
            )
            lines.append(
                f"| {profile} | {class_name} | {counts.get('accept', 0)} | "
                f"{counts.get('reject', 0)} | {counts.get('ambiguous', 0)} | "
                f"{other} |"
            )

    lines.extend(
        [
            "",
            "| profile | source/raw tag | accept | reject | ambiguous | other |",
            "| --- | --- | ---: | ---: | ---: | ---: |",
        ]
    )
    for profile in KFIND_PROFILES:
        outcomes = local_context["shadow_verification"][profile].get(
            "lattice_outcomes_by_target_group", {}
        )
        for target_group, counts in outcomes.items():
            other = sum(
                count
                for outcome, count in counts.items()
                if outcome not in {"accept", "reject", "ambiguous"}
            )
            lines.append(
                f"| {profile} | {target_group} | {counts.get('accept', 0)} | "
                f"{counts.get('reject', 0)} | {counts.get('ambiguous', 0)} | "
                f"{other} |"
            )
